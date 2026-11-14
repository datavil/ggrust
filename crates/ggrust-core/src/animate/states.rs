//! `transition_states(states, transition_length=1, state_length=1, wrap=True)`
//! (gganimate `TransitionStates`, MIT): each state is held for a while, then its
//! rows move to the next state's. The requested scene frames are shared between
//! the holds and the moves by their relative lengths (gganimate
//! `distribute_frames`); with `wrap` the last state moves back to the first.
//! Which rows move where and how is [`super::tween`]; this module only says,
//! per scene frame, which state is shown or which two states are tweened at
//! what progress, and the label variables `{previous_state}`, `{closest_state}`,
//! `{next_state}` and `{transitioning}` (gganimate `get_frame_info`).

use serde_json::{Map, Value};

use super::spec::{check_keys, shown};
use super::transition::{FrameShow, Transition};
use crate::build::StateVariable;
use crate::{Error, Result};

const KEYS: &[&str] = &[
    "name",
    "states",
    "transition_length",
    "state_length",
    "wrap",
];
const VARIABLES: &[&str] = &[
    "transitioning",
    "previous_state",
    "closest_state",
    "next_state",
];

/// A parsed `transition_states()`.
pub(super) struct States {
    variable: StateVariable,
    /// Relative lengths, recycled over the states (R `rep(length.out=)`).
    transition_length: Vec<f64>,
    state_length: Vec<f64>,
    wrap: bool,
    /// Per scene frame, planned by [`Transition::scene_frames`].
    frames: Vec<StateFrame>,
}

/// One scene frame of the plan.
#[derive(Debug, Clone, Copy, PartialEq)]
struct StateFrame {
    show: Show,
    transitioning: bool,
    previous: usize,
    closest: usize,
    next: usize,
}

/// What a scene frame draws.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Show {
    State(usize),
    Tween {
        from: usize,
        to: usize,
        progress: f64,
    },
}

/// Parse `{"name": "states", "states": "<column>", "transition_length": 1,
/// "state_length": 1, "wrap": true}`.
pub(super) fn parse(options: &Map<String, Value>, path: &str) -> Result<Box<dyn Transition>> {
    check_keys(options, path, KEYS)?;
    let states_path = format!("{path}.states");
    let name = match options.get("states") {
        Some(Value::String(name)) if !name.is_empty() => name.clone(),
        other => {
            return Err(Error::spec(
                &states_path,
                format!(
                    "transition_states() needs the column of the states as a string, got {}",
                    shown(other)
                ),
            ));
        }
    };
    let transition_length = lengths(options.get("transition_length"), path, "transition_length")?;
    let state_length = lengths(options.get("state_length"), path, "state_length")?;
    let wrap = match options.get("wrap") {
        Some(Value::Null) | None => true,
        Some(Value::Bool(wrap)) => *wrap,
        other => {
            return Err(Error::spec(
                format!("{path}.wrap"),
                format!("expected true or false, got {}", shown(other)),
            ));
        }
    };
    Ok(Box::new(States {
        variable: StateVariable {
            name,
            path: states_path,
        },
        transition_length,
        state_length,
        wrap,
        frames: Vec::new(),
    }))
}

/// A relative length or a list of them (one per state, recycled); default 1.
fn lengths(value: Option<&Value>, path: &str, key: &str) -> Result<Vec<f64>> {
    let path = format!("{path}.{key}");
    let invalid = |value: Option<&Value>| {
        Error::spec(
            &path,
            format!(
                "expected a number >= 0 or a non-empty list of them (one per state), got {}",
                shown(value)
            ),
        )
    };
    let number = |value: &Value| {
        value
            .as_f64()
            .filter(|number| number.is_finite() && *number >= 0.0)
    };
    match value {
        Some(Value::Null) | None => Ok(vec![1.0]),
        Some(Value::Array(values)) if !values.is_empty() => values
            .iter()
            .map(|item| number(item).ok_or_else(|| invalid(value)))
            .collect(),
        Some(other) => number(other).map(|n| vec![n]).ok_or_else(|| invalid(value)),
    }
}

/// R `rep(values, length.out = count)`.
fn recycled(values: &[f64], count: usize) -> Vec<f64> {
    (0..count)
        .map(|index| values[index % values.len()])
        .collect()
}

