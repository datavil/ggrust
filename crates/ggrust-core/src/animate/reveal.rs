//! `transition_reveal(along, range=None, keep_last=True)` (gganimate
//! `TransitionReveal` over tweenr `tween_along`, MIT): the frames are spaced
//! evenly over the range of `along`, and each row's `along` is rounded to its
//! nearest frame. In every group (sorted by `along`), a frame shows:
//!
//! - lines, paths, areas, polygons (geoms that join their rows): every row up
//!   to the frame, plus a row interpolated at the frame between the last row
//!   reached and the next, so the line grows smoothly;
//! - other geoms (points, texts, ...): only the interpolated row, a mark that
//!   travels along the group; after the group's last row, that row stays when
//!   `keep_last` is true.
//!
//! A group draws nothing before its first row. Label variable: `{frame_along}`.
//! Stats run once per `along` value, as gganimate's pre-stat transition does.

use serde_json::{Map, Value};

use super::along::{AlongFrames, Rounding, parse_range, parse_variable, plan};
use super::spec::{check_keys, shown};
use super::transition::{FrameShow, StateSet, Transition};
use crate::build::StateVariable;
use crate::build::frame::LayerFrame;
use crate::{Error, Result};

const KEYS: &[&str] = &["name", "along", "range", "keep_last"];
const VARIABLES: &[&str] = &["frame_along"];

/// A parsed `transition_reveal()`.
pub(super) struct Reveal {
    variable: StateVariable,
    range: Option<[f64; 2]>,
    keep_last: bool,
    /// Planned by [`Transition::scene_frames`].
    frames: AlongFrames,
}

/// Parse `{"name": "reveal", "along": "<column>", "range": [low, high] | null,
/// "keep_last": true}`.
pub(super) fn parse(options: &Map<String, Value>, path: &str) -> Result<Box<dyn Transition>> {
    check_keys(options, path, KEYS)?;
    let keep_last = match options.get("keep_last") {
        Some(Value::Null) | None => true,
        Some(Value::Bool(keep_last)) => *keep_last,
        other => {
            return Err(Error::spec(
                format!("{path}.keep_last"),
                format!("expected true or false, got {}", shown(other)),
            ));
        }
    };
    Ok(Box::new(Reveal {
        variable: parse_variable(options, path, "along", "transition_reveal")?,
        range: parse_range(options, path)?,
        keep_last,
        frames: AlongFrames::default(),
    }))
}

impl Transition for Reveal {
    fn state_variable(&self) -> &StateVariable {
        &self.variable
    }

    fn scene_frames(&mut self, states: &StateSet, requested: usize) -> Result<usize> {
        self.frames = plan(
            states,
            self.range,
            requested,
            &self.variable,
            "transition_reveal",
            Rounding::Reveal,
        )?;
        Ok(requested)
    }

    fn frame_show(&self, frame: usize) -> FrameShow {
        FrameShow::Reveal {
            position: frame as i64 + 1,
            keep_last: self.keep_last,
        }
    }

    fn state_positions(&self) -> &[i64] {
        &self.frames.positions
    }

    fn frame_variables(&self, frame: usize, _labels: &[String]) -> Vec<(&'static str, String)> {
        let along = self.frames.times.get(frame).copied().unwrap_or(f64::NAN);
        vec![("frame_along", self.frames.class.label(along))]
    }

    fn variable_names(&self) -> &'static [&'static str] {
        VARIABLES
    }
}

/// One layer of one tile, ordered for revealing: its rows that have an
/// `along` value, by group and then by frame position (stable, so rows of one
/// position keep their order).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct RevealRows {
    /// Indices into the layer's prepared rows.
    rows: Vec<u32>,
    /// Frame position of each row.
    positions: Vec<i64>,
    /// `[start, end)` of each group in `rows`.
    groups: Vec<(usize, usize)>,
}

