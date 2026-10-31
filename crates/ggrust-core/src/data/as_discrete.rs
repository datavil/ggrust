//! `as_discrete(...)`: turn a mapped column discrete and order its levels.
//!
//! What it does: converts a numeric column to levels (first appearance, as
//! lets-plot), then orders the levels by `levels=`, else by `order_by=` (the mean
//! of that column per level), else by `order=` (the level values). Measured
//! against lets-plot: `order_by` defaults to descending, ties fall back to the
//! level values in the same direction, levels without an `order_by` value go
//! last, `levels=` puts the given levels first and `order=-1` reverses them.
//!
//! What it does not do: read the spec or choose the scale title (`label=`).

use std::cmp::Ordering;
use std::collections::HashMap;

use arrow_array::UInt32Array;
use serde_json::Value;

use super::{Column, ContinuousColumn, DiscreteColumn};

/// How the levels of an `as_discrete` column are ordered.
#[derive(Debug, Clone, Copy, Default)]
pub struct DiscreteOrder<'a> {
    /// Levels to put first, in this order (`levels=`).
    pub levels: Option<&'a [Value]>,
    /// Per-row values whose mean orders the levels (`order_by=`).
    pub order_by: Option<&'a ContinuousColumn>,
    /// `1` ascending, `-1` descending (`order=`).
    pub order: Option<i64>,
}

/// The column as a discrete column with its levels ordered by `order`.
pub fn as_discrete(column: &Column, order: DiscreteOrder<'_>) -> Column {
    let (column, keys) = match column {
        Column::Discrete(column) => {
            let keys = column
                .levels()
                .iter()
                .map(|level| SortKey::Text(level.clone()))
                .collect();
            (column.clone(), keys)
        }
        Column::Continuous(column) => from_numbers(column),
    };
    let level_count = column.levels().len();
    let descending = |default: i64| order.order.unwrap_or(default) < 0;
    let mut indices: Vec<usize> = (0..level_count).collect();
    if let Some(levels) = order.levels {
        let position: HashMap<&str, usize> = column
            .levels()
            .iter()
            .enumerate()
            .map(|(index, level)| (level.as_str(), index))
            .collect();
        let mut first: Vec<usize> = Vec::new();
        for level in levels {
            if let Some(&index) = position.get(level_label(level).as_str())
                && !first.contains(&index)
            {
                first.push(index);
            }
        }
        indices.retain(|index| !first.contains(index));
        first.extend(indices);
        indices = first;
        if descending(1) {
            indices.reverse();
        }
    } else if let Some(order_by) = order.order_by {
        let means = level_means(&column, order_by);
        let reverse = descending(-1);
        indices.sort_by(|&left, &right| match (means[left], means[right]) {
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (left_mean, right_mean) => {
                let ordering = left_mean
                    .unwrap_or(0.0)
                    .total_cmp(&right_mean.unwrap_or(0.0))
                    .then_with(|| keys[left].cmp(&keys[right]));
                if reverse {
                    ordering.reverse()
                } else {
                    ordering
                }
            }
        });
    } else if order.order.is_some() {
        indices.sort_by(|&left, &right| keys[left].cmp(&keys[right]));
        if descending(1) {
            indices.reverse();
        }
    }
    Column::Discrete(column.reordered(&indices))
}

/// The label a `levels=` value matches: numbers as [`number_label`], strings as is.
fn level_label(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number
            .as_f64()
            .map(number_label)
            .unwrap_or_else(|| number.to_string()),
        other => other.to_string(),
    }
}

/// A numeric level label: whole numbers without a fraction (`3`), others in
/// their shortest form (`2.5`, `0.00001`).
fn number_label(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        value.to_string()
    }
}

/// What levels sort by under `order=`: numbers by value, text by characters.
#[derive(Debug, Clone, PartialEq)]
enum SortKey {
    Number(f64),
    Text(String),
}

impl SortKey {
    fn cmp(&self, other: &SortKey) -> Ordering {
        match (self, other) {
            (SortKey::Number(left), SortKey::Number(right)) => left.total_cmp(right),
            (SortKey::Text(left), SortKey::Text(right)) => left.cmp(right),
            (SortKey::Number(_), SortKey::Text(_)) => Ordering::Less,
            (SortKey::Text(_), SortKey::Number(_)) => Ordering::Greater,
        }
    }
}

/// Distinct finite values as levels, in order of first appearance.
fn from_numbers(column: &ContinuousColumn) -> (DiscreteColumn, Vec<SortKey>) {
    let mut level_index: HashMap<u64, u32> = HashMap::new();
    let mut levels: Vec<String> = Vec::new();
    let mut keys: Vec<SortKey> = Vec::new();
    let codes: UInt32Array = (0..column.len())
        .map(|row| {
            let value = column.get(row).filter(|value| value.is_finite())?;
            // -0.0 and 0.0 are one level.
            let value = if value == 0.0 { 0.0 } else { value };
            Some(*level_index.entry(value.to_bits()).or_insert_with(|| {
                levels.push(number_label(value));
                keys.push(SortKey::Number(value));
                (levels.len() - 1) as u32
            }))
        })
        .collect();
    (DiscreteColumn::from_parts(levels, codes), keys)
}

