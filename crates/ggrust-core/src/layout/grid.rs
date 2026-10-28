//! Cells of a grid of figures (lets-plot `CompositeFigureGridLayoutBase`).
//!
//! What it does: split an area into `columns` x `rows` cells with relative widths
//! and heights and fixed gaps, and shrink a cell to a figure's aspect ratio.
//!
//! What it does not do: lay out what goes in the cells (see [`crate::grid`]).

use crate::scene::Rect;

/// lets-plot `Defaults.SubplotsGrid.DEF_HSPACE` / `DEF_VSPACE`: gap between cells (px).
pub const DEFAULT_SPACING: f64 = 4.0;

/// Smallest cell extent (lets-plot keeps cells at least 1 px).
const MIN_CELL_SIZE: f64 = 1.0;

/// Cell rectangles in row-major order. `widths` and `heights` are relative sizes:
/// missing entries repeat the last one, extra ones are ignored, none means equal.
pub fn cell_rects(
    area: Rect,
    (columns, rows): (usize, usize),
    (horizontal_spacing, vertical_spacing): (f64, f64),
    widths: Option<&[f64]>,
    heights: Option<&[f64]>,
) -> Vec<Rect> {
    let gaps = |count: usize, spacing: f64| spacing * count.saturating_sub(1) as f64;
    let column_widths = cell_sizes(
        area.width - gaps(columns, horizontal_spacing),
        columns,
        widths,
    );
    let row_heights = cell_sizes(area.height - gaps(rows, vertical_spacing), rows, heights);
    let origins = |sizes: &[f64], start: f64, spacing: f64| -> Vec<f64> {
        sizes
            .iter()
            .scan(start, |next, size| {
                let origin = *next;
                *next += size + spacing;
                Some(origin)
            })
            .collect()
    };
    let column_origins = origins(&column_widths, area.x, horizontal_spacing);
    let row_origins = origins(&row_heights, area.y, vertical_spacing);
    let mut cells = Vec::with_capacity(columns * rows);
    for row in 0..rows {
        for column in 0..columns {
            cells.push(Rect {
                x: column_origins[column],
                y: row_origins[row],
                width: column_widths[column],
                height: row_heights[row],
            });
        }
    }
    cells
}

/// lets-plot `cellSizeList`: `total` split by relative `sizes`.
fn cell_sizes(total: f64, count: usize, sizes: Option<&[f64]>) -> Vec<f64> {
    let relative: Vec<f64> = match sizes {
        Some(sizes) if !sizes.is_empty() => {
            let last = sizes[sizes.len() - 1];
            (0..count)
                .map(|index| sizes.get(index).copied().unwrap_or(last))
                .collect()
        }
        _ => vec![1.0; count],
    };
    let sum: f64 = relative.iter().sum();
    relative
        .iter()
        .map(|size| (size / sum * total).max(MIN_CELL_SIZE))
        .collect()
}

/// The largest rectangle of the aspect ratio of `size` (width, height) that fits in
/// `cell`, centred in it (lets-plot `DoubleRectangle.shrinkToAspectRatio`).
pub fn shrink_to_aspect(cell: Rect, (width, height): (f64, f64)) -> Rect {
    let ratio = width / height;
    let (new_width, new_height) = if ratio >= 1.0 {
        let new_height = cell.width / ratio;
        let scaling = if new_height > cell.height {
            cell.height / new_height
        } else {
            1.0
        };
        (cell.width * scaling, new_height * scaling)
    } else {
        let new_width = cell.height * ratio;
        let scaling = if new_width > cell.width {
            cell.width / new_width
        } else {
            1.0
        };
        (new_width * scaling, cell.height * scaling)
    };
    Rect {
        x: cell.x + (cell.width - new_width) / 2.0,
        y: cell.y + (cell.height - new_height) / 2.0,
        width: new_width,
        height: new_height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(width: f64, height: f64) -> Rect {
        Rect {
            x: 6.0,
            y: 6.0,
            width,
            height,
        }
    }

    #[test]
    fn equal_cells_with_gaps() {
        // lets-plot: gggrid([p1, p2]) at 900 x 400 puts cells at x 6 and 452, 442 wide.
        let cells = cell_rects(area(888.0, 388.0), (2, 1), (4.0, 4.0), None, None);
        assert_eq!(cells.len(), 2);
        assert_eq!((cells[0].x, cells[0].width), (6.0, 442.0));
        assert_eq!((cells[1].x, cells[1].width), (452.0, 442.0));
        assert_eq!((cells[1].y, cells[1].height), (6.0, 388.0));
    }

    #[test]
    fn relative_sizes_repeat_the_last_one() {
        let cells = cell_rects(
            area(400.0, 100.0),
            (4, 1),
            (0.0, 0.0),
            Some(&[2.0, 1.0]),
            None,
        );
        let widths: Vec<f64> = cells.iter().map(|cell| cell.width).collect();
        assert_eq!(widths, vec![160.0, 80.0, 80.0, 80.0]);
        let cells = cell_rects(
            area(100.0, 300.0),
            (1, 2),
            (0.0, 0.0),
            None,
            Some(&[1.0, 2.0, 9.0]),
        );
        assert_eq!((cells[1].y, cells[1].height), (106.0, 200.0));
    }

    #[test]
    fn cells_keep_a_minimum_size() {
        let cells = cell_rects(area(2.0, 2.0), (2, 2), (10.0, 10.0), None, None);
        assert!(
            cells
                .iter()
                .all(|cell| cell.width == 1.0 && cell.height == 1.0)
        );
    }

    #[test]
    fn shrinks_to_the_figure_aspect_centred() {
        let cell = Rect {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 400.0,
        };
        let shrunk = shrink_to_aspect(cell, (600.0, 400.0));
        assert_eq!(
            (shrunk.y, shrunk.width, shrunk.height),
            (66.66666666666666, 400.0, 266.6666666666667)
        );
        let tall = shrink_to_aspect(cell, (200.0, 400.0));
        assert_eq!((tall.x, tall.width, tall.height), (100.0, 200.0, 400.0));
    }
}
