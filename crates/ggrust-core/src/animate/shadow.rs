//! Shadows (gganimate `shadow_wake`, `shadow_mark`; MIT): rows of other frames
//! drawn behind a frame's rows.
//!
//! - `shadow_wake`: the rows of the previous `round(nframes * wake_length)`
//!   frames, oldest first, each step `at = k / wake_length` (0 for the oldest)
//!   on the way from the wake's end values (`size`, `alpha`, `colour`, `fill`)
//!   to the row's own, eased by `falloff`. Only rows whose element is drawn in
//!   the frame, and not in an excluded phase (`enter`, `exit` by default), keep
//!   a wake; each element's wake is drawn right before it.
//! - `shadow_mark`: the rows of the frames that show a state as it is (the
//!   last frame of each state, tweenr's `raw` phase) before (`past`) and/or
//!   after (`future`) the frame, with constant style overrides.
//!
//! Static layers (without the state column) and `exclude_layer` layers
//! (1-based, as gganimate) never get a shadow. Shadow rows get group numbers of
//! their own, so a line's wake is separate lines. This module only composes
//! rows; [`super::Animation`] gives it the frames.

use std::collections::HashMap;

use serde_json::{Map, Value};

use super::ease::Ease;
use super::spec::{as_object, check_keys, shown};
use super::transmute::{MARK_SIZE_GEOMS, number, set_rows};
use super::tween::{Drawn, Phase, RowKey, mix_lab};
use crate::build::frame::LayerFrame;
use crate::data::SourceRows;
use crate::scene::{Color, PerItem};
use crate::spec::Aesthetic;
use crate::style::{StyleValues, Styles, parse_constant};
use crate::suggest::did_you_mean;
use crate::{Error, Result};

const PATH: &str = "animation.shadow";
const NAMES: &[&str] = &["wake", "mark"];
const WAKE_KEYS: &[&str] = &[
    "name",
    "wake_length",
    "size",
    "alpha",
    "colour",
    "fill",
    "falloff",
    "wrap",
    "exclude_layer",
    "exclude_phase",
];
const MARK_KEYS: &[&str] = &["name", "past", "future", "exclude_layer", "aesthetics"];

/// The plot's shadow.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Shadow {
    Wake(Wake),
    Mark(Mark),
}

/// A parsed `shadow_wake()`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Wake {
    pub wake_length: f64,
    /// End value of `size` and `stroke` (`None`: unchanged).
    pub size: Option<f64>,
    /// End value of `alpha` (`None`: unchanged).
    pub alpha: Option<f64>,
    pub colour: Option<Color>,
    pub fill: Option<Color>,
    pub falloff: Ease,
    pub wrap: bool,
    pub exclude_layer: Vec<usize>,
    pub exclude_phase: Vec<Phase>,
}

/// A parsed `shadow_mark()`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Mark {
    pub past: bool,
    pub future: bool,
    pub exclude_layer: Vec<usize>,
    /// Constant style overrides, e.g. `alpha = 0.3`, as given.
    pub aesthetics: Vec<(Aesthetic, Value)>,
}

impl Shadow {
    /// Parse the `animation.shadow` object.
    ///
    /// # Errors
    ///
    /// A spec error naming the option for an unknown shadow, option or value.
    pub(crate) fn parse(value: &Value) -> Result<Shadow> {
        let object = as_object(value, PATH)?;
        let name = match object.get("name") {
            Some(Value::String(name)) => name.as_str(),
            other => {
                return Err(Error::spec(
                    format!("{PATH}.name"),
                    format!("expected the shadow name as a string, got {}", shown(other)),
                ));
            }
        };
        match name {
            "wake" => parse_wake(object).map(Shadow::Wake),
            "mark" => parse_mark(object).map(Shadow::Mark),
            other => Err(Error::spec(
                format!("{PATH}.name"),
                format!(
                    "unknown shadow '{other}'{}; accepted: {}",
                    did_you_mean(other, NAMES.iter().copied()),
                    NAMES.join(", ")
                ),
            )),
        }
    }