impl RevealRows {
    /// Order the rows of `by_state` (rows per state) of `all`, whose states sit
    /// at `positions`.
    pub(super) fn new(all: &LayerFrame, by_state: &[Vec<u32>], positions: &[i64]) -> RevealRows {
        let mut entries: Vec<(u32, i64, u32)> = by_state
            .iter()
            .zip(positions)
            .flat_map(|(rows, position)| {
                rows.iter()
                    .map(move |row| (all.data.group_of(*row as usize), *position, *row))
            })
            .collect();
        entries.sort_by_key(|(group, position, _)| (*group, *position));
        let mut groups = Vec::new();
        let mut start = 0;
        for index in 1..=entries.len() {
            if index == entries.len() || entries[index].0 != entries[start].0 {
                groups.push((start, index));
                start = index;
            }
        }
        RevealRows {
            rows: entries.iter().map(|(_, _, row)| *row).collect(),
            positions: entries.iter().map(|(_, position, _)| *position).collect(),
            groups,
        }
    }

    /// The rows frame `position` draws, as `(from row, to row, progress)`
    /// for [`super::tween::mix_rows`]: a row as it is is `(row, row, 0)`.
    /// `connected` geoms keep the rows already reached (tweenr's `history`).
    pub(super) fn frame(
        &self,
        position: i64,
        keep_last: bool,
        connected: bool,
    ) -> Vec<(u32, u32, f64)> {
        let mut shown = Vec::new();
        for &(start, end) in &self.groups {
            let positions = &self.positions[start..end];
            // Rows start..reached are at or before the frame.
            let reached = start + positions.partition_point(|row| *row <= position);
            if reached == start {
                continue;
            }
            let at = |index: usize| (self.rows[index], self.rows[index], 0.0);
            if reached == end {
                match connected {
                    true => shown.extend((start..end).map(at)),
                    false if keep_last => shown.push(at(end - 1)),
                    false => {}
                }
                continue;
            }
            let last = reached - 1;
            // In f64: positions far outside the range would overflow i64.
            let start_position = self.positions[last] as f64;
            let progress = (position as f64 - start_position)
                / (self.positions[reached] as f64 - start_position);
            if connected {
                // At progress 0 the moving row is the last row reached: drawn
                // once (gganimate drops the repeated raw row).
                let before = if progress == 0.0 { last } else { reached };
                shown.extend((start..before).map(at));
            }
            shown.push((self.rows[last], self.rows[reached], progress));
        }
        shown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Column, ContinuousColumn, LayerData};
    use crate::spec::Aesthetic;
    use crate::style::Styles;

    /// Two groups: group 1 at positions 1, 4, 7; group 0 at 3, 5.
    fn layer() -> (LayerFrame, Vec<Vec<u32>>, Vec<i64>) {
        let mut data = LayerData::new(5);
        data.insert(
            Aesthetic::X,
            Column::Continuous(ContinuousColumn::from_values(vec![1.0, 4.0, 7.0, 3.0, 5.0])),
        )
        .unwrap();
        data.set_group_index(Some(vec![1, 1, 1, 0, 0])).unwrap();
        let all = LayerFrame {
            data,
            styles: Styles::default(),
        };
        // States at positions 1, 3, 4, 5, 7.
        let by_state = vec![vec![0], vec![3], vec![1], vec![4], vec![2]];
        (all, by_state, vec![1, 3, 4, 5, 7])
    }

    #[test]
    fn lines_keep_reached_rows_and_add_the_moving_end() {
        let (all, by_state, positions) = layer();
        let rows = RevealRows::new(&all, &by_state, &positions);
        assert_eq!(rows.rows, vec![3, 4, 0, 1, 2]);
        // Frame 2: group 0 not started; group 1 between rows 0 (1) and 1 (4).
        assert_eq!(
            rows.frame(2, true, true),
            vec![(0, 0, 0.0), (0, 1, 1.0 / 3.0)]
        );
        // Frame 4: group 0 between 3 and 5; group 1 exactly at row 1.
        assert_eq!(
            rows.frame(4, true, true),
            vec![(3, 3, 0.0), (3, 4, 0.5), (0, 0, 0.0), (1, 2, 0.0)]
        );
        // Frame 7: both groups complete, also without keep_last.
        assert_eq!(rows.frame(7, false, true).len(), 5);
    }

