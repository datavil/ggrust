//! What `transition_time` and `transition_reveal` share (gganimate `get_times`,
//! `get_row_along`, `recast_times`; MIT): the range of the time column, the
//! frame position of every time (gganimate rounds each time to its nearest
//! frame), the time each frame stands for, and how that time prints in
//! `{frame_time}` / `{frame_along}`. It does not choose which rows a frame
//! draws ([`super::time`], [`super::reveal`]).

use serde_json::{Map, Value};

use super::spec::shown;
use super::timeline::r_number;
use super::transition::StateSet;
use crate::build::StateVariable;
use crate::scale::datetime::DatePattern;
use crate::spec::PlotSpec;
use crate::{Error, Result};

/// The type of a time column, as gganimate's `recast_times` prints it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum TimeClass {
    /// Whole numbers (R `integer`): a frame's time is rounded.
    Integer,
    /// Any other number.
    #[default]
    Numeric,
    /// Days (R `Date`), in epoch milliseconds.
    Date,
    /// Instants (R `POSIXct`), in epoch milliseconds, printed in UTC.
    DateTime,
    /// Times of day (R `hms`), in milliseconds since midnight.
    TimeOfDay,
}

impl TimeClass {
    /// The class of `column` from the plot's series annotations (layer data
    /// first, then the plot's data); numbers without an annotation are
    /// [`TimeClass::Numeric`].
    pub(crate) fn of(spec: &PlotSpec, column: &str) -> TimeClass {
        spec.layers
            .iter()
            .map(|layer| &layer.data_meta)
            .chain(std::iter::once(&spec.data_meta))
            .flat_map(|meta| meta.series_annotations.iter())
            .find(|annotation| annotation.column == column && annotation.series_type.is_some())
            .and_then(|annotation| match annotation.series_type.as_deref() {
                Some("int") => Some(TimeClass::Integer),
                Some("date") => Some(TimeClass::Date),
                Some("datetime") => Some(TimeClass::DateTime),
                Some("time") => Some(TimeClass::TimeOfDay),
                _ => None,
            })
            .unwrap_or(TimeClass::Numeric)
    }

    /// `value` as gganimate's label shows it: integers rounded, dates as
    /// `YYYY-MM-DD` (the day it falls in), date-times as
    /// `YYYY-MM-DD HH:MM:SS` (the date alone at midnight), times of day as
    /// `HH:MM:SS`.
    pub(crate) fn label(self, value: f64) -> String {
        if !value.is_finite() {
            return "NA".to_owned();
        }
        let pattern = |text: &str| DatePattern::parse(text).format(value);
        match self {
            TimeClass::Integer => r_number(value.round_ties_even()),
            TimeClass::Numeric => r_number(value),
            TimeClass::Date => pattern("%Y-%m-%d"),
            TimeClass::DateTime if value.rem_euclid(86_400_000.0) < 1000.0 => pattern("%Y-%m-%d"),
            TimeClass::DateTime => pattern("%Y-%m-%d %H:%M:%S"),
            TimeClass::TimeOfDay => pattern("%H:%M:%S"),
        }
    }
}

/// `range=[low, high]` of `transition_time()` / `transition_reveal()`: two
/// numbers with `low < high` (dates and date-times in epoch milliseconds, as the
/// Python API sends them), or `None` for the data's range.
///
/// # Errors
///
/// A spec error at `<path>.range` for anything else.
pub(crate) fn parse_range(options: &Map<String, Value>, path: &str) -> Result<Option<[f64; 2]>> {
    let value = match options.get("range") {
        Some(Value::Null) | None => return Ok(None),
        Some(value) => value,
    };
    let numbers: Option<Vec<f64>> = value.as_array().and_then(|items| {
        items
            .iter()
            .map(|item| item.as_f64().filter(|number| number.is_finite()))
            .collect()
    });
    match numbers.as_deref() {
        Some([low, high]) if low < high => Ok(Some([*low, *high])),
        _ => Err(Error::spec(
            format!("{path}.range"),
            format!(
                "expected two increasing values [low, high] of the time column, got {}",
                shown(Some(value))
            ),
        )),
    }
}

