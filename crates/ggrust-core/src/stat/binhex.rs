//! `stat="binhex"`: counts of (x, y) in a grid of hexagons, as lets-plot's
//! `BinHexStat` (`geom_hex`).
//!
//! Hexagons stand on a point (pointy top); rows are `bin height` apart and every
//! odd row is shifted right by half a bin. The grid spans every layer's x and y,
//! widened by half a bin on each side. Points are assigned as lets-plot does:
//! the rectangle-grid guess or one of its six neighbours, by a polygon test in a
//! space where the hexagons are regular; a point on a border goes to the lowest
//! row, then the rightmost hexagon. Options as `stat="bin2d"` (`bins`,
//! `binwidth`, `drop`); the y bin width is the hexagon height.
//!
//! Computed: `..count..` (fill by default), `..density..`, and `..binwidth..` /
//! `..binheight..` (hexagon width and row spacing), which size the hexagons.
//! lets-plot sizes them from the data resolution and pads the output with NaN
//! rows to keep it; ggrust reads the sizes instead (no padding rows).
//!
//! What it does not do: bins on a discrete axis (the codes are binned as numbers).

use super::bin2d::{
    Axis, AxisOptions, Bin2dOptions, CellCounts, applicable_range, density_factor, reported_cells,
};
use super::count::row_weight;
use super::groups::per_group;
use super::{Stat, StatContext};
use crate::Result;
use crate::data::{Computed, ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::scale::breaks::is_degenerate_range;
use crate::spec::Aesthetic;

/// lets-plot `HEIGHT_TO_BINHEIGHT`: row spacing per unit of hexagon `height`.
const HEIGHT_TO_BIN_HEIGHT: f64 = 0.866_025_403_784_438_7; // 3 / (2 sqrt 3), as Kotlin rounds it
/// lets-plot `BINHEIGHT_TO_HEIGHT`.
const BIN_HEIGHT_TO_HEIGHT: f64 = 1.154_700_538_379_251_5; // 2 sqrt 3 / 3, as Kotlin rounds it
/// lets-plot `BinHexStat.EPSILON`: distances this close (relatively) tie.
const EPSILON: f64 = 1e-4;

/// Counts (x, y) per hexagon.
pub struct BinHexStat;

impl Stat for BinHexStat {
    fn name(&self) -> &'static str {
        "binhex"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["bins", "binwidth", "drop"]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Weight]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        &[(Aesthetic::Fill, Computed::Count)]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        context: &StatContext,
    ) -> Result<LayerData> {
        let options = Bin2dOptions::from_parameters(parameters, HEIGHT_TO_BIN_HEIGHT)?;
        let empty = ContinuousColumn::from_values(Vec::new());
        let x = data.continuous(Aesthetic::X).unwrap_or(&empty);
        let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
        let weight = data.continuous(Aesthetic::Weight);
        let ranges = context
            .x_range
            .or_else(|| super::bin::finite_range(x))
            .zip(context.y_range.or_else(|| super::bin::finite_range(y)));
        per_group(&data, |rows| {
            let Some((x_range, y_range)) = ranges.filter(|_| data.row_count() > 0) else {
                return Ok(Vec::new());
            };
            let (x_axis, y_axis) = hexagon_axes(x_range, y_range, &options);
            Ok(count_hexagons(
                rows,
                x,
                y,
                weight,
                &x_axis,
                &y_axis,
                options.drop,
            ))
        })
    }
}

/// lets-plot `BinHexStat.apply`: the x bins and the y rows of the grid.
fn hexagon_axes(x_range: (f64, f64), y_range: (f64, f64), options: &Bin2dOptions) -> (Axis, Axis) {
    // Leave room for half a hexagon past each end of the range.
    let reserve = |span: f64, count: usize| span / count.saturating_sub(1).max(1) as f64;
    let by_count = |span: f64, count: usize| span / count as f64;
    let x_initial = applicable_range(x_range);
    let y_initial = applicable_range(y_range);
    let (x_count, x_width) = options
        .x
        .count_and_width(x_initial.1 - x_initial.0, reserve);
    let (y_count, y_width) = options
        .y
        .count_and_width(y_initial.1 - y_initial.0, reserve);
    // One column of several rows: all data must fit in half of one hexagon.
    let (x_width, x_shift) = if !options.x.has_width() && x_count == 1 && y_count > 1 {
        (x_width * 2.0, -0.75)
    } else {
        (x_width, 0.0)
    };
    let x_final = final_range(x_range, x_width, x_shift);
    let y_final = final_range(y_range, y_width, 0.0);
    let axis = |(low, high): (f64, f64), options: AxisOptions| {
        let (count, width) = options.count_and_width(high - low, by_count);
        Axis {
            start: low,
            count,
            width,
        }
    };
    (axis(x_final, options.x), axis(y_final, options.y))
}

