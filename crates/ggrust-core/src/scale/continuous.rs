//! Continuous position scale: trained on data, expanded like lets-plot.

use super::Transform;
use super::breaks::linear_breaks;
use super::datetime::TemporalAxis;
use super::format::{Label, format_breaks, format_breaks_individually};
use super::label_pattern::{LabelPattern, PatternValue};
use super::number_format::ExponentFormat;
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
    /// `break_width=...` (number axes): a break at every multiple of this width,
    /// in transformed units, across the expanded domain; laid out as fixed breaks.
    pub break_width: Option<f64>,
    /// `na_value=...` in transformed units: missing values, and values outside the
    /// limits or the transform's domain, are drawn here instead of dropped (ggplot2).
    pub na_value: Option<f64>,
    /// `labels=[...]`, one per fixed break.
    pub labels: Option<Vec<String>>,
    /// Spec path of `labels=[...]` given without `breaks=`: once the scale is
    /// trained, [`Self::attach_labels`] fixes the generated breaks and labels them.
    pub labels_without_breaks: Option<String>,
    /// `format=...` for labels not given in `labels`.
    pub format: Option<LabelPattern>,
    /// `coord_*(xlim=...)` in transformed units: replaces that side of the
    /// expanded domain (no expansion, no data dropped).
    pub coord_limits: [Option<f64>; 2],
    /// Include 0 and do not pad past it (lets-plot `rangeIncludesZero`, bars).
    /// Ignored when the limits exclude 0.
    pub include_zero: bool,
    /// Domain of an axis nothing trained (lets-plot `preferableNullDomain`), unpadded.
    pub null_domain: (f64, f64),
    /// Date-time or duration axis: calendar breaks and date labels
    /// (`scale_*_datetime()`, `scale_*_time()`, temporal columns).
    pub temporal: Option<TemporalAxis>,
    /// The theme's `exponent_format` for generated and formatted labels (set
    /// with [`Self::set_exponent_format`]).
    pub exponent_format: ExponentFormat,
    /// Ranges that widen the domain even past the limits (bar width spans:
    /// lets-plot starts from the limits and joins every layer's range).
    beyond_limits: Option<(f64, f64)>,
    /// Sides of the panel the axis is drawn on (`position=`).
    pub sides: super::position::AxisSides,
    /// `guide_axis_mark()`: the labels are spread with links to their breaks.
    pub mark: Option<crate::layout::axis_mark::AxisMark>,
    /// `guide_axis_nested()`: brackets over ranges of the axis.
    pub nested: Option<std::sync::Arc<crate::layout::axis_nested::AxisNested>>,
    /// `guide_axis(cap=)`: ends of the axis line cut back to the outermost breaks.
    pub cap: crate::layout::axis_cap::AxisCap,
    /// `sec_axis=...`: a secondary axis on the other side.
    pub secondary: Option<super::secondary::SecondaryAxis>,
    /// `oob=...`: what values outside the limits become.
    pub oob: PositionOob,
    /// `minor_breaks=[...]` in transformed units; `None`: halfway between breaks.
    pub minor_breaks: Option<Vec<f64>>,
    /// `coord_trans(x=..., y=...)`: a warp of the axis after the stats. Breaks stay
    /// in scale units; the domain is expanded and mapped in warped units (ggplot2).
    pub coord_transform: Transform,
}

/// `oob=` of a continuous position scale (ggplot2 `scales::censor`, `squish`,
/// `oob_keep`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PositionOob {
    /// Outside the limits is missing (lets-plot, ggplot2 default).
    #[default]
    Censor,
    /// Clamped onto the nearest limit.
    Squish,
    /// Kept as is: stats see it, the panel clips its drawing.
    Keep,
}

