//! Iso-valued contours of a 2D grid by marching squares: a port of
//! `skimage.measure.find_contours` (scikit-image 0.26) with its defaults
//! (`fully_connected="low"`, `positive_orientation="low"`, no mask).
//!
//! Each square of four neighbouring grid values gives zero, one or two segments
//! (saddles separate the high corners), with vertices linearly interpolated on the
//! square edges. The segments are joined into contours exactly as scikit-image's
//! `_assemble_contours` does (same order, same direction), so vertices are in
//! `(row, column)` index space and closed contours repeat their first vertex.
//! Squares with a NaN corner are skipped.
//!
//! What it does not do: masks, `fully_connected="high"`, `positive_orientation`.
//! This is not the contour engine of `stat="contour"` (see `stat/contour.rs`,
//! lets-plot's triangle split).

use std::collections::{HashMap, VecDeque};

/// A contour vertex `(row, column)` in fractional grid indices.
pub type Vertex = [f64; 2];

/// The contours of `values` (row-major, `rows x columns`) at `level`, as
/// `skimage.measure.find_contours(values, level)`.
///
/// # Panics
///
/// If `values.len() != rows * columns`.
pub fn find_contours(values: &[f64], rows: usize, columns: usize, level: f64) -> Vec<Vec<Vertex>> {
    assert_eq!(values.len(), rows * columns, "grid size mismatch");
    assemble(&segments(values, rows, columns, level))
}

/// skimage `_get_fraction`.
fn fraction(from: f64, to: f64, level: f64) -> f64 {
    if to == from {
        return 0.0;
    }
    (level - from) / (to - from)
}

/// skimage `_get_contour_segments` (`vertex_connect_high = False`).
fn segments(values: &[f64], rows: usize, columns: usize, level: f64) -> Vec<(Vertex, Vertex)> {
    let mut segments = Vec::new();
    for r0 in 0..rows.saturating_sub(1) {
        for c0 in 0..columns.saturating_sub(1) {
            let (r1, c1) = (r0 + 1, c0 + 1);
            let upper_left = values[r0 * columns + c0];
            let upper_right = values[r0 * columns + c1];
            let lower_left = values[r1 * columns + c0];
            let lower_right = values[r1 * columns + c1];
            if upper_left.is_nan()
                || upper_right.is_nan()
                || lower_left.is_nan()
                || lower_right.is_nan()
            {
                continue;
            }
            let case = u8::from(upper_left > level)
                + 2 * u8::from(upper_right > level)
                + 4 * u8::from(lower_left > level)
                + 8 * u8::from(lower_right > level);
            if case == 0 || case == 15 {
                continue;
            }
            let (row, column) = (r0 as f64, c0 as f64);
            let top = [row, column + fraction(upper_left, upper_right, level)];
            let bottom = [r1 as f64, column + fraction(lower_left, lower_right, level)];
            let left = [row + fraction(upper_left, lower_left, level), column];
            let right = [row + fraction(upper_right, lower_right, level), c1 as f64];
            match case {
                1 => segments.push((top, left)),
                2 => segments.push((right, top)),
                3 => segments.push((right, left)),
                4 => segments.push((left, bottom)),
                5 => segments.push((top, bottom)),
                6 => {
                    segments.push((right, top));
                    segments.push((left, bottom));
                }
                7 => segments.push((right, bottom)),
                8 => segments.push((bottom, right)),
                9 => {
                    segments.push((top, left));
                    segments.push((bottom, right));
                }
                10 => segments.push((bottom, top)),
                11 => segments.push((bottom, left)),
                12 => segments.push((left, right)),
                13 => segments.push((top, right)),
                _ => segments.push((left, top)),
            }
        }
    }
    segments
}

/// Hash key of a vertex: equal floats (Python tuple equality, `-0.0 == 0.0`) give
/// equal keys.
fn key(vertex: Vertex) -> (u64, u64) {
    ((vertex[0] + 0.0).to_bits(), (vertex[1] + 0.0).to_bits())
}

/// skimage `_assemble_contours`: joins segments head to tail, contours numbered in
/// creation order (a merge keeps the lower number).
fn assemble(segments: &[(Vertex, Vertex)]) -> Vec<Vec<Vertex>> {
    let mut contours: Vec<Option<VecDeque<Vertex>>> = Vec::new();
    // Contour number by its first (starts) and last (ends) vertex.
    let mut starts: HashMap<(u64, u64), usize> = HashMap::new();
    let mut ends: HashMap<(u64, u64), usize> = HashMap::new();
    for &(from, to) in segments {
        if key(from) == key(to) {
            continue;
        }
        let tail = starts.remove(&key(to));
        let head = ends.remove(&key(from));
        match (tail, head) {
            (Some(tail), Some(head)) if tail == head => {
                if let Some(contour) = contours[head].as_mut() {
                    contour.push_back(to);
                }
            }
            (Some(tail), Some(head)) => {
                if tail > head {
                    let tail_contour = contours[tail].take().unwrap_or_default();
                    if let Some(contour) = contours[head].as_mut() {
                        contour.extend(tail_contour);
                        if let (Some(&first), Some(&last)) = (contour.front(), contour.back()) {
                            starts.insert(key(first), head);
                            ends.insert(key(last), head);
                        }
                    }
                } else {
                    let head_contour = contours[head].take().unwrap_or_default();
                    if let Some(&first) = head_contour.front() {
                        starts.remove(&key(first));
                    }
                    if let Some(contour) = contours[tail].as_mut() {
                        for &vertex in head_contour.iter().rev() {
                            contour.push_front(vertex);
                        }
                        if let (Some(&first), Some(&last)) = (contour.front(), contour.back()) {
                            starts.insert(key(first), tail);
                            ends.insert(key(last), tail);
                        }
                    }
                }
            }
            (None, None) => {
                let number = contours.len();
                contours.push(Some(VecDeque::from([from, to])));
                starts.insert(key(from), number);
                ends.insert(key(to), number);
            }
            (Some(tail), None) => {
                if let Some(contour) = contours[tail].as_mut() {
                    contour.push_front(from);
                }
                starts.insert(key(from), tail);
            }
            (None, Some(head)) => {
                if let Some(contour) = contours[head].as_mut() {
                    contour.push_back(to);
                }
                ends.insert(key(to), head);
            }
        }
    }
    contours
        .into_iter()
        .flatten()
        .map(|contour| contour.into_iter().collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bump_gives_one_closed_contour() {
        // 3 x 3 grid with a peak in the middle.
        let values = [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
        let contours = find_contours(&values, 3, 3, 0.5);
        assert_eq!(contours.len(), 1);
        let contour = &contours[0];
        assert_eq!(contour.first(), contour.last());
        // skimage's output for the same grid.
        assert_eq!(
            contour,
            &vec![[1.5, 1.0], [1.0, 0.5], [0.5, 1.0], [1.0, 1.5], [1.5, 1.0]]
        );
    }

    #[test]
    fn a_ramp_gives_one_open_contour() {
        let values = [0.0, 1.0, 2.0, 0.0, 1.0, 2.0];
        let contours = find_contours(&values, 2, 3, 1.5);
        assert_eq!(contours, vec![vec![[1.0, 1.5], [0.0, 1.5]]]);
    }

    #[test]
    fn flat_and_nan_grids_give_nothing() {
        assert!(find_contours(&[1.0; 4], 2, 2, 0.5).is_empty());
        assert!(find_contours(&[f64::NAN, 1.0, 0.0, 0.0], 2, 2, 0.5).is_empty());
    }
}