/// Mean of the finite `values` per level; `None` for a level without one.
fn level_means(column: &DiscreteColumn, values: &ContinuousColumn) -> Vec<Option<f64>> {
    let mut sums = vec![(0.0, 0_usize); column.levels().len()];
    for row in 0..column.len().min(values.len()) {
        if let (Some(code), Some(value)) = (column.code(row), values.get(row))
            && value.is_finite()
        {
            let (sum, count) = &mut sums[code as usize];
            *sum += value;
            *count += 1;
        }
    }
    sums.into_iter()
        .map(|(sum, count)| (count > 0).then(|| sum / count as f64))
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn numbers(values: &[f64]) -> Column {
        Column::Continuous(ContinuousColumn::from_values(values.to_vec()))
    }

    fn strings(values: &[&str]) -> Column {
        let mut levels: Vec<String> = Vec::new();
        let codes: UInt32Array = values
            .iter()
            .map(|value| {
                let index = match levels.iter().position(|level| level == value) {
                    Some(index) => index,
                    None => {
                        levels.push((*value).to_owned());
                        levels.len() - 1
                    }
                };
                Some(index as u32)
            })
            .collect();
        Column::Discrete(DiscreteColumn::from_parts(levels, codes))
    }

    fn levels(column: &Column) -> Vec<String> {
        match column {
            Column::Discrete(column) => column.levels().to_vec(),
            Column::Continuous(_) => panic!("expected a discrete column"),
        }
    }

    // Expected orders measured with lets_plot 4.x `to_svg()` legends.
    #[test]
    fn numbers_become_levels_in_first_appearance() {
        let column = numbers(&[3.0, 1.0, 2.5, 1.0, 10.0]);
        let plain = as_discrete(&column, DiscreteOrder::default());
        assert_eq!(levels(&plain), ["3", "1", "2.5", "10"]);
        let Column::Discrete(discrete) = &plain else {
            unreachable!()
        };
        assert_eq!(discrete.label(3), Some("1"));
    }

    #[test]
    fn order_sorts_numbers_by_value() {
        let column = numbers(&[3.0, 1.0, 2.5, 1.0, 10.0]);
        let ascending = DiscreteOrder {
            order: Some(1),
            ..Default::default()
        };
        assert_eq!(
            levels(&as_discrete(&column, ascending)),
            ["1", "2.5", "3", "10"]
        );
        let descending = DiscreteOrder {
            order: Some(-1),
            ..Default::default()
        };
        assert_eq!(
            levels(&as_discrete(&column, descending)),
            ["10", "3", "2.5", "1"]
        );
    }

    #[test]
    fn order_by_uses_the_mean_descending_by_default() {
        let column = strings(&["a", "a", "b"]);
        let weights = ContinuousColumn::from_values(vec![0.0, 10.0, 6.0]);
        let order = DiscreteOrder {
            order_by: Some(&weights),
            ..Default::default()
        };
        assert_eq!(levels(&as_discrete(&column, order)), ["b", "a"]);
    }

    #[test]
    fn order_by_ties_follow_the_levels_and_missing_means_go_last() {
        let column = strings(&["c", "a", "b"]);
        let tied = ContinuousColumn::from_values(vec![1.0, 1.0, 1.0]);
        let descending = DiscreteOrder {
            order_by: Some(&tied),
            ..Default::default()
        };
        assert_eq!(levels(&as_discrete(&column, descending)), ["c", "b", "a"]);
        let ascending = DiscreteOrder {
            order: Some(1),
            ..descending
        };
        assert_eq!(levels(&as_discrete(&column, ascending)), ["a", "b", "c"]);
        let missing = ContinuousColumn::from_options(vec![Some(1.0), None, Some(3.0)]);
        for order in [Some(1), Some(-1)] {
            let order = DiscreteOrder {
                order_by: Some(&missing),
                order,
                ..Default::default()
            };
            assert_eq!(levels(&as_discrete(&column, order)).last().unwrap(), "a");
        }
    }

    #[test]
    fn given_levels_come_first_and_order_reverses_them() {
        let column = strings(&["b", "a", "c", "a", "d"]);
        let given = [json!("c"), json!("a"), json!("x")];
        let order = DiscreteOrder {
            levels: Some(&given),
            ..Default::default()
        };
        assert_eq!(levels(&as_discrete(&column, order)), ["c", "a", "b", "d"]);
        let column = numbers(&[3.0, 1.0, 2.5, 1.0, 10.0]);
        let given = [json!(10), json!(1), json!(3), json!(2.5)];
        let order = DiscreteOrder {
            levels: Some(&given),
            order: Some(-1),
            ..Default::default()
        };
        assert_eq!(
            levels(&as_discrete(&column, order)),
            ["2.5", "3", "1", "10"]
        );
    }
}
