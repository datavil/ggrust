//! Discrete position scale: categories placed at 0, 1, 2, ... (lets-plot
//! `DiscreteScale` + `DiscreteTransform`).
//!
//! Levels are joined across layers in order of first appearance; `limits` replace
//! (and order) them, `reverse` flips them. Numbers on a discrete axis pass through
//! unchanged, as in lets-plot (e.g. a constant x). Mapping a position is O(1): it
//! runs once per data point.

use std::collections::{HashMap, HashSet};

use super::ScaleBreaks;
use super::format::Label;
use super::label_pattern::{LabelPattern, PatternValue};
use crate::data::{ContinuousColumn, DiscreteColumn};

/// Discrete position scale.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscretePositionScale {
    levels: Vec<String>,
    known: HashSet<String>,
    /// Numbers seen on this axis (they pass through unchanged).
    numeric_range: Option<(f64, f64)>,
    /// `scale_*_discrete(limits=[...])` without repeats: the categories shown.
    limits: Option<Vec<String>>,
    /// Reverse the category order.
    pub reverse: bool,
    /// Fraction of the domain width added on each side (lets-plot: 0).
    pub multiplicative_expand: f64,
    /// Units (categories) added on each side (lets-plot: 0.2).
    pub additive_expand: f64,
    /// `breaks=[...]`: only these categories get a tick and label.
    pub break_levels: Option<Vec<String>>,
    /// `labels=[...]`, aligned with `breaks` (or the categories); missing ones are
    /// empty.
    pub labels: Option<Vec<String>>,
    /// `format=...` for the category names.
    pub format: Option<LabelPattern>,
}

impl Default for DiscretePositionScale {
    fn default() -> Self {
        DiscretePositionScale {
            levels: Vec::new(),
            known: HashSet::new(),
            numeric_range: None,
            limits: None,
            reverse: false,
            multiplicative_expand: 0.0,
            additive_expand: 0.2,
            break_levels: None,
            labels: None,
            format: None,
        }
    }
}

impl DiscretePositionScale {
    /// Show only these categories, in this order (repeats ignored). An empty list
    /// means no limits, as in lets-plot.
    pub fn set_limits(&mut self, limits: Vec<String>) {
        let mut seen = HashSet::new();
        let unique: Vec<String> = limits
            .into_iter()
            .filter(|limit| seen.insert(limit.clone()))
            .collect();
        self.limits = (!unique.is_empty()).then_some(unique);
    }

    /// Add the levels of a column that are not known yet, keeping their order.
    pub fn train_levels(&mut self, levels: &[String]) {
        for level in levels {
            if self.known.insert(level.clone()) {
                self.levels.push(level.clone());
            }
        }
    }

    /// Widen the numeric range to include every finite value of `column`.
    pub fn train_numbers(&mut self, column: &ContinuousColumn) {
        for row in 0..column.len() {
            if let Some(value) = column.get(row).filter(|value| value.is_finite()) {
                self.numeric_range = Some(match self.numeric_range {
                    Some((low, high)) => (low.min(value), high.max(value)),
                    None => (value, value),
                });
            }
        }
    }

    /// Whether any categories are shown (trained or from limits).
    pub fn has_levels(&self) -> bool {
        self.level_count() > 0
    }

    /// Number of categories shown.
    fn level_count(&self) -> usize {
        self.limits.as_ref().map_or(self.levels.len(), Vec::len)
    }

    /// Categories shown on the axis, in axis order (position 0 first).
    pub fn effective_levels(&self) -> Vec<&str> {
        let mut levels: Vec<&str> = self
            .limits
            .as_ref()
            .unwrap_or(&self.levels)
            .iter()
            .map(String::as_str)
            .collect();
        if self.reverse {
            levels.reverse();
        }
        levels
    }

    /// Positions of a discrete column: each category's index on the axis, or
    /// missing when the category is not shown (outside `limits`).
    pub fn positions(&self, column: &DiscreteColumn) -> ContinuousColumn {
        let index_by_level: HashMap<&str, usize> = self
            .effective_levels()
            .into_iter()
            .enumerate()
            .map(|(index, level)| (level, index))
            .collect();
        let index_by_code: Vec<Option<f64>> = column
            .levels()
            .iter()
            .map(|level| {
                index_by_level
                    .get(level.as_str())
                    .map(|index| *index as f64)
            })
            .collect();
        ContinuousColumn::from_options(
            (0..column.len())
                .map(|row| {
                    column
                        .code(row)
                        .and_then(|code| index_by_code[code as usize])
                })
                .collect(),
        )
    }

