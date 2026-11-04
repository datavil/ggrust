//! `geom_half_point` (R package gghalves): points jittered within one half of
//! their x slot, the rain of a raincloud plot. Drawing is [`PointGeom`]'s.
//!
//! Setup gives every row a span `width` (default 0.75) x the x resolution, so
//! dodging moves and narrows whole groups. Drawing moves each point to the
//! middle of its half (`side` `"r"`, default, or `"l"`: a quarter of the span
//! from x), then jitters it, as gghalves' default `position_jitter()`:
//! uniform within +/- `jitter_width` (default `range_scale` (0.75) x a quarter
//! of the span) and +/- `jitter_height` (default 0.4 x the resolution of the
//! group's y). Offsets come from a seeded generator (`seed`), so renders are
//! reproducible; they are not R's random numbers.
//!
//! What it does not do: transformations other than jitter or none (0 width
//! and height).

use super::half_violin::{HALF_WIDTH, HalfSide, positive_width};
use super::violin::setup_spans;
use super::{DrawLayer, Geom, PointGeom};
use crate::coord::Panel;
use crate::data::{Column, ContinuousColumn, LayerData};
use crate::parameters::Parameters;
use crate::position::DefaultPosition;
use crate::position::random::{DEFAULT_SEED, KotlinRandom};
use crate::scene::{Node, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::tooltip::{HoverTargets, TooltipBehavior};
use crate::{Error, Result};

/// gghalves `range_scale`.
const DEFAULT_RANGE_SCALE: f64 = 0.75;
/// ggplot2 `position_jitter()` height, in y resolutions.
const DEFAULT_HEIGHT: f64 = 0.4;

/// Points in one half of their slot.
pub struct HalfPointGeom;

impl Geom for HalfPointGeom {
    fn name(&self) -> &'static str {
        "half_point"
    }

    fn default_position(&self) -> DefaultPosition {
        DefaultPosition::named("dodge")
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        PointGeom.required_aesthetics()
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        PointGeom.style_defaults()
    }

    fn parameters(&self) -> &'static [&'static str] {
        &[
            "width",
            "side",
            "range_scale",
            "jitter_width",
            "jitter_height",
            "seed",
        ]
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Xmin, Aesthetic::Xmax]
    }

    fn span_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Xmin, Aesthetic::Xmax]
    }

    fn setup(&self, data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        Placement::from_parameters(parameters)?;
        positive_width(parameters)?;
        setup_spans(data, parameters.number("width")?.unwrap_or(HALF_WIDTH))
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let data = placed(layer.data, &Placement::from_parameters(&layer.parameters)?)?;
        PointGeom.draw(
            &DrawLayer {
                data: &data,
                ..*layer
            },
            panel,
        )
    }

    fn tooltip_behavior(&self) -> TooltipBehavior {
        PointGeom.tooltip_behavior()
    }

    fn hover_targets(&self, layer: &DrawLayer, panel: &Panel) -> Result<Option<HoverTargets>> {
        let data = placed(layer.data, &Placement::from_parameters(&layer.parameters)?)?;
        PointGeom.hover_targets(
            &DrawLayer {
                data: &data,
                ..*layer
            },
            panel,
        )
    }

    fn legend_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        PointGeom.legend_defaults()
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        PointGeom.minimum_key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        PointGeom.draw_key(styles, cell)
    }
}

/// The layer's placement options.
struct Placement {
    side: HalfSide,
    range_scale: f64,
    jitter_width: Option<f64>,
    jitter_height: Option<f64>,
    seed: i64,
}

impl Placement {
    fn from_parameters(parameters: &Parameters) -> Result<Self> {
        let non_negative = |name: &str| -> Result<Option<f64>> {
            match parameters.number(name)? {
                Some(value) if value < 0.0 => Err(Error::spec(
                    parameters.path_of(name),
                    format!("{name} must be zero or positive, got {value}"),
                )),
                value => Ok(value),
            }
        };
        Ok(Placement {
            side: HalfSide::from_parameters(parameters, HalfSide::Right)?,
            range_scale: non_negative("range_scale")?.unwrap_or(DEFAULT_RANGE_SCALE),
            jitter_width: non_negative("jitter_width")?,
            jitter_height: non_negative("jitter_height")?,
            seed: parameters
                .number("seed")?
                .map_or(DEFAULT_SEED, |seed| seed as i64),
        })
    }
}

/// The data with every point moved into its half and jittered.
fn placed(data: &LayerData, placement: &Placement) -> Result<LayerData> {
    let columns = [Aesthetic::X, Aesthetic::Y, Aesthetic::Xmin, Aesthetic::Xmax]
        .map(|aesthetic| data.continuous(aesthetic).cloned());
    let [Some(x), Some(y), Some(xmin), Some(xmax)] = columns else {
        return Ok(data.clone());
    };
    let row_count = data.row_count();
    let finite =
        |column: &ContinuousColumn, row: usize| column.get(row).filter(|value| value.is_finite());
    // gghalves jitters each group's y by its own resolution.
    let mut heights = vec![0.0; data.group_count()];
    if placement.jitter_height.is_none() {
        let mut values: Vec<Vec<f64>> = vec![Vec::new(); data.group_count()];
        for row in 0..row_count {
            if let Some(value) = finite(&y, row) {
                values[data.group_of(row) as usize].push(value);
            }
        }
        for (height, values) in heights.iter_mut().zip(values) {
            *height =
                DEFAULT_HEIGHT * crate::data::resolution(&ContinuousColumn::from_values(values));
        }
    }
    let mut random = KotlinRandom::new(placement.seed);
    let (mut new_x, mut new_y) = (Vec::with_capacity(row_count), Vec::with_capacity(row_count));
    for row in 0..row_count {
        let values = (
            finite(&x, row),
            finite(&y, row),
            finite(&xmin, row),
            finite(&xmax, row),
        );
        let (Some(center), Some(value), Some(low), Some(high)) = values else {
            new_x.push(x.get(row).unwrap_or(f64::NAN));
            new_y.push(y.get(row).unwrap_or(f64::NAN));
            continue;
        };
        let span = high - low;
        let width = placement
            .jitter_width
            .unwrap_or(span / 4.0 * placement.range_scale);
        let height = placement
            .jitter_height
            .unwrap_or(heights[data.group_of(row) as usize]);
        let dx = (2.0 * random.next_double() - 1.0) * width;
        let dy = (2.0 * random.next_double() - 1.0) * height;
        new_x.push(center + placement.side.sign() * span / 4.0 + dx);
        new_y.push(value + dy);
    }
    let mut data = data.clone();
    let continuous = |values| Column::Continuous(ContinuousColumn::from_values(values));
    data.insert(Aesthetic::X, continuous(new_x))?;
    data.insert(Aesthetic::Y, continuous(new_y))?;
    Ok(data)
}
