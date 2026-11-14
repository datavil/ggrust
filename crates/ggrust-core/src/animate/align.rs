//! Aligning lines and polygons before they are tweened (transformr
//! `tween_path` / `tween_polygon`, MIT, Thomas Lin Pedersen), as gganimate
//! does for path and polygon layers: every group present in both states gets
//! the same number of points, at least [`MIN_POINTS`], by inserting points
//! along its longest segments (`add_points`, `find_splits_c`,
//! `insert_points_c`). Polygons are first drawn clockwise and the second ring
//! is rotated so the points travel the least (`as_clockwise`, `rotate_c`).
//! Inserted rows copy the other values of the vertex before them.
//!
//! It does not tween ([`super::tween`]). Not ported: multi-paths and holes
//! (rows with a missing `x` or `y` inside a group; such groups are matched
//! row by row) and the cutting of shapes when the number of shapes differs
//! (`match = FALSE` only; gganimate matches by group).

use std::collections::HashMap;

use crate::build::frame::LayerFrame;
use crate::data::{Column, ContinuousColumn};
use crate::spec::Aesthetic;
use crate::{Error, Result};

/// transformr's `min_n`: every aligned shape has at least this many points.
pub(crate) const MIN_POINTS: usize = 50;

/// How a layer's rows move between states (gganimate `layer_type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shape {
    /// Rows move one by one (`tween_state`).
    Points,
    /// Each group is an open line (`tween_path`); `sort_by_x` when the geom
    /// draws its rows sorted by x (`geom_line`, as ggplot2's `GeomLine` sorts).
    Paths { sort_by_x: bool },
    /// Each group is a closed ring (`tween_polygon`).
    Polygons,
}

impl Shape {
    /// The shape of geom `name`: gganimate's `GeomPath` and `GeomPolygon`
    /// subclasses, points otherwise.
    pub(crate) fn of(name: &str) -> Shape {
        match name {
            "path" | "contour" | "density2d" | "quantile" => Shape::Paths { sort_by_x: false },
            "line" | "step" | "freqpoly" => Shape::Paths { sort_by_x: true },
            "polygon" => Shape::Polygons,
            _ => Shape::Points,
        }
    }
}

/// The columns interpolated along a segment for inserted points (transformr
/// interpolates `x`, `y` and, when they vary, `xmin`/`xmax`/`ymin`/`ymax`).
const MOVED: [Aesthetic; 6] = [
    Aesthetic::X,
    Aesthetic::Y,
    Aesthetic::Xmin,
    Aesthetic::Xmax,
    Aesthetic::Ymin,
    Aesthetic::Ymax,
];

/// `from` and `to` with every group present in both aligned (see the module
/// docs); `None` for [`Shape::Points`] or when no group needs a change.
///
/// # Errors
///
/// An internal error when a rebuilt column does not fit the frame.
pub(crate) fn align(
    from: &LayerFrame,
    to: &LayerFrame,
    shape: Shape,
) -> Result<Option<(LayerFrame, LayerFrame)>> {
    align_to(from, to, shape, MIN_POINTS)
}

/// [`align`] with at least `min_points` points per shape.
fn align_to(
    from: &LayerFrame,
    to: &LayerFrame,
    shape: Shape,
    min_points: usize,
) -> Result<Option<(LayerFrame, LayerFrame)>> {
    let closed = match shape {
        Shape::Points => return Ok(None),
        Shape::Paths { .. } => false,
        Shape::Polygons => true,
    };
    let sort_by_x = matches!(shape, Shape::Paths { sort_by_x: true });
    let from_groups = rings(from, sort_by_x);
    let to_groups = rings(to, sort_by_x);
    let mut from_aligned: HashMap<u32, Ring> = HashMap::new();
    let mut to_aligned: HashMap<u32, Ring> = HashMap::new();
    for (group, from_ring) in &from_groups.rings {
        let Some(to_ring) = to_groups.rings.get(group) else {
            continue;
        };
        let (Some(mut from_ring), Some(mut to_ring)) = (from_ring.clone(), to_ring.clone()) else {
            continue;
        };
        if closed {
            from_ring.prepare_polygon();
            to_ring.prepare_polygon();
        } else {
            from_ring.prepare_path();
            to_ring.prepare_path();
        }
        let count = min_points.max(from_ring.len()).max(to_ring.len());
        let from_ring = from_ring.add_points_to(count, closed);
        let mut to_ring = to_ring.add_points_to(count, closed);
        if closed {
            to_ring.rotate_to(&from_ring);
        }
        from_aligned.insert(*group, from_ring);
        to_aligned.insert(*group, to_ring);
    }
    if from_aligned.is_empty() {
        return Ok(None);
    }
    Ok(Some((
        rebuild(from, &from_groups.order, &from_aligned)?,
        rebuild(to, &to_groups.order, &to_aligned)?,
    )))
}