/// lets-plot `BinHexStat.adjustRangeFinal`.
fn final_range((low, high): (f64, f64), width: f64, shift: f64) -> (f64, f64) {
    let offset = shift * width / 2.0;
    let low = low - width / 2.0 + offset;
    let high = high + width / 2.0 + offset;
    if is_degenerate_range(low, high) {
        (low - 0.5, high + 0.5)
    } else {
        (low, high)
    }
}

/// A hexagon grid in a space where the hexagons are regular (y scaled).
struct Grid {
    x_start: f64,
    y_start: f64,
    width: f64,
    height: f64,
}

impl Grid {
    /// lets-plot `getHexagonCenter`, used to place points. The odd-row shift is added
    /// to the column offset first, as lets-plot does: points on a border tie on it.
    fn center(&self, (column, row): (i64, i64)) -> (f64, f64) {
        let offset = if row % 2 == 0 {
            column as f64 * self.width
        } else {
            column as f64 * self.width + self.width / 2.0
        };
        (
            self.x_start + self.width / 2.0 + offset,
            self.y_start + self.height / 2.0 + row as f64 * self.height,
        )
    }

    /// The centre lets-plot `computeBins` reports (summed left to right).
    fn reported_center(&self, (column, row): (i64, i64)) -> (f64, f64) {
        let shift = if row % 2 == 0 { 0.0 } else { self.width / 2.0 };
        (
            self.x_start + self.width / 2.0 + column as f64 * self.width + shift,
            self.y_start + self.height / 2.0 + row as f64 * self.height,
        )
    }

    /// lets-plot `getCoarseGridIndex`: the cell as if the tiling were rectangles.
    fn coarse_index(&self, (x, y): (f64, f64)) -> (i64, i64) {
        let row = ((y - self.y_start) / self.height).floor() as i64;
        let start = self.x_start + (row % 2) as f64 * self.width / 2.0;
        (((x - start) / self.width).floor() as i64, row)
    }

    /// lets-plot `isPointInHexagon` (ray casting, `polygonContainsCoordinate`).
    fn contains(&self, (x, y): (f64, f64), hexagon: (i64, i64)) -> bool {
        let (center_x, center_y) = self.center(hexagon);
        let (qx, qy) = (x - center_x, y - center_y);
        let half_height = 2.0 * self.height / 3.0;
        let half_width = self.width / 2.0;
        let vertices = [
            (0.0, half_height),
            (half_width, half_height / 2.0),
            (half_width, -half_height / 2.0),
            (0.0, -half_height),
            (-half_width, -half_height / 2.0),
            (-half_width, half_height / 2.0),
            (0.0, half_height),
        ];
        let mut crossings = 0;
        for edge in vertices.windows(2) {
            let ((start_x, start_y), (end_x, end_y)) = (edge[0], edge[1]);
            if (start_y >= qy && end_y >= qy) || (start_y < qy && end_y < qy) {
                continue;
            }
            let crossing = start_x + (qy - start_y) * (end_x - start_x) / (end_y - start_y);
            if crossing <= qx {
                crossings += 1;
            }
        }
        crossings % 2 != 0
    }

