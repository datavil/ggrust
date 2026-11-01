//! Line types (lets-plot `LineTypeMapper` + `createWithDiscreteOutput`):
//! categories take the palette in order, cycling; continuous data is quantized
//! into equal buckets, one per line type. `scale_linetype_manual(values=...)`
//! uses its own list.

use super::StyleMapper;
use super::manual::{cycle, quantize};
use crate::data::ContinuousColumn;
use crate::scene::PerItem;
use crate::style::{LineType, StyleValues, ValueKind};

/// lets-plot `LineTypeMapper.allLineTypes()`: solid, dashed, dotted, dotdash,
/// longdash, twodash (codes 1..6).
pub const ALL_LINETYPES: [i64; 6] = [1, 2, 3, 4, 5, 6];

/// Line types from a palette.
#[derive(Debug, Clone, PartialEq)]
pub struct LinetypeMapper {
    /// The line types, never empty.
    pub linetypes: Vec<LineType>,
    /// Line type of missing and unknown values (lets-plot: solid).
    pub na: LineType,
}

impl LinetypeMapper {
    /// The default palette.
    pub fn all() -> Self {
        let named = |code: i64| {
            LineType::parse(&crate::spec::Value::from(code)).unwrap_or(LineType(Vec::new()))
        };
        LinetypeMapper {
            linetypes: ALL_LINETYPES.iter().map(|code| named(*code)).collect(),
            na: LineType(Vec::new()),
        }
    }

    fn pick(&self, slots: Vec<usize>) -> StyleValues {
        StyleValues::Linetypes(PerItem::Varying(
            slots
                .into_iter()
                .map(|slot| self.linetypes.get(slot).unwrap_or(&self.na).clone())
                .collect(),
        ))
    }

    fn slots(&self) -> Vec<usize> {
        (0..self.linetypes.len()).collect()
    }
}

impl StyleMapper for LinetypeMapper {
    fn name(&self) -> &'static str {
        "linetype"
    }

    fn value_kind(&self) -> ValueKind {
        ValueKind::Linetype
    }

    fn map_continuous(&self, column: &ContinuousColumn, domain: (f64, f64)) -> StyleValues {
        self.pick(quantize(&self.slots(), usize::MAX, column, domain))
    }

    fn map_discrete(&self, indices: &[Option<usize>], _level_count: usize) -> StyleValues {
        self.pick(cycle(&self.slots(), usize::MAX, indices))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_cycle_and_missing_values_are_solid() {
        let mapper = LinetypeMapper::all();
        let StyleValues::Linetypes(PerItem::Varying(linetypes)) =
            mapper.map_discrete(&[Some(0), Some(1), Some(7), None], 8)
        else {
            panic!("expected line types");
        };
        assert_eq!(
            linetypes,
            [
                LineType(vec![]),
                LineType(vec![4.0, 4.0]),
                LineType(vec![4.0, 4.0]),
                LineType(vec![])
            ]
        );
    }
}
