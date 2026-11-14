//! `transition_time(time, range=None)` (gganimate `TransitionTime`, MIT): the
//! frames are spaced evenly in time over the range of `time`, and every time in
//! the data is rounded to its nearest frame. Between two frames that hold
//! observed times, each layer's rows move from the earlier time to the later
//! (matched by group, eased, as [`super::tween`]); a frame that holds a time
//! shows its rows as they are. Each layer uses its own times: before its first
//! time and after its last it draws nothing. When two times of a layer round to
//! the same frame, only the earlier one is drawn (gganimate drops the other).
//! Label variable: `{frame_time}`, the time the frame stands for.

use serde_json::{Map, Value};

use super::along::{AlongFrames, Rounding, parse_range, parse_variable, plan};
use super::spec::check_keys;
use super::transition::{FrameShow, StateSet, Transition};
use crate::Result;
use crate::build::StateVariable;

const KEYS: &[&str] = &["name", "time", "range"];
const VARIABLES: &[&str] = &["frame_time"];

/// A parsed `transition_time()`.
pub(super) struct Time {
    variable: StateVariable,
    range: Option<[f64; 2]>,
    /// Planned by [`Transition::scene_frames`].
    frames: AlongFrames,
}

/// Parse `{"name": "time", "time": "<column>", "range": [low, high] | null}`.
pub(super) fn parse(options: &Map<String, Value>, path: &str) -> Result<Box<dyn Transition>> {
    check_keys(options, path, KEYS)?;
    Ok(Box::new(Time {
        variable: parse_variable(options, path, "time", "transition_time")?,
        range: parse_range(options, path)?,
        frames: AlongFrames::default(),
    }))
}

impl Transition for Time {
    fn state_variable(&self) -> &StateVariable {
        &self.variable
    }

    fn scene_frames(&mut self, states: &StateSet, requested: usize) -> Result<usize> {
        self.frames = plan(
            states,
            self.range,
            requested,
            &self.variable,
            "transition_time",
            Rounding::Time,
        )?;
        Ok(requested)
    }

    fn frame_show(&self, frame: usize) -> FrameShow {
        FrameShow::Time {
            position: frame as i64 + 1,
        }
    }

    fn state_positions(&self) -> &[i64] {
        &self.frames.positions
    }

    fn frame_variables(&self, frame: usize, _labels: &[String]) -> Vec<(&'static str, String)> {
        let time = self.frames.times.get(frame).copied().unwrap_or(f64::NAN);
        vec![("frame_time", self.frames.class.label(time))]
    }

    fn variable_names(&self) -> &'static [&'static str] {
        VARIABLES
    }
}

/// Per state of one layer, whether it is drawn: the layer's states (those with
/// rows in any tile, in `present`) in time order, minus those whose frame
/// position an earlier state of the layer already holds.
pub(super) fn kept_states(present: &[bool], positions: &[i64]) -> Vec<bool> {
    let mut kept = vec![false; present.len()];
    let mut last: Option<i64> = None;
    for (state, is_present) in present.iter().enumerate() {
        let Some(position) = positions.get(state) else {
            continue;
        };
        if *is_present && last != Some(*position) {
            kept[state] = true;
            last = Some(*position);
        }
    }
    kept
}

/// What one layer of one tile draws at frame `position`, given its anchors
/// (`(frame position, state)` of its kept states with rows in the tile, in
/// time order).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum TimeShow {
    /// Nothing: before the layer's first time or after its last.
    Nothing,
    /// The rows of one state as they are.
    State(usize),
    /// The rows of `from` moved `progress` of the way to `to`.
    Tween {
        from: usize,
        to: usize,
        progress: f64,
    },
}

/// [`TimeShow`] of frame `position` over `anchors`.
pub(super) fn time_show(anchors: &[(i64, usize)], position: i64) -> TimeShow {
    let after = anchors.partition_point(|(anchor, _)| *anchor <= position);
    if after == 0 {
        return TimeShow::Nothing;
    }
    let (start, from) = anchors[after - 1];
    if start == position {
        return TimeShow::State(from);
    }
    match anchors.get(after) {
        None => TimeShow::Nothing,
        Some((end, to)) => TimeShow::Tween {
            from,
            to: *to,
            // In f64: positions far outside the range would overflow i64.
            progress: (position as f64 - start as f64) / (*end as f64 - start as f64),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicates_keep_the_earlier_time() {
        let present = [true, true, false, true, true];
        let positions = [1, 1, 2, 2, 5];
        assert_eq!(
            kept_states(&present, &positions),
            vec![true, false, false, true, true]
        );
    }

    #[test]
    fn frames_between_times_tween_and_outside_draw_nothing() {
        let anchors = [(3, 0), (7, 2), (8, 3)];
        assert_eq!(time_show(&anchors, 1), TimeShow::Nothing);
        assert_eq!(time_show(&anchors, 3), TimeShow::State(0));
        assert_eq!(
            time_show(&anchors, 4),
            TimeShow::Tween {
                from: 0,
                to: 2,
                progress: 0.25
            }
        );
        assert_eq!(time_show(&anchors, 7), TimeShow::State(2));
        assert_eq!(time_show(&anchors, 8), TimeShow::State(3));
        assert_eq!(time_show(&anchors, 9), TimeShow::Nothing);
        assert_eq!(time_show(&[], 1), TimeShow::Nothing);
        let far = [(-(1 << 62), 0), (1 << 62, 1)];
        assert_eq!(
            time_show(&far, 0),
            TimeShow::Tween {
                from: 0,
                to: 1,
                progress: 0.5
            }
        );
    }
}