/// gganimate `distribute_frames`: `frames` scene frames shared between the
/// holds (`statics`) and the moves (`transitions`) by their lengths. Returns
/// the frames of each hold and each move.
fn distribute_frames(
    statics: &[f64],
    transitions: &[f64],
    frames: usize,
) -> (Vec<usize>, Vec<usize>) {
    let total: f64 = statics.iter().chain(transitions).sum();
    let share = |length: &f64| (length * frames as f64 / total).ceil() as usize;
    let mut static_frames: Vec<usize> = statics.iter().map(share).collect();
    let mut transition_frames: Vec<usize> = transitions.iter().map(share).collect();
    // Round robin: every part takes its first frame, then its second, ...,
    // until `frames` are taken (`order(ind)[seq_len(frames)]`, a stable order).
    let all: Vec<usize> = static_frames
        .iter()
        .chain(&transition_frames)
        .copied()
        .collect();
    let mut taken = vec![0usize; all.len()];
    let mut remaining = frames;
    let mut level = 1;
    while remaining > 0 && all.iter().any(|count| *count >= level) {
        for (part, count) in all.iter().enumerate() {
            if remaining == 0 {
                break;
            }
            if *count >= level {
                taken[part] += 1;
                remaining -= 1;
            }
        }
        level += 1;
    }
    // gganimate overwrites only the parts that took a frame (`table()` skips
    // the others, which keep their ceiling).
    let count = statics.len();
    for (part, taken) in taken.iter().enumerate() {
        if *taken > 0 {
            match part < count {
                true => static_frames[part] = *taken,
                false => transition_frames[part - count] = *taken,
            }
        }
    }
    // A move of zero frames before a hold that has frames takes one frame from
    // that hold. gganimate's test recycles the hold check over the zero moves
    // (`zero & static_frames[which(zero) + 1] != 0`); ported as written. A hold
    // past the last state is R's NA, never selected.
    let zero: Vec<usize> = (0..count).filter(|i| transition_frames[*i] == 0).collect();
    if !zero.is_empty() {
        let next_hold: Vec<Option<bool>> = zero
            .iter()
            .map(|i| static_frames.get(i + 1).map(|frames| *frames != 0))
            .collect();
        let selected: Vec<usize> = (0..count)
            .filter(|i| transition_frames[*i] == 0 && next_hold[i % zero.len()] == Some(true))
            .collect();
        for i in selected {
            // In R a selected last move would extend the holds with NA; skipped.
            if i + 1 < count {
                transition_frames[i] = 1;
                static_frames[i + 1] = static_frames[i + 1].saturating_sub(1);
            }
        }
    }
    (static_frames, transition_frames)
}

/// The scene frames of `states` states with these hold and move frames:
/// gganimate's `expand_panel` (`keep_state` and `tween_state` one after the
/// other, each new run replacing the last frame of the one before) for what is
/// drawn, and `get_frame_info` for the label variables.
fn plan_frames(holds: &[usize], moves: &[usize]) -> Vec<StateFrame> {
    let states = holds.len();
    let mut shows: Vec<Show> = Vec::new();
    // The state on the last frame so far. gganimate's `keep_state` and
    // `tween_state` start from the last frame, not from state `i`: after a move
    // of zero frames the next hold keeps the earlier state and the next move
    // starts from it (the labels still name state `i`). Ported as is.
    let mut current = 0;
    let mut has_frames = false;
    for state in 0..states {
        if holds[state] > 0 {
            let mut count = holds[state];
            if has_frames {
                shows.pop();
                count += 1;
            }
            shows.extend(std::iter::repeat_n(Show::State(current), count));
            has_frames = true;
        }
        if moves[state] > 0 {
            let to = (state + 1) % states;
            let mut count = moves[state];
            if has_frames {
                shows.pop();
                count += 1;
            }
            for k in 0..count {
                // tweenr: frame k of n has progress (k - 1) / (n - 1), 1-based;
                // the first is the `from` state, the last the `to` state.
                let progress = match count {
                    1 => 1.0,
                    _ => k as f64 / (count - 1) as f64,
                };
                shows.push(if progress <= 0.0 {
                    Show::State(current)
                } else if progress >= 1.0 {
                    Show::State(to)
                } else {
                    Show::Tween {
                        from: current,
                        to,
                        progress,
                    }
                });
            }
            has_frames = true;
            current = to;
        }
    }
    let mut frames = Vec::with_capacity(shows.len());
    for state in 0..states {
        for (length, transitioning) in [(holds[state], false), (moves[state], true)] {
            let next = match transitioning {
                true => (state + 1) % states,
                false => state,
            };
            for position in 0..length {
                // tweenr `tween_constant`: the `from` value while the linear
                // progress is below one half.
                let closest = match (position as f64 / length as f64) < 0.5 {
                    true => state,
                    false => next,
                };
                let show = shows
                    .get(frames.len())
                    .or(shows.last())
                    .copied()
                    .unwrap_or(Show::State(0));
                frames.push(StateFrame {
                    show,
                    transitioning,
                    previous: state,
                    closest,
                    next,
                });
            }
        }
    }
    frames
}