    /// Whether the user layer `layer` (0-based) is excluded.
    pub(crate) fn excludes(&self, layer: usize) -> bool {
        let excluded = match self {
            Shadow::Wake(wake) => &wake.exclude_layer,
            Shadow::Mark(mark) => &mark.exclude_layer,
        };
        excluded.contains(&(layer + 1))
    }
}

fn boolean(object: &Map<String, Value>, key: &str, default: bool) -> Result<bool> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Bool(value)) => Ok(*value),
        other => Err(Error::spec(
            format!("{PATH}.{key}"),
            format!("expected true or false, got {}", shown(other)),
        )),
    }
}

/// gganimate's `size`/`alpha`: a number (the end value), `true` or absent (0,
/// gganimate's default) or `false`/`null` (unchanged).
fn end_value(object: &Map<String, Value>, key: &str) -> Result<Option<f64>> {
    match object.get(key) {
        Some(Value::Null) | Some(Value::Bool(false)) => Ok(None),
        None | Some(Value::Bool(true)) => Ok(Some(0.0)),
        Some(Value::Number(_)) => {
            let value = number(object, PATH, key, 0.0)?;
            if value < 0.0 || (key == "alpha" && value > 1.0) {
                return Err(Error::spec(
                    format!("{PATH}.{key}"),
                    format!(
                        "{key} must be {}, got {value}",
                        if key == "alpha" {
                            "between 0 and 1"
                        } else {
                            "0 or more"
                        }
                    ),
                ));
            }
            Ok(Some(value))
        }
        other => Err(Error::spec(
            format!("{PATH}.{key}"),
            format!("expected a number, true or false, got {}", shown(other)),
        )),
    }
}

fn color(object: &Map<String, Value>, key: &str) -> Result<Option<Color>> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => match parse_constant(crate::style::ValueKind::Color, value) {
            Ok(StyleValues::Colors(PerItem::Uniform(color))) => Ok(Some(color)),
            Ok(_) => Ok(None),
            Err(message) => Err(Error::spec(format!("{PATH}.{key}"), message)),
        },
    }
}

fn layers(object: &Map<String, Value>) -> Result<Vec<usize>> {
    let path = format!("{PATH}.exclude_layer");
    let one = |value: &Value| -> Result<usize> {
        value
            .as_u64()
            .filter(|value| *value >= 1)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| {
                Error::spec(
                    &path,
                    format!(
                        "expected layer numbers counted from 1 (as gganimate), got {}",
                        shown(Some(value))
                    ),
                )
            })
    };
    match object.get("exclude_layer") {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(values)) => values.iter().map(one).collect(),
        Some(value) => Ok(vec![one(value)?]),
    }
}

fn parse_wake(object: &Map<String, Value>) -> Result<Wake> {
    check_keys(object, PATH, WAKE_KEYS)?;
    let wake_length = match object.get("wake_length") {
        Some(Value::Number(_)) => number(object, PATH, "wake_length", 0.0)?,
        other => {
            return Err(Error::spec(
                format!("{PATH}.wake_length"),
                format!(
                    "expected the wake length as a share of the frames (e.g. 0.1), got {}",
                    shown(other)
                ),
            ));
        }
    };
    if !(wake_length > 0.0 && wake_length <= 1.0) {
        return Err(Error::spec(
            format!("{PATH}.wake_length"),
            format!(
                "wake_length is a share of the frames and must be above 0 and at most 1, \
                 got {wake_length}"
            ),
        ));
    }
    let falloff = match object.get("falloff") {
        None | Some(Value::Null) => Ease::parse("cubic-in", &format!("{PATH}.falloff"))?,
        Some(Value::String(name)) => Ease::parse(name, &format!("{PATH}.falloff"))?,
        other => {
            return Err(Error::spec(
                format!("{PATH}.falloff"),
                format!("expected an ease name, got {}", shown(other)),
            ));
        }
    };
    let exclude_phase = match object.get("exclude_phase") {
        None => vec![Phase::Enter, Phase::Exit],
        Some(Value::Null) => Vec::new(),
        Some(Value::Array(values)) => values.iter().map(phase).collect::<Result<Vec<Phase>>>()?,
        Some(value) => vec![phase(value)?],
    };
    Ok(Wake {
        wake_length,
        size: end_value(object, "size")?,
        alpha: end_value(object, "alpha")?,
        colour: color(object, "colour")?,
        fill: color(object, "fill")?,
        falloff,
        wrap: boolean(object, "wrap", true)?,
        exclude_layer: layers(object)?,
        exclude_phase,
    })
}

