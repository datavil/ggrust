//! `position="stack"`: stack rows with the same x on top of each other, as
//! lets-plot's `StackablePos` / `StackPos`.
//!
//! Groups stack in group order (rows in data order within a group). Positive and
//! negative values stack separately, away from 0. With `mode="groups"` (default)
//! rows of one group at the same x do not stack on each other: the group adds its
//! largest value. `mode="all"` stacks every row. Identity bars default to
//! `"all"`, as ggplot2 ([`crate::geom::Geom::stacks_each_row`]); lets-plot
//! overlaps them (#1398). `vjust` (default 1) places a row's y
//! within its stacked span (0 bottom, 1 top); bars move as a whole.
//! `reverse=True` (ggplot2) stacks the groups in the opposite order.
//!
//! `position="fill"` stacks the same way, then divides each x's stack by its
//! total height (positive and negative stacks each by their own total), so every
//! stack spans 0..1 (lets-plot `FillPos`).

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
        &["vjust", "mode", "reverse"]
    }

    fn adjust(&self, data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        stack(data, parameters, false)
    }
}

/// Stacks overlapping rows, then scales every stack to span 0..1.
pub struct FillPosition;

impl Position for FillPosition {
    fn name(&self) -> &'static str {
        "fill"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["vjust", "mode", "reverse"]
    }

    fn adjust(&self, data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        stack(data, parameters, true)
    }
}

/// `stack` and `fill`: `normalize` divides each stack by its total.
fn stack(data: LayerData, parameters: &Parameters, normalize: bool) -> Result<LayerData> {
    let vjust = parameters.number("vjust")?.unwrap_or(1.0);
    let all = match parameters.string("mode")? {
        None | Some("groups") => false,
        Some("all") => true,
        Some(other) => {
            return Err(Error::spec(
                parameters.path_of("mode"),
                format!("unsupported stacking mode '{other}'; use one of: groups, all"),
            ));
        }
    };
    let (Some(x), Some(y)) = (data.continuous(Aesthetic::X), data.continuous(Aesthetic::Y)) else {
        return Ok(data);
    };
    let mut groups = rows_by_group_id(&data);
    if parameters.boolean("reverse")?.unwrap_or(false) {
        groups.reverse();
    }
    let mut context = StackingContext::new(all);
    let mut offsets = vec![0.0; data.row_count()];
    // The stack each row joined: x bits and whether it stacks above 0.
    let mut stacks: Vec<Option<(u64, bool)>> = vec![None; data.row_count()];
    for rows in groups {
        for row in rows {
            let (Some(location), Some(value)) = (x.get(row), y.get(row)) else {
                continue;
            };
            if !(location.is_finite() && value.is_finite()) {
                continue;
            }
            let offset = context.total_offset(location, value);
            offsets[row] = offset - value * (1.0 - vjust);
            stacks[row] = Some((location.to_bits(), value >= 0.0));
        }
        context.finish_group();
    }
    let scales: Vec<f64> = stacks
        .iter()
        .map(|stack| match (normalize, stack) {
            (true, Some((key, positive))) => context.total(*key, *positive),
            _ => 1.0,
        })
        .collect();
    shift_positions(data, Aesthetic::is_y_position, |row, value| {
        (value + offsets[row]) / scales[row]
    })
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

    /// The height of a finished stack, as a positive divisor (1 when it is 0).
    fn total(&self, key: u64, positive: bool) -> f64 {
        let map = if positive {
            &self.positive
        } else {
            &self.negative
        };
        match map.get(&key).map(|(_, stack)| stack.abs()) {
            Some(total) if total > 0.0 => total,
            _ => 1.0,
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
        adjusted(&StackPosition, x, y, groups, options)
    }

    fn adjusted(
        position: &dyn Position,
        x: &[f64],
        y: &[f64],
        groups: Vec<u32>,
        options: &str,
    ) -> Vec<f64> {
        let mut data = LayerData::new(x.len());
        let column =
            |values: &[f64]| Column::Continuous(ContinuousColumn::from_values(values.to_vec()));
        data.insert(Aesthetic::X, column(x)).unwrap();
        data.insert(Aesthetic::Y, column(y)).unwrap();
        data.insert(Aesthetic::Ymin, column(&vec![0.0; x.len()]))
            .unwrap();
        data.set_group_index(Some(groups)).unwrap();
        let options: Options = serde_json::from_str(options).unwrap();
        let data = position
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
    fn reverse_stacks_groups_the_other_way() {
        let y = stacked(
            &[0.0, 0.0, 0.0, 0.0],
            &[2.0, 1.0, -1.0, -3.0],
            vec![1, 0, 0, 1],
            r#"{"reverse": true}"#,
        );
        assert_eq!(y, [2.0, 3.0, -4.0, -3.0]);
        let filled = adjusted(
            &FillPosition,
            &[0.0, 0.0],
            &[1.0, 3.0],
            vec![0, 1],
            r#"{"reverse": true}"#,
        );
        assert_eq!(filled, [1.0, 0.75]);
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

    #[test]
    fn fill_scales_each_stack_by_sign_to_one() {
        let y = adjusted(
            &FillPosition,
            &[0.0, 0.0, 0.0, 1.0, 1.0],
            &[1.0, 3.0, -2.0, 2.0, 2.0],
            vec![0, 1, 1, 0, 1],
            "{}",
        );
        assert_eq!(y, [0.25, 1.0, -1.0, 0.5, 1.0]);
    }
}