impl PositionOob {
    /// The accepted names, for messages.
    pub const NAMES: [&'static str; 3] = ["censor", "squish", "keep"];

    /// `censor`, `squish` or `keep`.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "censor" => Some(PositionOob::Censor),
            "squish" => Some(PositionOob::Squish),
            "keep" => Some(PositionOob::Keep),
            _ => None,
        }
    }
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
            break_width: None,
            na_value: None,
            labels: None,
            labels_without_breaks: None,
            format: None,
            coord_limits: [None, None],
            include_zero: false,
            null_domain: (-0.5, 0.5),
            temporal: None,
            exponent_format: ExponentFormat::default(),
            beyond_limits: None,
            sides: Default::default(),
            mark: None,
            nested: None,
            cap: Default::default(),
            secondary: None,
            oob: PositionOob::Censor,
            minor_breaks: None,
            coord_transform: Transform::Identity,
        }
    }
}

/// Breaks generated for `labels=[...]` without `breaks=`: ggplot2's default
/// count, the same at any plot size.
pub const LABELLED_BREAKS_COUNT: usize = 5;

/// Most breaks a `break_width` may give before it is widened.
const MAX_WIDTH_BREAKS: f64 = 10_000.0;

/// Relative slack for breaks on a domain edge (in widths, or in domain spans).
const EDGE_TOLERANCE: f64 = 1e-9;

impl ContinuousPositionScale {
    /// `labels=[...]` without `breaks=` (ggplot2): the trained scale generates
    /// [`LABELLED_BREAKS_COUNT`] breaks, which become fixed breaks (they stay
    /// attached to their labels under HTML zoom) labelled in order. An error when
    /// the counts differ, naming the generated breaks. Call it again after the
    /// range changes: the breaks are generated anew.
    pub fn attach_labels(&mut self) -> crate::Result<()> {
        let Some(path) = &self.labels_without_breaks else {
            return Ok(());
        };
        let path = path.clone();
        // Generated again after a range change (gggrid sharing).
        self.fixed_breaks = None;
        let generated = self.breaks(LABELLED_BREAKS_COUNT);
        let values: Vec<Option<f64>> = generated
            .positions
            .iter()
            .map(|&position| Some(self.transform.inverse(position)))
            .collect();
        let count = self.labels.as_ref().map_or(0, Vec::len);
        if count != values.len() {
            let shown: Vec<String> = generated.labels.iter().map(Label::to_string).collect();
            return Err(crate::Error::spec(
                path,
                format!(
                    "{count} labels for {} computed breaks ({}); give one label per break, \
                     or set breaks=[...] too",
                    values.len(),
                    shown.join(", ")
                ),
            ));
        }
        self.fixed_breaks = Some(values);
        Ok(())
    }

    /// Write exponents as the theme's `exponent_format` says (generated labels
    /// and `format=` patterns).
    pub fn set_exponent_format(&mut self, exponent: ExponentFormat) {
        self.exponent_format = exponent;
        if let Some(format) = &self.format {
            self.format = Some(format.with_exponent(exponent));
        }
    }

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

    /// The scale as its axis draws it: the trained domain and the ranges past the
    /// limits joined into one domain, so scales that differ only in how they got
    /// their range compare equal (`gggrid(axes=)` after `sharex`).
    pub fn axis_identity(&self) -> ContinuousPositionScale {
        ContinuousPositionScale {
            domain: self.raw_domain(),
            beyond_limits: None,
            ..self.clone()
        }
    }

