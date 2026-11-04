//! `geom_pointinterval` (R package ggdist): a point with intervals, the interval
//! part of `stat_halfeye` / `stat_pointinterval`. Drawing is the point range's
//! (`interval.rs`): every line from `ymin` to `ymax`, then every point at `y`.
//!
//! Line widths follow ggdist: from the stat's `...width..`, the layer's widest
//! interval gets the first of `interval_size_range`, the narrowest the second,
//! others in between linearly (one level: the middle). A constant `linewidth`
//! replaces that. Points are `fatten_point` x the line width, or `fatten_point`
//! x a constant `size`. The defaults (0.78 to 1.81, fatten 2.5) are ggdist's
//! (0.6 to 1.4, 1.8) converted from ggplot2 to ggrust units, so plots look as
//! in R (deviations.md).
//!
//! What it does not do: `point_colour`/`interval_colour`-style part aesthetics,
//! a legend for the interval widths (ggdist hides it too).

use super::interval::{
    draw_point_range_key, draw_point_ranges, point_range_key_size, point_range_targets,
};
use super::{DrawLayer, Geom};
use crate::coord::Panel;
use crate::data::Computed;
use crate::parameters::Parameters;
use crate::scene::{Node, PerItem, Rect};
use crate::spec::{Aesthetic, Value};
use crate::style::{StyleDefault, StyleValues, Styles};
use crate::tooltip::{HoverTargets, TooltipBehavior};
use crate::{Error, Result};

/// ggdist interval style: ink lines and points, solid circles.
const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Color, StyleDefault::PEN),
    (Aesthetic::Fill, StyleDefault::PEN),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
    (Aesthetic::Shape, StyleDefault::Shape(16)),
    (Aesthetic::Size, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Stroke, StyleDefault::Number(0.75)),
    (Aesthetic::Linewidth, StyleDefault::Number(f64::NAN)),
];

/// ggdist's `fatten_point` (1.8) in ggrust units: the median dot of the
/// thickest line is as wide (10 px) as ggplot2 draws it.
const DEFAULT_FATTEN: f64 = 2.5;
/// ggdist's `interval_size_range` (0.6, 1.4) in ggrust units: ggplot2 draws a
/// line width of 1 at 2.845 px, ggrust (lets-plot) at 2.2 px.
const DEFAULT_SIZE_RANGE: (f64, f64) = (0.78, 1.81);
/// Line width without `...width..` (the middle of the size range).
const FALLBACK_LINEWIDTH: f64 = 1.295;

/// Points with intervals.
pub struct PointIntervalGeom;

impl Geom for PointIntervalGeom {
    fn name(&self) -> &'static str {
        "pointinterval"
    }

    fn default_stat(&self) -> &'static str {
        "pointinterval"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Ymin, Aesthetic::Ymax]
    }

    fn optional_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Y]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["fatten_point", "interval_size_range"]
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let (styles, fatten) = sized_styles(layer)?;
        draw_point_ranges(
            &DrawLayer {
                styles: &styles,
                ..*layer
            },
            panel,
            fatten,
        )
    }

    fn tooltip_behavior(&self) -> TooltipBehavior {
        TooltipBehavior {
            hidden: &[Aesthetic::Linewidth],
            ..TooltipBehavior::NEAREST_XY
        }
    }

    fn hover_targets(&self, layer: &DrawLayer, panel: &Panel) -> Result<Option<HoverTargets>> {
        let (styles, fatten) = sized_styles(layer)?;
        Ok(point_range_targets(
            &DrawLayer {
                styles: &styles,
                ..*layer
            },
            panel,
            fatten,
        ))
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        point_range_key_size(&key_styles(styles)?, DEFAULT_FATTEN)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        draw_point_range_key(&key_styles(styles)?, cell, DEFAULT_FATTEN)
    }
}

/// The layer's styles with per-row line widths and point sizes, and the point
/// fattening relative to `size`.
fn sized_styles(layer: &DrawLayer) -> Result<(Styles, f64)> {
    let parameters = &layer.parameters;
    let fatten = parameters.number("fatten_point")?.unwrap_or(DEFAULT_FATTEN);
    if fatten < 0.0 {
        return Err(Error::spec(
            parameters.path_of("fatten_point"),
            format!("fatten_point must be zero or positive, got {fatten}"),
        ));
    }
    let range = size_range(parameters)?;
    let row_count = layer.data.row_count();
    let constant =
        |aesthetic: Aesthetic| -> Result<Option<f64>> { parameters.number(aesthetic.as_str()) };
    let linewidths: Vec<f64> = match constant(Aesthetic::Linewidth)? {
        Some(width) => vec![width; row_count],
        None => interval_sizes(layer, range),
    };
    let mut styles = layer.styles.clone();
    let sizes = match constant(Aesthetic::Size)? {
        Some(size) => vec![size; row_count],
        None => linewidths.clone(),
    };
    styles.insert(
        Aesthetic::Linewidth,
        StyleValues::Numbers(PerItem::Varying(linewidths)),
    )?;
    styles.insert(
        Aesthetic::Size,
        StyleValues::Numbers(PerItem::Varying(sizes)),
    )?;
    Ok((styles, fatten))
}

/// ggdist: `-.width` over the layer's levels onto `range`, thickest for the
/// narrowest interval.
fn interval_sizes(layer: &DrawLayer, range: (f64, f64)) -> Vec<f64> {
    let row_count = layer.data.row_count();
    let Some(widths) = layer.data.continuous(Computed::IntervalWidth) else {
        return vec![FALLBACK_LINEWIDTH; row_count];
    };
    let finite: Vec<f64> = (0..row_count)
        .filter_map(|row| widths.get(row).filter(|value| value.is_finite()))
        .collect();
    let widest = finite.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let narrowest = finite.iter().copied().fold(f64::INFINITY, f64::min);
    (0..row_count)
        .map(|row| {
            let Some(width) = widths.get(row).filter(|value| value.is_finite()) else {
                return f64::NAN;
            };
            let share = if widest > narrowest {
                (widest - width) / (widest - narrowest)
            } else {
                0.5
            };
            range.0 + share * (range.1 - range.0)
        })
        .collect()
}

/// `interval_size_range`: two non-negative line widths, for the widest and the
/// narrowest interval.
fn size_range(parameters: &Parameters) -> Result<(f64, f64)> {
    let Some(value) = parameters.value("interval_size_range") else {
        return Ok(DEFAULT_SIZE_RANGE);
    };
    let pair =
        match value {
            Value::Array(items) if items.len() == 2 => items[0]
                .as_f64()
                .zip(items[1].as_f64())
                .filter(|(low, high)| {
                    low.is_finite() && high.is_finite() && *low >= 0.0 && *high >= 0.0
                }),
            _ => None,
        };
    pair.ok_or_else(|| {
        Error::spec(
            parameters.path_of("interval_size_range"),
            format!("expected two non-negative line widths, e.g. [0.6, 1.4], got {value}"),
        )
    })
}

/// Legend key styles: unset sizes become the middle line width.
fn key_styles(styles: &Styles) -> Result<Styles> {
    let mut styles = styles.clone();
    for aesthetic in [Aesthetic::Linewidth, Aesthetic::Size] {
        let value = styles.numbers(aesthetic)?.get(0);
        if !value.is_finite() {
            styles.insert(
                aesthetic,
                StyleValues::Numbers(PerItem::Uniform(FALLBACK_LINEWIDTH)),
            )?;
        }
    }
    Ok(styles)
}