/// What `range` was given as (`range_kind`, set by the Python API).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RangeKind {
    /// Numbers.
    Number,
    /// Dates.
    Date,
    /// Date-times (or dates mixed with date-times).
    DateTime,
    /// Times of day.
    Time,
}

impl RangeKind {
    /// `value` of a range of this kind as the user gave it.
    fn shown(self, value: f64) -> String {
        match self {
            RangeKind::Number => r_number(value),
            RangeKind::Date => TimeClass::Date.label(value),
            RangeKind::DateTime => TimeClass::DateTime.label(value),
            RangeKind::Time => TimeClass::TimeOfDay.label(value),
        }
    }

    fn name(self) -> &'static str {
        match self {
            RangeKind::Number => "numbers",
            RangeKind::Date => "dates",
            RangeKind::DateTime => "date-times",
            RangeKind::Time => "times of day",
        }
    }
}

/// `range_kind` of `transition_time()` / `transition_reveal()`; `None` when
/// not given (a hand-written spec: the range is not checked).
///
/// # Errors
///
/// A spec error at `<path>.range_kind` for an unknown kind.
pub(crate) fn parse_range_kind(
    options: &Map<String, Value>,
    path: &str,
) -> Result<Option<RangeKind>> {
    match options.get("range_kind") {
        Some(Value::Null) | None => Ok(None),
        Some(Value::String(kind)) if kind == "number" => Ok(Some(RangeKind::Number)),
        Some(Value::String(kind)) if kind == "date" => Ok(Some(RangeKind::Date)),
        Some(Value::String(kind)) if kind == "datetime" => Ok(Some(RangeKind::DateTime)),
        Some(Value::String(kind)) if kind == "time" => Ok(Some(RangeKind::Time)),
        other => Err(Error::spec(
            format!("{path}.range_kind"),
            format!(
                "expected 'number', 'date', 'datetime' or 'time', got {}",
                shown(other)
            ),
        )),
    }
}

impl TimeClass {
    /// What the column holds, in words, for errors.
    fn name(self) -> &'static str {
        match self {
            TimeClass::Integer => "integers",
            TimeClass::Numeric => "numbers",
            TimeClass::Date => "dates",
            TimeClass::DateTime => "date-times",
            TimeClass::TimeOfDay => "times of day",
        }
    }

    /// What `range` this column type accepts, in words.
    fn accepted_range(self) -> &'static str {
        match self {
            TimeClass::Integer | TimeClass::Numeric => "numbers, e.g. range=(2000, 2020)",
            TimeClass::Date | TimeClass::DateTime => {
                "dates or date-times (datetime.date, datetime.datetime, numpy.datetime64, \
                 polars or pandas values), e.g. range=(date(2020, 1, 1), date(2020, 12, 31))"
            }
            TimeClass::TimeOfDay => "times of day (datetime.time), e.g. range=(time(9), time(17))",
        }
    }

    /// Whether a `range` of `kind` fits this column type.
    fn accepts(self, kind: RangeKind) -> bool {
        match self {
            TimeClass::Integer | TimeClass::Numeric => kind == RangeKind::Number,
            TimeClass::Date | TimeClass::DateTime => {
                matches!(kind, RangeKind::Date | RangeKind::DateTime)
            }
            TimeClass::TimeOfDay => kind == RangeKind::Time,
        }
    }
}

/// The column name option (`time`, `along`) as a [`StateVariable`].
///
/// # Errors
///
/// A spec error at `<path>.<key>` when it is not a non-empty string.
pub(crate) fn parse_variable(
    options: &Map<String, Value>,
    path: &str,
    key: &str,
    function: &str,
) -> Result<StateVariable> {
    let path = format!("{path}.{key}");
    match options.get(key) {
        Some(Value::String(name)) if !name.is_empty() => Ok(StateVariable {
            name: name.clone(),
            path,
        }),
        other => Err(Error::spec(
            &path,
            format!(
                "{function}() needs the column of the {key} as a string, got {}",
                shown(other)
            ),
        )),
    }
}

