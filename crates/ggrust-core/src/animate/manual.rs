//! `transition_manual(frames, cumulative=False)` (gganimate `TransitionManual`):
//! one scene frame per state, no tweening. With `cumulative`, frame `k` draws
//! states `0..=k`. Label variables: `{previous_frame}`, `{current_frame}`,
//! `{next_frame}` (the state labels around the frame, `""` past the ends).

use serde_json::{Map, Value};

use super::spec::{check_keys, shown};
use super::transition::{FrameShow, Transition};
use crate::build::StateVariable;
use crate::{Error, Result};

const KEYS: &[&str] = &["name", "frames", "cumulative"];
const VARIABLES: &[&str] = &["previous_frame", "current_frame", "next_frame"];

/// A parsed `transition_manual()`.
pub(super) struct Manual {
    variable: StateVariable,
    cumulative: bool,
    /// The number of states, set by [`Transition::scene_frames`].
    states: usize,
}

/// Parse `{"name": "manual", "frames": "<column>", "cumulative": false}`.
pub(super) fn parse(options: &Map<String, Value>, path: &str) -> Result<Box<dyn Transition>> {
    check_keys(options, path, KEYS)?;
    let frames_path = format!("{path}.frames");
    let name = match options.get("frames") {
        Some(Value::String(name)) if !name.is_empty() => name.clone(),
        other => {
            return Err(Error::spec(
                &frames_path,
                format!(
                    "transition_manual() needs the column of the frames as a string, got {}",
                    shown(other)
                ),
            ));
        }
    };
    let cumulative = match options.get("cumulative") {
        Some(Value::Null) | None => false,
        Some(Value::Bool(cumulative)) => *cumulative,
        other => {
            return Err(Error::spec(
                format!("{path}.cumulative"),
                format!("expected true or false, got {}", shown(other)),
            ));
        }
    };
    Ok(Box::new(Manual {
        variable: StateVariable {
            name,
            path: frames_path,
        },
        cumulative,
        states: 0,
    }))
}

impl Transition for Manual {
    fn state_variable(&self) -> &StateVariable {
        &self.variable
    }

    fn scene_frames(&mut self, states: usize, _requested: usize) -> Result<usize> {
        self.states = states;
        Ok(states)
    }

    fn frame_show(&self, frame: usize) -> FrameShow {
        FrameShow::States(match self.cumulative {
            true => (0..=frame.min(self.states.saturating_sub(1))).collect(),
            false => vec![frame],
        })
    }

    fn frame_variables(&self, frame: usize, labels: &[String]) -> Vec<(&'static str, String)> {
        let label = |index: Option<usize>| {
            index
                .and_then(|index| labels.get(index))
                .cloned()
                .unwrap_or_default()
        };
        vec![
            ("previous_frame", label(frame.checked_sub(1))),
            ("current_frame", label(Some(frame))),
            ("next_frame", label(Some(frame + 1))),
        ]
    }

    fn variable_names(&self) -> &'static [&'static str] {
        VARIABLES
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn manual(options: Value) -> Result<Box<dyn Transition>> {
        parse(options.as_object().unwrap(), "animation.transition")
    }

    #[test]
    fn one_frame_per_state_or_all_states_so_far() {
        let mut plain = manual(json!({"name": "manual", "frames": "year"})).unwrap();
        assert_eq!(plain.scene_frames(4, 100).unwrap(), 4);
        assert_eq!(plain.frame_show(2), FrameShow::States(vec![2]));
        let mut cumulative =
            manual(json!({"name": "manual", "frames": "year", "cumulative": true})).unwrap();
        cumulative.scene_frames(4, 100).unwrap();
        assert_eq!(cumulative.frame_show(2), FrameShow::States(vec![0, 1, 2]));
    }

    #[test]
    fn frame_variables_are_the_neighbouring_states() {
        let plain = manual(json!({"name": "manual", "frames": "year"})).unwrap();
        let labels: Vec<String> = ["1952", "1957", "1962"].map(String::from).to_vec();
        assert_eq!(
            plain.frame_variables(0, &labels),
            vec![
                ("previous_frame", String::new()),
                ("current_frame", "1952".to_owned()),
                ("next_frame", "1957".to_owned()),
            ]
        );
        assert_eq!(plain.frame_variables(2, &labels)[2].1, "");
    }

    #[test]
    fn frames_must_name_a_column() {
        let error = manual(json!({"name": "manual"})).err().unwrap().to_string();
        assert!(error.contains("animation.transition.frames"), "{error}");
        let error = manual(json!({"name": "manual", "frames": "x", "cumulative": 1}))
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("cumulative"), "{error}");
    }
}
