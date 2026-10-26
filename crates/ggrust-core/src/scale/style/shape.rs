//! Point shapes (lets-plot `ShapeMapper` + `createWithDiscreteOutput`): categories
//! take the palette in order, cycling; continuous data is quantized into equal
//! buckets, one per shape. `scale_shape(solid=False)` uses the hollow palette and
//! `scale_shape_manual(values=...)` its own list.

use super::StyleMapper;
use crate::data::ContinuousColumn;
use crate::scene::PerItem;
use crate::style::{StyleValues, ValueKind};

/// lets-plot `ShapeMapper.allShapes()`: the six best, then every other code.
pub const ALL_SHAPES: [u8; 26] = [
    16, 17, 15, 3, 7, 8, 0, 1, 2, 4, 5, 6, 9, 10, 11, 12, 13, 14, 18, 19, 20, 21, 22, 23, 24, 25,
];
/// lets-plot `ShapeMapper.hollowShapes()`: the three best hollow, then the other
/// hollow codes.
pub const HOLLOW_SHAPES: [u8; 15] = [1, 2, 0, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];
/// lets-plot `ShapeMapper.NA_VALUE` (`TinyPointShape`): not a shape code, drawn as
/// a 1 px dot.
pub const NA_SHAPE: u8 = u8::MAX;

/// Shapes from a palette.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeMapper {
    /// The shape codes, never empty.
    pub shapes: Vec<u8>,
    /// Shape of missing and unknown values.
    pub na: u8,
}

impl ShapeMapper {
    /// The default palette.
    pub fn all() -> Self {
        ShapeMapper {
            shapes: ALL_SHAPES.to_vec(),
            na: NA_SHAPE,
        }
    }
}

impl StyleMapper for ShapeMapper {
    fn name(&self) -> &'static str {
        "shape"
    }

    fn value_kind(&self) -> ValueKind {
        ValueKind::Shape
    }

    fn map_continuous(&self, column: &ContinuousColumn, domain: (f64, f64)) -> StyleValues {
        // lets-plot `continuousToDiscrete` (`QuantizeScale`): equal buckets, clamped.
        let count = self.shapes.len();
        let bucket = (domain.1 - domain.0) / count as f64;
        let shapes = (0..column.len())
            .map(
                |row| match column.get(row).filter(|value| value.is_finite()) {
                    Some(value) => {
                        let index = ((value - domain.0) / bucket) as i64;
                        self.shapes[index.clamp(0, count as i64 - 1) as usize]
                    }
                    None => self.na,
                },
            )
            .collect();
        StyleValues::Shapes(PerItem::Varying(shapes))
    }

    fn map_discrete(&self, indices: &[Option<usize>], _level_count: usize) -> StyleValues {
        let shapes = indices
            .iter()
            .map(|index| index.map_or(self.na, |index| self.shapes[index % self.shapes.len()]))
            .collect();
        StyleValues::Shapes(PerItem::Varying(shapes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_cycle_and_missing_values_are_dots() {
        let mapper = ShapeMapper::all();
        let StyleValues::Shapes(PerItem::Varying(shapes)) =
            mapper.map_discrete(&[Some(0), Some(1), Some(27), None], 28)
        else {
            panic!("expected shapes");
        };
        assert_eq!(shapes, [16, 17, 17, NA_SHAPE]);
    }

    #[test]
    fn palettes_hold_every_code_once() {
        let mut all = ALL_SHAPES.to_vec();
        all.sort_unstable();
        assert_eq!(all, (0..=25).collect::<Vec<u8>>());
        assert!(HOLLOW_SHAPES.iter().all(|code| *code <= 14));
    }
}