/// The planned frames of a time-based transition.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct AlongFrames {
    /// Frame position of every state (1-based, may lie outside the frames).
    pub positions: Vec<i64>,
    /// The time of every scene frame (R `seq(low, high, length.out = n)`).
    pub times: Vec<f64>,
    /// How times print.
    pub class: TimeClass,
}

/// How a time becomes a frame position: gganimate writes the same rounding
/// two ways, so a time halfway between frames can round differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rounding {
    /// `transition_time`: `round(1 + (n - 1) * (v - low) / (high - low))`.
    Time,
    /// `transition_reveal`: `round((n - 1) * (v - low) / (high - low)) + 1`.
    Reveal,
}

/// Plan `frames` scene frames over the states' numbers.
///
/// # Errors
///
/// A spec error at the variable's path when the column is categorical, or when
/// its values span no time and no `range` is given; at `range` when the range
/// is not of the column's type (gganimate: "range must be given in the same
/// class as time").
pub(crate) fn plan(
    states: &StateSet,
    range: Option<[f64; 2]>,
    range_kind: Option<RangeKind>,
    frames: usize,
    variable: &StateVariable,
    function: &str,
    rounding: Rounding,
) -> Result<AlongFrames> {
    let Some(values) = states.values else {
        return Err(Error::spec(
            &variable.path,
            format!(
                "{function}() needs a numeric, date or date-time column; '{}' is \
                 categorical (use transition_states() for categories)",
                variable.name
            ),
        ));
    };
    if let (Some(kind), Some(range)) = (range_kind, range)
        && !states.class.accepts(kind)
    {
        let path = variable
            .path
            .rsplit_once('.')
            .map_or("range".to_owned(), |(parent, _)| format!("{parent}.range"));
        return Err(Error::spec(
            path,
            format!(
                "{function}(range=...) holds {} ({} to {}) but the column '{}' holds {}; \
                 give the range as {}",
                kind.name(),
                kind.shown(range[0]),
                kind.shown(range[1]),
                variable.name,
                states.class.name(),
                states.class.accepted_range()
            ),
        ));
    }
    let [low, high] = match range {
        Some(range) => range,
        None => match (values.first(), values.last()) {
            (Some(low), Some(high)) if low < high => [*low, *high],
            _ => {
                return Err(Error::spec(
                    &variable.path,
                    format!(
                        "{function}() needs at least two different values in '{}', or a \
                         range: the column has one value only",
                        variable.name
                    ),
                ));
            }
        },
    };
    let span = high - low;
    let steps = frames.saturating_sub(1) as f64;
    let positions = values
        .iter()
        .map(|value| {
            let position = match rounding {
                Rounding::Time => (1.0 + steps * (value - low) / span).round_ties_even(),
                Rounding::Reveal => (steps * (value - low) / span).round_ties_even() + 1.0,
            };
            position as i64
        })
        .collect();
    Ok(AlongFrames {
        positions,
        times: r_sequence(low, high, frames),
        class: states.class,
    })
}

