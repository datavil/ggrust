//! `position="stack"`: stack rows with the same x on top of each other, as
//! lets-plot's `StackablePos` / `StackPos`.
//!
//! Groups stack in group order (rows in data order within a group). Positive and
//! negative values stack separately, away from 0. With `mode="groups"` (default)
//! rows of one group at the same x do not stack on each other: the group adds its
//! largest value. `mode="all"` stacks every row. `vjust` (default 1) places a
//! row's y within its stacked span (0 bottom, 1 top); bars move as a whole.
//!
//! What it does not do: `position_fill` (normalised stacks).

use std::collections::HashMap;

use super::Position;
use super::shift::shift_positions;
use crate::data::LayerData;
use crate::parameters::Parameters;
use crate::spec::Aesthetic;
use crate::stat::groups::rows_by_group_id;
use crate::{Error, Result};

/// Stacks overlapping rows.
pub struct StackPosition;

impl Position for StackPosition {
    fn name(&self) -> &'static str {
        "stack"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["vjust", "mode"]
    }

    fn adjust(&self, data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        let vjust = parameters.number("vjust")?.unwrap_or(1.0);
        let all = match parameters.string("mode")? {
            None | Some("groups") => false,
            Some("all") => true,
            Some(other) => {
                return Err(Error::InvalidSpec(format!(
                    "at '{}': unsupported stacking mode '{other}'; use one of: groups, all",
                    parameters.path_of("mode")
                )));
            }
        };
        let (Some(x), Some(y)) = (data.continuous(Aesthetic::X), data.continuous(Aesthetic::Y))
        else {
            return Ok(data);
        };
        let mut context = StackingContext::new(all);
        let mut offsets = vec![0.0; data.row_count()];
        for rows in rows_by_group_id(&data) {
            for row in rows {
                let (Some(location), Some(value)) = (x.get(row), y.get(row)) else {
                    continue;
                };
                if !(location.is_finite() && value.is_finite()) {
                    continue;
                }
                let offset = context.total_offset(location, value);
                offsets[row] = offset - value * (1.0 - vjust);
            }
            context.finish_group();
        }
        shift_positions(data, Aesthetic::is_y_position, |row, value| {
            value + offsets[row]
        })
    }
}

/// lets-plot `StackablePos.StackingContext`: per x, the height reached by earlier
/// groups (`stack`) and by the current group (`value`), above and below 0.
struct StackingContext {
    inside_groups: bool,
    positive: HashMap<u64, (f64, f64)>,
    negative: HashMap<u64, (f64, f64)>,
}

impl StackingContext {
    fn new(inside_groups: bool) -> Self {
        StackingContext {
            inside_groups,
            positive: HashMap::new(),
            negative: HashMap::new(),
        }
    }

    /// Where a row starts; records its height for the rows after it.
    fn total_offset(&mut self, location: f64, value: f64) -> f64 {
        // Kotlin compares boxed doubles by bits: -0.0 and 0.0 are two stacks.
        let key = location.to_bits();
        let inside_groups = self.inside_groups;
        let group_offset = |current: f64, value: f64| {
            if inside_groups {
                current + value
            } else {
                current.max(value)
            }
        };
        let (map, sign) = if value >= 0.0 {
            (&mut self.positive, 1.0)
        } else {
            (&mut self.negative, -1.0)
        };
        let (current_value, stack) = *map.entry(key).or_insert((0.0, 0.0));
        let next = sign * group_offset(sign * current_value, sign * value);
        map.insert(key, (next, stack));
        if inside_groups {
            stack + current_value
        } else {
            stack
        }
    }

    /// Close a group: its heights join the stacks.
    fn finish_group(&mut self) {
        for map in [&mut self.positive, &mut self.negative] {
            for entry in map.values_mut() {
                *entry = (0.0, entry.1 + entry.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Column, ContinuousColumn};
    use crate::spec::Options;

    fn stacked(x: &[f64], y: &[f64], groups: Vec<u32>, options: &str) -> Vec<f64> {
        let mut data = LayerData::new(x.len());
        let column =
            |values: &[f64]| Column::Continuous(ContinuousColumn::from_values(values.to_vec()));
        data.insert(Aesthetic::X, column(x)).unwrap();
        data.insert(Aesthetic::Y, column(y)).unwrap();
        data.insert(Aesthetic::Ymin, column(&vec![0.0; x.len()]))
            .unwrap();
        data.set_group_index(Some(groups)).unwrap();
        let options: Options = serde_json::from_str(options).unwrap();
        let data = StackPosition
            .adjust(data, &Parameters::new(&options, ""))
            .unwrap();
        let y = data.continuous(Aesthetic::Y).unwrap();
        (0..y.len()).map(|row| y.get(row).unwrap()).collect()
    }

    #[test]
    fn groups_stack_in_group_order_by_sign() {
        // Group 1 comes first in the data but stacks after group 0.
        let y = stacked(
            &[0.0, 0.0, 0.0, 0.0],
            &[2.0, 1.0, -1.0, -3.0],
            vec![1, 0, 0, 1],
            "{}",
        );
        assert_eq!(y, [3.0, 1.0, -1.0, -4.0]);
    }

    #[test]
    fn one_group_takes_its_largest_value_unless_mode_all() {
        let y = stacked(&[0.0, 0.0, 0.0], &[2.0, 1.0, 1.0], vec![0, 0, 1], "{}");
        assert_eq!(y, [2.0, 1.0, 3.0]);
        let y = stacked(
            &[0.0, 0.0, 0.0],
            &[2.0, 1.0, 1.0],
            vec![0, 0, 1],
            r#"{"mode": "all"}"#,
        );
        assert_eq!(y, [2.0, 3.0, 4.0]);
    }

    #[test]
    fn vjust_places_the_value_within_its_span() {
        let y = stacked(&[0.0, 0.0], &[2.0, 4.0], vec![0, 1], r#"{"vjust": 0.5}"#);
        assert_eq!(y, [1.0, 4.0]);
    }
}