fn phase(value: &Value) -> Result<Phase> {
    let path = format!("{PATH}.exclude_phase");
    let names = Phase::NAMES.iter().map(|(name, _)| *name);
    match value {
        Value::String(text) => Phase::NAMES
            .iter()
            .find(|(name, _)| name == text)
            .map(|(_, phase)| *phase)
            .ok_or_else(|| {
                Error::spec(
                    &path,
                    format!(
                        "unknown phase '{text}'{}; accepted: {}",
                        did_you_mean(text, names.clone()),
                        names.clone().collect::<Vec<_>>().join(", ")
                    ),
                )
            }),
        other => Err(Error::spec(
            &path,
            format!("expected a phase name, got {}", shown(Some(other))),
        )),
    }
}

fn parse_mark(object: &Map<String, Value>) -> Result<Mark> {
    check_keys(object, PATH, MARK_KEYS)?;
    let mut aesthetics = Vec::new();
    if let Some(value) = object.get("aesthetics").filter(|value| !value.is_null()) {
        let path = format!("{PATH}.aesthetics");
        for (name, value) in as_object(value, &path)? {
            let path = format!("{path}.{name}");
            let aesthetic: Aesthetic =
                name.parse()
                    .map_err(|error: crate::spec::UnknownAesthetic| {
                        Error::spec(&path, error.to_string())
                    })?;
            let Some(kind) = aesthetic.kind().style() else {
                return Err(Error::spec(
                    &path,
                    format!(
                        "shadow_mark() takes constant styles (alpha, size, colour, fill, \
                         stroke, shape, linetype, ...), not the position '{name}'"
                    ),
                ));
            };
            let parsed =
                parse_constant(kind, value).map_err(|message| Error::spec(&path, message))?;
            if let StyleValues::Numbers(PerItem::Uniform(number)) = parsed {
                let alpha = aesthetic == Aesthetic::Alpha;
                if number < 0.0 || (alpha && number > 1.0) {
                    return Err(Error::spec(
                        &path,
                        format!(
                            "{name} must be {}, got {number}",
                            if alpha {
                                "between 0 and 1"
                            } else {
                                "0 or more"
                            }
                        ),
                    ));
                }
            }
            if aesthetics
                .iter()
                .any(|(known, given): &(Aesthetic, Value)| *known == aesthetic && given != value)
            {
                return Err(Error::spec(
                    &path,
                    format!("{name} is given twice with different values (color and colour)"),
                ));
            }
            aesthetics.push((aesthetic, value.clone()));
        }
    }
    Ok(Mark {
        past: boolean(object, "past", true)?,
        future: boolean(object, "future", false)?,
        exclude_layer: layers(object)?,
        aesthetics,
    })
}

impl Wake {
    /// The scene frames (0-based) of the wake of frame `frame` out of
    /// `nframes`, oldest first, with their `at` (gganimate `get_frames` and
    /// `setup_params`).
    pub(crate) fn frames(&self, frame: usize, nframes: usize) -> Vec<(usize, f64)> {
        let length = r_round(nframes as f64 * self.wake_length);
        if length <= 0.0 || nframes == 0 {
            return Vec::new();
        }
        let length = length as i64;
        let nframes = nframes as i64;
        let current = frame as i64 + 1;
        let frames: Vec<i64> = ((current - length)..current)
            .filter_map(|wake| match self.wrap {
                true => {
                    let wrapped = wake.rem_euclid(nframes);
                    Some(if wrapped == 0 { nframes } else { wrapped })
                }
                false => (wake > 0 && wake <= nframes).then_some(wake),
            })
            .collect();
        frames
            .into_iter()
            .enumerate()
            .map(|(index, wake)| ((wake - 1) as usize, index as f64 / length as f64))
            .collect()
    }

