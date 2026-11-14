//! The `Transition` trait and its registry. A transition says which column holds
//! the states, how many scene frames there are, which states each scene frame
//! draws and which label variables it has. One file per transition; add it to
//! [`TRANSITIONS`]. Transitions do not draw or plan output frames.

use serde_json::{Map, Value};

use super::manual;
use super::spec::PATH;
use crate::build::StateVariable;
use crate::suggest::did_you_mean;
use crate::{Error, Result};

/// One gganimate transition with its parameters.
pub(crate) trait Transition: Send + Sync {
    /// The column whose values are the states.
    fn state_variable(&self) -> &StateVariable;
    /// gganimate `adjust_nframes`: scene frames for `states` states when
    /// `requested` were asked for.
    fn scene_frames(&self, states: usize, requested: usize) -> usize;
    /// States (0-based, in drawing order) whose rows scene frame `frame` draws.
    fn frame_states(&self, frame: usize, states: usize) -> Vec<usize>;
    /// The transition's label variables of scene frame `frame`, by name, given
    /// the state labels.
    fn frame_variables(&self, frame: usize, labels: &[String]) -> Vec<(&'static str, String)>;
    /// Names of [`Self::frame_variables`], for template errors.
    fn variable_names(&self) -> &'static [&'static str];
}

/// Parses a transition's spec (`animation.transition`) at `path`.
type Parse = fn(&Map<String, Value>, &str) -> Result<Box<dyn Transition>>;

/// Every transition by gganimate name (`transition_<name>`).
const TRANSITIONS: &[(&str, Parse)] = &[("manual", manual::parse)];

/// gganimate transitions scheduled for later steps (plan.md Phase 13).
const PLANNED: &[&str] = &["states", "time", "reveal"];

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
        let error = transition("states", &options).err().unwrap().to_string();
        assert!(error.contains("not available yet"), "{error}");
        let options = json!({"name": "manual", "frames": "year"});
        assert!(transition("manual", options.as_object().unwrap()).is_ok());
    }
}
