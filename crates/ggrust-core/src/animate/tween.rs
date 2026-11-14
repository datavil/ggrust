//! Tweening one layer between two states (tweenr `tween_state`, MIT), as
//! gganimate's `transition_states` does on the built layer data.
//!
//! Rows are matched by `group`: the k-th row of a group in the first state
//! moves to the k-th row of the same group in the second (tweenr's
//! `<id>_<occurrence>` rule). Matched rows are drawn in the first state's order;
//! rows without a partner are not drawn while the rows move (tweenr with no
//! `enter`/`exit`: they show in the states themselves, so they appear and
//! disappear at the state boundaries). Values move by their column's ease
//! ([`Easing`]): numbers linearly, colors in CIE Lab with alpha (tweenr via
//! farver), anything discrete (categories, shapes, line types, texts) switches
//! once the eased progress reaches one half. It does not plan frames
//! ([`super::states`]) or draw.

use std::collections::HashMap;

use super::ease::Easing;
use crate::build::frame::LayerFrame;
use crate::data::{Column, ContinuousColumn, Variable};
use crate::scene::{Color, PerItem};
use crate::style::{StyleValues, Styles};
use crate::{Error, Result};

/// Row pairs `(from row, to row)` matched by group and occurrence, in the
/// order of `from`.
pub(crate) fn match_rows(from: &LayerFrame, to: &LayerFrame) -> Vec<(u32, u32)> {
    let mut seen: HashMap<u32, u32> = HashMap::new();
    let mut to_rows: HashMap<(u32, u32), u32> = HashMap::new();
    for row in 0..to.data.row_count() {
        let group = to.data.group_of(row);
        let occurrence = seen.entry(group).or_insert(0);
        to_rows.insert((group, *occurrence), row as u32);
        *occurrence += 1;
    }
    seen.clear();
    let mut pairs = Vec::new();
    for row in 0..from.data.row_count() {
        let group = from.data.group_of(row);
        let occurrence = seen.entry(group).or_insert(0);
        if let Some(to_row) = to_rows.get(&(group, *occurrence)) {
            pairs.push((row as u32, *to_row));
        }
        *occurrence += 1;
    }
    pairs
}

/// Groups whose row counts differ between the two states while both have
/// more than one row (a line or polygon whose points do not all have a
/// partner), as `(group, from rows, to rows)`.
pub(crate) fn uneven_groups(from: &LayerFrame, to: &LayerFrame) -> Vec<(u32, usize, usize)> {
    let count = |frame: &LayerFrame| {
        let mut counts: HashMap<u32, usize> = HashMap::new();
        for row in 0..frame.data.row_count() {
            *counts.entry(frame.data.group_of(row)).or_insert(0) += 1;
        }
        counts
    };
    let (from_counts, to_counts) = (count(from), count(to));
    let mut uneven: Vec<(u32, usize, usize)> = from_counts
        .iter()
        .filter_map(|(group, from_rows)| {
            let to_rows = *to_counts.get(group)?;
            (*from_rows > 1 && to_rows > 1 && *from_rows != to_rows)
                .then_some((*group, *from_rows, to_rows))
        })
        .collect();
    uneven.sort_unstable();
    uneven
}