/// The rows of one group as a shape: row indices into the frame and the
/// values of the [`MOVED`] columns (`None` for a column the frame lacks).
#[derive(Debug, Clone)]
struct Ring {
    rows: Vec<u32>,
    values: [Option<Vec<f64>>; 6],
}

/// The groups of a frame, in order of first row.
struct Groups {
    /// `None` for a group that cannot be aligned (a missing `x` or `y`).
    rings: HashMap<u32, Option<Ring>>,
    /// The groups in order of their first row.
    order: Vec<u32>,
}

fn rings(frame: &LayerFrame, sort_by_x: bool) -> Groups {
    let columns: [Option<&ContinuousColumn>; 6] =
        MOVED.map(|aesthetic| frame.data.continuous(aesthetic));
    let mut rows_by_group: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut order = Vec::new();
    for row in 0..frame.data.row_count() {
        let group = frame.data.group_of(row);
        let rows = rows_by_group.entry(group).or_default();
        if rows.is_empty() {
            order.push(group);
        }
        rows.push(row as u32);
    }
    let rings = rows_by_group
        .into_iter()
        .map(|(group, mut rows)| {
            let finite = |column: Option<&ContinuousColumn>, row: u32| {
                column
                    .and_then(|column| column.get(row as usize))
                    .filter(|value| value.is_finite())
            };
            let (x, y) = (columns[0], columns[1]);
            if rows
                .iter()
                .any(|row| finite(x, *row).is_none() || finite(y, *row).is_none())
            {
                return (group, None);
            }
            if sort_by_x {
                rows.sort_by(|left, right| {
                    let key = |row: u32| finite(x, row).unwrap_or(f64::NAN);
                    key(*left).total_cmp(&key(*right))
                });
            }
            let values = columns.map(|column| {
                column.map(|column| {
                    rows.iter()
                        .map(|row| column.get(*row as usize).unwrap_or(f64::NAN))
                        .collect()
                })
            });
            (group, Some(Ring { rows, values }))
        })
        .collect();
    Groups { rings, order }
}

impl Ring {
    fn len(&self) -> usize {
        self.rows.len()
    }

    fn x(&self) -> &[f64] {
        self.values[0].as_deref().unwrap_or(&[])
    }

    fn y(&self) -> &[f64] {
        self.values[1].as_deref().unwrap_or(&[])
    }

    /// Keeps the rows at `indices` (repeats allowed).
    fn select(&mut self, indices: &[usize]) {
        self.rows = indices.iter().map(|index| self.rows[*index]).collect();
        for values in self.values.iter_mut().flatten() {
            *values = indices.iter().map(|index| values[*index]).collect();
        }
    }

    /// transformr `to_path`: a one-point line is that point twice.
    fn prepare_path(&mut self) {
        if self.len() == 1 {
            self.select(&[0, 0]);
        }
    }