    /// The hexagon of a point (lets-plot `computeCounts`).
    fn hexagon_of(&self, point: (f64, f64)) -> (i64, i64) {
        let (column, row) = self.coarse_index(point);
        let neighbours = if row % 2 == 0 {
            [
                (column, row),
                (column, row + 1),
                (column + 1, row),
                (column, row - 1),
                (column - 1, row - 1),
                (column - 1, row),
                (column - 1, row + 1),
            ]
        } else {
            [
                (column, row),
                (column + 1, row + 1),
                (column + 1, row),
                (column + 1, row - 1),
                (column, row - 1),
                (column - 1, row),
                (column, row + 1),
            ]
        };
        let candidates = neighbours
            .into_iter()
            .filter(|(column, row)| *column >= 0 && *row >= 0);
        if let Some(hexagon) = candidates
            .clone()
            .find(|hexagon| self.contains(point, *hexagon))
        {
            return hexagon;
        }
        // On a border: the nearest centres tie; take the lowest row, then the
        // rightmost hexagon.
        let distance = |hexagon: (i64, i64)| {
            let (center_x, center_y) = self.center(hexagon);
            (point.0 - center_x).hypot(point.1 - center_y)
        };
        let mut by_distance: Vec<((i64, i64), f64)> = candidates
            .map(|hexagon| (hexagon, distance(hexagon)))
            .collect();
        by_distance.sort_by(|a, b| a.1.total_cmp(&b.1));
        let nearest = by_distance.first().map_or(0.0, |(_, distance)| *distance);
        by_distance
            .iter()
            .take_while(|(_, distance)| (distance / nearest - 1.0).abs() < EPSILON)
            .map(|(hexagon, _)| *hexagon)
            .min_by_key(|(column, row)| (*row, -*column))
            .or_else(|| by_distance.first().map(|(hexagon, _)| *hexagon))
            .unwrap_or((column, row))
    }
}

