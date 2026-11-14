//! Enter and exit effects (gganimate `enter_fade`, `exit_fade`, `enter_grow`,
//! `exit_shrink`; MIT): how a row that has no partner in the other state looks
//! at the start of its entrance or the end of its exit.
//!
//! As tweenr's `tween_state`, an entering row starts from its effect state and
//! moves to the row as it is over the transition, and an exiting row moves
//! from the row as it is to its effect state ([`super::tween`] does the moving).
//! Effects added with `+` apply one after the other (gganimate's factory
//! list). Without an effect, rows appear and disappear at the state boundaries
//! (gganimate's default `enter_appear()`/`exit_disappear()`). This module only
//! changes rows; it does not plan or tween.
//!
//! Per geom, as gganimate (`transmuters.R`):
//! - fade: rows with an `alpha` get `alpha`; rows without one (NaN) get their
//!   `color` and `fill` with that alpha. `geom_smooth` fades its line color
//!   always.
//! - grow / shrink: `size` and `stroke` scaled by `size`, only where `size` is
//!   the mark size (ggplot2's `size`; ggrust's line widths are ggplot2's
//!   `linewidth`, which gganimate does not scale); bars scale `y` and `ymax`;
//!   paths, lines, steps, polygons and violins move to their group's mean
//!   point; boxplots close around `middle`.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use super::spec::{as_object, check_keys, shown};
use crate::build::frame::LayerFrame;
use crate::data::{Column, ContinuousColumn};
use crate::scene::{Color, PerItem};
use crate::spec::Aesthetic;
use crate::style::StyleValues;
use crate::suggest::did_you_mean;
use crate::{Error, Result};

/// One enter or exit effect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Effect {
    /// `enter_fade(alpha)` / `exit_fade(alpha)`.
    Fade { alpha: f64 },
    /// `enter_grow(size)` / `exit_shrink(size)`.
    Grow { size: f64 },
}

/// The plot's enter and exit effects, in the order they were added.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Transmuters {
    pub enter: Vec<Effect>,
    pub exit: Vec<Effect>,
}

const ENTER_NAMES: &[&str] = &["fade", "grow"];
const EXIT_NAMES: &[&str] = &["fade", "shrink"];

impl Transmuters {
    /// Parse the `animation.enter` and `animation.exit` lists
    /// (`[{"name": "fade", "alpha": 0}, {"name": "grow", "size": 0}]`).
    ///
    /// # Errors
    ///
    /// A spec error naming the entry for an unknown effect, an unknown option or
    /// a value out of range.
    pub(crate) fn parse(enter: Option<&Value>, exit: Option<&Value>) -> Result<Transmuters> {
        Ok(Transmuters {
            enter: parse_effects(enter, "animation.enter", ENTER_NAMES)?,
            exit: parse_effects(exit, "animation.exit", EXIT_NAMES)?,
        })
    }

    /// Whether rows that enter are drawn while they move in.
    pub(crate) fn has_enter(&self) -> bool {
        !self.enter.is_empty()
    }

    /// Whether rows that exit are drawn while they move out.
    pub(crate) fn has_exit(&self) -> bool {
        !self.exit.is_empty()
    }
}

fn parse_effects(value: Option<&Value>, path: &str, names: &[&str]) -> Result<Vec<Effect>> {
    let entries = match value {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(Value::Array(entries)) => entries,
        Some(other) => {
            return Err(Error::spec(
                path,
                format!("expected a list of effects, got {}", shown(Some(other))),
            ));
        }
    };
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let path = format!("{path}[{index}]");
            let object = as_object(entry, &path)?;
            let name = match object.get("name") {
                Some(Value::String(name)) => name.as_str(),
                other => {
                    return Err(Error::spec(
                        format!("{path}.name"),
                        format!("expected the effect name as a string, got {}", shown(other)),
                    ));
                }
            };
            match name {
                "fade" => {
                    check_keys(object, &path, &["name", "alpha"])?;
                    let alpha = number(object, &path, "alpha", 0.0)?;
                    if !(0.0..=1.0).contains(&alpha) {
                        return Err(Error::spec(
                            format!("{path}.alpha"),
                            format!("alpha must be between 0 and 1, got {alpha}"),
                        ));
                    }
                    Ok(Effect::Fade { alpha })
                }
                "grow" | "shrink" if names.contains(&name) => {
                    check_keys(object, &path, &["name", "size"])?;
                    let size = number(object, &path, "size", 0.0)?;
                    if size < 0.0 {
                        return Err(Error::spec(
                            format!("{path}.size"),
                            format!(
                                "size must be 0 or more (a share of the full size), got {size}"
                            ),
                        ));
                    }
                    Ok(Effect::Grow { size })
                }
                other => Err(Error::spec(
                    format!("{path}.name"),
                    format!(
                        "unknown effect '{other}'{}; accepted: {}",
                        did_you_mean(other, names.iter().copied()),
                        names.join(", ")
                    ),
                )),
            }
        })
        .collect()
}

