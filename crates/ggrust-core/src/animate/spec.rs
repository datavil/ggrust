//! The `animation` key of a plot spec: the transition, the `animate()` options and
//! an optional still frame. Parsed by hand from JSON so that every error names its
//! `animation.*` path. It does not plan frames ([`super::timeline`]) or build
//! transitions ([`super::transition`]).

use serde_json::{Map, Value};

use crate::suggest::did_you_mean;
use crate::{Error, Result};

/// Root path of every error of this module.
pub(crate) const PATH: &str = "animation";

/// `p + transition_manual(...)`, `animate(p, ...)` and `Animation.frame(i)` in one
/// spec.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationSpec {
    /// Transition name (`"manual"`) and every key of `animation.transition`.
    pub transition: (String, Map<String, Value>),
    /// `animate()` arguments; all unset by default.
    pub options: AnimateOptions,
    /// Draw only this output frame (0-based), as a still plot.
    pub frame: Option<usize>,
}

/// gganimate `animate()` arguments that shape the frames. Unset values take
/// gganimate's defaults when the frames are planned ([`super::timeline`]).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AnimateOptions {
    /// Number of frames (default 100).
    pub nframes: Option<f64>,
    /// Frames per second (default 10).
    pub fps: Option<f64>,
    /// Length in seconds; sets `nframes` (when `fps` is given) or `fps`.
    pub duration: Option<f64>,
    /// Scene frames computed per shown frame (default 1).
    pub detail: Option<f64>,
    /// Frames the first frame is repeated at the start (default 0).
    pub start_pause: Option<f64>,
    /// Frames the last frame is repeated at the end (default 0).
    pub end_pause: Option<f64>,
    /// Play the frames backwards after playing them forwards (default false).
    pub rewind: Option<bool>,
}

const ANIMATION_KEYS: &[&str] = &["transition", "animate", "frame"];
const OPTION_KEYS: &[&str] = &[
    "nframes",
    "fps",
    "duration",
    "detail",
    "start_pause",
    "end_pause",
    "rewind",
];

impl AnimationSpec {
    /// Parse the `animation` value of a plot spec.
    ///
    /// # Errors
    ///
    /// A spec error at `animation.*` for a missing transition, an unknown key or a
    /// value of the wrong type.
    pub fn parse(value: &Value) -> Result<AnimationSpec> {
        let object = as_object(value, PATH)?;
        check_keys(object, PATH, ANIMATION_KEYS)?;
        let transition_path = format!("{PATH}.transition");
        let transition = match object.get("transition") {
            Some(Value::Null) | None => {
                return Err(Error::spec(
                    &transition_path,
                    "the animation has no transition; add one, e.g. \
                     `+ transition_manual(\"year\")`",
                ));
            }
            Some(value) => as_object(value, &transition_path)?.clone(),
        };
        let name = match transition.get("name") {
            Some(Value::String(name)) => name.clone(),
            other => {
                return Err(Error::spec(
                    format!("{transition_path}.name"),
                    format!(
                        "expected the transition name as a string, got {}",
                        shown(other)
                    ),
                ));
            }
        };
        let options = match object.get("animate") {
            Some(Value::Null) | None => AnimateOptions::default(),
            Some(value) => AnimateOptions::parse(value, &format!("{PATH}.animate"))?,
        };
        let frame = match object.get("frame") {
            Some(Value::Null) | None => None,
            Some(value) => Some(count(value, &format!("{PATH}.frame"))?),
        };
        Ok(AnimationSpec {
            transition: (name, transition),
            options,
            frame,
        })
    }
}

impl AnimateOptions {
    fn parse(value: &Value, path: &str) -> Result<AnimateOptions> {
        let object = as_object(value, path)?;
        check_keys(object, path, OPTION_KEYS)?;
        let number = |key: &str| -> Result<Option<f64>> {
            match object.get(key) {
                Some(Value::Null) | None => Ok(None),
                Some(Value::Number(number)) => Ok(number.as_f64()),
                other => Err(Error::spec(
                    format!("{path}.{key}"),
                    format!("expected a number, got {}", shown(other)),
                )),
            }
        };
        let rewind = match object.get("rewind") {
            Some(Value::Null) | None => None,
            Some(Value::Bool(rewind)) => Some(*rewind),
            other => {
                return Err(Error::spec(
                    format!("{path}.rewind"),
                    format!("expected true or false, got {}", shown(other)),
                ));
            }
        };
        Ok(AnimateOptions {
            nframes: number("nframes")?,
            fps: number("fps")?,
            duration: number("duration")?,
            detail: number("detail")?,
            start_pause: number("start_pause")?,
            end_pause: number("end_pause")?,
            rewind,
        })
    }
}

/// `value` as a JSON object, or a spec error at `path`.
pub(crate) fn as_object<'a>(value: &'a Value, path: &str) -> Result<&'a Map<String, Value>> {
    value.as_object().ok_or_else(|| {
        Error::spec(
            path,
            format!("expected an object, got {}", shown(Some(value))),
        )
    })
}

/// A spec error for the first key of `object` not in `known`, with a close match.
pub(crate) fn check_keys(object: &Map<String, Value>, path: &str, known: &[&str]) -> Result<()> {
    match object.keys().find(|key| !known.contains(&key.as_str())) {
        Some(key) => Err(Error::spec(
            format!("{path}.{key}"),
            format!(
                "unknown option '{key}'{}; accepted: {}",
                did_you_mean(key, known.iter().copied()),
                known.join(", ")
            ),
        )),
        None => Ok(()),
    }
}

/// A non-negative whole number.
fn count(value: &Value, path: &str) -> Result<usize> {
    value
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| {
            Error::spec(
                path,
                format!("expected a whole number >= 0, got {}", shown(Some(value))),
            )
        })
}

/// A JSON value for an error message.
pub(crate) fn shown(value: Option<&Value>) -> String {
    match value {
        None => "nothing".to_owned(),
        Some(value) => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parses_a_transition_with_options_and_a_frame() {
        let spec = AnimationSpec::parse(&json!({
            "transition": {"name": "manual", "frames": "year"},
            "animate": {"nframes": 20, "fps": 5, "rewind": true},
            "frame": 3
        }))
        .unwrap();
        assert_eq!(spec.transition.0, "manual");
        assert_eq!(spec.options.nframes, Some(20.0));
        assert_eq!(spec.options.rewind, Some(true));
        assert_eq!(spec.options.duration, None);
        assert_eq!(spec.frame, Some(3));
    }

    #[test]
    fn errors_name_the_path_and_a_close_match() {
        let error = AnimationSpec::parse(&json!({
            "transition": {"name": "manual"},
            "animate": {"nframe": 20}
        }))
        .unwrap_err();
        let text = error.to_string();
        assert!(text.contains("animation.animate.nframe"), "{text}");
        assert!(text.contains("did you mean 'nframes'"), "{text}");
        let error = AnimationSpec::parse(&json!({"animate": {}})).unwrap_err();
        assert!(error.to_string().contains("no transition"), "{error}");
        let error = AnimationSpec::parse(&json!({
            "transition": {"name": "manual"}, "frame": -1
        }))
        .unwrap_err();
        assert!(error.to_string().contains("animation.frame"), "{error}");
    }
}