    /// transformr `make_polygons` (rings under 4 points repeat their first
    /// point up to 4) and `as_clockwise` (reversed when the shoelace sum
    /// `sum((x[i+1] - x[i]) * (y[i+1] + y[i]))` of the closed ring is negative).
    fn prepare_polygon(&mut self) {
        let count = self.len();
        if count > 0 && count < 4 {
            let indices: Vec<usize> = (0..count)
                .chain(std::iter::repeat_n(0, 4 - count))
                .collect();
            self.select(&indices);
        }
        let (x, y) = (self.x(), self.y());
        let count = x.len();
        let sum: f64 = (0..count)
            .map(|index| {
                let next = (index + 1) % count;
                (x[next] - x[index]) * (y[next] + y[index])
            })
            .sum();
        if sum < 0.0 {
            let reversed: Vec<usize> = (0..count).rev().collect();
            self.select(&reversed);
        }
    }

    /// transformr `add_points`: `target - len` more points along the longest segments
    /// (`closed`: the segment from the last point back to the first counts).
    fn add_points_to(mut self, target: usize, closed: bool) -> Ring {
        let n = target.saturating_sub(self.len());
        if n == 0 {
            return self;
        }
        let count = self.len();
        if count == 1 {
            self.select(&vec![0; n + 1]);
            return self;
        }
        let (x, y) = (self.x(), self.y());
        let segment_count = if closed { count } else { count - 1 };
        let lengths: Vec<f64> = (0..segment_count)
            .map(|index| {
                let next = (index + 1) % count;
                let (dx, dy) = (x[next] - x[index], y[next] - y[index]);
                (dx * dx + dy * dy).sqrt()
            })
            .collect();
        // R `order(l, decreasing = TRUE)`: ties keep their order.
        let mut longest: Vec<usize> = (0..segment_count).collect();
        longest.sort_by(|left, right| lengths[*right].total_cmp(&lengths[*left]));
        let sorted: Vec<f64> = longest.iter().map(|index| lengths[*index]).collect();
        let sorted_splits = find_splits(&sorted, n);
        let mut splits = vec![0usize; count];
        for (rank, segment) in longest.iter().enumerate() {
            splits[*segment] = sorted_splits[rank];
        }
        let indices: Vec<usize> = (0..count)
            .flat_map(|index| std::iter::repeat_n(index, splits[index] + 1))
            .collect();
        let inserted: [Option<Vec<f64>>; 6] = std::array::from_fn(|column| {
            let values = self.values[column].as_ref()?;
            // transformr moves xmin/xmax/ymin/ymax only when they vary.
            let varies = column < 2
                || values
                    .iter()
                    .any(|value| value.to_bits() != values[0].to_bits());
            Some(if varies {
                insert_points(values, &splits)
            } else {
                indices.iter().map(|index| values[*index]).collect()
            })
        });
        self.rows = indices.iter().map(|index| self.rows[*index]).collect();
        self.values = inserted;
        self
    }

    /// transformr `rotate_c`: starts this ring at the point that makes the sum
    /// of squared distances to `reference` (same length) the smallest.
    fn rotate_to(&mut self, reference: &Ring) {
        let offset = rotation(self.x(), self.y(), reference.x(), reference.y());
        if offset > 0 {
            let count = self.len();
            let indices: Vec<usize> = (0..count).map(|index| (index + offset) % count).collect();
            self.select(&indices);
        }
    }
}

/// transformr `find_splits_c`: how many of `n` new points go into each
/// segment, for `lengths` sorted longest first.
fn find_splits(lengths: &[f64], n: usize) -> Vec<usize> {
    let size = lengths.len();
    let mut splits = vec![0usize; size];
    if size == 0 {
        return splits;
    }
    let mut total = 0.0;
    let mut index = 0;
    while index < size - 1 {
        total += lengths[index];
        if total / (n + 1) as f64 > lengths[index + 1] {
            break;
        }
        index += 1;
    }
    if index == size - 1 {
        total += lengths[index];
    }
    let mut remaining = n;
    let mut segment = 0;
    while remaining > 0 {
        if segment >= size {
            splits[0] += remaining;
            break;
        }
        // C++ std::round: halves away from zero.
        let mut split = if total == 0.0 {
            remaining
        } else {
            (n as f64 * lengths[segment] / total).round() as usize
        };
        split = split.max(1).min(remaining);
        splits[segment] = split;
        remaining -= split;
        segment += 1;
    }
    splits
}

