//! Numeric style mappings: `size` and `alpha` (lets-plot `SizeMapperProvider`,
//! `AlphaMapperProvider`, both `LinearNormalizingMapperProvider`): data mapped
//! linearly onto an output range (values outside the domain extrapolate, as in
//! lets-plot); categories spread evenly over the range.

use super::{StyleMapper, linear};
use crate::data::ContinuousColumn;
use crate::scene::PerItem;
use crate::style::{StyleValues, ValueKind};

/// lets-plot `POINT_UNIT_SIZE` (`AesScaling.sizeFromCircleDiameter` divides by it).
const POINT_UNIT_SIZE: f64 = 2.2;

/// Linear mapping onto `range`; missing values get `missing`.
pub struct LinearMapper {
    name: &'static str,
    range: (f64, f64),
    missing: f64,
}

/// `size`: circle diameters 3..21 px; missing values 1 px.
pub static SIZE: LinearMapper = LinearMapper {
    name: "size",
    range: (3.0 / POINT_UNIT_SIZE, 21.0 / POINT_UNIT_SIZE),
    missing: 1.0 / POINT_UNIT_SIZE,
};

/// `alpha`: 0.1..1; missing values fully transparent (lets-plot `DefaultNaValue`).
pub static ALPHA: LinearMapper = LinearMapper {
    name: "alpha",
    range: (0.1, 1.0),
    missing: 0.0,
};

impl StyleMapper for LinearMapper {
    fn name(&self) -> &'static str {
        self.name
    }

    fn value_kind(&self) -> ValueKind {
        ValueKind::Number
    }

    fn map_continuous(&self, column: &ContinuousColumn, domain: (f64, f64)) -> StyleValues {
        let map = linear(domain, self.range);
        let values = (0..column.len())
            .map(|row| {
                column
                    .get(row)
                    .filter(|value| value.is_finite())
                    .map_or(self.missing, &map)
            })
            .collect();
        StyleValues::Numbers(PerItem::Varying(values))
    }

    fn map_discrete(&self, indices: &[Option<usize>], level_count: usize) -> StyleValues {
        // lets-plot `discreteToContinuous`: indices 0..n-1 onto the range (one
        // category: the middle of the range).
        let map = linear((0.0, level_count.saturating_sub(1) as f64), self.range);
        let values = indices
            .iter()
            .map(|index| index.map_or(self.missing, |index| map(index as f64)))
            .collect();
        StyleValues::Numbers(PerItem::Varying(values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_span_three_to_twenty_one_pixels() {
        let column = ContinuousColumn::from_options(vec![Some(10.0), Some(20.0), None]);
        let StyleValues::Numbers(PerItem::Varying(sizes)) =
            SIZE.map_continuous(&column, (10.0, 20.0))
        else {
            panic!("expected varying sizes");
        };
        let diameters: Vec<f64> = sizes.iter().map(|size| size * POINT_UNIT_SIZE).collect();
        assert!((diameters[0] - 3.0).abs() < 1e-12 && (diameters[1] - 21.0).abs() < 1e-12);
        assert!((diameters[2] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn one_category_gets_the_middle() {
        let StyleValues::Numbers(PerItem::Varying(alphas)) =
            ALPHA.map_discrete(&[Some(0), None], 1)
        else {
            panic!("expected varying alphas");
        };
        assert_eq!(alphas, [0.55, 0.0]);
    }
}