/// A finite number option, `default` when absent.
pub(crate) fn number(
    object: &Map<String, Value>,
    path: &str,
    key: &str,
    default: f64,
) -> Result<f64> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Number(value)) => value
            .as_f64()
            .filter(|value| value.is_finite())
            .ok_or_else(|| Error::spec(format!("{path}.{key}"), "expected a finite number")),
        other => Err(Error::spec(
            format!("{path}.{key}"),
            format!("expected a number, got {}", shown(other)),
        )),
    }
}

/// Geoms whose `size` is the mark size (ggplot2's `size` column). In the others
/// ggrust's `size` is a line width, ggplot2's `linewidth`.
pub(crate) const MARK_SIZE_GEOMS: &[&str] = &[
    "point",
    "jitter",
    "count",
    "text",
    "label",
    "text_repel",
    "label_repel",
    "pointrange",
    "beeswarm",
    "quasirandom",
    "sina",
    "qq",
    "half_point",
    "pointdensity",
    "lollipop",
];

/// Apply `effects`, one after the other, to the rows at `rows` of `frame`, a
/// layer drawn by `geom`.
///
/// # Errors
///
/// An internal error when a column or style cannot be replaced (never for one
/// prepared layer).
pub(crate) fn apply(
    frame: &mut LayerFrame,
    rows: &[usize],
    effects: &[Effect],
    geom: &str,
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    for effect in effects {
        match *effect {
            Effect::Fade { alpha } => fade(frame, rows, alpha, geom == "smooth")?,
            Effect::Grow { size } => grow(frame, rows, size, geom)?,
        }
    }
    Ok(())
}

/// gganimate `transmute_fade`: `alpha` on rows with an alpha, `color` and
/// `fill` with that alpha on the others (`smooth`: `color` always).
fn fade(frame: &mut LayerFrame, rows: &[usize], alpha: f64, smooth: bool) -> Result<()> {
    let alphas = frame.styles.numbers(Aesthetic::Alpha).ok().cloned();
    let no_alpha: Vec<bool> = rows
        .iter()
        .map(|row| {
            alphas
                .as_ref()
                .is_none_or(|alphas| alphas.get(*row).is_nan())
        })
        .collect();
    if let Some(alphas) = alphas {
        let changed: Vec<(usize, f64)> = rows
            .iter()
            .zip(&no_alpha)
            .filter(|(_, no_alpha)| !**no_alpha)
            .map(|(row, _)| (*row, alpha))
            .collect();
        let values = set_rows(&alphas, frame.data.row_count(), &changed);
        frame
            .styles
            .insert(Aesthetic::Alpha, StyleValues::Numbers(values))?;
    }
    let channel = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
    for aesthetic in [Aesthetic::Color, Aesthetic::Fill] {
        let Ok(colors) = frame.styles.colors(aesthetic).cloned() else {
            continue;
        };
        let every_row = smooth && aesthetic == Aesthetic::Color;
        let changed: Vec<(usize, Color)> = rows
            .iter()
            .zip(&no_alpha)
            .filter(|(_, no_alpha)| every_row || **no_alpha)
            // A fully transparent color is "no color" (R's NA): it stays so.
            .filter(|(row, _)| colors.get(**row).alpha > 0)
            .map(|(row, _)| {
                let color = colors.get(*row);
                (
                    *row,
                    Color::rgba(color.red, color.green, color.blue, channel),
                )
            })
            .collect();
        let values = set_rows(&colors, frame.data.row_count(), &changed);
        frame
            .styles
            .insert(aesthetic, StyleValues::Colors(values))?;
    }
    Ok(())
}

