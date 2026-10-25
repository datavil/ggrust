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

    /// Domain after limits, before expansion. Never empty or zero-width.
    pub fn limited_domain(&self) -> (f64, f64) {
        let (trained_low, trained_high) = self.domain.unwrap_or((0.0, 0.0));
        let low = self.limits[0].unwrap_or(trained_low);
        let high = self.limits[1].unwrap_or(trained_high);
        let (low, high) = if low <= high {
            (low, high)
        } else {
            (high, low)
        };
        if self.domain.is_none() && self.limits.iter().all(Option::is_none) {
            return (-0.5, 0.5);
        }
        if high - low <= f64::EPSILON * low.abs().max(high.abs()).max(1.0) {
            return (low - 0.5, low + 0.5);
        }
        (low, high)
    }

    /// Domain after limits and expansion: the panel spans exactly this range.
    pub fn expanded_domain(&self) -> (f64, f64) {
        let (low, high) = self.limited_domain();
        let padding = (high - low) * self.multiplicative_expand + self.additive_expand;
        (low - padding, high + padding)
    }

    /// Map a data value to 0..1 across the panel (outside for out-of-range values).
    /// Non-finite input gives NaN.
    pub fn map(&self, value: f64) -> f64 {
        if !value.is_finite() {
            return f64::NAN;
        }
        let (low, high) = self.expanded_domain();
        (value - low) / (high - low)
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
    fn limits_override_training() {
        let mut scale = ContinuousPositionScale {
            limits: [Some(0.0), None],
            ..ContinuousPositionScale::default()
        };
        scale.train_range(2.0, 10.0);
        assert_eq!(scale.limited_domain(), (0.0, 10.0));
    }
}