/// R `seq(from, to, length.out = count)` (`seq.default`): the ends exactly,
/// the inner values `from + k * ((to - from) / (count - 1))`.
pub(crate) fn r_sequence(from: f64, to: f64, count: usize) -> Vec<f64> {
    match count {
        0 => Vec::new(),
        1 => vec![from],
        2 => vec![from, to],
        _ if from == to => vec![from; count],
        _ => {
            let step = (to - from) / (count - 1) as f64;
            let mut values = Vec::with_capacity(count);
            values.push(from);
            values.extend((1..count - 1).map(|k| from + k as f64 * step));
            values.push(to);
            values
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use serde_json::json;

    use super::*;

    fn variable() -> StateVariable {
        StateVariable {
            name: "year".to_owned(),
            path: "animation.transition.time".to_owned(),
        }
    }

    #[test]
    fn times_round_to_their_nearest_frame() {
        let values = [1952.0, 1957.0, 1962.0, 2007.0];
        let states = StateSet {
            count: values.len(),
            values: Some(&values),
            class: TimeClass::Integer,
        };
        let planned = plan(
            &states,
            None,
            None,
            100,
            &variable(),
            "transition_time",
            Rounding::Time,
        )
        .unwrap();
        // round(1 + 99 * (v - 1952) / 55): 1, 10, 19, 100.
        assert_eq!(planned.positions, vec![1, 10, 19, 100]);
        assert_eq!(planned.times.len(), 100);
        assert_eq!(planned.times[0], 1952.0);
        assert_eq!(planned.times[99], 2007.0);
        assert_eq!(TimeClass::Integer.label(planned.times[1]), "1953");
        assert_eq!(
            TimeClass::Numeric.label(planned.times[1]),
            "1952.55555555556"
        );
        // A range: times outside it get positions outside the frames.
        let planned = plan(
            &states,
            Some([1957.0, 1962.0]),
            None,
            11,
            &variable(),
            "transition_time",
            Rounding::Time,
        )
        .unwrap();
        assert_eq!(planned.positions, vec![-9, 1, 11, 101]);
    }

    #[test]
    fn half_frames_round_to_even_in_both_spellings() {
        let values = [0.0, 0.5, 1.5, 4.0];
        let states = StateSet {
            count: 4,
            values: Some(&values),
            class: TimeClass::Numeric,
        };
        let time = plan(&states, None, None, 5, &variable(), "f", Rounding::Time).unwrap();
        // 1 + 4 * v / 4 = 1, 1.5 -> 2, 2.5 -> 2, 5.
        assert_eq!(time.positions, vec![1, 2, 2, 5]);
        let reveal = plan(&states, None, None, 5, &variable(), "f", Rounding::Reveal).unwrap();
        // round(v) + 1: 1, round(0.5) = 0 -> 1, round(1.5) = 2 -> 3, 5.
        assert_eq!(reveal.positions, vec![1, 1, 3, 5]);
    }

    #[test]
    fn errors_name_the_option() {
        let one = [3.0];
        let states = StateSet {
            count: 1,
            values: Some(&one),
            class: TimeClass::Numeric,
        };
        let error = plan(
            &states,
            None,
            None,
            10,
            &variable(),
            "transition_time",
            Rounding::Time,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("animation.transition.time"), "{error}");
        assert!(error.contains("one value"), "{error}");
        let categories = StateSet {
            count: 2,
            values: None,
            class: TimeClass::Numeric,
        };
        let error = plan(
            &categories,
            None,
            None,
            10,
            &variable(),
            "transition_time",
            Rounding::Time,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("categorical"), "{error}");
        let options = json!({"range": [3, 1]});
        let error = parse_range(options.as_object().unwrap(), "animation.transition")
            .unwrap_err()
            .to_string();
        assert!(error.contains("animation.transition.range"), "{error}");
    }

    /// The animation reference fixture (`tests/fixtures/generate_animation_reference.R`).
    pub(crate) fn reference() -> Value {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/animation_reference.json"
        );
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    /// `[1, "NA", 2]` as numbers, `None` for R's NA.
    pub(crate) fn r_values(value: &Value) -> Vec<Option<f64>> {
        value
            .as_array()
            .unwrap()
            .iter()
            .map(Value::as_f64)
            .collect()
    }

    /// gganimate `get_times`: frame positions (NA for a time whose frame an
    /// earlier time holds) and frame times.
    #[test]
    fn time_frames_match_gganimate() {
        for case in reference()["time"].as_array().unwrap() {
            let data: Vec<f64> = r_values(&case["values"]).into_iter().flatten().collect();
            let mut values = data.clone();
            values.sort_by(f64::total_cmp);
            values.dedup();
            let states = StateSet {
                count: values.len(),
                values: Some(&values),
                class: TimeClass::Numeric,
            };
            let range = case["range"]
                .as_array()
                .map(|range| [range[0].as_f64().unwrap(), range[1].as_f64().unwrap()]);
            let frames = case["nframes"].as_u64().unwrap() as usize;
            let planned = plan(
                &states,
                range,
                None,
                frames,
                &variable(),
                "f",
                Rounding::Time,
            )
            .unwrap();
            let kept =
                super::super::time::kept_states(&vec![true; values.len()], &planned.positions);
            let positions: Vec<Option<f64>> = data
                .iter()
                .map(|value| {
                    let state = values.iter().position(|known| known == value).unwrap();
                    kept[state].then_some(planned.positions[state] as f64)
                })
                .collect();
            assert_eq!(positions, r_values(&case["positions"]), "{case}");
            // jsonlite writes 15 significant digits.
            let expected = r_values(&case["frame_time"]);
            assert_eq!(planned.times.len(), expected.len());
            for (time, want) in planned.times.iter().zip(expected) {
                assert!((time - want.unwrap()).abs() < 1e-9, "{time} vs {want:?}");
            }
        }
    }

    #[test]
    fn a_range_must_match_the_column_type() {
        let days = [0.0, 86_400_000.0];
        let dates = StateSet {
            count: 2,
            values: Some(&days),
            class: TimeClass::Date,
        };
        let numbers = Some(RangeKind::Number);
        let error = plan(
            &dates,
            Some([1.0, 5.0]),
            numbers,
            5,
            &variable(),
            "transition_time",
            Rounding::Time,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("animation.transition.range"), "{error}");
        assert!(error.contains("holds numbers (1 to 5)"), "{error}");
        assert!(error.contains("column 'year' holds dates"), "{error}");
        assert!(error.contains("datetime.date"), "{error}");
        for kind in [RangeKind::Date, RangeKind::DateTime] {
            assert!(
                plan(
                    &dates,
                    Some([0.0, 9e7]),
                    Some(kind),
                    5,
                    &variable(),
                    "f",
                    Rounding::Time
                )
                .is_ok()
            );
        }
        // A spec without a kind (written by hand) is not checked.
        assert!(
            plan(
                &dates,
                Some([1.0, 5.0]),
                None,
                5,
                &variable(),
                "f",
                Rounding::Time
            )
            .is_ok()
        );
        let years = [2000.0, 2010.0];
        let integers = StateSet {
            count: 2,
            values: Some(&years),
            class: TimeClass::Integer,
        };
        let error = plan(
            &integers,
            Some([0.0, 9e7]),
            Some(RangeKind::Date),
            5,
            &variable(),
            "f",
            Rounding::Time,
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("holds dates (1970-01-01 to 1970-01-02)")
                && error.contains("holds integers"),
            "{error}"
        );
        let options = json!({"range_kind": "days"});
        assert!(parse_range_kind(options.as_object().unwrap(), "animation.transition").is_err());
    }

    #[test]
    fn labels_follow_the_column_type() {
        // 2001-03-17 05:12:33 UTC.
        let instant = 984_805_953_000.0;
        assert_eq!(TimeClass::Date.label(instant), "2001-03-17");
        assert_eq!(TimeClass::DateTime.label(instant), "2001-03-17 05:12:33");
        assert_eq!(TimeClass::DateTime.label(984_787_200_000.0), "2001-03-17");
        assert_eq!(TimeClass::TimeOfDay.label(3_723_000.0), "01:02:03");
        assert_eq!(TimeClass::Integer.label(2.5), "2");
        assert_eq!(TimeClass::Numeric.label(2.5), "2.5");
        assert_eq!(r_sequence(0.0, 1.0, 3), vec![0.0, 0.5, 1.0]);
    }
}