    /// gganimate `prepare_shadow` for one wake frame at step `at`.
    pub(crate) fn fade(&self, frame: &mut LayerFrame, at: f64, geom: &str) -> Result<()> {
        let share = self.falloff.apply(at);
        let count = frame.data.row_count();
        let all: Vec<usize> = (0..count).collect();
        let towards = |end: f64, value: f64| end + share * (value - end);
        for (aesthetic, end) in [
            (Aesthetic::Color, self.colour),
            (Aesthetic::Fill, self.fill),
        ] {
            let (Some(end), Ok(colors)) = (end, frame.styles.colors(aesthetic).cloned()) else {
                continue;
            };
            let changed: Vec<(usize, Color)> = all
                .iter()
                .map(|row| (*row, mix_lab(end, colors.get(*row), share)))
                .collect();
            let values = set_rows(&colors, count, &changed);
            frame
                .styles
                .insert(aesthetic, StyleValues::Colors(values))?;
        }
        if let Some(end) = self.alpha {
            let alphas = frame.styles.numbers(Aesthetic::Alpha).ok().cloned();
            let no_alpha = |row: usize| {
                alphas
                    .as_ref()
                    .is_none_or(|alphas| alphas.get(row).is_nan())
            };
            if let Some(alphas) = &alphas {
                let changed: Vec<(usize, f64)> = all
                    .iter()
                    .filter(|row| !no_alpha(**row))
                    .map(|row| (*row, towards(end, alphas.get(*row))))
                    .collect();
                let values = set_rows(alphas, count, &changed);
                frame
                    .styles
                    .insert(Aesthetic::Alpha, StyleValues::Numbers(values))?;
            }
            for aesthetic in [Aesthetic::Color, Aesthetic::Fill] {
                let Ok(colors) = frame.styles.colors(aesthetic).cloned() else {
                    continue;
                };
                let changed: Vec<(usize, Color)> = all
                    .iter()
                    // A fully transparent color is "no color" (R's NA).
                    .filter(|row| no_alpha(**row) && colors.get(**row).alpha > 0)
                    .map(|row| {
                        let color = colors.get(*row);
                        let alpha = towards(end, f64::from(color.alpha) / 255.0);
                        let channel = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
                        (
                            *row,
                            Color::rgba(color.red, color.green, color.blue, channel),
                        )
                    })
                    .collect();
                let values = set_rows(&colors, count, &changed);
                frame
                    .styles
                    .insert(aesthetic, StyleValues::Colors(values))?;
            }
        }
        if let Some(end) = self.size.filter(|_| MARK_SIZE_GEOMS.contains(&geom)) {
            for aesthetic in [Aesthetic::Size, Aesthetic::Stroke] {
                let Ok(values) = frame.styles.numbers(aesthetic).cloned() else {
                    continue;
                };
                let changed: Vec<(usize, f64)> = all
                    .iter()
                    .map(|row| (*row, towards(end, values.get(*row))))
                    .collect();
                let values = set_rows(&values, count, &changed);
                frame
                    .styles
                    .insert(aesthetic, StyleValues::Numbers(values))?;
            }
        }
        Ok(())
    }