    /// Domain after expansion (in position units), like lets-plot: categories span
    /// 0..n-1 joined with any numbers, plus the expansion; zero-width widens to +/- 0.5.
    pub fn expanded_domain(&self) -> (f64, f64) {
        let count = self.level_count();
        let categories = (count > 0).then(|| (0.0, (count - 1) as f64));
        let range = match (categories, self.numeric_range) {
            (Some((low, high)), Some((number_low, number_high))) => {
                (low.min(number_low), high.max(number_high))
            }
            (Some(range), None) | (None, Some(range)) => range,
            (None, None) => return (-0.5, 0.5),
        };
        let padding = self.additive_expand + (range.1 - range.0) * self.multiplicative_expand;
        let (low, high) = (range.0 - padding, range.1 + padding);
        if high - low < 1e-50 {
            return (low - 0.5, low + 0.5);
        }
        (low, high)
    }

    /// Map a position to 0..1 across the panel. Non-finite input gives NaN.
    pub fn map(&self, value: f64) -> f64 {
        if !value.is_finite() {
            return f64::NAN;
        }
        let (low, high) = self.expanded_domain();
        (value - low) / (high - low)
    }

    /// One break per shown category (lets-plot `DiscreteScale.createScaleBreaks`),
    /// only those in `breaks` when given. Labels come from `labels`, aligned with
    /// `breaks` or else with the categories in axis order (missing ones are empty),
    /// otherwise from `format` or the category name.
    pub fn breaks(&self) -> ScaleBreaks {
        let levels = self.effective_levels();
        let mut breaks = ScaleBreaks {
            positions: Vec::new(),
            labels: Vec::new(),
        };
        for (position, &level) in levels.iter().enumerate() {
            let label_index = match &self.break_levels {
                Some(break_levels) => match break_levels.iter().position(|known| known == level) {
                    Some(index) => index,
                    None => continue,
                },
                None => position,
            };
            let label = match (&self.labels, &self.format) {
                (Some(labels), _) => {
                    Label::plain(labels.get(label_index).cloned().unwrap_or_default())
                }
                (None, Some(format)) => format.format(PatternValue::Text(level)),
                (None, None) => Label::plain(level),
            };
            breaks.positions.push(position as f64);
            breaks.labels.push(label);
        }
        breaks
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow_array::{ArrayRef, RecordBatch, StringArray};

    use super::*;
    use crate::data::{Column, DataFrame};

    fn discrete(values: Vec<Option<&str>>) -> DiscreteColumn {
        let array: ArrayRef = Arc::new(StringArray::from(values));
        let batch = RecordBatch::try_from_iter([("value", array)]).unwrap();
        match DataFrame::new(batch.schema(), vec![batch])
            .unwrap()
            .column("value")
            .unwrap()
        {
            Column::Discrete(column) => column,
            Column::Continuous(_) => panic!("expected a discrete column"),
        }
    }

    fn texts(breaks: &ScaleBreaks) -> Vec<String> {
        breaks.labels.iter().map(Label::to_string).collect()
    }

    #[test]
    fn levels_join_in_first_appearance_and_expand_by_a_fifth() {
        let mut scale = DiscretePositionScale::default();
        let first = discrete(vec![Some("b"), Some("a"), Some("b")]);
        let second = discrete(vec![Some("c"), Some("a")]);
        scale.train_levels(first.levels());
        scale.train_levels(second.levels());
        assert_eq!(scale.effective_levels(), ["b", "a", "c"]);
        assert_eq!(scale.expanded_domain(), (-0.2, 2.2));
        let positions = scale.positions(&second);
        assert_eq!((positions.get(0), positions.get(1)), (Some(2.0), Some(1.0)));
        assert_eq!(texts(&scale.breaks()), ["b", "a", "c"]);
        assert_eq!(scale.breaks().positions, [0.0, 1.0, 2.0]);
    }

    #[test]
    fn limits_order_and_drop_categories_and_reverse_flips() {
        let column = discrete(vec![Some("b"), Some("a"), None, Some("c")]);
        let mut scale = DiscretePositionScale::default();
        scale.set_limits(vec!["c".into(), "b".into(), "c".into()]);
        scale.train_levels(column.levels());
        assert_eq!(scale.effective_levels(), ["c", "b"]);
        let positions = scale.positions(&column);
        assert_eq!(
            (0..4).map(|row| positions.get(row)).collect::<Vec<_>>(),
            [Some(1.0), None, None, Some(0.0)]
        );
        scale.reverse = true;
        assert_eq!(scale.effective_levels(), ["b", "c"]);
        // An empty list is no limits (lets-plot).
        scale.set_limits(Vec::new());
        assert_eq!(scale.effective_levels(), ["c", "a", "b"]);
    }

    #[test]
    fn numbers_pass_through_and_widen_the_domain() {
        let mut scale = DiscretePositionScale::default();
        scale.train_levels(&["a".to_owned()]);
        assert_eq!(scale.expanded_domain(), (-0.2, 0.2));
        scale.train_numbers(&ContinuousColumn::from_values(vec![3.0]));
        assert_eq!(scale.expanded_domain(), (-0.2, 3.2));
        assert_eq!(
            DiscretePositionScale::default().expanded_domain(),
            (-0.5, 0.5)
        );
    }
}
