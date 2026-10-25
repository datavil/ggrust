//! Continuous position scale: trained on data, expanded like lets-plot.

use crate::data::ContinuousColumn;

/// Linear continuous position scale.
///
/// Defaults follow lets-plot: 5% multiplicative expansion on each side, no additive
/// expansion, and an empty or zero-width domain widened to +/- 0.5.
#[derive(Debug, Clone, PartialEq)]
pub struct ContinuousPositionScale {
    domain: Option<(f64, f64)>,
    /// Fraction of the domain width added on each side.
    pub multiplicative_expand: f64,
    /// Data units added on each side.
    pub additive_expand: f64,
    /// User limits; `None` on a side keeps the trained value.
    pub limits: [Option<f64>; 2],
}

impl Default for ContinuousPositionScale {
    fn default() -> Self {
        ContinuousPositionScale {
            domain: None,
            multiplicative_expand: 0.05,
            additive_expand: 0.0,
            limits: [None, None],
        }
    }
}

impl ContinuousPositionScale {
    /// Widen the domain to include every finite value of `column`.
    pub fn train(&mut self, column: &ContinuousColumn) {
        let mut low = f64::INFINITY;
        let mut high = f64::NEG_INFINITY;
        for (row, &value) in column.values().iter().enumerate() {
            if value.is_finite() && column.is_valid(row) {
                low = low.min(value);
                high = high.max(value);
            }
        }
        if low <= high {
            self.train_range(low, high);
        }
    }

    /// Widen the domain to include `[low, high]`.
    pub fn train_range(&mut self, low: f64, high: f64) {
        self.domain = Some(match self.domain {
            Some((current_low, current_high)) => (current_low.min(low), current_high.max(high)),
            None => (low, high),
        });
    }

    /// Domain after limits, before expansion. Never empty, zero-width or infinite.
    ///
    /// Non-finite limits are ignored. A one-sided limit without data gives a
    /// one-unit range at the limit. A degenerate range is widened by +/- 0.5, or
    /// relatively for huge values (where +/- 0.5 would be lost to rounding).
    pub fn limited_domain(&self) -> (f64, f64) {
        let finite = |limit: Option<f64>| limit.filter(|value| value.is_finite());
        let (low_limit, high_limit) = (finite(self.limits[0]), finite(self.limits[1]));
        let (low, high) = match (self.domain, low_limit, high_limit) {
            (_, Some(low), Some(high)) => (low, high),
            (Some((low, high)), low_limit, high_limit) => {
                (low_limit.unwrap_or(low), high_limit.unwrap_or(high))
            }
            (None, Some(value), None) | (None, None, Some(value)) => (value, value),
            (None, None, None) => return (-0.5, 0.5),
        };
        let (low, high) = if low <= high {
            (low, high)
        } else {
            (high, low)
        };
        // A span that overflows f64 (e.g. -1e308..1e308) is fine: expansion and
        // mapping work on halves.
        if super::breaks::is_degenerate_range(low, high) {
            let half_width = 0.5_f64.max(low.abs() * 1e-9);
            return (low - half_width, low + half_width);
        }
        (low, high)
    }

    /// Domain after limits and expansion: the panel spans exactly this range.
    pub fn expanded_domain(&self) -> (f64, f64) {
        let (low, high) = self.limited_domain();
        // Halves keep the arithmetic finite near f64::MAX.
        let padding =
            (high / 2.0 - low / 2.0) * 2.0 * self.multiplicative_expand + self.additive_expand;
        (
            (low - padding).max(-f64::MAX),
            (high + padding).min(f64::MAX),
        )
    }

    /// Map a data value to 0..1 across the panel (outside for out-of-range values).
    /// Non-finite input gives NaN.
    pub fn map(&self, value: f64) -> f64 {
        if !value.is_finite() {
            return f64::NAN;
        }
        let (low, high) = self.expanded_domain();
        (value / 2.0 - low / 2.0) / (high / 2.0 - low / 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trains_and_expands_like_lets_plot() {
        let mut scale = ContinuousPositionScale::default();
        scale.train(&ContinuousColumn::from_options(vec![
            Some(1.0),
            None,
            Some(f64::NAN),
            Some(4.5),
        ]));
        assert_eq!(scale.limited_domain(), (1.0, 4.5));
        let (low, high) = scale.expanded_domain();
        assert!((low - 0.825).abs() < 1e-12 && (high - 4.675).abs() < 1e-12);
        assert!((scale.map(1.0) - 0.05 / 1.1).abs() < 1e-12);
        assert!(scale.map(f64::NAN).is_nan());
    }

    #[test]
    fn empty_and_single_value_domains_widen() {
        let empty = ContinuousPositionScale::default();
        assert_eq!(empty.limited_domain(), (-0.5, 0.5));
        let mut single = ContinuousPositionScale::default();
        single.train_range(3.0, 3.0);
        assert_eq!(single.limited_domain(), (2.5, 3.5));
    }

    #[test]
    fn tiny_ranges_are_real_ranges() {
        let mut scale = ContinuousPositionScale::default();
        scale.train_range(1e-20, 3e-20);
        assert_eq!(scale.limited_domain(), (1e-20, 3e-20));
        assert!(scale.map(1e-20) < 0.1 && scale.map(3e-20) > 0.9);
    }

    #[test]
    fn extreme_magnitudes_stay_finite() {
        for (low, high) in [(1e17, 1e17), (1e300, 1e300), (-1e308, 1e308)] {
            let mut scale = ContinuousPositionScale::default();
            scale.train_range(low, high);
            let (expanded_low, expanded_high) = scale.expanded_domain();
            assert!(expanded_low.is_finite() && expanded_high.is_finite());
            let mapped = scale.map(low);
            assert!(
                mapped.is_finite() && (0.0..=1.0).contains(&mapped),
                "{low}: {mapped}"
            );
        }
    }

    #[test]
    fn odd_limits() {
        let one_sided = ContinuousPositionScale {
            limits: [Some(5.0), None],
            ..ContinuousPositionScale::default()
        };
        assert_eq!(one_sided.limited_domain(), (4.5, 5.5));
        let mut not_a_number = ContinuousPositionScale {
            limits: [Some(f64::NAN), None],
            ..ContinuousPositionScale::default()
        };
        not_a_number.train_range(2.0, 10.0);
        assert_eq!(not_a_number.limited_domain(), (2.0, 10.0));
    }

    #[test]
    fn limits_override_training() {
        let mut scale = ContinuousPositionScale {
            limits: [Some(0.0), None],
            ..ContinuousPositionScale::default()
        };
        scale.train_range(2.0, 10.0);
        assert_eq!(scale.limited_domain(), (0.0, 10.0));
    }
}