    /// gganimate `ShadowWake$prepare_frame_data`: the wake rows of elements
    /// drawn in `current` outside the excluded phases, then `current`, each
    /// element's rows together in the order elements first appear.
    pub(crate) fn compose(&self, wake: Vec<Drawn>, current: Drawn) -> Result<LayerFrame> {
        let kept: std::collections::HashSet<RowKey> = current
            .keys
            .iter()
            .zip(&current.phases)
            .filter(|(_, phase)| !self.exclude_phase.contains(phase))
            .map(|(key, _)| *key)
            .collect();
        let mut parts = Vec::with_capacity(wake.len() + 1);
        let mut keys = Vec::new();
        for drawn in wake {
            let rows: Vec<u32> = (0..drawn.keys.len())
                .filter(|row| kept.contains(&drawn.keys[*row]))
                .map(|row| row as u32)
                .collect();
            keys.extend(rows.iter().map(|row| drawn.keys[*row as usize]));
            parts.push(drawn.frame.take(&rows));
        }
        keys.extend(current.keys.iter().copied());
        parts.push(current.frame);
        let stacked = stack(parts)?;
        // R `order(match(id, unique(id)))`: stable, by first appearance.
        let mut first: HashMap<RowKey, usize> = HashMap::new();
        for key in &keys {
            let next = first.len();
            first.entry(*key).or_insert(next);
        }
        let mut order: Vec<u32> = (0..keys.len() as u32).collect();
        order.sort_by_key(|row| first[&keys[*row as usize]]);
        Ok(stacked.take(&order))
    }
}

impl Mark {
    /// `frame` with the constant style overrides, where the layer has them.
    pub(crate) fn restyle(&self, frame: &mut LayerFrame) -> Result<()> {
        for (aesthetic, value) in &self.aesthetics {
            let Some(kind) = aesthetic.kind().style() else {
                continue;
            };
            if frame.styles.iter().any(|(other, _)| other == *aesthetic) {
                let values = parse_constant(kind, value).map_err(|message| {
                    Error::spec(format!("{PATH}.aesthetics.{aesthetic}"), message)
                })?;
                frame.styles.insert(*aesthetic, values)?;
            }
        }
        Ok(())
    }

    /// Whether the marks of raw frame `raw` show in frame `frame`.
    pub(crate) fn shows(&self, raw: usize, frame: usize) -> bool {
        match (self.past, self.future) {
            (true, true) => raw != frame,
            (true, false) => raw < frame,
            (false, true) => raw > frame,
            (false, false) => false,
        }
    }
}

/// Parts of one layer drawn in order, each with group numbers of its own.
pub(crate) fn stack(parts: Vec<LayerFrame>) -> Result<LayerFrame> {
    if parts.len() == 1 {
        return parts
            .into_iter()
            .next()
            .ok_or_else(|| Error::data("internal: no part to stack"));
    }
    let mut groups: Vec<u32> = Vec::new();
    let mut offset: u32 = 0;
    for part in &parts {
        let mut top = 0;
        for row in 0..part.data.row_count() {
            let group = part.data.group_of(row);
            top = top.max(group + 1);
            groups.push(
                offset.checked_add(group).ok_or_else(|| {
                    Error::data("internal: too many groups in an animation shadow")
                })?,
            );
        }
        offset = offset.saturating_add(top);
    }
    let mut joined = join_frames(&parts.iter().collect::<Vec<_>>())?;
    joined.data.set_group_index(Some(groups))?;
    Ok(joined)
}

/// Parts of one layer one after the other, groups as they are.
pub(crate) fn join_frames(parts: &[&LayerFrame]) -> Result<LayerFrame> {
    let mut datas = Vec::with_capacity(parts.len());
    for part in parts {
        let mut data = part.data.clone();
        data.set_source_rows(SourceRows::Aggregated)?;
        datas.push(data);
    }
    let pieces: Vec<(&crate::data::LayerData, &[u32])> =
        datas.iter().map(|data| (data, &[][..])).collect();
    let data = crate::data::LayerData::concat(&pieces)?;
    let styles = concat_styles(parts)?;
    Ok(LayerFrame { data, styles })
}

