//! Hover targets shared by several geoms (lets-plot `GeomTargetCollector` hit
//! shapes): rectangles from `xmin`..`ymax`, circles at `x`/`y`, and the vertices
//! of lines. Rows with missing or non-finite positions get no target.

use super::DrawLayer;
use crate::coord::Panel;
use crate::spec::Aesthetic;
use crate::tooltip::HoverTargets;

/// One rectangle per row from the setup columns (bars, tiles, rasters, rects).
/// `y_columns` names the bottom and top aesthetics (boxes span their whiskers).
pub(super) fn rectangles(
    layer: &DrawLayer,
    panel: &Panel,
    y_columns: (Aesthetic, Aesthetic),
) -> Option<HoverTargets> {
    let [xmin, xmax, ymin, ymax] = [Aesthetic::Xmin, Aesthetic::Xmax, y_columns.0, y_columns.1]
        .map(|aesthetic| layer.data.continuous(aesthetic));
    let (xmin, xmax, ymin, ymax) = (xmin?, xmax?, ymin?, ymax?);
    let (mut x, mut y, mut width, mut height, mut rows) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for row in 0..layer.data.row_count() {
        let [Some(x0), Some(y0), Some(x1), Some(y1)] =
            [xmin.get(row), ymin.get(row), xmax.get(row), ymax.get(row)]
        else {
            continue;
        };
        let (left, top) = panel.map(x0, y0);
        let (right, bottom) = panel.map(x1, y1);
        if ![left, top, right, bottom]
            .iter()
            .all(|value| value.is_finite())
        {
            continue;
        }
        x.push(left.min(right));
        y.push(top.min(bottom));
        // lets-plot keeps empty bars hoverable (`GeomHelper.toClient`).
        width.push((right - left).abs().max(0.1));
        height.push((bottom - top).abs().max(0.1));
        rows.push(row as u32);
    }
    Some(HoverTargets::Rects {
        x,
        y,
        width,
        height,
        rows,
    })
}

/// One circle per row at `x`/`y`; `radius(row)` in pixels.
pub(super) fn circles(
    layer: &DrawLayer,
    panel: &Panel,
    radius: impl Fn(usize) -> f64,
) -> Option<HoverTargets> {
    let (x_column, y_column) = (
        layer.data.continuous(Aesthetic::X)?,
        layer.data.continuous(Aesthetic::Y)?,
    );
    let (mut x, mut y, mut radii, mut rows) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for row in 0..layer.data.row_count() {
        let (Some(value_x), Some(value_y)) = (x_column.get(row), y_column.get(row)) else {
            continue;
        };
        let (pixel_x, pixel_y) = panel.map(value_x, value_y);
        if !(pixel_x.is_finite() && pixel_y.is_finite()) {
            continue;
        }
        x.push(pixel_x);
        y.push(pixel_y);
        radii.push(radius(row).max(0.0));
        rows.push(row as u32);
    }
    Some(HoverTargets::Points {
        x,
        y,
        radius: radii,
        rows,
    })
}

/// Vertices at `x`/`y`, one line per group (lines, paths).
pub(super) fn vertices(layer: &DrawLayer, panel: &Panel) -> Option<HoverTargets> {
    let (x_column, y_column) = (
        layer.data.continuous(Aesthetic::X)?,
        layer.data.continuous(Aesthetic::Y)?,
    );
    let mut targets = VertexBuilder::default();
    for row in 0..layer.data.row_count() {
        let (Some(value_x), Some(value_y)) = (x_column.get(row), y_column.get(row)) else {
            continue;
        };
        targets.push(panel.map(value_x, value_y), layer.data.group_of(row), row);
    }
    Some(targets.finish())
}

/// Collects [`HoverTargets::Vertices`].
#[derive(Default)]
pub(super) struct VertexBuilder {
    x: Vec<f64>,
    y: Vec<f64>,
    line: Vec<u32>,
    rows: Vec<u32>,
}

impl VertexBuilder {
    /// Add a vertex in pixels (skipped when not finite).
    pub(super) fn push(&mut self, (x, y): (f64, f64), line: u32, row: usize) {
        if x.is_finite() && y.is_finite() {
            self.x.push(x);
            self.y.push(y);
            self.line.push(line);
            self.rows.push(row as u32);
        }
    }

    /// The targets.
    pub(super) fn finish(self) -> HoverTargets {
        HoverTargets::Vertices {
            x: self.x,
            y: self.y,
            line: self.line,
            rows: self.rows,
        }
    }
}