/// transformr `insert_points_c` for one column: after point `i`, `splits[i]`
/// points evenly along the segment to the next point (the first after the last).
fn insert_points(values: &[f64], splits: &[usize]) -> Vec<f64> {
    let count = values.len();
    let mut out = Vec::with_capacity(count + splits.iter().sum::<usize>());
    for (index, value) in values.iter().enumerate() {
        out.push(*value);
        if splits[index] == 0 {
            continue;
        }
        let next = if index == count - 1 { 0 } else { index + 1 };
        let step = (values[next] - value) / (splits[index] + 1) as f64;
        out.extend((1..=splits[index]).map(|part| value + step * part as f64));
    }
    out
}

/// transformr `rotate_c` (0-based): the shift `i` minimising
/// `sum_j |reference[j] - ring[(i + j) % n]|^2`, the first on ties.
fn rotation(x: &[f64], y: &[f64], reference_x: &[f64], reference_y: &[f64]) -> usize {
    let count = x.len().min(reference_x.len());
    let mut best = (f64::INFINITY, 0);
    for shift in 0..count {
        let sum: f64 = (0..count)
            .map(|index| {
                let other = (shift + index) % count;
                let (dx, dy) = (reference_x[index] - x[other], reference_y[index] - y[other]);
                dx * dx + dy * dy
            })
            .sum();
        if sum < best.0 {
            best = (sum, shift);
        }
    }
    best.1
}