/// The styles of `parts` one after the other; a value shared by every part
/// stays shared.
fn concat_styles(parts: &[&LayerFrame]) -> Result<Styles> {
    let mut styles = Styles::default();
    let Some(first) = parts.first() else {
        return Ok(styles);
    };
    for (aesthetic, values) in first.styles.iter() {
        let all: Vec<(&StyleValues, usize)> = parts
            .iter()
            .map(|part| {
                part.styles
                    .iter()
                    .find(|(other, _)| *other == aesthetic)
                    .map(|(_, values)| (values, part.data.row_count()))
                    .ok_or_else(|| {
                        Error::data(format!(
                            "internal: style '{aesthetic}' missing in a shadow part"
                        ))
                    })
            })
            .collect::<Result<_>>()?;
        let joined = match values {
            StyleValues::Colors(_) => StyleValues::Colors(join(&all, |values| match values {
                StyleValues::Colors(items) => Some(items),
                _ => None,
            })?),
            StyleValues::Numbers(_) => StyleValues::Numbers(join(&all, |values| match values {
                StyleValues::Numbers(items) => Some(items),
                _ => None,
            })?),
            StyleValues::Shapes(_) => StyleValues::Shapes(join(&all, |values| match values {
                StyleValues::Shapes(items) => Some(items),
                _ => None,
            })?),
            StyleValues::Linetypes(_) => {
                StyleValues::Linetypes(join(&all, |values| match values {
                    StyleValues::Linetypes(items) => Some(items),
                    _ => None,
                })?)
            }
            StyleValues::Texts(_) => StyleValues::Texts(join(&all, |values| match values {
                StyleValues::Texts(items) => Some(items),
                _ => None,
            })?),
        };
        styles.insert(aesthetic, joined)?;
    }
    Ok(styles)
}

fn join<T: Clone + PartialEq>(
    parts: &[(&StyleValues, usize)],
    pick: impl Fn(&StyleValues) -> Option<&PerItem<T>>,
) -> Result<PerItem<T>> {
    let items: Vec<(&PerItem<T>, usize)> = parts
        .iter()
        .map(|(values, count)| {
            pick(values)
                .map(|items| (items, *count))
                .ok_or_else(|| Error::data("internal: a style differs between shadow parts"))
        })
        .collect::<Result<_>>()?;
    if let Some((PerItem::Uniform(first), _)) = items.first() {
        if items
            .iter()
            .all(|(items, _)| matches!(items, PerItem::Uniform(value) if value == first))
        {
            return Ok(PerItem::Uniform(first.clone()));
        }
    }
    let mut values = Vec::new();
    for (items, count) in items {
        match items {
            PerItem::Uniform(value) => values.extend(std::iter::repeat_n(value.clone(), count)),
            PerItem::Varying(varying) => values.extend(varying.iter().cloned()),
        }
    }
    Ok(PerItem::Varying(values))
}

