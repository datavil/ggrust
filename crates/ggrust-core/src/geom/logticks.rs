//! `annotation_logticks()`: log tick marks along the panel edges (ggplot2).
//!
//! Per power `base^n` in the panel's range, ticks at `k * base^n` for multiples
//! k from 1 to base - 1 (ggplot2 `calc_logticks`): long at 1, mid halfway (5 for
//! base 10), short elsewhere.
//! With `scaled` (the default) the axis is already in log units (`scale_x_log10()`,
//! or data logged by hand), so a tick sits at `n + log_base(k)`; without it the
//! axis is in data units (`coord_trans(x="log10")`) and a tick sits at
//! `k * base^n`. Drawn once per panel from the layer's first row styles, inside
//! the panel unless `outside`.
//!
//! What it does not do: draw anything without a continuous axis on the side.

use super::lines;
use super::{DrawLayer, Geom};
use crate::coord::Panel;
use crate::scale::PositionScale;
use crate::scene::{Color, Node, Path, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::{Error, Result};

/// ggplot2 `annotation_logticks()` defaults: black, linewidth 0.5.
const DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Color, StyleDefault::Color(Color::BLACK)),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Size, StyleDefault::Number(0.5)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

/// Pixels per centimetre at 96 dpi: ggplot2's tick lengths are 0.1, 0.2, 0.3 cm.
const PIXELS_PER_CM: f64 = 96.0 / 2.54;

/// No more decades than this per axis (a guard against absurd ranges).
const MAX_DECADES: f64 = 400.0;

/// Log tick marks.
pub struct LogticksGeom;

impl Geom for LogticksGeom {
    fn name(&self) -> &'static str {
        "logticks"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["base", "sides", "outside", "scaled", "short", "mid", "long"]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        DEFAULTS
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        if layer.data.row_count() == 0 {
            return Ok(Vec::new());
        }
        let parameters = &layer.parameters;
        let base = parameters.number("base")?.unwrap_or(10.0);
        if !(base.is_finite() && base > 1.0) {
            return Err(Error::spec(
                parameters.path_of("base"),
                format!("expected a log base above 1, got {base}"),
            ));
        }
        let sides = parameters.string("sides")?.unwrap_or("bl");
        if let Some(bad) = sides.chars().find(|side| !"trbl".contains(*side)) {
            return Err(Error::spec(
                parameters.path_of("sides"),
                format!(
                    "unknown side '{bad}' in \"{sides}\"; use any of 't' (top), 'r' (right), \
                     'b' (bottom), 'l' (left), e.g. \"bl\""
                ),
            ));
        }
        let outside = parameters.boolean("outside")?.unwrap_or(false);
        let scaled = parameters.boolean("scaled")?.unwrap_or(true);
        let length = |name: &str, default_cm: f64| -> Result<f64> {
            let value = parameters
                .number(name)?
                .unwrap_or(default_cm * PIXELS_PER_CM);
            if value.is_finite() && value >= 0.0 {
                Ok(value)
            } else {
                Err(Error::spec(
                    parameters.path_of(name),
                    format!("expected a tick length in pixels (0 or more), got {value}"),
                ))
            }
        };
        let lengths = [
            length("short", 0.1)?,
            length("mid", 0.2)?,
            length("long", 0.3)?,
        ];
        let Some(stroke) = lines::stroke_at(layer.styles, 0)? else {
            return Ok(Vec::new());
        };
        let rect = panel.rect;
        let mut nodes = Vec::new();
        for side in sides.chars() {
            let horizontal = matches!(side, 'b' | 't');
            let scale = if horizontal {
                panel.x_scale
            } else {
                panel.y_scale
            };
            let PositionScale::Continuous(continuous) = scale else {
                continue;
            };
            let domain = continuous.expanded_domain();
            let Some(ticks) = tick_values(domain, base, scaled) else {
                return Err(Error::spec(
                    parameters.path_of("sides"),
                    format!(
                        "the {} axis spans more than {MAX_DECADES} powers of {base}; \
                         log ticks need a narrower range (set limits) or a larger base",
                        if horizontal { "horizontal" } else { "vertical" }
                    ),
                ));
            };
            for (value, kind) in ticks {
                let length = if outside {
                    -lengths[kind]
                } else {
                    lengths[kind]
                };
                let (start, end) = if horizontal {
                    let x = panel.x_pixel(value);
                    let edge = if side == 'b' {
                        rect.y + rect.height
                    } else {
                        rect.y
                    };
                    let inward = if side == 'b' { -length } else { length };
                    ((x, edge), (x, edge + inward))
                } else {
                    let y = panel.y_pixel(value);
                    let edge = if side == 'l' {
                        rect.x
                    } else {
                        rect.x + rect.width
                    };
                    let inward = if side == 'l' { length } else { -length };
                    ((edge, y), (edge + inward, y))
                };
                if [start.0, start.1, end.0, end.1]
                    .iter()
                    .all(|v| v.is_finite())
                {
                    nodes.push(Node::Path(Path {
                        points: vec![start, end],
                        closed: false,
                        fill: None,
                        stroke: Some(stroke.clone()),
                    }));
                }
            }
        }
        Ok(nodes)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        lines::key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        lines::key_line(styles, cell, false)
    }
}

