//! Numeric style mappings: `size` and `alpha` (lets-plot `SizeMapperProvider`,
//! `AlphaMapperProvider`, both `LinearNormalizingMapperProvider`): data mapped
//! linearly onto an output range (values outside the domain extrapolate, as in
//! lets-plot); categories spread evenly over the range. `scale_size_area` maps
//! 0..max of the data onto 0..`max_size` with area proportional to the value
//! (ggplot2 `abs_area`, not lets-plot's linear diameter).

use super::{StyleMapper, linear};
use crate::data::ContinuousColumn;
use crate::scene::PerItem;
use crate::style::{StyleValues, ValueKind};

/// lets-plot `POINT_UNIT_SIZE` (`AesScaling.sizeFromCircleDiameter` divides by it).
const POINT_UNIT_SIZE: f64 = 2.2;

/// Linear mapping onto `range`; missing values get `missing`.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearMapper {
    name: &'static str,
    range: (f64, f64),
    /// Value of missing data (`na_value`).
    pub missing: f64,
}

impl LinearMapper {
    /// `scale_size(range=...)`, `scale_alpha(range=...)`: the same mapping onto
    /// another range.
    pub fn with_range(&self, range: (f64, f64), missing: f64) -> LinearMapper {
        LinearMapper {
            name: self.name,
            range,
            missing,
        }
    }

    /// The value of missing data.
    pub fn missing(&self) -> f64 {
        self.missing
    }
}

/// `scale_size_area`: values from 0 to the largest one onto sizes 0..`max`,
/// the circle area proportional to the value
/// (continuous data only).
#[derive(Debug, Clone, PartialEq)]
pub struct ProportionalMapper {
    /// Size of the largest value.
    pub max: f64,
    /// Value of missing data.
    pub missing: f64,
}

/// lets-plot `SizeAreaMapperProvider.DEF_MAX`: a 21 px circle.
pub const SIZE_AREA_MAX: f64 = 21.0 / POINT_UNIT_SIZE;

impl StyleMapper for ProportionalMapper {
    fn name(&self) -> &'static str {
        "size_area"
    }

    fn value_kind(&self) -> ValueKind {
        ValueKind::Number
    }

    fn map_continuous(&self, column: &ContinuousColumn, domain: (f64, f64)) -> StyleValues {
        // Area, not diameter, grows with the value (ggplot2 `rescale_max` +
        // `abs_area`; lets-plot scales the diameter linearly, issue lets-plot#1452).
        // As in ggplot2, a negative value draws as large as its absolute value.
        let largest = domain.0.abs().max(domain.1.abs());
        let map = |value: f64| {
            if largest > 0.0 {
                self.max * (value.abs() / largest).sqrt()
            } else {
                0.0
            }
        };
        let values = (0..column.len())
            .map(|row| {
                column
                    .get(row)
                    .filter(|value| value.is_finite())
                    .map_or(self.missing, map)
            })
            .collect();
        StyleValues::Numbers(PerItem::Varying(values))
    }

    fn map_discrete(&self, indices: &[Option<usize>], _level_count: usize) -> StyleValues {
        StyleValues::Numbers(PerItem::Varying(vec![self.missing; indices.len()]))
    }

    fn continuous_only(&self) -> bool {
        true
    }
}

/// `size`: circle diameters 3..21 px; missing values 1 px.
pub static SIZE: LinearMapper = LinearMapper {
    name: "size",
    range: (3.0 / POINT_UNIT_SIZE, 21.0 / POINT_UNIT_SIZE),
    missing: 1.0 / POINT_UNIT_SIZE,
};

/// `linewidth` (ggplot2 `scale_linewidth`, lets-plot has none): 1..6 line
/// units; missing values are not drawn (width 0), as ggplot2's `NA`.
pub static LINEWIDTH: LinearMapper = LinearMapper {
    name: "linewidth",
    range: (1.0, 6.0),
    missing: 0.0,
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
    fn size_area_scales_the_area_not_the_diameter() {
        let mapper = ProportionalMapper {
            max: 10.0,
            missing: 1.0,
        };
        let column = ContinuousColumn::from_options(vec![Some(20.0), Some(40.0), Some(-5.0)]);
        let StyleValues::Numbers(PerItem::Varying(sizes)) =
            mapper.map_continuous(&column, (-5.0, 40.0))
        else {
            panic!("expected varying sizes");
        };
        // Twice the value, twice the area.
        assert!(((sizes[1] / sizes[0]).powi(2) - 2.0).abs() < 1e-12);
        assert_eq!(sizes[1], 10.0);
        // ggplot2 `abs_area`: -5 draws as 5.
        assert!((sizes[2] - 10.0 * (5.0_f64 / 40.0).sqrt()).abs() < 1e-12);
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
