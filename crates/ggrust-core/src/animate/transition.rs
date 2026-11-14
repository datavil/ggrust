//! The `Transition` trait and its registry. A transition says which column holds
//! the states, how many scene frames there are, which states each scene frame
//! draws (or which two it tweens) and which label variables it has. One file per transition; add it to
//! [`TRANSITIONS`]. Transitions do not draw or plan output frames.

use serde_json::{Map, Value};

use super::along::TimeClass;
use super::spec::PATH;
use super::{manual, reveal, states, time};
use crate::build::StateVariable;
use crate::suggest::did_you_mean;
use crate::{Error, Result};

/// One gganimate transition with its parameters.
pub(crate) trait Transition: Send + Sync {
    /// The column whose values are the states.
    fn state_variable(&self) -> &StateVariable;
    /// Plan the scene frames for `states` states when `requested` were asked
    /// for (gganimate `setup_params` and `adjust_nframes`) and return how many
    /// there are. Called once, before the other frame methods.
    ///
    /// # Errors
    ///
    /// A spec error when the transition's options cannot plan any frame.
    fn scene_frames(&mut self, states: &StateSet, requested: usize) -> Result<usize>;
    /// What scene frame `frame` draws.
    fn frame_show(&self, frame: usize) -> FrameShow;
    /// The frame position (1-based, gganimate's numbering, may lie outside the
    /// frames) of every state, for [`FrameShow::Time`] and
    /// [`FrameShow::Reveal`]; empty for transitions that show whole states.
    fn state_positions(&self) -> &[i64] {
        &[]
    }
    /// The transition's label variables of scene frame `frame`, by name, given
    /// the state labels.
    fn frame_variables(&self, frame: usize, labels: &[String]) -> Vec<(&'static str, String)>;
    /// Names of [`Self::frame_variables`], for template errors.
    fn variable_names(&self) -> &'static [&'static str];
}

/// What one scene frame draws of the layers that have states.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FrameShow {
    /// The rows of these states (0-based, in drawing order).
    States(Vec<usize>),
    /// The rows of `from` moved toward the matching rows of `to`; `progress`
    /// (0 to 1, not eased) is the share of the way.
    Tween {
        from: usize,
        to: usize,
        progress: f64,
    },
    /// `transition_time`: every layer moves between its own two observed
    /// states around frame position `position` (see
    /// [`Transition::state_positions`]) and draws nothing outside them.
    Time { position: i64 },
    /// `transition_reveal`: every group shows its rows up to frame position
    /// `position`, plus a row interpolated at `position`.
    Reveal { position: i64, keep_last: bool },
}

/// What a transition knows of the states when it plans its frames.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StateSet<'a> {
    /// The number of states.
    pub count: usize,
    /// The number of every state, ascending, when the column is numeric
    /// (dates in epoch milliseconds); `None` for a categorical column.
    pub values: Option<&'a [f64]>,
    /// How the column's numbers print in labels (`{frame_time}`).
    pub class: TimeClass,
}

#[cfg(test)]
impl StateSet<'static> {
    /// `count` categorical states (tests of transitions that need no numbers).
    pub(crate) fn counted(count: usize) -> StateSet<'static> {
        StateSet {
            count,
            values: None,
            class: TimeClass::Numeric,
        }
    }
}

/// Parses a transition's spec (`animation.transition`) at `path`.
type Parse = fn(&Map<String, Value>, &str) -> Result<Box<dyn Transition>>;

/// Every transition by gganimate name (`transition_<name>`).
const TRANSITIONS: &[(&str, Parse)] = &[
    ("manual", manual::parse),
    ("states", states::parse),
    ("time", time::parse),
    ("reveal", reveal::parse),
];

/// gganimate transitions not available (yet): plan.md Phase 13, on request.
const PLANNED: &[&str] = &["filter", "layers", "events", "components", "null"];

/// The transition named in `animation.transition`.
///
/// # Errors
///
/// A spec error at `animation.transition.name` for an unknown or not yet
/// available transition, or the transition's own parse error.
pub(crate) fn transition(name: &str, options: &Map<String, Value>) -> Result<Box<dyn Transition>> {
    let path = format!("{PATH}.transition");
    if let Some((_, parse)) = TRANSITIONS.iter().find(|(known, _)| *known == name) {
        return parse(options, &path);
    }
    let names: Vec<&str> = TRANSITIONS.iter().map(|(known, _)| *known).collect();
    let message = if PLANNED.contains(&name) {
        format!(
            "transition_{name}() is not available yet; available: {}",
            listing(&names)
        )
    } else {
        format!(
            "unknown transition '{name}'{}; available: {}",
            did_you_mean(name, names.iter().chain(PLANNED).copied()),
            listing(&names)
        )
    };
    Err(Error::spec(format!("{path}.name"), message))
}

fn listing(names: &[&str]) -> String {
    names
        .iter()
        .map(|name| format!("transition_{name}()"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn unknown_and_planned_names_are_spec_errors() {
        let options = Map::new();
        let error = transition("manul", &options).err().unwrap().to_string();
        assert!(error.contains("animation.transition.name"), "{error}");
        assert!(error.contains("did you mean 'manual'"), "{error}");
        let error = transition("filter", &options).err().unwrap().to_string();
        assert!(error.contains("not available yet"), "{error}");
        let options = json!({"name": "manual", "frames": "year"});
        assert!(transition("manual", options.as_object().unwrap()).is_ok());
    }
}