    /// Widen the domain to include `[low, high]`, even past the limits.
    pub fn train_beyond_limits(&mut self, low: f64, high: f64) {
        self.beyond_limits = Some(match self.beyond_limits {
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
            .map_or(self.null_domain, |(low, high)| widen_degenerate(low, high))
    }

    /// Trained domain with limits applied, ordered; may be zero-width. `None`
    /// without data or limits.
    /// Domain after limits, before expansion; `None` without data or limits
    /// (a grid sharing axes takes the union of these).
    pub fn trained_domain(&self) -> Option<(f64, f64)> {
        self.raw_domain()
    }

    fn raw_domain(&self) -> Option<(f64, f64)> {
        let finite = |limit: Option<f64>| limit.filter(|value| value.is_finite());
        let (low_limit, high_limit) = (finite(self.limits[0]), finite(self.limits[1]));
        let (low, high) = match (self.domain, low_limit, high_limit) {
            (_, Some(low), Some(high)) => (low, high),
            (Some((low, high)), low_limit, high_limit) => {
                (low_limit.unwrap_or(low), high_limit.unwrap_or(high))
            }
            (None, Some(value), None) | (None, None, Some(value)) => (value, value),
            (None, None, None) => return self.beyond_limits,
        };
        let (low, high) = (low.min(high), low.max(high));
        Some(match self.beyond_limits {
            Some((outer_low, outer_high)) => (low.min(outer_low), high.max(outer_high)),
            None => (low, high),
        })
    }

    /// Domain after limits and expansion: the panel spans exactly this range.
    ///
    /// As lets-plot (`PositionalScalesUtil.finalizeDomains`): expand, then widen a
    /// zero-width result (unexpanded); no data gives -0.5..0.5. With a transform the
    /// expansion is in transformed units, as ggplot2 (lets-plot pads in data units).
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
            _ if self.transform.is_decreasing() => [high, low],
            _ => [low, high],
        };
    }

    /// The scale's own expanded domain, without coord limits.
    pub fn own_domain(&self) -> (f64, f64) {
        self.scale_domain()
    }

    /// The scale's own expanded domain (without coord limits).
    fn scale_domain(&self) -> (f64, f64) {
        let Some((mut low, mut high)) = self.raw_domain() else {
            return self.null_domain;
        };
        let [low_limit, high_limit] = self
            .limits
            .map(|limit| limit.filter(|value| value.is_finite()));
        let warp = self.coord_transform;
        let zero_based = self.include_zero
            && warp.is_in_domain(0.0)
            && low_limit.is_none_or(|limit| limit <= 0.0)
            && high_limit.is_none_or(|limit| limit >= 0.0);
        if zero_based {
            low = low.min(0.0);
            high = high.max(0.0);
        }
        if warp != Transform::Identity {
            return self.warped_domain(low, high);
        }
        // Padding is in transformed units for every transform, as ggplot2
        // (`expand_range4` on the transformed limits). lets-plot pads in data units,
        // which leaves a log axis's ends on the panel edge (deviations.md).
        // Halves keep the arithmetic finite near f64::MAX; a span that overflows
        // f64 (e.g. -1e308..1e308) is fine.
        let padding =
            (high / 2.0 - low / 2.0) * 2.0 * self.multiplicative_expand + self.additive_expand;
        let (pad_low, pad_high) = zero_end_padding(zero_based, low, high, padding);
        let (low, high) = (
            (low - pad_low).max(-f64::MAX),
            (high + pad_high).min(f64::MAX),
        );
        widen_degenerate(low, high)
    }

    /// `coord_trans`: expand `low..high` (scale units) in warped units, back in
    /// scale units (for the breaks). An expanded end the warp cannot invert (below
    /// 0 under sqrt) keeps the unexpanded end; [`Self::map`] uses the warped
    /// range itself.
    fn warped_domain(&self, low: f64, high: f64) -> (f64, f64) {
        let warp = self.coord_transform;
        let Some((warped_low, warped_high)) = self.warped_range_of(low, high) else {
            return widen_degenerate(low, high);
        };
        let back = |warped: f64, fallback: f64| {
            let value = warp.inverse(warped);
            let round_trip = warp
                .apply(value)
                .is_some_and(|again| (again - warped).abs() <= 1e-9 * (1.0 + warped.abs()));
            if round_trip { value } else { fallback }
        };
        let (first, second) = if warp.is_decreasing() {
            (back(warped_high, low), back(warped_low, high))
        } else {
            (back(warped_low, low), back(warped_high, high))
        };
        let (low, high) = (first.min(second), first.max(second));
        if high > low {
            (low, high)
        } else {
            widen_degenerate(low, high)
        }
    }