/// The rows of `from` moved `progress` (0 to 1, not eased) of the way to their
/// partners in `to`.
///
/// # Errors
///
/// An internal error when the two states do not have the same columns and
/// styles (they come from one prepared layer, so they always do).
pub(crate) fn tween(
    from: &LayerFrame,
    to: &LayerFrame,
    progress: f64,
    easing: &Easing,
) -> Result<LayerFrame> {
    let pairs = match_rows(from, to);
    let from_rows: Vec<u32> = pairs.iter().map(|(row, _)| *row).collect();
    let to_rows: Vec<u32> = pairs.iter().map(|(_, row)| *row).collect();
    let start = from.take(&from_rows);
    let end = to.take(&to_rows);
    let mut data = start.data.clone();
    for (variable, column) in start.data.columns() {
        let name = match variable {
            Variable::Aesthetic(aesthetic) => aesthetic.as_str(),
            Variable::Computed(computed) => computed.as_str(),
        };
        let eased = easing.ease(name).apply(progress);
        let tweened = match (column, end.data.get(variable)) {
            (Column::Continuous(start), Some(Column::Continuous(end))) => {
                Column::Continuous(ContinuousColumn::from_options(
                    (0..start.len())
                        .map(|row| match (start.get(row), end.get(row)) {
                            (Some(start), Some(end)) => Some(mix_number(start, end, eased)),
                            _ => None,
                        })
                        .collect(),
                ))
            }
            (Column::Discrete(start), Some(Column::Discrete(end)))
                if eased >= 0.5 && start.levels() == end.levels() =>
            {
                let codes: Vec<Option<u32>> = (0..end.len()).map(|row| end.code(row)).collect();
                Column::Discrete(start.with_codes(&codes))
            }
            (Column::Discrete(_), Some(Column::Discrete(_))) => continue,
            _ => {
                return Err(Error::data(format!(
                    "internal: column '{variable}' differs between two animation states"
                )));
            }
        };
        data.insert(variable, tweened)?;
    }
    let mut styles = Styles::default();
    for (aesthetic, values) in start.styles.iter() {
        let eased = easing.ease(aesthetic.as_str()).apply(progress);
        let other = end.styles.iter().find(|(other, _)| *other == aesthetic);
        let tweened = match (values, other.map(|(_, values)| values)) {
            (StyleValues::Numbers(start), Some(StyleValues::Numbers(end))) => {
                StyleValues::Numbers(between(start, end, |start, end| {
                    mix_number(start, end, eased)
                }))
            }
            (StyleValues::Colors(start), Some(StyleValues::Colors(end))) => {
                StyleValues::Colors(between(start, end, |start, end| mix_lab(start, end, eased)))
            }
            (values, Some(end)) if eased >= 0.5 && values.kind() == end.kind() => end.clone(),
            (values, Some(end)) if values.kind() == end.kind() => values.clone(),
            _ => {
                return Err(Error::data(format!(
                    "internal: style '{aesthetic}' differs between two animation states"
                )));
            }
        };
        styles.insert(aesthetic, tweened)?;
    }
    Ok(LayerFrame { data, styles })
}

/// `start` moved `share` of the way to `end`. Equal ends stay as they are, so
/// an infinite bound (`xmin=-Inf` in both states) does not become NaN (R's
/// `-Inf + p * (Inf - Inf)` is NaN and drops the row).
fn mix_number(start: f64, end: f64, share: f64) -> f64 {
    if start == end {
        start
    } else {
        start + share * (end - start)
    }
}

/// Per row `mix(start, end)`; uniform values stay uniform.
fn between<T: Clone + PartialEq>(
    start: &PerItem<T>,
    end: &PerItem<T>,
    mix: impl Fn(T, T) -> T,
) -> PerItem<T> {
    match (start, end) {
        (PerItem::Uniform(start), PerItem::Uniform(end)) if start == end => {
            PerItem::Uniform(start.clone())
        }
        (PerItem::Uniform(start), PerItem::Uniform(end)) => {
            PerItem::Uniform(mix(start.clone(), end.clone()))
        }
        _ => {
            let count = match (start, end) {
                (PerItem::Varying(values), _) | (_, PerItem::Varying(values)) => values.len(),
                _ => 0,
            };
            let at = |items: &PerItem<T>, row: usize| match items {
                PerItem::Uniform(value) => value.clone(),
                PerItem::Varying(values) => values[row].clone(),
            };
            PerItem::Varying(
                (0..count)
                    .map(|row| mix(at(start, row), at(end, row)))
                    .collect(),
            )
        }
    }
}

/// `start` moved `share` of the way to `end` in CIE Lab (D65), alpha linearly
/// (tweenr via farver's `decode_colour(to = "lab")`).
pub(crate) fn mix_lab(start: Color, end: Color, share: f64) -> Color {
    if start == end {
        return start;
    }
    let (from, to) = (to_lab(start), to_lab(end));
    let mix = |from: f64, to: f64| from + share * (to - from);
    let lab = [
        mix(from[0], to[0]),
        mix(from[1], to[1]),
        mix(from[2], to[2]),
    ];
    let [red, green, blue] = from_lab(lab);
    let alpha = mix(f64::from(start.alpha), f64::from(end.alpha));
    Color::rgba(red, green, blue, channel(alpha))
}

/// D65 reference white (farver / ColorSpace).
const WHITE: [f64; 3] = [0.95047, 1.0, 1.08883];