/// Tick positions inside `domain` (axis units) with their kind: 0 short,
/// 1 mid, 2 long (ggplot2 `calc_logticks`). Per power of `base`, `ceil(base - 1)`
/// multiples evenly spaced from 1 to `base - 1`: the first long, the one at
/// index `floor((base - 1) / 2)` mid (winning over long, so base 2 is all mid),
/// the rest short; plus the closing power, kind as the first. `None` when the
/// range spans more than [`MAX_DECADES`] powers.
fn tick_values(domain: (f64, f64), base: f64, scaled: bool) -> Option<Vec<(f64, usize)>> {
    let (low, high) = (domain.0.min(domain.1), domain.0.max(domain.1));
    // The powers covered, as exponents of `base`.
    let (first, last) = if scaled {
        (low.floor(), high.ceil())
    } else {
        if high <= 0.0 {
            return Some(Vec::new());
        }
        let low = low.max(high * f64::EPSILON).max(f64::MIN_POSITIVE);
        (low.log(base).floor(), high.log(base).ceil())
    };
    if !(first.is_finite() && last.is_finite()) {
        return Some(Vec::new());
    }
    if last - first > MAX_DECADES {
        return None;
    }
    let per_base = base - 1.0;
    let count = (per_base.ceil() as usize).max(1);
    let multiples: Vec<f64> = if count == 1 {
        vec![1.0]
    } else {
        (0..count)
            .map(|index| 1.0 + (base - 2.0) * index as f64 / (count - 1) as f64)
            .collect()
    };
    let mid_index = (per_base / 2.0).floor();
    let kind_of = |multiple: f64| {
        let cycle = multiple - 1.0;
        if (cycle - mid_index).abs() < 1e-9 {
            1
        } else if cycle.abs() < 1e-9 {
            2
        } else {
            0
        }
    };
    let at = |multiple: f64, exponent: f64| {
        if scaled {
            exponent + multiple.log(base)
        } else {
            multiple * base.powf(exponent)
        }
    };
    let mut ticks = Vec::new();
    let mut exponent = first;
    while exponent < last {
        for &multiple in &multiples {
            ticks.push((at(multiple, exponent), kind_of(multiple)));
        }
        exponent += 1.0;
    }
    ticks.push((at(1.0, last), kind_of(1.0)));
    ticks.retain(|(value, _)| *value >= low && *value <= high);
    Some(ticks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaled_ticks_sit_at_log_multiples() {
        let ticks = tick_values((0.0, 1.0), 10.0, true).unwrap();
        let values: Vec<f64> = ticks.iter().map(|(value, _)| *value).collect();
        assert_eq!(values.len(), 10);
        assert_eq!(ticks[0], (0.0, 2));
        assert!((values[1] - 2f64.log10()).abs() < 1e-12);
        assert_eq!(ticks[4].1, 1, "5 is a mid tick");
        assert_eq!(ticks[9], (1.0, 2));
    }

    #[test]
    fn unscaled_ticks_are_in_data_units() {
        let ticks = tick_values((1.0, 100.0), 10.0, false).unwrap();
        let values: Vec<f64> = ticks.iter().map(|(value, _)| *value).collect();
        assert_eq!(values[..3], [1.0, 2.0, 3.0]);
        assert!(values.contains(&50.0) && values.contains(&100.0));
        assert_eq!(values.len(), 19);
    }

    #[test]
    fn other_bases_follow_ggplot2() {
        // calc_logticks: base 2 is all mid; base 3 has 1 long, 2 mid.
        let two = tick_values((0.0, 3.0), 2.0, true).unwrap();
        assert_eq!(two, [(0.0, 1), (1.0, 1), (2.0, 1), (3.0, 1)]);
        let three = tick_values((0.0, 1.0), 3.0, true).unwrap();
        assert_eq!(
            three.iter().map(|(_, kind)| *kind).collect::<Vec<_>>(),
            [2, 1, 2]
        );
        // Base 100: the mid tick is 50.
        let hundred = tick_values((1.0, 100.0), 100.0, false).unwrap();
        assert!(hundred.contains(&(50.0, 1)));
        // Base e: multiples 1 and e - 1.
        let e = std::f64::consts::E;
        let natural = tick_values((1.0, e), e, false).unwrap();
        assert_eq!(natural.len(), 3);
        assert!((natural[1].0 - (e - 1.0)).abs() < 1e-12 && natural[1].1 == 0);
    }

    #[test]
    fn too_many_powers_is_none() {
        assert!(tick_values((-300.0, 300.0), 10.0, true).is_none());
    }

    #[test]
    fn negative_unscaled_ranges_have_no_ticks() {
        assert!(tick_values((-5.0, -1.0), 10.0, false).unwrap().is_empty());
    }
}
