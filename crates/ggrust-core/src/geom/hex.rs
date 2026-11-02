//! `geom_hex`: one hexagon per row, centred on (x, y), as lets-plot's `HexGeom`.
//!
//! Hexagons stand on a point. `width` is the flat-to-flat width and `height` the
//! row spacing, both 1 by default, in `width_unit`/`height_unit`: `"res"` (the
//! default: the stat's bin size, else twice the x resolution and the y
//! resolution, as lets-plot) or
//! `"identity"` (axis units; the point-to-point height is `2 height / sqrt 3`).
//! Setup turns them into `xmin`/`xmax`/`ymin`/`ymax`, which train the axes as
//! lets-plot's `HexGeom` spans: y -/+ `height / sqrt 3` in the unit, which with
//! `"res"` is a bit short of the drawn tips (`2 height / 3`; the default axis
//! expansion still covers them). Drawing reads the hexagon back from them.
//!
//! Drawing follows lets-plot: rows in order, one closed path each, `alpha` on
//! the fill only, outline width 0 unless `size` is given.
//!
//! What it does not do (yet): `width_unit`/`height_unit` `"size"` and `"px"`;
//! hover hit-tests the bounding boxes, not the hexagons.

use super::tile::{TileGeom, spans};
use super::{DrawLayer, Geom, lines};
use crate::coord::Panel;
use crate::data::{Column, Computed, ContinuousColumn, LayerData};
use crate::parameters::Parameters;
use crate::scene::{Node, Path, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::{Error, Result};

const BOUNDING_AESTHETICS: &[Aesthetic] = &[
    Aesthetic::Xmin,
    Aesthetic::Xmax,
    Aesthetic::Ymin,
    Aesthetic::Ymax,
];

/// Hexagons (2D bins).
pub struct HexGeom;

impl Geom for HexGeom {
    fn name(&self) -> &'static str {
        "hex"
    }

    fn default_stat(&self) -> &'static str {
        "binhex"
    }

    fn tooltip_behavior(&self) -> crate::tooltip::TooltipBehavior {
        TileGeom.tooltip_behavior()
    }

    fn hover_targets(
        &self,
        layer: &DrawLayer,
        panel: &Panel,
    ) -> Result<Option<crate::tooltip::HoverTargets>> {
        TileGeom.hover_targets(layer, panel)
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn optional_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Width, Aesthetic::Height]
    }

    /// lets-plot `DefaultGeomTheme` for `HEX`: as tiles.
    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        TileGeom.style_defaults()
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["width", "height", "width_unit", "height_unit"]
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        BOUNDING_AESTHETICS
    }

    fn span_aesthetics(&self) -> &'static [Aesthetic] {
        BOUNDING_AESTHETICS
    }

    fn setup(&self, mut data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        let (Some(x), Some(y)) = (
            data.continuous(Aesthetic::X).cloned(),
            data.continuous(Aesthetic::Y).cloned(),
        ) else {
            return Ok(data);
        };
        // Without the stat's bin width, lets-plot's "res" width is twice the x
        // resolution (staggered rows halve it).
        let bin_width = data.continuous(Computed::BinWidth).cloned().or_else(|| {
            let unit = 2.0 * data.resolution(Aesthetic::X);
            Some(ContinuousColumn::from_values(vec![unit; data.row_count()]))
        });
        let bin_height = data.continuous(Computed::BinHeight).cloned();
        let (xmin, xmax) = spans(
            &data,
            parameters,
            &x,
            bin_width.as_ref(),
            Aesthetic::Width,
            "width",
        )?;
        let (ymin, ymax) = spans(
            &data,
            parameters,
            &y,
            bin_height.as_ref(),
            Aesthetic::Height,
            "height",
        )?;
        // `spans` gives half the size; lets-plot's height span is `1 / sqrt 3`
        // of the size.
        let (ymin, ymax): (Vec<f64>, Vec<f64>) = ymin
            .iter()
            .zip(&ymax)
            .map(|(low, high)| {
                let center = (low + high) / 2.0;
                let half = (high - low) / 3f64.sqrt();
                (center - half, center + half)
            })
            .unzip();
        let continuous = |values| Column::Continuous(ContinuousColumn::from_values(values));
        data.insert(Aesthetic::Xmin, continuous(xmin))?;
        data.insert(Aesthetic::Xmax, continuous(xmax))?;
        data.insert(Aesthetic::Ymin, continuous(ymin))?;
        data.insert(Aesthetic::Ymax, continuous(ymax))?;
        Ok(data)
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let columns = BOUNDING_AESTHETICS
            .iter()
            .map(|aesthetic| layer.data.continuous(*aesthetic))
            .collect::<Option<Vec<_>>>();
        let Some([xmin, xmax, ymin, ymax]) = columns.as_deref() else {
            return Err(Error::data(
                "hexagons were not set up (missing xmin, xmax, ymin or ymax)".to_owned(),
            ));
        };
        // The tips are `2 height / 3` from the centre with "res" (the span is
        // `height / sqrt 3`), the span itself with "identity".
        let tip_factor = match layer.parameters.string("height_unit")? {
            Some("identity") => 1.0,
            _ => 2.0 / 3f64.sqrt(),
        };
        let fills = layer.styles.colors(Aesthetic::Fill)?;
        let alphas = layer.styles.numbers(Aesthetic::Alpha)?;
        let mut nodes = Vec::with_capacity(layer.data.row_count());
        for row in 0..layer.data.row_count() {
            let corners = [xmin.get(row), xmax.get(row), ymin.get(row), ymax.get(row)];
            let [Some(left), Some(right), Some(bottom), Some(top)] = corners else {
                continue;
            };
            let center_x = (left + right) / 2.0;
            let center_y = (bottom + top) / 2.0;
            let tip = (top - bottom) / 2.0 * tip_factor;
            let (bottom, top, shoulder) = (center_y - tip, center_y + tip, tip / 2.0);
            // lets-plot `clientHexByDataPoint` order: bottom tip, counterclockwise.
            let points: Vec<(f64, f64)> = [
                (center_x, bottom),
                (right, center_y - shoulder),
                (right, center_y + shoulder),
                (center_x, top),
                (left, center_y + shoulder),
                (left, center_y - shoulder),
            ]
            .into_iter()
            .map(|(x, y)| panel.map(x, y))
            .collect();
            if !points.iter().all(|(x, y)| x.is_finite() && y.is_finite()) {
                continue;
            }
            let fill = fills.get(row).with_opacity(alphas.get(row));
            nodes.push(Node::Path(Path {
                points,
                closed: true,
                fill: (fill.alpha > 0).then_some(fill),
                stroke: lines::stroke_with_alpha(layer.styles, row, false)?,
            }));
        }
        Ok(nodes)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        TileGeom.minimum_key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        TileGeom.draw_key(styles, cell)
    }
}