    /// `low..high` (scale units) warped and expanded, ordered; `None` when an end
    /// is outside the warp's domain.
    fn warped_range_of(&self, low: f64, high: f64) -> Option<(f64, f64)> {
        let warp = self.coord_transform;
        let (first, second) = (warp.apply(low)?, warp.apply(high)?);
        let (low, high) = (first.min(second), first.max(second));
        let padding = (high - low) * self.multiplicative_expand + self.additive_expand;
        Some((low - padding, high + padding))
    }

    /// The panel range in warped units: the expanded warped data range, or the
    /// warped coord limits.
    fn warped_range(&self) -> (f64, f64) {
        let warp = self.coord_transform;
        let unexpanded = || {
            let (low, high) = self.expanded_domain();
            let (first, second) = (warp.apply_unchecked(low), warp.apply_unchecked(high));
            (first.min(second), first.max(second))
        };
        if self.coord_limits != [None, None] {
            return unexpanded();
        }
        let Some((low, high)) = self.raw_domain() else {
            return unexpanded();
        };
        let zero_based = self.include_zero && warp.is_in_domain(0.0);
        let (low, high) = if zero_based {
            (low.min(0.0), high.max(0.0))
        } else {
            (low, high)
        };
        let (low, high) = self.warped_range_of(low, high).unwrap_or_else(unexpanded);
        if high > low {
            (low, high)
        } else {
            // Constant data: the same widening as the unwarped axis.
            widen_degenerate(low, high)
        }
    }

    /// Why the trained range cannot be warped by `coord_trans`, if it cannot (a
    /// log of 0 or less, a square root of a negative).
    pub fn coord_transform_problem(&self) -> Option<String> {
        let warp = self.coord_transform;
        if warp == Transform::Identity {
            return None;
        }
        // Coord limits replace their side of the range.
        let (raw_low, raw_high) = self.raw_domain()?;
        let [coord_low, coord_high] = self.coord_limits;
        let (low, high) = (coord_low.unwrap_or(raw_low), coord_high.unwrap_or(raw_high));
        [low, high]
            .into_iter()
            .find(|value| !warp.is_in_domain(*value))
            .map(|value| {
                format!(
                    "the '{}' transform is undefined at {value}, inside the axis range \
                     {low} to {high}; set limits inside its domain or use another transform",
                    warp.as_str()
                )
            })
    }

    /// A data value in transformed units, or `None` when it is outside the
    /// transform's domain or the scale limits (lets-plot drops those rows).
    pub fn transform_value(&self, value: f64) -> Option<f64> {
        self.limit(self.transform.apply(value)?)
    }

    /// A value in transformed units after `oob`: kept inside the limits; outside
    /// them missing (censor), on the nearest limit (squish), or kept (keep).
    pub fn limit(&self, transformed: f64) -> Option<f64> {
        if self.within_limits(transformed) {
            return Some(transformed);
        }
        match self.oob {
            PositionOob::Censor => None,
            PositionOob::Keep => Some(transformed),
            PositionOob::Squish => {
                let [low, high] = self.limits;
                let value = low
                    .filter(|low| low.is_finite())
                    .map_or(transformed, |low| transformed.max(low));
                Some(
                    high.filter(|high| high.is_finite())
                        .map_or(value, |high| value.min(high)),
                )
            }
        }
    }