impl Transition for States {
    fn state_variable(&self) -> &StateVariable {
        &self.variable
    }

    fn scene_frames(&mut self, states: usize, requested: usize) -> Result<usize> {
        let mut transitions = recycled(&self.transition_length, states);
        if !self.wrap {
            if let Some(last) = transitions.last_mut() {
                *last = 0.0;
            }
        }
        let holds = recycled(&self.state_length, states);
        let total: f64 = holds.iter().chain(&transitions).sum();
        if total <= 0.0 {
            return Err(Error::spec(
                self.variable.path.replace(".states", ".state_length"),
                format!(
                    "transition_states() has no length to share frames by: state_length and \
                     transition_length are all 0{}; give one of them a positive length",
                    if self.wrap {
                        ""
                    } else {
                        " (without wrap the last transition has no length)"
                    }
                ),
            ));
        }
        let (holds, moves) =
            distribute_frames(&holds, &transitions, requested + usize::from(self.wrap));
        self.frames = plan_frames(&holds, &moves);
        Ok(self.frames.len())
    }

    fn frame_show(&self, frame: usize) -> FrameShow {
        match self.frames.get(frame).map(|frame| frame.show) {
            Some(Show::Tween { from, to, progress }) => FrameShow::Tween { from, to, progress },
            Some(Show::State(state)) => FrameShow::States(vec![state]),
            None => FrameShow::States(Vec::new()),
        }
    }

    fn frame_variables(&self, frame: usize, labels: &[String]) -> Vec<(&'static str, String)> {
        let label = |state: usize| labels.get(state).cloned().unwrap_or_default();
        match self.frames.get(frame) {
            Some(frame) => vec![
                (
                    "transitioning",
                    if frame.transitioning { "TRUE" } else { "FALSE" }.to_owned(),
                ),
                ("previous_state", label(frame.previous)),
                ("closest_state", label(frame.closest)),
                ("next_state", label(frame.next)),
            ],
            None => Vec::new(),
        }
    }