/// `frame` with its rows ordered by group (groups in order of first row) and
/// each aligned group's rows replaced by its ring.
fn rebuild(frame: &LayerFrame, order: &[u32], aligned: &HashMap<u32, Ring>) -> Result<LayerFrame> {
    let mut rows: Vec<u32> = Vec::new();
    let mut moved: [Vec<f64>; 6] = Default::default();
    let columns = MOVED.map(|aesthetic| frame.data.continuous(aesthetic));
    let original = |column: usize, row: u32| {
        columns[column]
            .and_then(|values| values.get(row as usize))
            .unwrap_or(f64::NAN)
    };
    let mut group_rows: HashMap<u32, Vec<u32>> = HashMap::new();
    for row in 0..frame.data.row_count() {
        group_rows
            .entry(frame.data.group_of(row))
            .or_default()
            .push(row as u32);
    }
    let mut push = |row: u32, values: [f64; 6], rows: &mut Vec<u32>| {
        rows.push(row);
        for (column, value) in values.into_iter().enumerate() {
            moved[column].push(value);
        }
    };
    for group in order {
        match aligned.get(group) {
            Some(ring) => {
                for (index, row) in ring.rows.iter().enumerate() {
                    let values = std::array::from_fn(|column| {
                        ring.values[column]
                            .as_ref()
                            .map_or(f64::NAN, |values| values[index])
                    });
                    push(*row, values, &mut rows);
                }
            }
            None => {
                for row in group_rows.get(group).map_or(&[][..], Vec::as_slice) {
                    push(*row, std::array::from_fn(|c| original(c, *row)), &mut rows);
                }
            }
        }
    }
    let mut result = frame.take(&rows);
    for (column, aesthetic) in MOVED.iter().enumerate() {
        if columns[column].is_none() {
            continue;
        }
        let values = std::mem::take(&mut moved[column]);
        let options = values
            .into_iter()
            .map(|value| (!value.is_nan()).then_some(value))
            .collect();
        result
            .data
            .insert(
                *aesthetic,
                Column::Continuous(ContinuousColumn::from_options(options)),
            )
            .map_err(|error| Error::data(format!("internal: aligning shapes: {error}")))?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;
    use crate::animate::along::tests::reference;
    use crate::data::LayerData;
    use crate::style::Styles;

    fn numbers(value: &Value) -> Vec<f64> {
        value
            .as_array()
            .map(|items| items.iter().filter_map(Value::as_f64).collect())
            .unwrap_or_else(|| value.as_f64().into_iter().collect())
    }

    /// One group with `x`, `y` and a row number column (`Aesthetic::Size`).
    fn shape_frame(points: &Value) -> LayerFrame {
        let (x, y) = (numbers(&points["x"]), numbers(&points["y"]));
        let mut data = LayerData::new(x.len());
        let column = |values: Vec<f64>| Column::Continuous(ContinuousColumn::from_values(values));
        data.insert(Aesthetic::X, column(x)).unwrap();
        data.insert(Aesthetic::Y, column(y)).unwrap();
        LayerFrame {
            data,
            styles: Styles::default(),
        }
    }

    fn coordinates(frame: &LayerFrame) -> (Vec<f64>, Vec<f64>) {
        let values = |aesthetic| {
            let column = frame.data.continuous(aesthetic).unwrap();
            (0..column.len())
                .map(|row| column.get(row).unwrap())
                .collect()
        };
        (values(Aesthetic::X), values(Aesthetic::Y))
    }

    fn assert_close(actual: &[f64], expected: &[f64], what: &str) {
        assert_eq!(
            actual.len(),
            expected.len(),
            "{what}: {actual:?} vs {expected:?}"
        );
        for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            assert!(
                (actual - expected).abs() < 1e-12,
                "{what}[{index}]: {actual} vs {expected}"
            );
        }
    }

    #[test]
    fn shapes_align_as_transformr() {
        let reference = reference();
        let cases = reference["align"].as_array().unwrap();
        assert!(!cases.is_empty());
        for (index, case) in cases.iter().enumerate() {
            let shape = match case["kind"].as_str().unwrap() {
                "polygon" => Shape::Polygons,
                _ => Shape::Paths { sort_by_x: false },
            };
            let min_points = case["min_n"].as_u64().unwrap() as usize;
            let (from, to) = align_to(
                &shape_frame(&case["from"]),
                &shape_frame(&case["to"]),
                shape,
                min_points,
            )
            .unwrap()
            .unwrap();
            for (frame, key) in [(&from, "aligned_from"), (&to, "aligned_to")] {
                let (x, y) = coordinates(frame);
                assert_close(
                    &x,
                    &numbers(&case[key]["x"]),
                    &format!("case {index} {key} x"),
                );
                assert_close(
                    &y,
                    &numbers(&case[key]["y"]),
                    &format!("case {index} {key} y"),
                );
            }
        }
    }

    #[test]
    fn splits_follow_transformr() {
        assert_eq!(find_splits(&[4.0, 1.0, 1.0], 3), vec![2, 1, 0]);
        assert_eq!(find_splits(&[0.0, 0.0], 5), vec![5, 0]);
        assert_eq!(
            insert_points(&[0.0, 3.0, 4.0], &[2, 0, 0]),
            vec![0.0, 1.0, 2.0, 3.0, 4.0]
        );
        assert_eq!(
            rotation(&[1.0, 2.0, 0.0], &[0.0; 3], &[0.0, 1.0, 2.0], &[0.0; 3]),
            2
        );
    }

    #[test]
    fn inserted_rows_keep_their_vertex_and_other_groups() {
        // Group 0 changes from 2 to 3 points, group 1 is only in `from`.
        let mut from =
            shape_frame(&serde_json::json!({"x": [0.0, 1.0, 9.0], "y": [0.0, 0.0, 9.0]}));
        from.data.set_group_index(Some(vec![0, 0, 1])).unwrap();
        let mut to = shape_frame(&serde_json::json!({"x": [0.0, 1.0, 2.0], "y": [1.0, 1.0, 1.0]}));
        to.data.set_group_index(Some(vec![0, 0, 0])).unwrap();
        let (from, to) = align_to(&from, &to, Shape::Paths { sort_by_x: true }, 4)
            .unwrap()
            .unwrap();
        let (x, _) = coordinates(&from);
        assert_eq!(x, vec![0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0, 9.0]);
        assert_eq!(from.data.group_index().unwrap(), &[0, 0, 0, 0, 1]);
        assert_eq!(to.data.row_count(), 4);
        assert!(align(&from, &to, Shape::Points).unwrap().is_none());
    }
}
