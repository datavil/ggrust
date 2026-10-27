//! Continuous position scale: trained on data, expanded like lets-plot.

use super::Transform;
use super::breaks::linear_breaks;
use super::format::{Label, format_breaks, format_breaks_individually};
use super::label_pattern::{LabelPattern, PatternValue};
use crate::data::ContinuousColumn;

/// Breaks of a scale: positions in transformed units with their labels.
#[derive(Debug, Clone, PartialEq)]
pub struct ScaleBreaks {
    /// Break positions in transformed units (what [`ContinuousPositionScale::map`] takes).
    pub positions: Vec<f64>,
    /// Labels, from the break values in data units.
    pub labels: Vec<Label>,
}

/// Continuous position scale. Domain, limits and expansion are in transformed
/// units: data is transformed when it is mapped (see [`Transform`]).
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
    /// User limits in transformed units; `None` on a side keeps the trained value.
    pub limits: [Option<f64>; 2],
    /// Transform applied to the data; decides how breaks are generated.
    pub transform: Transform,
    /// `breaks=[...]` in data units (`None` items are skipped): fixed breaks
    /// instead of generated ones.
    pub fixed_breaks: Option<Vec<Option<f64>>>,
    /// `labels=[...]`, one per fixed break.
    pub labels: Option<Vec<String>>,
    /// `format=...` for labels not given in `labels`.
    pub format: Option<LabelPattern>,
    /// `coord_*(xlim=...)` in transformed units: replaces that side of the
    /// expanded domain (no expansion, no data dropped).
    pub coord_limits: [Option<f64>; 2],
}

