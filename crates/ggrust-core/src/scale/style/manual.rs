//! `scale_*_manual(values=...)` (lets-plot `createWithDiscreteOutput`): categories
//! take the values in order, cycling; continuous data is quantized into equal
//! buckets, one per value (`QuantizeScale`). Colors and numbers here; shapes use
//! the same helpers ([`super::ShapeMapper`]).

use super::StyleMapper;
use crate::data::ContinuousColumn;
use crate::scene::{Color, PerItem};
use crate::style::{StyleValues, ValueKind};

/// lets-plot `QuantizeScale`: equal buckets over the domain, clamped at both ends;
/// missing values get `na`. `values` is never empty.
pub(super) fn quantize<T: Copy>(
    values: &[T],
    na: T,
    column: &ContinuousColumn,
    domain: (f64, f64),
) -> Vec<T> {
    let count = values.len();
    let bucket = (domain.1 - domain.0) / count as f64;
    (0..column.len())
        .map(
            |row| match column.get(row).filter(|value| value.is_finite()) {
                Some(value) => {
                    let index = ((value - domain.0) / bucket) as i64;
                    values[index.clamp(0, count as i64 - 1) as usize]
                }
                None => na,
            },
        )
        .collect()
}

/// Category `index` takes value `index % len`; missing categories get `na`.
pub(super) fn cycle<T: Copy>(values: &[T], na: T, indices: &[Option<usize>]) -> Vec<T> {
    indices
        .iter()
        .map(|index| index.map_or(na, |index| values[index % values.len()]))
        .collect()
}

/// Manual colors.
#[derive(Debug, Clone, PartialEq)]
pub struct ManualColorMapper {
    /// The colors, never empty.
    pub colors: Vec<Color>,
    /// Color of missing and unknown values.
    pub na: Color,
}

impl StyleMapper for ManualColorMapper {
    fn name(&self) -> &'static str {
        "color_manual"
    }

    fn value_kind(&self) -> ValueKind {
        ValueKind::Color
    }

    fn map_continuous(&self, column: &ContinuousColumn, domain: (f64, f64)) -> StyleValues {
        StyleValues::Colors(PerItem::Varying(quantize(
            &self.colors,
            self.na,
            column,
            domain,
        )))
    }

    fn map_discrete(&self, indices: &[Option<usize>], _level_count: usize) -> StyleValues {
        StyleValues::Colors(PerItem::Varying(cycle(&self.colors, self.na, indices)))
    }
}

/// Manual numbers (`scale_size_manual`, `scale_alpha_manual`).
#[derive(Debug, Clone, PartialEq)]
pub struct ManualNumberMapper {
    /// The numbers, never empty.
    pub numbers: Vec<f64>,
    /// Value of missing and unknown values.
    pub na: f64,
}

impl StyleMapper for ManualNumberMapper {
    fn name(&self) -> &'static str {
        "manual"
    }

    fn value_kind(&self) -> ValueKind {
        ValueKind::Number
    }

    fn map_continuous(&self, column: &ContinuousColumn, domain: (f64, f64)) -> StyleValues {
        StyleValues::Numbers(PerItem::Varying(quantize(
            &self.numbers,
            self.na,
            column,
            domain,
        )))
    }

    fn map_discrete(&self, indices: &[Option<usize>], _level_count: usize) -> StyleValues {
        StyleValues::Numbers(PerItem::Varying(cycle(&self.numbers, self.na, indices)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantize_clamps_and_cycle_wraps() {
        let column = ContinuousColumn::from_options(vec![Some(0.0), Some(5.0), Some(10.0), None]);
        assert_eq!(quantize(&[1, 2], 0, &column, (0.0, 10.0)), [1, 2, 2, 0]);
        assert_eq!(cycle(&[1, 2], 0, &[Some(0), Some(3), None]), [1, 2, 0]);
    }
}