    /// Whether a value in transformed units lies within the scale limits.
    pub fn within_limits(&self, transformed: f64) -> bool {
        let [low, high] = self.limits;
        low.is_none_or(|low| !low.is_finite() || transformed >= low)
            && high.is_none_or(|high| !high.is_finite() || transformed <= high)
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
        if let Some(temporal) = &self.temporal {
            // Date breaks are made in data units, then transformed (reverse, log).
            let (low, high) = self.data_domain();
            let (values, labels) = temporal.breaks(low, high, target_count);
            let (positions, labels) = values
                .iter()
                .zip(labels)
                .filter_map(|(&value, label)| {
                    let position = self.transform.apply(value)?;
                    Some((position, Label::plain(label)))
                })
                .unzip();
            return ScaleBreaks { positions, labels };
        }
        if let Some(width) = self.break_width {
            return self.fixed(&self.width_multiples(width));
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

    /// `break_width=...` breaks (lets-plot `breaks = multiples of the width`): every
    /// multiple of `width` inside the expanded domain, in transformed units (2 on
    /// a log10 axis is every other power of ten), as data values. A width far
    /// below the span is widened to a whole multiple giving at most
    /// [`MAX_WIDTH_BREAKS`] (lets-plot would make them all, or hang).
    fn width_multiples(&self, width: f64) -> Vec<Option<f64>> {
        let (low, high) = self.expanded_domain();
        let (low, high) = (low.min(high), low.max(high));
        if !(low.is_finite() && high.is_finite() && width.is_finite() && width > 0.0) {
            return Vec::new();
        }
        let factor = ((high - low) / width / MAX_WIDTH_BREAKS).ceil().max(1.0);
        let width = width * factor;
        // A multiple on an edge can land an ulp past it (0.3 / 0.1 = 2.9999999999999996).
        let (first, last) = (
            (low / width - EDGE_TOLERANCE).ceil(),
            (high / width + EDGE_TOLERANCE).floor(),
        );
        if !(first.is_finite() && last.is_finite()) || last < first {
            return Vec::new();
        }
        let mut values: Vec<f64> = (0..=(last - first) as u64)
            .map(|step| (first + step as f64) * width)
            .filter(|&position| self.transform.is_in_range(position))
            .map(|position| self.transform.inverse(position))
            .collect();
        // Ascending data values, as lets-plot lists them (reverse axes too).
        values.sort_by(f64::total_cmp);
        values.into_iter().map(Some).collect()
    }

    /// `breaks=[...]` in their given order (lets-plot `ScaleBreaks.Fixed`): nulls,
    /// values outside the transform's domain and positions outside the expanded
    /// domain are dropped with their labels. Labels are `labels`, else `format`,
    /// else lets-plot's default number format `{,~g}`.
    fn fixed(&self, values: &[Option<f64>]) -> ScaleBreaks {
        let (low, high) = self.expanded_domain();
        let (low, high) = (low.min(high), low.max(high));
        // Breaks on an edge survive rounding (3 x 0.1 is 0.30000000000000004).
        let tolerance = (high - low) * EDGE_TOLERANCE;
        let (low, high) = (low - tolerance, high + tolerance);
        let default = LabelPattern::parse("{,~g}")
            .ok()
            .map(|pattern| pattern.with_exponent(self.exponent_format));
        let pattern = self.format.as_ref().or(default.as_ref());
        // Date labels unless a number `format` is given.
        let temporal = self.temporal.as_ref().filter(|_| self.format.is_none());
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
            let label = match (&self.labels, temporal, pattern) {
                (Some(labels), _, _) => {
                    Label::plain(labels.get(index).cloned().unwrap_or_default())
                }
                (None, Some(temporal), _) => {
                    let (data_low, data_high) = self.data_domain();
                    Label::plain(temporal.label(value, data_low, data_high, values.len().max(1)))
                }
                (None, None, Some(pattern)) => pattern.format(PatternValue::Number(value)),
                (None, None, None) => Label::plain(value.to_string()),
            };
            breaks.positions.push(position);
            breaks.labels.push(label);
        }
        breaks
    }

    /// The expanded domain in data units, low end first.
    fn data_domain(&self) -> (f64, f64) {
        let (low, high) = self.expanded_domain();
        let (low, high) = (self.transform.inverse(low), self.transform.inverse(high));
        (low.min(high), low.max(high))
    }

    /// Generated breaks with their values in data units.
    fn generated_breaks(&self, target_count: usize) -> (Vec<f64>, ScaleBreaks) {
        let (low, high) = self.expanded_domain();
        let transform = self.transform;
        let ordered = |a: f64, b: f64| if a <= b { (a, b) } else { (b, a) };
        let (data_low, data_high) = ordered(transform.inverse(low), transform.inverse(high));
        if transform.is_linear() {
            let values = linear_breaks(data_low, data_high, target_count);
            let labels = format_breaks(&values, self.exponent_format);
            let positions = values
                .iter()
                .map(|&value| transform.inverse(value))
                .collect();
            return (values, ScaleBreaks { positions, labels });
        }
        // Straight from the expanded domain: a padded end can lie outside the
        // transform's range (sqrt below 0), where a round trip through data units
        // would fold it back.
        let (low, high) = ordered(low, high);
        let mut count = target_count;
        if matches!(transform, Transform::Log10 | Transform::Symlog) {
            let whole_powers = (high.floor() - low.ceil()).round() + 1.0;
            if (3.0..=target_count as f64).contains(&whole_powers) {
                count = whole_powers as usize;
            }
        }
        let values: Vec<f64> = linear_breaks(low, high, count)
            .into_iter()
            .filter(|&position| transform.is_in_range(position))
            .map(|position| transform.inverse(position))
            .filter(|value| transform.is_in_domain(*value))
            .collect();
        let labels = format_breaks_individually(&values, self.exponent_format);
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
        let warp = self.coord_transform;
        if warp != Transform::Identity {
            // Across the warped range, so a decreasing warp flips the axis.
            let (low, high) = self.warped_range();
            return (warp.apply_unchecked(value) - low) / (high - low);
        }
        let (low, high) = self.expanded_domain();
        (value / 2.0 - low / 2.0) / (high / 2.0 - low / 2.0)
    }
}

/// Padding below and above a domain (in data units). Zero-based ranges (bars)
/// are not padded on their zero end, as lets-plot `PlotUtil.rangeWithExpand`.
fn zero_end_padding(zero_based: bool, low: f64, high: f64, padding: f64) -> (f64, f64) {
    let same_side = low == 0.0 || high == 0.0 || low.signum() == high.signum();
    match (zero_based && same_side, low >= 0.0) {
        (false, _) => (padding, padding),
        (true, true) => (0.0, padding),
        (true, false) => (padding, 0.0),
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
    use crate::data::Column;
    use crate::scale::PositionScale;

    fn warped(warp: Transform, low: f64, high: f64) -> ContinuousPositionScale {
        let mut scale = ContinuousPositionScale {
            multiplicative_expand: 0.0,
            coord_transform: warp,
            ..ContinuousPositionScale::default()
        };
        scale.train_range(low, high);
        scale
    }

    #[test]
    fn coord_transform_maps_through_the_warp() {
        let scale = warped(Transform::Log10, 1.0, 1000.0);
        assert!((scale.map(1.0)).abs() < 1e-12);
        assert!((scale.map(10.0) - 1.0 / 3.0).abs() < 1e-12);
        assert!((scale.map(1000.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn coord_transform_reverse_flips_the_axis() {
        let scale = warped(Transform::Reverse, 0.0, 10.0);
        assert!((scale.map(0.0) - 1.0).abs() < 1e-12);
        assert!((scale.map(10.0)).abs() < 1e-12);
    }

    #[test]
    fn coord_transform_expands_in_warped_units() {
        let mut scale = warped(Transform::Log10, 1.0, 100.0);
        scale.multiplicative_expand = 0.05;
        // 0.1 decades of padding on each side of 0..2 decades.
        let (low, high) = scale.expanded_domain();
        assert!((low - 10f64.powf(-0.1)).abs() < 1e-9, "{low}");
        assert!((high - 10f64.powf(2.1)).abs() < 1e-9, "{high}");
        assert!((scale.map(1.0) - 0.1 / 2.2).abs() < 1e-12);
    }

    #[test]
    fn coord_transform_sqrt_keeps_zero_when_padding_would_go_negative() {
        let mut scale = warped(Transform::Sqrt, 0.0, 100.0);
        scale.multiplicative_expand = 0.05;
        let (low, _) = scale.expanded_domain();
        assert_eq!(low, 0.0);
        // The panel still pads below zero in warped units.
        assert!(scale.map(0.0) > 0.0);
    }

    #[test]
    fn coord_transform_problem_names_the_bad_end() {
        let scale = warped(Transform::Log10, 0.0, 100.0);
        let problem = scale.coord_transform_problem().expect("log10 of 0");
        assert!(problem.contains("'log10'") && problem.contains("undefined at 0"));
        let mut limited = warped(Transform::Log10, 0.0, 100.0);
        limited.set_coord_limits([Some(1.0), None]);
        assert_eq!(limited.coord_transform_problem(), None);
        assert_eq!(
            warped(Transform::Identity, -1.0, 1.0).coord_transform_problem(),
            None
        );
    }

    fn labelled(labels: &[&str]) -> ContinuousPositionScale {
        let mut scale = ContinuousPositionScale {
            labels: Some(labels.iter().map(|label| label.to_string()).collect()),
            labels_without_breaks: Some("scales[0].labels".to_string()),
            multiplicative_expand: 0.0,
            ..ContinuousPositionScale::default()
        };
        scale.train_range(0.0, 100.0);
        scale
    }

    #[test]
    fn labels_without_breaks_fix_the_generated_breaks() {
        let mut scale = labelled(&["a", "b", "c", "d", "e", "f"]);
        scale.attach_labels().expect("six breaks over 0..100");
        assert_eq!(
            scale.fixed_breaks,
            Some(vec![
                Some(0.0),
                Some(20.0),
                Some(40.0),
                Some(60.0),
                Some(80.0),
                Some(100.0)
            ])
        );
        // A wider range generates the breaks anew.
        let mut wider = scale.clone();
        wider.train_range(0.0, 1000.0);
        wider.attach_labels().expect("six breaks over 0..1000");
        assert_eq!(wider.fixed_breaks.as_ref().map(Vec::len), Some(6));
        assert_eq!(wider.fixed_breaks.as_ref().and_then(|v| v[5]), Some(1000.0));
        // Any target count gives the same, labelled breaks now.
        let breaks = scale.breaks(1);
        let texts: Vec<String> = breaks.labels.iter().map(Label::to_string).collect();
        assert_eq!(texts, ["a", "b", "c", "d", "e", "f"]);
    }

    #[test]
    fn labels_without_breaks_must_match_the_generated_count() {
        let error = labelled(&["a", "b"])
            .attach_labels()
            .expect_err("two labels for six breaks");
        let message = error.to_string();
        assert!(message.contains("scales[0].labels"), "{message}");
        assert!(
            message.contains("2 labels for 6 computed breaks (0, 20, 40, 60, 80, 100)"),
            "{message}"
        );
        assert!(message.contains("breaks=[...]"), "{message}");
    }

    #[test]
    fn attach_labels_leaves_given_breaks_alone() {
        let mut scale = ContinuousPositionScale {
            fixed_breaks: Some(vec![Some(1.0)]),
            labels: Some(vec!["one".to_string()]),
            ..ContinuousPositionScale::default()
        };
        scale.train_range(0.0, 2.0);
        scale.attach_labels().expect("no labels without breaks");
        assert_eq!(scale.fixed_breaks, Some(vec![Some(1.0)]));
    }

    #[test]
    fn spans_widen_the_domain_past_limits() {
        let mut scale = ContinuousPositionScale {
            limits: [Some(1.5), Some(3.0)],
            multiplicative_expand: 0.0,
            ..ContinuousPositionScale::default()
        };
        scale.train_beyond_limits(1.55, 3.45);
        assert_eq!(scale.expanded_domain(), (1.5, 3.45));
    }

    #[test]
    fn null_domain_applies_only_to_an_untrained_axis() {
        let mut scale = ContinuousPositionScale {
            null_domain: (0.0, 1.0),
            ..Default::default()
        };
        assert_eq!(scale.expanded_domain(), (0.0, 1.0));
        scale.train_range(2.0, 4.0);
        assert_ne!(scale.expanded_domain(), (0.0, 1.0));
    }

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

    fn widths(low: f64, high: f64, width: f64, transform: Transform) -> Vec<String> {
        let mut scale = ContinuousPositionScale {
            break_width: Some(width),
            transform,
            ..ContinuousPositionScale::default()
        };
        scale.train_range(low, high);
        scale
            .breaks(5)
            .labels
            .iter()
            .map(Label::to_string)
            .collect()
    }

    #[test]
    fn break_width_gives_every_multiple_in_the_expanded_domain() {
        // 0..10 expands to -0.5..10.5 (lets-plot: 0, 3, 6, 9 and 0, 10.25).
        assert_eq!(
            widths(0.0, 10.0, 3.0, Transform::Identity),
            ["0", "3", "6", "9"]
        );
        assert_eq!(
            widths(0.0, 10.0, 10.25, Transform::Identity),
            ["0", "10.25"]
        );
        assert_eq!(
            widths(0.13, 0.91, 0.25, Transform::Identity),
            ["0.25", "0.5", "0.75"]
        );
        assert_eq!(
            widths(-7.0, 13.0, 5.0, Transform::Identity),
            ["-5", "0", "5", "10"]
        );
    }

    #[test]
    fn break_width_counts_in_transformed_units() {
        // log10 0..4 expands to -0.2..4.2: every second power of ten.
        assert_eq!(
            widths(0.0, 4.0, 2.0, Transform::Log10),
            ["1", "100", "10,000"]
        );
        // Reverse axes list ascending data values, as lets-plot.
        assert_eq!(
            widths(-10.0, 0.0, 3.0, Transform::Reverse),
            ["0", "3", "6", "9"]
        );
    }

    #[test]
    fn break_width_keeps_breaks_on_the_domain_edges() {
        let edges = |low: f64, high: f64, width: f64| {
            let mut scale = ContinuousPositionScale {
                break_width: Some(width),
                multiplicative_expand: 0.0,
                ..ContinuousPositionScale::default()
            };
            scale.train_range(low, high);
            let labels: Vec<String> = scale
                .breaks(5)
                .labels
                .iter()
                .map(Label::to_string)
                .collect();
            labels
        };
        assert_eq!(edges(0.0, 0.3, 0.1), ["0", "0.1", "0.2", "0.3"]);
        assert_eq!(
            edges(-0.3, 0.3, 0.1),
            ["-0.3", "-0.2", "-0.1", "0", "0.1", "0.2", "0.3"]
        );
        assert_eq!(edges(1e-5, 3e-5, 1e-5).len(), 3);
    }

    #[test]
    fn tiny_break_width_is_widened() {
        assert!(widths(0.0, 1e6, 1e-9, Transform::Identity).len() <= MAX_WIDTH_BREAKS as usize);
    }

    #[test]
    fn break_width_breaks_are_fixed() {
        let scale = PositionScale::Continuous(ContinuousPositionScale {
            break_width: Some(1.0),
            ..ContinuousPositionScale::default()
        });
        assert!(scale.has_fixed_breaks());
    }

    #[test]
    fn na_value_places_missing_and_dropped_positions() {
        let scale = PositionScale::Continuous(ContinuousPositionScale {
            na_value: Some(0.0),
            limits: [Some(1.0), Some(3.0)],
            ..ContinuousPositionScale::default()
        });
        let column =
            ContinuousColumn::from_options(vec![Some(2.0), None, Some(f64::NAN), Some(9.0)]);
        let Ok(Column::Continuous(positions)) = scale.positions(Column::Continuous(column)) else {
            panic!("continuous positions");
        };
        let values: Vec<Option<f64>> = (0..positions.len()).map(|row| positions.get(row)).collect();
        assert_eq!(values, [Some(2.0), Some(0.0), Some(0.0), Some(0.0)]);
    }
}