/// R's `round` (half to even) for the wake length.
fn r_round(value: f64) -> f64 {
    let rounded = value.round();
    if (value - value.trunc()).abs() == 0.5 && rounded % 2.0 != 0.0 {
        rounded - value.signum()
    } else {
        rounded
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn wake(options: Value) -> Wake {
        match Shadow::parse(&options).unwrap() {
            Shadow::Wake(wake) => wake,
            Shadow::Mark(_) => panic!("wake expected"),
        }
    }

    #[test]
    fn wake_frames_follow_gganimate() {
        let shadow = wake(json!({"name": "wake", "wake_length": 0.1}));
        // nframes 20: wake of 2 frames, at 0 and 0.5.
        assert_eq!(shadow.frames(5, 20), vec![(3, 0.0), (4, 0.5)]);
        // Frame 1 wraps to the last frames.
        assert_eq!(shadow.frames(0, 20), vec![(18, 0.0), (19, 0.5)]);
        let shadow = wake(json!({"name": "wake", "wake_length": 0.1, "wrap": false}));
        assert_eq!(shadow.frames(0, 20), vec![]);
        // Only one frame back: it takes the first `at`.
        assert_eq!(shadow.frames(1, 20), vec![(0, 0.0)]);
        // R rounds half to even: 0.125 * 20 = 2.5 -> 2.
        let shadow = wake(json!({"name": "wake", "wake_length": 0.125}));
        assert_eq!(shadow.frames(5, 20).len(), 2);
    }

    /// Wake frames and steps against gganimate's `get_frames` and
    /// `prepare_shadow` (`tests/fixtures/generate_animation_reference.R`).
    #[test]
    fn wake_matches_gganimate() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/animation_reference.json"
        );
        let reference: Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let cases = reference["wake"].as_array().unwrap();
        assert_eq!(cases.len(), 4);
        for case in cases {
            let shadow = wake(json!({
                "name": "wake",
                "wake_length": case["wake_length"],
                "wrap": case["wrap"],
            }));
            let frame = case["i"].as_u64().unwrap() as usize - 1;
            let nframes = case["nframes"].as_u64().unwrap() as usize;
            let planned = shadow.frames(frame, nframes);
            let frames: Vec<u64> = case["frames"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_u64().unwrap())
                .collect();
            assert_eq!(
                planned
                    .iter()
                    .map(|(frame, _)| *frame as u64 + 1)
                    .collect::<Vec<_>>(),
                frames,
                "{case}"
            );
            for (index, (_, at)) in planned.iter().enumerate() {
                assert!((at - case["at"][index].as_f64().unwrap()).abs() < 1e-12);
                // One point of size 4 in #3366CC80 at this step.
                let mut data = crate::data::LayerData::new(1);
                data.set_group_index(None).unwrap();
                let mut styles = Styles::default();
                styles
                    .insert(Aesthetic::Size, StyleValues::Numbers(PerItem::Uniform(4.0)))
                    .unwrap();
                styles
                    .insert(
                        Aesthetic::Color,
                        StyleValues::Colors(PerItem::Uniform(Color::rgba(0x33, 0x66, 0xCC, 0x80))),
                    )
                    .unwrap();
                let mut layer = LayerFrame { data, styles };
                shadow.fade(&mut layer, *at, "point").unwrap();
                let size = layer.styles.numbers(Aesthetic::Size).unwrap().get(0);
                assert!((size - case["size"][index].as_f64().unwrap()).abs() < 1e-12);
                let color = layer.styles.colors(Aesthetic::Color).unwrap().get(0);
                let expected = case["colour"][index].as_str().unwrap();
                let alpha = u8::from_str_radix(&expected[7..9], 16).unwrap();
                assert_eq!(color.alpha, alpha, "{case} step {index}");
            }
        }
    }

    #[test]
    fn shadow_options_have_typed_errors() {
        let error = |options: Value| Shadow::parse(&options).unwrap_err().to_string();
        let message = error(json!({"name": "wkae", "wake_length": 0.1}));
        assert!(message.contains("did you mean 'wake'"), "{message}");
        let message = error(json!({"name": "wake", "wake_length": 0}));
        assert!(
            message.contains("animation.shadow.wake_length"),
            "{message}"
        );
        let message = error(json!({"name": "wake", "wake_length": 0.1, "exclude_phase": ["entr"]}));
        assert!(message.contains("did you mean 'enter'"), "{message}");
        let message = error(json!({"name": "wake", "wake_length": 0.1, "exclude_layer": 0}));
        assert!(message.contains("counted from 1"), "{message}");
        let message = error(json!({"name": "mark", "aesthetics": {"x": 1}}));
        assert!(message.contains("not the position 'x'"), "{message}");
        let message = error(json!({"name": "mark", "aesthetics": {"alpha": "a"}}));
        assert!(
            message.contains("animation.shadow.aesthetics.alpha"),
            "{message}"
        );
    }

    #[test]
    fn wake_defaults_and_flags() {
        let shadow = wake(json!({"name": "wake", "wake_length": 0.1, "size": false}));
        assert_eq!(shadow.size, None);
        assert_eq!(shadow.alpha, Some(0.0));
        assert_eq!(shadow.exclude_phase, vec![Phase::Enter, Phase::Exit]);
        assert!(shadow.wrap);
    }
}