fn to_lab(color: Color) -> [f64; 3] {
    let linear = |channel: u8| {
        let value = f64::from(channel) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    let (red, green, blue) = (linear(color.red), linear(color.green), linear(color.blue));
    let x = 0.4124564 * red + 0.3575761 * green + 0.1804375 * blue;
    let y = 0.2126729 * red + 0.7151522 * green + 0.0721750 * blue;
    let z = 0.0193339 * red + 0.1191920 * green + 0.9503041 * blue;
    let f = |value: f64| {
        if value > 0.008856 {
            value.cbrt()
        } else {
            (903.3 * value + 16.0) / 116.0
        }
    };
    let (fx, fy, fz) = (f(x / WHITE[0]), f(y / WHITE[1]), f(z / WHITE[2]));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn from_lab(lab: [f64; 3]) -> [u8; 3] {
    let fy = (lab[0] + 16.0) / 116.0;
    let fx = lab[1] / 500.0 + fy;
    let fz = fy - lab[2] / 200.0;
    let inverse = |f: f64| {
        let cube = f * f * f;
        if cube > 0.008856 {
            cube
        } else {
            (116.0 * f - 16.0) / 903.3
        }
    };
    let x = inverse(fx) * WHITE[0];
    let y = inverse(fy) * WHITE[1];
    let z = inverse(fz) * WHITE[2];
    let red = 3.2404542 * x - 1.5371385 * y - 0.4985314 * z;
    let green = -0.9692660 * x + 1.8760108 * y + 0.0415560 * z;
    let blue = 0.0556434 * x - 0.2040259 * y + 1.0572252 * z;
    let gamma = |value: f64| {
        let value = if value <= 0.0031308 {
            12.92 * value
        } else {
            1.055 * value.powf(1.0 / 2.4) - 0.055
        };
        channel(value * 255.0)
    };
    [gamma(red), gamma(green), gamma(blue)]
}

/// A channel value rounded and clamped to 0..255.
fn channel(value: f64) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::data::{DiscreteColumn, LayerData};
    use crate::spec::Aesthetic;

    /// A frame with x, y per row, a group per row, and a color per row.
    fn frame(rows: &[(u32, f64, f64, Color)]) -> LayerFrame {
        let mut data = LayerData::new(rows.len());
        let column = |values: Vec<f64>| Column::Continuous(ContinuousColumn::from_values(values));
        data.insert(Aesthetic::X, column(rows.iter().map(|row| row.1).collect()))
            .unwrap();
        data.insert(Aesthetic::Y, column(rows.iter().map(|row| row.2).collect()))
            .unwrap();
        data.set_group_index(Some(rows.iter().map(|row| row.0).collect()))
            .unwrap();
        let mut styles = Styles::default();
        styles
            .insert(
                Aesthetic::Color,
                StyleValues::Colors(PerItem::Varying(rows.iter().map(|row| row.3).collect())),
            )
            .unwrap();
        styles
            .insert(Aesthetic::Size, StyleValues::Numbers(PerItem::Uniform(3.0)))
            .unwrap();
        LayerFrame { data, styles }
    }

    const RED: Color = Color::rgb(255, 0, 0);
    const BLUE: Color = Color::rgb(0, 0, 255);

    #[test]
    fn rows_match_by_group_and_occurrence() {
        let from = frame(&[(0, 0.0, 0.0, RED), (1, 1.0, 1.0, RED), (0, 2.0, 2.0, RED)]);
        let to = frame(&[
            (1, 5.0, 5.0, BLUE),
            (0, 6.0, 6.0, BLUE),
            (2, 7.0, 7.0, BLUE),
        ]);
        // Group 0: rows 0 -> 1 (first), row 2 has no second partner; group 1:
        // row 1 -> 0; group 2 only in `to`.
        assert_eq!(match_rows(&from, &to), vec![(0, 1), (1, 0)]);
        let tweened = tween(&from, &to, 0.5, &Easing::default()).unwrap();
        let x = tweened
            .data
            .continuous(Aesthetic::X)
            .unwrap()
            .values()
            .to_vec();
        assert_eq!(x, vec![3.0, 3.0]);
        assert_eq!(tweened.data.group_index(), Some(&[0, 1][..]));
    }

    #[test]
    fn eases_apply_per_column_and_styles_tween() {
        let from = frame(&[(0, 0.0, 0.0, RED)]);
        let to = frame(&[(0, 10.0, 10.0, BLUE)]);
        let easing = Easing::from_map(
            json!({"default": "linear", "aesthetics": {"y": "quadratic-in"}})
                .as_object()
                .unwrap(),
        )
        .unwrap();
        let tweened = tween(&from, &to, 0.5, &easing).unwrap();
        assert_eq!(
            tweened.data.continuous(Aesthetic::X).unwrap().values(),
            &[5.0]
        );
        assert_eq!(
            tweened.data.continuous(Aesthetic::Y).unwrap().values(),
            &[2.5]
        );
        let colors = tweened.styles.colors(Aesthetic::Color).unwrap();
        let PerItem::Varying(colors) = colors else {
            panic!("varying colors expected");
        };
        // Red to blue through Lab, halfway: R farver gives "#CA0088"
        // (`encode_colour(colMeans(decode_colour(c("red", "blue"), to = "lab")), from = "lab")`).
        assert_eq!(colors[0], Color::rgb(0xCA, 0x00, 0x88));
        assert_eq!(
            tweened.styles.numbers(Aesthetic::Size).unwrap(),
            &PerItem::Uniform(3.0)
        );
    }

    #[test]
    fn equal_infinite_values_stay_infinite() {
        let from = frame(&[(0, f64::NEG_INFINITY, 1.0, RED)]);
        let to = frame(&[(0, f64::NEG_INFINITY, 3.0, RED)]);
        let tweened = tween(&from, &to, 0.5, &Easing::default()).unwrap();
        let x = tweened.data.continuous(Aesthetic::X).unwrap().values()[0];
        assert_eq!(x, f64::NEG_INFINITY);
    }

    #[test]
    fn lab_round_trips_and_ends_are_exact() {
        for color in [
            RED,
            BLUE,
            Color::rgb(12, 200, 99),
            Color::rgb(250, 250, 250),
        ] {
            assert_eq!(mix_lab(color, Color::BLACK, 0.0), color);
            assert_eq!(mix_lab(Color::BLACK, color, 1.0), color);
        }
        let half = mix_lab(Color::rgba(0, 0, 0, 0), Color::rgba(0, 0, 0, 255), 0.5);
        assert_eq!(half.alpha, 128);
    }

    /// Colours mixed in Lab by R farver
    /// (`tests/fixtures/generate_animation_reference.R`).
    #[test]
    fn lab_mixes_match_farver() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/animation_reference.json"
        );
        let reference: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let hex = |text: &str| {
            let byte = |at: usize| u8::from_str_radix(&text[at..at + 2], 16).unwrap();
            let alpha = if text.len() == 9 { byte(7) } else { 255 };
            Color::rgba(byte(1), byte(3), byte(5), alpha)
        };
        let cases = reference["lab"].as_array().unwrap();
        assert!(cases.len() >= 5);
        for case in cases {
            let mixed = mix_lab(
                hex(case["from"].as_str().unwrap()),
                hex(case["to"].as_str().unwrap()),
                case["share"].as_f64().unwrap(),
            );
            assert_eq!(mixed, hex(case["colour"].as_str().unwrap()), "{case}");
        }
    }

    #[test]
    fn discrete_values_switch_at_one_half() {
        let mut from = frame(&[(0, 0.0, 0.0, RED)]);
        let mut to = frame(&[(0, 1.0, 1.0, RED)]);
        let levels = || vec!["a".to_owned(), "b".to_owned()];
        let column =
            |code: u32| Column::Discrete(DiscreteColumn::from_parts(levels(), vec![code].into()));
        from.data.insert(Aesthetic::Label, column(0)).unwrap();
        to.data.insert(Aesthetic::Label, column(1)).unwrap();
        let label = |progress: f64| {
            let tweened = tween(&from, &to, progress, &Easing::default()).unwrap();
            match tweened.data.get(Aesthetic::Label) {
                Some(Column::Discrete(column)) => column.label(0).map(str::to_owned),
                _ => None,
            }
        };
        assert_eq!(label(0.49).as_deref(), Some("a"));
        assert_eq!(label(0.5).as_deref(), Some("b"));
    }

    #[test]
    fn uneven_groups_are_reported() {
        let from = frame(&[(0, 0.0, 0.0, RED), (0, 1.0, 1.0, RED), (1, 0.0, 0.0, RED)]);
        let to = frame(&[
            (0, 0.0, 0.0, RED),
            (0, 1.0, 1.0, RED),
            (0, 2.0, 2.0, RED),
            (1, 5.0, 5.0, RED),
        ]);
        assert_eq!(uneven_groups(&from, &to), vec![(0, 2, 3)]);
    }
}
