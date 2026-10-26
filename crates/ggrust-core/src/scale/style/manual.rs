//! `scale_*_manual(values=...)` for colors (lets-plot `ColorManualMapperProvider`):
//! categories take the values in order, cycling; continuous data is quantized into
//! equal buckets, one per value.

use super::StyleMapper;
use crate::data::ContinuousColumn;
use crate::scene::{Color, PerItem};
use crate::style::{StyleValues, ValueKind};

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
        // lets-plot `QuantizeScale`: equal buckets over the domain, clamped at both ends.
        let count = self.colors.len();
        let bucket = (domain.1 - domain.0) / count as f64;
        let colors = (0..column.len())
            .map(
                |row| match column.get(row).filter(|value| value.is_finite()) {
                    Some(value) => {
                        let index = ((value - domain.0) / bucket) as i64;
                        self.colors[index.clamp(0, count as i64 - 1) as usize]
                    }
                    None => self.na,
                },
            )
            .collect();
        StyleValues::Colors(PerItem::Varying(colors))
    }

    fn map_discrete(&self, indices: &[Option<usize>], _level_count: usize) -> StyleValues {
        let colors = indices
            .iter()
            .map(|index| index.map_or(self.na, |index| self.colors[index % self.colors.len()]))
            .collect();
        StyleValues::Colors(PerItem::Varying(colors))
    }
}