/// gganimate `transmute_grow`.
fn grow(frame: &mut LayerFrame, rows: &[usize], size: f64, geom: &str) -> Result<()> {
    match geom {
        "bar" | "col" | "histogram" => {
            for aesthetic in [Aesthetic::Y, Aesthetic::Ymax] {
                scale_column(frame, aesthetic, rows, |value, _| value * size)?;
            }
        }
        "path" | "line" | "step" | "polygon" | "violin" => shrink_to_mean(frame, rows, size)?,
        "boxplot" => {
            let Some(middle) = frame.data.continuous(Aesthetic::Middle).cloned() else {
                return Ok(());
            };
            for aesthetic in [
                Aesthetic::Ymin,
                Aesthetic::Lower,
                Aesthetic::Upper,
                Aesthetic::Ymax,
            ] {
                scale_column(frame, aesthetic, rows, |value, row| match middle.get(row) {
                    Some(middle) => middle + size * (value - middle),
                    None => value,
                })?;
            }
        }
        _ if MARK_SIZE_GEOMS.contains(&geom) => {
            for aesthetic in [Aesthetic::Size, Aesthetic::Stroke] {
                let Ok(values) = frame.styles.numbers(aesthetic).cloned() else {
                    continue;
                };
                let changed: Vec<(usize, f64)> = rows
                    .iter()
                    .map(|row| (*row, values.get(*row) * size))
                    .collect();
                let values = set_rows(&values, frame.data.row_count(), &changed);
                frame
                    .styles
                    .insert(aesthetic, StyleValues::Numbers(values))?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// gganimate `shrink_multielement`: the rows' `x` and `y` moved `size` of the
/// way from their group's mean (over these rows) to where they are.
fn shrink_to_mean(frame: &mut LayerFrame, rows: &[usize], size: f64) -> Result<()> {
    for aesthetic in [Aesthetic::X, Aesthetic::Y] {
        let Some(column) = frame.data.continuous(aesthetic) else {
            continue;
        };
        // R `mean` over the group's values (NA propagates).
        let mut sums: BTreeMap<u32, (f64, usize)> = BTreeMap::new();
        for row in rows {
            let entry = sums.entry(frame.data.group_of(*row)).or_insert((0.0, 0));
            entry.0 += column.get(*row).unwrap_or(f64::NAN);
            entry.1 += 1;
        }
        let means: BTreeMap<u32, f64> = sums
            .into_iter()
            .map(|(group, (sum, count))| (group, sum / count as f64))
            .collect();
        let groups: Vec<u32> = (0..frame.data.row_count())
            .map(|row| frame.data.group_of(row))
            .collect();
        scale_column(frame, aesthetic, rows, |value, row| {
            let mean = means.get(&groups[row]).copied().unwrap_or(f64::NAN);
            if size == 0.0 {
                mean
            } else {
                mean + size * (value - mean)
            }
        })?;
    }
    Ok(())
}

/// Replace the continuous column of `variable` at `rows` by `change(value, row)`.
fn scale_column(
    frame: &mut LayerFrame,
    variable: Aesthetic,
    rows: &[usize],
    change: impl Fn(f64, usize) -> f64,
) -> Result<()> {
    let Some(column) = frame.data.continuous(variable) else {
        return Ok(());
    };
    let mut values: Vec<Option<f64>> = (0..column.len()).map(|row| column.get(row)).collect();
    for row in rows {
        if let Some(Some(value)) = values.get(*row).copied() {
            values[*row] = Some(change(value, *row));
        }
    }
    frame.data.insert(
        variable,
        Column::Continuous(ContinuousColumn::from_options(values)),
    )
}

/// `items` (for `count` rows) with the given rows replaced.
pub(crate) fn set_rows<T: Clone>(
    items: &PerItem<T>,
    count: usize,
    changed: &[(usize, T)],
) -> PerItem<T> {
    if changed.is_empty() {
        return items.clone();
    }
    let mut values: Vec<T> = match items {
        PerItem::Uniform(value) => vec![value.clone(); count],
        PerItem::Varying(values) => values.clone(),
    };
    for (row, value) in changed {
        if let Some(slot) = values.get_mut(*row) {
            *slot = value.clone();
        }
    }
    PerItem::Varying(values)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::data::LayerData;
    use crate::style::Styles;

    fn points(alphas: PerItem<f64>) -> LayerFrame {
        let mut data = LayerData::new(2);
        data.insert(
            Aesthetic::X,
            Column::Continuous(ContinuousColumn::from_values(vec![1.0, 2.0])),
        )
        .unwrap();
        let mut styles = Styles::default();
        let style = |aesthetic, values| (aesthetic, values);
        for (aesthetic, values) in [
            style(Aesthetic::Alpha, StyleValues::Numbers(alphas)),
            style(Aesthetic::Size, StyleValues::Numbers(PerItem::Uniform(3.0))),
            style(
                Aesthetic::Stroke,
                StyleValues::Numbers(PerItem::Uniform(0.75)),
            ),
            style(
                Aesthetic::Color,
                StyleValues::Colors(PerItem::Uniform(Color::rgb(255, 0, 0))),
            ),
        ] {
            styles.insert(aesthetic, values).unwrap();
        }
        LayerFrame { data, styles }
    }

    #[test]
    fn effects_parse_with_gganimate_defaults_and_typed_errors() {
        let parsed = Transmuters::parse(
            Some(&json!([{"name": "fade"}, {"name": "grow", "size": 0.5}])),
            Some(&json!([{"name": "shrink"}])),
        )
        .unwrap();
        assert_eq!(
            parsed.enter,
            vec![Effect::Fade { alpha: 0.0 }, Effect::Grow { size: 0.5 }]
        );
        assert_eq!(parsed.exit, vec![Effect::Grow { size: 0.0 }]);
        let error = Transmuters::parse(Some(&json!([{"name": "fdae"}])), None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("animation.enter[0].name"), "{error}");
        assert!(error.contains("did you mean 'fade'"), "{error}");
        let error = Transmuters::parse(None, Some(&json!([{"name": "grow"}])))
            .unwrap_err()
            .to_string();
        assert!(error.contains("accepted: fade, shrink"), "{error}");
        let error = Transmuters::parse(Some(&json!([{"name": "fade", "alpha": 2}])), None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("between 0 and 1"), "{error}");
    }

    #[test]
    fn fade_sets_alpha_or_the_color_alpha() {
        // Row 0 has an alpha, row 1 has none (NaN): its color gets the alpha.
        let mut frame = points(PerItem::Varying(vec![0.8, f64::NAN]));
        apply(&mut frame, &[0, 1], &[Effect::Fade { alpha: 0.2 }], "point").unwrap();
        let alphas = frame.styles.numbers(Aesthetic::Alpha).unwrap();
        assert_eq!(alphas.get(0), 0.2);
        assert!(alphas.get(1).is_nan());
        let colors = frame.styles.colors(Aesthetic::Color).unwrap();
        assert_eq!(colors.get(0), Color::rgb(255, 0, 0));
        assert_eq!(colors.get(1), Color::rgba(255, 0, 0, 51));
        // No color (fully transparent) stays invisible.
        let mut frame = points(PerItem::Uniform(f64::NAN));
        frame
            .styles
            .insert(
                Aesthetic::Color,
                StyleValues::Colors(PerItem::Uniform(Color::rgba(0, 0, 0, 0))),
            )
            .unwrap();
        apply(&mut frame, &[0], &[Effect::Fade { alpha: 0.5 }], "point").unwrap();
        assert_eq!(
            frame.styles.colors(Aesthetic::Color).unwrap().get(0),
            Color::rgba(0, 0, 0, 0)
        );
    }

    #[test]
    fn grow_scales_mark_sizes_only() {
        let mut frame = points(PerItem::Uniform(f64::NAN));
        apply(&mut frame, &[1], &[Effect::Grow { size: 0.5 }], "point").unwrap();
        let sizes = frame.styles.numbers(Aesthetic::Size).unwrap();
        assert_eq!((sizes.get(0), sizes.get(1)), (3.0, 1.5));
        assert_eq!(
            frame.styles.numbers(Aesthetic::Stroke).unwrap().get(1),
            0.375
        );
        // A line width is ggplot2's linewidth: unchanged.
        let mut frame = points(PerItem::Uniform(f64::NAN));
        apply(&mut frame, &[1], &[Effect::Grow { size: 0.0 }], "rect").unwrap();
        assert_eq!(frame.styles.numbers(Aesthetic::Size).unwrap().get(1), 3.0);
    }

    #[test]
    fn paths_shrink_to_their_group_mean() {
        let mut data = LayerData::new(3);
        let column = |values: Vec<f64>| Column::Continuous(ContinuousColumn::from_values(values));
        data.insert(Aesthetic::X, column(vec![0.0, 2.0, 9.0]))
            .unwrap();
        data.insert(Aesthetic::Y, column(vec![1.0, 3.0, 9.0]))
            .unwrap();
        data.set_group_index(Some(vec![0, 0, 1])).unwrap();
        let mut frame = LayerFrame {
            data,
            styles: Styles::default(),
        };
        apply(&mut frame, &[0, 1], &[Effect::Grow { size: 0.0 }], "line").unwrap();
        assert_eq!(
            frame.data.continuous(Aesthetic::X).unwrap().values(),
            &[1.0, 1.0, 9.0]
        );
        assert_eq!(
            frame.data.continuous(Aesthetic::Y).unwrap().values(),
            &[2.0, 2.0, 9.0]
        );
    }
}
