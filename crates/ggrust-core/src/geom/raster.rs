//! `geom_raster`: equal cells on a regular grid, drawn as one bitmap image.
//!
//! Follows lets-plot `RasterGeom`: the grid step is the x and y resolution, the
//! image spans the data bounds plus half a step each side, each row paints the
//! cell nearest its (x, y) with its fill and alpha, empty cells stay transparent.
//! The plot defaults to `coord_fixed()` (lets-plot `GeomProto`); the axes cover
//! whole cells. Under `coord_flip` the bitmap is transposed, so each cell stays at
//! its data position (lets-plot draws it untransposed).
//!
//! What it does not do: draw grids over 5 million cells (an error; lets-plot
//! writes a notice into the plot), mapped `width`/`height`.

use super::{DrawLayer, Geom};
use crate::coord::Panel;
use crate::data::{Column, ContinuousColumn, LayerData};
use crate::parameters::Parameters;
use crate::scene::{Color, Image, Node, PerItem, Rect, Rects};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::{Error, Result};

/// lets-plot raster style: brush fill.
const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (
        Aesthetic::Fill,
        StyleDefault::Color(Color::rgb(0x11, 0x8e, 0xd8)),
    ),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
];

/// Cell edges made by setup; they train the axes.
const CELL_AESTHETICS: &[Aesthetic] = &[
    Aesthetic::Xmin,
    Aesthetic::Xmax,
    Aesthetic::Ymin,
    Aesthetic::Ymax,
];

/// lets-plot's limit on the image size (`width * height > 5000000`).
const MAX_CELLS: f64 = 5_000_000.0;

/// Below this, a step is too small to lay out a grid (lets-plot `SeriesUtil.TINY`).
const TINY_STEP: f64 = 1e-50;

/// A raster image of tiles.
pub struct RasterGeom;

impl Geom for RasterGeom {
    fn name(&self) -> &'static str {
        "raster"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn default_coord(&self) -> Option<&'static str> {
        Some("fixed")
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        CELL_AESTHETICS
    }

    fn span_aesthetics(&self) -> &'static [Aesthetic] {
        CELL_AESTHETICS
    }

    /// Cell edges (x, y -/+ half a step), so the axes cover whole cells.
    fn setup(&self, mut data: LayerData, _parameters: &Parameters) -> Result<LayerData> {
        for (center, low, high) in [
            (Aesthetic::X, Aesthetic::Xmin, Aesthetic::Xmax),
            (Aesthetic::Y, Aesthetic::Ymin, Aesthetic::Ymax),
        ] {
            let Some(location) = data.continuous(center).cloned() else {
                continue;
            };
            let half = data.resolution(center) / 2.0;
            let edge = |offset: f64| {
                let values = (0..location.len())
                    .map(|row| location.get(row).unwrap_or(f64::NAN) + offset)
                    .collect();
                Column::Continuous(ContinuousColumn::from_values(values))
            };
            data.insert(low, edge(-half))?;
            data.insert(high, edge(half))?;
        }
        Ok(data)
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let (Some(x), Some(y)) = (
            layer.data.continuous(Aesthetic::X),
            layer.data.continuous(Aesthetic::Y),
        ) else {
            return Ok(Vec::new());
        };
        let rows: Vec<(usize, f64, f64)> = (0..layer.data.row_count())
            .filter_map(|row| {
                let (x, y) = (x.get(row)?, y.get(row)?);
                (x.is_finite() && y.is_finite()).then_some((row, x, y))
            })
            .collect();
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let bounds = |values: &mut dyn Iterator<Item = f64>| {
            values.fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), value| {
                (low.min(value), high.max(value))
            })
        };
        let (x0, x1) = bounds(&mut rows.iter().map(|&(_, x, _)| x));
        let (y0, y1) = bounds(&mut rows.iter().map(|&(_, _, y)| y));
        let (step_x, step_y) = (
            layer.data.resolution(Aesthetic::X),
            layer.data.resolution(Aesthetic::Y),
        );
        if !(step_x > TINY_STEP && step_y > TINY_STEP) {
            return Err(Error::InvalidData(format!(
                "geom_raster: the x or y step is too small ({step_x}, {step_y})"
            )));
        }
        let columns = ((x1 - x0) / step_x).round() + 1.0;
        let grid_rows = ((y1 - y0) / step_y).round() + 1.0;
        if columns * grid_rows > MAX_CELLS {
            return Err(Error::InvalidData(format!(
                "geom_raster: the image would have {columns} x {grid_rows} cells, more than \
                 {MAX_CELLS}; are x and y on a regular grid?"
            )));
        }
        let (columns, grid_rows) = (columns as usize, grid_rows as usize);

        // Corners half a step outside the data bounds, in pixels; data x may run
        // vertically (coord_flip) and either axis may be reversed.
        let corner0 = panel.map(x0 - step_x / 2.0, y0 - step_y / 2.0);
        let corner2 = panel.map(x1 + step_x / 2.0, y1 + step_y / 2.0);
        let rect = Rect {
            x: corner0.0.min(corner2.0),
            y: corner0.1.min(corner2.1),
            width: (corner0.0 - corner2.0).abs(),
            height: (corner0.1 - corner2.1).abs(),
        };
        let (screen_columns, screen_rows) = if panel.flipped {
            (grid_rows, columns)
        } else {
            (columns, grid_rows)
        };
        let fills = layer.styles.colors(Aesthetic::Fill)?;
        let alphas = layer.styles.numbers(Aesthetic::Alpha)?;
        let mut pixels = vec![Color::TRANSPARENT; screen_columns * screen_rows];
        for &(row, x, y) in &rows {
            let column = ((x - x0) / step_x).round() as usize;
            let grid_row = ((y - y0) / step_y).round() as usize;
            let (mut horizontal, mut vertical) = if panel.flipped {
                (grid_row, column)
            } else {
                (column, grid_row)
            };
            // Image rows run top-down; flip an axis whose pixels run the other way.
            if corner2.0 < corner0.0 {
                horizontal = screen_columns - 1 - horizontal;
            }
            if corner2.1 < corner0.1 {
                vertical = screen_rows - 1 - vertical;
            }
            pixels[vertical * screen_columns + horizontal] =
                fills.get(row).with_opacity(alphas.get(row));
        }
        Ok(vec![Node::Image(Image {
            rect,
            columns: screen_columns as u32,
            rows: screen_rows as u32,
            pixels,
        })])
    }

    fn minimum_key_size(&self, _styles: &Styles) -> Result<(f64, f64)> {
        Ok((0.0, 0.0))
    }

    /// lets-plot `FilledRectLegendKeyElementFactory`: the whole cell, no outline.
    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        let alpha = styles.numbers(Aesthetic::Alpha)?.get(0);
        let fill = styles.colors(Aesthetic::Fill)?.get(0).with_opacity(alpha);
        Ok(vec![Node::Rects(Rects {
            rects: vec![cell],
            fill: PerItem::Uniform((fill.alpha > 0).then_some(fill)),
            stroke: None,
        })])
    }
}