/// lets-plot `BinHexStat.computeBins` for the rows of one group.
fn count_hexagons(
    rows: &[usize],
    x: &ContinuousColumn,
    y: &ContinuousColumn,
    weight: Option<&ContinuousColumn>,
    x_axis: &Axis,
    y_axis: &Axis,
    drop: bool,
) -> Vec<(Variable, Vec<f64>)> {
    // Hexagons are tested regular: y scaled by `ratio`, unscaled on output.
    let ratio = x_axis.width / (y_axis.width * BIN_HEIGHT_TO_HEIGHT);
    let grid = Grid {
        x_start: x_axis.start,
        y_start: y_axis.start * ratio,
        width: x_axis.width,
        height: y_axis.width * ratio,
    };
    let mut counts = CellCounts::new(x_axis.count, y_axis.count);
    let mut total = 0.0;
    for &row in rows {
        let (Some(x_value), Some(y_value)) = (
            x.get(row).filter(|value| value.is_finite()),
            y.get(row).filter(|value| value.is_finite()),
        ) else {
            continue;
        };
        let weight = row_weight(weight, row);
        total += weight;
        let (column, row_index) = grid.hexagon_of((x_value, y_value * ratio));
        // Hexagons past the grid count towards the total only, as lets-plot.
        if (column as u64) < x_axis.count as u64 && (row_index as u64) < y_axis.count as u64 {
            counts.add(column as usize, row_index as usize, weight);
        }
    }
    let factor = density_factor(x_axis, y_axis);
    let cells = reported_cells(&counts, x_axis.count, y_axis.count, drop, false);
    let mut xs = Vec::with_capacity(cells.len());
    let mut ys = Vec::with_capacity(cells.len());
    let mut count_values = Vec::with_capacity(cells.len());
    let mut densities = Vec::with_capacity(cells.len());
    for (column, row) in cells {
        let count = counts.get(column, row);
        let (center_x, center_y) = grid.reported_center((column as i64, row as i64));
        if drop && count == 0.0 {
            // lets-plot's "resolution hack": an empty corner hexagon becomes two
            // rows, one without y and one without x; each still trains its axis.
            xs.extend([center_x, f64::NAN]);
            ys.extend([f64::NAN, center_y / ratio]);
            count_values.extend([f64::NAN; 2]);
            densities.extend([f64::NAN; 2]);
            continue;
        }
        xs.push(center_x);
        ys.push(center_y / ratio);
        count_values.push(count);
        densities.push(count / total * factor);
    }
    let n = xs.len();
    vec![
        (Variable::Aesthetic(Aesthetic::X), xs),
        (Variable::Aesthetic(Aesthetic::Y), ys),
        (Variable::Computed(Computed::Count), count_values),
        (Variable::Computed(Computed::Density), densities),
        (
            Variable::Computed(Computed::BinWidth),
            vec![x_axis.width; n],
        ),
        (
            Variable::Computed(Computed::BinHeight),
            vec![y_axis.width; n],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Column;
    use crate::spec::Options;

    fn column(values: &[f64]) -> Column {
        Column::Continuous(ContinuousColumn::from_values(values.to_vec()))
    }

    fn values(data: &LayerData, variable: impl Into<Variable>) -> Vec<f64> {
        let column = data.continuous(variable).unwrap();
        (0..column.len())
            .map(|row| column.get(row).unwrap())
            .collect()
    }

    fn compute(x: &[f64], y: &[f64], options: &str) -> LayerData {
        let mut data = LayerData::new(x.len());
        data.insert(Aesthetic::X, column(x)).unwrap();
        data.insert(Aesthetic::Y, column(y)).unwrap();
        let options: Options = serde_json::from_str(options).unwrap();
        BinHexStat
            .compute(
                data,
                &Parameters::new(&options, "layers[0]"),
                &StatContext::default(),
            )
            .unwrap()
    }

    #[test]
    fn every_point_lands_in_its_nearest_hexagon() {
        let x: Vec<f64> = (0..500).map(|index| (index as f64 * 0.37).sin()).collect();
        let y: Vec<f64> = (0..500).map(|index| (index as f64 * 0.91).cos()).collect();
        let data = compute(&x, &y, r#"{"bins": [6, 5], "drop": false}"#);
        assert_eq!(values(&data, Computed::Count).iter().sum::<f64>(), 500.0);
        let centers_x = values(&data, Aesthetic::X);
        let centers_y = values(&data, Aesthetic::Y);
        let width = values(&data, Computed::BinWidth)[0];
        let height = values(&data, Computed::BinHeight)[0];
        // In the regular space, a hexagon is the set of points nearest its centre.
        let ratio = width / (height * BIN_HEIGHT_TO_HEIGHT);
        let mut expected = vec![0.0; centers_x.len()];
        for (px, py) in x.iter().zip(&y) {
            let nearest = (0..centers_x.len())
                .min_by(|a, b| {
                    let distance = |index: usize| {
                        (px - centers_x[index]).hypot((py - centers_y[index]) * ratio)
                    };
                    distance(*a).total_cmp(&distance(*b))
                })
                .unwrap();
            expected[nearest] += 1.0;
        }
        assert_eq!(values(&data, Computed::Count), expected);
    }

    #[test]
    fn an_empty_corner_hexagon_pads_two_rows() {
        // Hexagon (0, 0) is empty: lets-plot keeps its x and its y in two rows.
        let data = compute(&[1.0, 0.0, 1.0], &[0.0, 1.0, 1.0], r#"{"bins": [2, 2]}"#);
        let counts = values(&data, Computed::Count);
        let x = values(&data, Aesthetic::X);
        let y = values(&data, Aesthetic::Y);
        assert!(counts[0].is_nan() && counts[1].is_nan());
        assert!(x[0].is_finite() && y[0].is_nan());
        assert!(x[1].is_nan() && y[1].is_finite());
        let total: f64 = counts.iter().filter(|count| count.is_finite()).sum();
        assert_eq!(total, 3.0);
    }

    #[test]
    fn odd_rows_shift_by_half_a_bin() {
        let data = compute(
            &[0.0, 1.0, 0.0, 1.0],
            &[0.0, 0.0, 1.0, 1.0],
            r#"{"bins": [2, 2], "drop": false}"#,
        );
        let x = values(&data, Aesthetic::X);
        let width = values(&data, Computed::BinWidth)[0];
        assert_eq!(data.row_count(), 4);
        // Row order: y outer, x inner; row 1 is shifted right.
        assert!((x[2] - x[0] - width / 2.0).abs() < 1e-12);
        assert!((x[1] - x[0] - width).abs() < 1e-12);
        let y = values(&data, Aesthetic::Y);
        assert_eq!(y[0], y[1]);
        assert!((y[2] - y[0] - values(&data, Computed::BinHeight)[0]).abs() < 1e-12);
    }

    #[test]
    fn border_points_go_to_the_lowest_then_rightmost_hexagon() {
        let grid = Grid {
            x_start: 0.0,
            y_start: 0.0,
            width: 2.0,
            height: 2.0 / BIN_HEIGHT_TO_HEIGHT,
        };
        // Midway between the centres of (0, 0) and (1, 0): a vertical border.
        let (left_x, center_y) = grid.center((0, 0));
        let (right_x, _) = grid.center((1, 0));
        assert_eq!(
            grid.hexagon_of(((left_x + right_x) / 2.0, center_y)),
            (1, 0)
        );
    }
}