impl Default for ContinuousPositionScale {
    fn default() -> Self {
        ContinuousPositionScale {
            domain: None,
            multiplicative_expand: 0.05,
            additive_expand: 0.0,
            limits: [None, None],
            transform: Transform::Identity,
            fixed_breaks: None,
            labels: None,
            format: None,
            coord_limits: [None, None],
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
    /// one-unit range at the limit. See [`widen_degenerate`] for zero-width ranges.
    pub fn limited_domain(&self) -> (f64, f64) {
        self.raw_domain()
            .map_or((-0.5, 0.5), |(low, high)| widen_degenerate(low, high))
    }

    /// Trained domain with limits applied, ordered; may be zero-width. `None`
    /// without data or limits.
    fn raw_domain(&self) -> Option<(f64, f64)> {
        let finite = |limit: Option<f64>| limit.filter(|value| value.is_finite());
        let (low_limit, high_limit) = (finite(self.limits[0]), finite(self.limits[1]));
        let (low, high) = match (self.domain, low_limit, high_limit) {
            (_, Some(low), Some(high)) => (low, high),
            (Some((low, high)), low_limit, high_limit) => {
                (low_limit.unwrap_or(low), high_limit.unwrap_or(high))
            }
            (None, Some(value), None) | (None, None, Some(value)) => (value, value),
            (None, None, None) => return None,
        };
        Some((low.min(high), low.max(high)))
    }

    /// Domain after limits and expansion: the panel spans exactly this range.
    ///
    /// As lets-plot (`PositionalScalesUtil.finalizeDomains`): expand, then widen a
    /// zero-width result (unexpanded); no data gives -0.5..0.5. With a transform the
    /// expansion is computed in data units (`PlotUtil.rangeWithExpand`): ends that
    /// cannot be transformed back (e.g. a padded log axis reaching below 0) stay
    /// unexpanded.
    ///
    /// Coord limits then replace their sides exactly (lets-plot zooms the panel
    /// to them without expansion).
    pub fn expanded_domain(&self) -> (f64, f64) {
        let (low, high) = self.scale_domain();
        match self.coord_limits {
            [None, None] => (low, high),
            [coord_low, coord_high] => {
                let (low, high) = (coord_low.unwrap_or(low), coord_high.unwrap_or(high));
                widen_degenerate(low.min(high), low.max(high))
            }
        }
    }

    /// Set coord limits given in data units (transformed here; a side outside the
    /// transform's domain stays open).
    pub fn set_coord_limits(&mut self, limits: [Option<f64>; 2]) {
        let [low, high] = limits.map(|limit| limit.and_then(|value| self.transform.apply(value)));
        self.coord_limits = match (low, high) {
            (Some(low), Some(high)) => [Some(low.min(high)), Some(low.max(high))],
            // Reverse runs the other way: a lower data limit is an upper position.
            _ if self.transform == Transform::Reverse => [high, low],
            _ => [low, high],
        };
    }

    /// The scale's own expanded domain (without coord limits).
    fn scale_domain(&self) -> (f64, f64) {
        let Some((low, high)) = self.raw_domain() else {
            return (-0.5, 0.5);
        };
        let transform = self.transform;
        let (low, high) = if transform == Transform::Identity {
            // Halves keep the arithmetic finite near f64::MAX; a span that overflows
            // f64 (e.g. -1e308..1e308) is fine.
            let padding =
                (high / 2.0 - low / 2.0) * 2.0 * self.multiplicative_expand + self.additive_expand;
            (
                (low - padding).max(-f64::MAX),
                (high + padding).min(f64::MAX),
            )
        } else {
            let (first, second) = (transform.inverse(low), transform.inverse(high));
            let (data_low, data_high) = (first.min(second), first.max(second));
            let padding =
                self.additive_expand + (data_high - data_low) * self.multiplicative_expand;
            let end = |value: f64, fallback: f64| {
                let transformed = transform.apply_unchecked(value);
                if transformed.is_nan() {
                    fallback
                } else {
                    transformed
                }
            };
            let (first, second) = (end(data_low - padding, low), end(data_high + padding, high));
            (first.min(second), first.max(second))
        };
        widen_degenerate(low, high)
    }

    /// A data value in transformed units, or `None` when it is outside the
    /// transform's domain or the scale limits (lets-plot drops those rows).
    pub fn transform_value(&self, value: f64) -> Option<f64> {
        let transformed = self.transform.apply(value)?;
        let [low, high] = self.limits;
        let inside = low.is_none_or(|low| !low.is_finite() || transformed >= low)
            && high.is_none_or(|high| !high.is_finite() || transformed <= high);
        inside.then_some(transformed)
    }

    /// About `target_count` breaks across the expanded domain, as lets-plot's
    /// `TransformedDomainBreaksGenerator`: linear in data units for identity and
    /// reverse; otherwise linear in transformed units (whole powers for log10 when
    /// that gives 3 or more), each labelled with the step to its neighbour.
    /// With `breaks=[...]`, those inside the expanded domain instead (see
    /// [`Self::fixed`]); `format` replaces the generated labels.
    pub fn breaks(&self, target_count: usize) -> ScaleBreaks {
        if let Some(values) = &self.fixed_breaks {
            return self.fixed(values);
        }
        let (values, mut breaks) = self.generated_breaks(target_count);
        if let Some(format) = &self.format {
            // The break values themselves: inverse(apply(1000)) is 999.999... on log axes.
            breaks.labels = values
                .iter()
                .map(|&value| format.format(PatternValue::Number(value)))
                .collect();
        }
        breaks
    }

    /// `breaks=[...]` in their given order (lets-plot `ScaleBreaks.Fixed`): nulls,
    /// values outside the transform's domain and positions outside the expanded
    /// domain are dropped with their labels. Labels are `labels`, else `format`,
    /// else lets-plot's default number format `{,~g}`.
    fn fixed(&self, values: &[Option<f64>]) -> ScaleBreaks {
        let (low, high) = self.expanded_domain();
        let (low, high) = (low.min(high), low.max(high));
        let default = LabelPattern::parse("{,~g}").ok();
        let pattern = self.format.as_ref().or(default.as_ref());
        let mut breaks = ScaleBreaks {
            positions: Vec::new(),
            labels: Vec::new(),
        };
        for (index, value) in values.iter().enumerate() {
            let Some(value) = *value else { continue };
            let Some(position) = self.transform.apply(value) else {
                continue;
            };
            if !(low..=high).contains(&position) {
                continue;
            }
            let label = match (&self.labels, pattern) {
                (Some(labels), _) => Label::plain(labels.get(index).cloned().unwrap_or_default()),
                (None, Some(pattern)) => pattern.format(PatternValue::Number(value)),
                (None, None) => Label::plain(value.to_string()),
            };
            breaks.positions.push(position);
            breaks.labels.push(label);
        }
        breaks
    }

    /// Generated breaks with their values in data units.
    fn generated_breaks(&self, target_count: usize) -> (Vec<f64>, ScaleBreaks) {
        let (low, high) = self.expanded_domain();
        let transform = self.transform;
        let ordered = |a: f64, b: f64| if a <= b { (a, b) } else { (b, a) };
        let (data_low, data_high) = ordered(transform.inverse(low), transform.inverse(high));
        if transform.is_linear() {
            let values = linear_breaks(data_low, data_high, target_count);
            let labels = format_breaks(&values);
            let positions = values
                .iter()
                .map(|&value| transform.inverse(value))
                .collect();
            return (values, ScaleBreaks { positions, labels });
        }
        let (Some(low), Some(high)) = (transform.apply(data_low), transform.apply(data_high))
        else {
            let empty = ScaleBreaks {
                positions: Vec::new(),
                labels: Vec::new(),
            };
            return (Vec::new(), empty);
        };
        let (low, high) = ordered(low, high);
        let mut count = target_count;
        if transform == Transform::Log10 {
            let whole_powers = (high.floor() - low.ceil()).round() + 1.0;
            if (3.0..=target_count as f64).contains(&whole_powers) {
                count = whole_powers as usize;
            }
        }
        let values: Vec<f64> = linear_breaks(low, high, count)
            .into_iter()
            .map(|position| transform.inverse(position))
            .filter(|value| transform.is_in_domain(*value))
            .collect();
        let labels = format_breaks_individually(&values);
        let positions = values
            .iter()
            .map(|&value| transform.apply(value).unwrap_or(f64::NAN))
            .collect();
        (values, ScaleBreaks { positions, labels })
    }

    /// Map a transformed value to 0..1 across the panel (outside for out-of-range values).
    /// Non-finite input gives NaN.
    pub fn map(&self, value: f64) -> f64 {
        if !value.is_finite() {
            return f64::NAN;
        }
        let (low, high) = self.expanded_domain();
        (value / 2.0 - low / 2.0) / (high / 2.0 - low / 2.0)
    }
}

/// A zero-width (or rounding-noise) range widened to +/- 0.5 around its low end
/// (lets-plot `ensureApplicableRange`), or relatively for huge values where
/// +/- 0.5 would be lost to rounding.
fn widen_degenerate(low: f64, high: f64) -> (f64, f64) {
    if super::breaks::is_degenerate_range(low, high) {
        let half_width = 0.5_f64.max(low.abs() * 1e-9);
        return (low - half_width, low + half_width);
    }
    (low, high)
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
        // lets-plot widens after expanding: the +/- 0.5 is not padded.
        assert_eq!(single.expanded_domain(), (2.5, 3.5));
        assert_eq!(empty.expanded_domain(), (-0.5, 0.5));
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
    fn values_outside_limits_or_domain_are_dropped() {
        let scale = ContinuousPositionScale {
            limits: [Some(0.0), Some(2.0)],
            transform: Transform::Log10,
            ..ContinuousPositionScale::default()
        };
        assert_eq!(scale.transform_value(10.0), Some(1.0));
        assert_eq!(scale.transform_value(1000.0), None);
        assert_eq!(scale.transform_value(0.5), None);
        assert_eq!(scale.transform_value(-1.0), None);
        let open = ContinuousPositionScale {
            limits: [None, Some(f64::NAN)],
            ..ContinuousPositionScale::default()
        };
        assert_eq!(open.transform_value(-1e9), Some(-1e9));
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