    fn variable_names(&self) -> &'static [&'static str] {
        VARIABLES
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn states(options: Value) -> Result<Box<dyn Transition>> {
        parse(options.as_object().unwrap(), "animation.transition")
    }

    #[test]
    fn frames_are_shared_by_relative_length() {
        // 3 states, wrap: 101 frames over 6 equal parts: ceil(16.83) = 17 each,
        // round robin takes 101 = 5 * 17 + 16.
        let (holds, moves) = distribute_frames(&[1.0; 3], &[1.0; 3], 101);
        assert_eq!(holds, vec![17, 17, 17]);
        assert_eq!(moves, vec![17, 17, 16]);
        // No wrap: the last move has no length.
        let (holds, moves) = distribute_frames(&[1.0; 3], &[1.0, 1.0, 0.0], 100);
        assert_eq!(holds, vec![20, 20, 20]);
        assert_eq!(moves, vec![20, 20, 0]);
        // Lengths 2:1 over 2 states without wrap: total 5, ceil(2*10/5) = 4.
        let (holds, moves) = distribute_frames(&[2.0, 2.0], &[1.0, 0.0], 10);
        assert_eq!(holds, vec![4, 4]);
        assert_eq!(moves, vec![2, 0]);
        // Zero-length moves take one frame from the next hold.
        let (holds, moves) = distribute_frames(&[1.0; 3], &[0.0; 3], 9);
        assert_eq!(holds, vec![3, 2, 2]);
        assert_eq!(moves, vec![1, 1, 0]);
    }

    /// gganimate's `distribute_frames` run in R
    /// (`tests/fixtures/generate_animation_reference.R`). A case where R
    /// extends the holds with NA (gganimate then fails) is skipped.
    #[test]
    fn distribution_matches_gganimate() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/animation_reference.json"
        );
        let reference: Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let numbers = |value: &Value| -> Vec<f64> {
            value
                .as_array()
                .unwrap()
                .iter()
                .map(|item| item.as_f64().unwrap())
                .collect()
        };
        let counts = |value: &Value| -> Option<Vec<usize>> {
            value
                .as_array()
                .unwrap()
                .iter()
                .map(|item| item.as_u64().map(|count| count as usize))
                .collect()
        };
        let mut checked = 0;
        for case in reference["distribute"].as_array().unwrap() {
            let Some(holds) = counts(&case["holds"]) else {
                continue;
            };
            let frames = case["frames"].as_u64().unwrap() as usize;
            let result = distribute_frames(
                &numbers(&case["statics"]),
                &numbers(&case["transitions"]),
                frames,
            );
            assert_eq!(result.0, holds, "holds for {case}");
            assert_eq!(
                result.1,
                counts(&case["moves"]).unwrap(),
                "moves for {case}"
            );
            checked += 1;
        }
        assert!(checked >= 8);
    }

    #[test]
    fn holds_then_tweens_with_the_last_tween_frame_on_the_next_state() {
        let frames = plan_frames(&[2, 2], &[3, 0]);
        let shows: Vec<Show> = frames.iter().map(|frame| frame.show).collect();
        let tween = |progress| Show::Tween {
            from: 0,
            to: 1,
            progress,
        };
        assert_eq!(
            shows,
            vec![
                Show::State(0),
                Show::State(0),
                tween(1.0 / 3.0),
                tween(2.0 / 3.0),
                Show::State(1),
                Show::State(1),
                Show::State(1),
            ]
        );
        let closest: Vec<usize> = frames.iter().map(|frame| frame.closest).collect();
        // Transition frames at positions 0, 1, 2 of 3: closest switches at 1/2.
        assert_eq!(closest, vec![0, 0, 0, 0, 1, 1, 1]);
        let transitioning: Vec<bool> = frames.iter().map(|frame| frame.transitioning).collect();
        assert_eq!(
            transitioning,
            vec![false, false, true, true, true, false, false]
        );
        assert_eq!(frames[3].previous, 0);
        assert_eq!(frames[3].next, 1);
    }

    #[test]
    fn a_zero_move_keeps_the_earlier_state_as_gganimate() {
        // Holds [1, 0, 1], moves [0, 2, 0]: state 1 never shows; the move after
        // it starts from state 0 (gganimate tweens from the last frame).
        let shows: Vec<Show> = plan_frames(&[1, 0, 1], &[0, 2, 0])
            .iter()
            .map(|frame| frame.show)
            .collect();
        assert_eq!(
            shows,
            vec![
                Show::State(0),
                Show::Tween {
                    from: 0,
                    to: 2,
                    progress: 0.5
                },
                Show::State(2),
                Show::State(2),
            ]
        );
    }

    #[test]
    fn wrap_moves_back_to_the_first_state() {
        let mut transition = states(json!({"name": "states", "states": "year"})).unwrap();
        // 3 states, 100 frames asked: 101 planned (gganimate plans nframes + 1
        // with wrap; the timeline samples 100 of them).
        assert_eq!(transition.scene_frames(3, 100).unwrap(), 101);
        let labels: Vec<String> = ["a", "b", "c"].map(String::from).to_vec();
        let last = transition.frame_variables(100, &labels);
        assert_eq!(last[0], ("transitioning", "TRUE".to_owned()));
        assert_eq!(last[1].1, "c");
        assert_eq!(last[3].1, "a");
        match transition.frame_show(95) {
            FrameShow::Tween { from, to, .. } => assert_eq!((from, to), (2, 0)),
            other => panic!("expected a tween, got {other:?}"),
        }
        assert_eq!(transition.frame_show(0), FrameShow::States(vec![0]));
        assert_eq!(transition.frame_show(100), FrameShow::States(vec![0]));
        let mut no_wrap =
            states(json!({"name": "states", "states": "year", "wrap": false})).unwrap();
        assert_eq!(no_wrap.scene_frames(3, 100).unwrap(), 100);
        assert_eq!(no_wrap.frame_show(99), FrameShow::States(vec![2]));
    }

    #[test]
    fn options_are_checked() {
        let error = states(json!({"name": "states"})).err().unwrap().to_string();
        assert!(error.contains("animation.transition.states"), "{error}");
        let error = states(json!({"name": "states", "states": "s", "state_length": -1}))
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("state_length"), "{error}");
        let error = states(json!({"name": "states", "states": "s", "wrap": "yes"}))
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("wrap"), "{error}");
        let mut zero = states(json!({"name": "states", "states": "s",
            "state_length": 0, "transition_length": [0, 2], "wrap": false}))
        .unwrap();
        let error = zero.scene_frames(2, 10).unwrap_err().to_string();
        assert!(error.contains("no length"), "{error}");
        let mut per_state = states(json!({"name": "states", "states": "s",
            "state_length": [1, 3], "transition_length": 1}))
        .unwrap();
        assert!(per_state.scene_frames(2, 10).is_ok());
    }
}
