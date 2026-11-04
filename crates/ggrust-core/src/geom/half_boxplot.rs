//! `geom_half_boxplot` (R package gghalves): the half of a box on the `side`
//! `"l"` (default) or `"r"` of x. As gghalves draws it: the box runs from its
//! outer edge to x, less `nudge` (data units, default 0); the whiskers and the
//! outliers stand at x moved `nudge` towards the box; fences (`errorbar_draw`,
//! default true) reach `errorbar_length` (default 0.5) of the half box outwards.
//! Spans are `width` (default 0.75) x the x resolution, dodged by group.
//! Outliers are a separate point layer that the Python API adds.
//!
//! What it does not do: gghalves' `center`, notches, `varwidth`.

use super::boxplot::{box_key_size, draw_box_key, draw_boxes, line, setup_box_spans};
use super::half_violin::{HALF_WIDTH, HalfSide, nudge, positive_width};
use super::{BoxplotGeom, DrawLayer, Geom};
use crate::coord::Panel;
use crate::data::{Column, ContinuousColumn, LayerData};
use crate::parameters::Parameters;
use crate::position::DefaultPosition;
use crate::scene::{Node, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::tooltip::{HoverTargets, TooltipBehavior};
use crate::{Error, Result};

/// ggplot2's crossbar midline fattening (gghalves draws the box as one).
const DEFAULT_FATTEN: f64 = 2.0;
/// gghalves `errorbar.length`.
const DEFAULT_ERRORBAR_LENGTH: f64 = 0.5;

/// Half box-and-whisker plots.
pub struct HalfBoxplotGeom;

impl Geom for HalfBoxplotGeom {
    fn name(&self) -> &'static str {
        "half_boxplot"
    }

    fn default_stat(&self) -> &'static str {
        "boxplot"
    }

    fn default_position(&self) -> DefaultPosition {
        DefaultPosition::named("dodge")
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        BoxplotGeom.required_aesthetics()
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        BoxplotGeom.style_defaults()
    }

    fn parameters(&self) -> &'static [&'static str] {
        &[
            "width",
            "fatten",
            "side",
            "nudge",
            "errorbar_draw",
            "errorbar_length",
        ]
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        BoxplotGeom.setup_aesthetics()
    }

    fn span_aesthetics(&self) -> &'static [Aesthetic] {
        BoxplotGeom.span_aesthetics()
    }

    fn setup(&self, data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        HalfSide::from_parameters(parameters, HalfSide::Left)?;
        errorbar_length(parameters)?;
        positive_width(parameters)?;
        setup_box_spans(data, parameters, HALF_WIDTH)
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let parameters = &layer.parameters;
        let side = HalfSide::from_parameters(parameters, HalfSide::Left)?;
        let fatten = parameters.number("fatten")?.unwrap_or(DEFAULT_FATTEN);
        let draws_fences = parameters.boolean("errorbar_draw")?.unwrap_or(true);
        let length = errorbar_length(parameters)?;
        let data = halved(layer.data, side, nudge(parameters)?)?;
        let half_layer = DrawLayer {
            data: &data,
            ..*layer
        };
        let mut nodes = draw_boxes(&half_layer, panel, fatten, 0.0)?;
        if draws_fences {
            let column = |aesthetic| layer.data.continuous(aesthetic);
            let (Some(x), Some(xmin), Some(xmax), Some(ymin), Some(ymax)) = (
                data.continuous(Aesthetic::X),
                column(Aesthetic::Xmin),
                column(Aesthetic::Xmax),
                column(Aesthetic::Ymin),
                column(Aesthetic::Ymax),
            ) else {
                return Ok(nodes);
            };
            let finite = |column: &ContinuousColumn, row: usize| {
                column.get(row).filter(|value| value.is_finite())
            };
            for row in 0..layer.data.row_count() {
                let (Some(center), Some(low), Some(high)) =
                    (finite(x, row), finite(xmin, row), finite(xmax, row))
                else {
                    continue;
                };
                let end = center + side.sign() * (high - low) / 2.0 * length;
                let stroke = super::lines::stroke_with_alpha(layer.styles, row, false)?;
                for fence in [ymin, ymax] {
                    if let Some(y) = finite(fence, row) {
                        nodes.extend(line(panel, (center, y), (end, y), stroke.clone()));
                    }
                }
            }
        }
        Ok(nodes)
    }

    fn tooltip_behavior(&self) -> TooltipBehavior {
        BoxplotGeom.tooltip_behavior()
    }

    fn hover_targets(&self, layer: &DrawLayer, panel: &Panel) -> Result<Option<HoverTargets>> {
        let side = HalfSide::from_parameters(&layer.parameters, HalfSide::Left)?;
        let data = halved(layer.data, side, nudge(&layer.parameters)?)?;
        BoxplotGeom.hover_targets(
            &DrawLayer {
                data: &data,
                ..*layer
            },
            panel,
        )
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        box_key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        draw_box_key(styles, cell)
    }
}

/// The data with the box spans cut at x (moved `nudge` towards the box), and x
/// (the whiskers) moved the same way.
fn halved(data: &LayerData, side: HalfSide, nudge: f64) -> Result<LayerData> {
    let mut data = data.clone();
    let columns = [Aesthetic::X, Aesthetic::Xmin, Aesthetic::Xmax]
        .map(|aesthetic| data.continuous(aesthetic).cloned());
    let [Some(x), Some(xmin), Some(xmax)] = columns else {
        return Ok(data);
    };
    let value = |column: &ContinuousColumn, row: usize| column.get(row).unwrap_or(f64::NAN);
    let rows = 0..data.row_count();
    let center: Vec<f64> = rows
        .clone()
        .map(|row| value(&x, row) + side.sign() * nudge)
        .collect();
    let (left, right): (Vec<f64>, Vec<f64>) = rows
        .map(|row| match side {
            HalfSide::Left => (value(&xmin, row), center[row]),
            HalfSide::Right => (center[row], value(&xmax, row)),
        })
        .unzip();
    let continuous = |values| Column::Continuous(ContinuousColumn::from_values(values));
    data.insert(Aesthetic::X, continuous(center))?;
    data.insert(Aesthetic::Xmin, continuous(left))?;
    data.insert(Aesthetic::Xmax, continuous(right))?;
    Ok(data)
}

/// `errorbar_length`: a share of the half box, from 0 to 1.
fn errorbar_length(parameters: &Parameters) -> Result<f64> {
    let length = parameters
        .number("errorbar_length")?
        .unwrap_or(DEFAULT_ERRORBAR_LENGTH);
    if !(0.0..=1.0).contains(&length) {
        return Err(Error::spec(
            parameters.path_of("errorbar_length"),
            format!("errorbar_length must be between 0 and 1, got {length}"),
        ));
    }
    Ok(length)
}