    /// tweenr's `numeric_along_interpolator` (as R in the fixture) plus
    /// gganimate's repeated-row removal: the x values per frame and group.
    #[test]
    fn revealed_rows_match_tweenr() {
        use crate::animate::along::tests::{r_values, reference};
        use crate::animate::along::{Rounding, TimeClass, plan};
        use crate::animate::ease::Easing;
        use crate::animate::transition::StateSet;
        use crate::animate::tween::mix_rows;
        for case in reference()["reveal"].as_array().unwrap() {
            let x: Vec<f64> = r_values(&case["x"]).into_iter().flatten().collect();
            let along: Vec<f64> = r_values(&case["along"]).into_iter().flatten().collect();
            let groups: Vec<u32> = r_values(&case["group"])
                .into_iter()
                .map(|group| group.unwrap() as u32)
                .collect();
            let frames = case["nframes"].as_u64().unwrap() as usize;
            let mut values = along.clone();
            values.sort_by(f64::total_cmp);
            values.dedup();
            let states = StateSet {
                count: values.len(),
                values: Some(&values),
                class: TimeClass::Numeric,
            };
            let variable = crate::build::StateVariable {
                name: "along".to_owned(),
                path: "along".to_owned(),
            };
            let planned = plan(&states, None, frames, &variable, "f", Rounding::Reveal).unwrap();
            let state_of = |value: f64| values.iter().position(|known| *known == value).unwrap();
            let positions: Vec<Option<f64>> = along
                .iter()
                .map(|value| Some(planned.positions[state_of(*value)] as f64))
                .collect();
            assert_eq!(positions, r_values(&case["positions"]), "{case}");
            let mut data = LayerData::new(x.len());
            data.insert(
                Aesthetic::X,
                Column::Continuous(ContinuousColumn::from_values(x.clone())),
            )
            .unwrap();
            data.set_group_index(Some(groups.clone())).unwrap();
            let all = LayerFrame {
                data,
                styles: Styles::default(),
            };
            let mut by_state = vec![Vec::new(); values.len()];
            for (row, value) in along.iter().enumerate() {
                by_state[state_of(*value)].push(row as u32);
            }
            let rows = RevealRows::new(&all, &by_state, &planned.positions);
            for (key, keep_last, connected) in [
                ("path", true, true),
                ("point", true, false),
                ("point_no_last", false, false),
            ] {
                let expected = &case[key];
                for position in 1..=frames as i64 {
                    let want: Vec<(u32, f64)> = r_values(&expected["frame"])
                        .iter()
                        .zip(r_values(&expected["group"]))
                        .zip(r_values(&expected["data"]))
                        .filter(|((frame, _), _)| **frame == Some(position as f64))
                        .map(|((_, group), value)| (group.unwrap() as u32, value.unwrap()))
                        .collect();
                    let shown = rows.frame(position, keep_last, connected);
                    let frame = mix_rows(&all, &shown, &Easing::default()).unwrap();
                    let Some(Column::Continuous(xs)) = frame.data.get(Aesthetic::X) else {
                        panic!("no x");
                    };
                    let got: Vec<(u32, f64)> = (0..frame.data.row_count())
                        .map(|row| (frame.data.group_of(row), xs.get(row).unwrap()))
                        .collect();
                    assert_eq!(
                        got.len(),
                        want.len(),
                        "{key} frame {position}: {got:?} vs {want:?}"
                    );
                    for ((group, value), (want_group, want_value)) in got.iter().zip(&want) {
                        assert_eq!(group, want_group, "{key} frame {position}");
                        assert!((value - want_value).abs() < 1e-12, "{key} frame {position}");
                    }
                }
            }
        }
    }

    #[test]
    fn points_travel_and_keep_last_holds_the_end() {
        let (all, by_state, positions) = layer();
        let rows = RevealRows::new(&all, &by_state, &positions);
        assert_eq!(rows.frame(4, true, false), vec![(3, 4, 0.5), (1, 2, 0.0)]);
        assert_eq!(
            rows.frame(6, true, false),
            vec![(4, 4, 0.0), (1, 2, 2.0 / 3.0)]
        );
        assert_eq!(rows.frame(6, false, false), vec![(1, 2, 2.0 / 3.0)]);
        assert!(rows.frame(0, true, false).is_empty());
    }
}
