//! `stat="bin"`: counts of x in equal-width bins (or between given `breaks`), as
//! lets-plot's `BinStat` / `BinStatUtil`.
//!
//! Bins are laid out once over the x range of every layer (scale limits replace
//! its ends), then filled per group.
//! Options: `bins` (default 30, clamped to 1..500), `binwidth`, `boundary` or
//! `center` (boundary wins), `breaks`, `threshold`.
//!
//! Computed: `..count..`, `..density..` (area 1), `..sumprop..`, `..sumpct..`.
//!
//! What it does not do: transform `breaks` given for a transformed axis (they are
//! used as given, in axis units), or round bins on a discrete axis.

use std::collections::HashMap;

use super::count::row_weight;
use super::groups::per_group;
use super::{Stat, StatContext};
use crate::data::{Computed, ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::spec::{Aesthetic, Value};
use crate::{Error, Result};

/// lets-plot `BinStat.DEF_BIN_COUNT`.
const DEFAULT_BIN_COUNT: f64 = 30.0;
/// lets-plot `BinStatUtil.MAX_BIN_COUNT`.
const MAX_BIN_COUNT: usize = 500;

/// Counts x per bin.
pub struct BinStat;

impl Stat for BinStat {
    fn name(&self) -> &'static str {
        "bin"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &[
            "bins",
            "binwidth",
            "center",
            "boundary",
            "breaks",
            "threshold",
        ]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Weight]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X]
    }

    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        &[(Aesthetic::Y, Computed::Count)]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        context: &StatContext,
    ) -> Result<LayerData> {
        let options = BinOptions::from_parameters(parameters)?;
        let breaks = breaks(parameters)?;
        let threshold = parameters.number("threshold")?;
        let empty = ContinuousColumn::from_values(Vec::new());
        let x = data.continuous(Aesthetic::X).unwrap_or(&empty);
        let weight = data.continuous(Aesthetic::Weight);
        // Bins span every layer's x (lets-plot `overallXRange`); a stat run on its
        // own falls back to this layer's.
        let range = context.x_range.or_else(|| finite_range(x));
        per_group(&data, |rows| {
            let values: Vec<(f64, f64)> = rows
                .iter()
                .filter_map(|&row| {
                    let value = x.get(row).filter(|value| value.is_finite())?;
                    Some((value, row_weight(weight, row)))
                })
                .collect();
            // A layer without rows (e.g. in a facet panel) has no bins, as in lets-plot.
            let mut bins = if data.row_count() == 0 {
                Bins::default()
            } else if !breaks.is_empty() {
                bins_between_breaks(&values, &breaks)
            } else if let Some(range) = range {
                let (count, width, start) = binning_parameters(range, &options);
                equal_bins(&values, start, count, width)
            } else {
                Bins::default()
            };
            if let Some(threshold) = threshold {
                bins.drop_below(threshold);
            }
            Ok(bins.into_columns())
        })
    }
}

/// `bins` and `binwidth`.
struct BinOptions {
    count: usize,
    /// Positive, or `None`.
    width: Option<f64>,
    /// `boundary` or `center`, if given.
    anchor: Anchor,
}

#[derive(Clone, Copy)]
enum Anchor {
    None,
    Center(f64),
    Boundary(f64),
}

impl BinOptions {
    fn from_parameters(parameters: &Parameters) -> Result<Self> {
        let count = parameters.number("bins")?.unwrap_or(DEFAULT_BIN_COUNT);
        // lets-plot reads `bins` as an integer and clamps it to 1..500.
        let count = (count.trunc().max(1.0) as usize).min(MAX_BIN_COUNT);
        let width = parameters.number("binwidth")?.filter(|width| *width > 0.0);
        let anchor = match (parameters.number("boundary")?, parameters.number("center")?) {
            (Some(boundary), _) => Anchor::Boundary(boundary),
            (None, Some(center)) => Anchor::Center(center),
            (None, None) => Anchor::None,
        };
        Ok(BinOptions {
            count,
            width,
            anchor,
        })
    }

    /// lets-plot `binCountAndWidth`.
    fn count_and_width(&self, span: f64) -> (usize, f64) {
        match self.width {
            Some(width) => {
                let count = (span / width).min(MAX_BIN_COUNT as f64).ceil();
                (count.max(0.0) as usize, width)
            }
            None => (self.count, span / self.count as f64),
        }
    }
}

/// `breaks=[...]`: finite, distinct, sorted. Fewer than two breaks bin nothing.
fn breaks(parameters: &Parameters) -> Result<Vec<f64>> {
    let Some(value) = parameters.value("breaks") else {
        return Ok(Vec::new());
    };
    let Value::Array(items) = value else {
        return Err(Error::InvalidSpec(format!(
            "at '{}': expected a list of numbers, got {value}",
            parameters.path_of("breaks")
        )));
    };
    let mut breaks = Vec::with_capacity(items.len());
    for item in items {
        match item {
            Value::Null => {}
            item => match item.as_f64() {
                Some(number) if number.is_finite() => breaks.push(number),
                Some(_) => {}
                None => {
                    return Err(Error::InvalidSpec(format!(
                        "at '{}': expected numbers, got {item}",
                        parameters.path_of("breaks")
                    )));
                }
            },
        }
    }
    breaks.sort_by(f64::total_cmp);
    breaks.dedup();
    Ok(breaks)
}

fn finite_range(column: &ContinuousColumn) -> Option<(f64, f64)> {
    (0..column.len())
        .filter_map(|row| column.get(row).filter(|value| value.is_finite()))
        .fold(None, |range, value| match range {
            None => Some((value, value)),
            Some((low, high)) => Some((f64::min(low, value), f64::max(high, value))),
        })
}

/// lets-plot `getBinningParameters`: (bin count, bin width, left edge of the first bin).
fn binning_parameters((low, high): (f64, f64), options: &BinOptions) -> (usize, f64, f64) {
    let mut start = low;
    let mut span = high - start;
    let (_, initial_width) = options.count_and_width(span);
    // Widen by 0.7 bin on each side to leave room for the anchor shift.
    start -= initial_width * 0.7;
    span += initial_width * 1.4;
    let (count, width) = options.count_and_width(span);
    let (position, center) = match options.anchor {
        Anchor::None => return (count, width, start),
        Anchor::Center(position) => (position, true),
        Anchor::Boundary(position) => (position, false),
    };
    let mut smallest = if center { f64::MAX } else { position - start };
    for index in 0..count {
        let left = start + index as f64 * width;
        let delta = if center {
            position - (left + width / 2.0)
        } else {
            position - (left + width)
        };
        if delta.abs() < smallest.abs() {
            smallest = delta;
        }
    }
    // At most half a bin (Kotlin `%` is Rust `%`: the sign follows the dividend).
    start += smallest % (width / 2.0);
    (count, width, start)
}

/// Bin centers and the computed variables of one group.
#[derive(Default)]
struct Bins {
    x: Vec<f64>,
    count: Vec<f64>,
    density: Vec<f64>,
    sum_proportion: Vec<f64>,
}

impl Bins {
    /// lets-plot `threshold`: bins at the ends with a count at or below it are
    /// dropped (counts NaN; centers too, except where that would leave fewer than
    /// two centers, which the bar width needs).
    fn drop_below(&mut self, threshold: f64) {
        let left: Vec<usize> = (0..self.count.len())
            .take_while(|&index| self.count[index] <= threshold)
            .collect();
        let right: Vec<usize> = (0..self.count.len())
            .rev()
            .take_while(|&index| self.count[index] <= threshold)
            .collect();
        let dropped: Vec<usize> = left.iter().chain(&right).copied().collect();
        for &index in &dropped {
            self.count[index] = f64::NAN;
            self.density[index] = f64::NAN;
            self.sum_proportion[index] = f64::NAN;
        }
        let without_center: Vec<usize> = if self.x.len() - dropped.len().min(self.x.len()) > 1 {
            dropped
        } else if !left.is_empty() {
            left[..left.len() - 1]
                .iter()
                .chain(&right)
                .copied()
                .collect()
        } else if !right.is_empty() {
            right[..right.len() - 1].to_vec()
        } else {
            Vec::new()
        };
        for index in without_center {
            self.x[index] = f64::NAN;
        }
    }

    fn into_columns(self) -> Vec<(Variable, Vec<f64>)> {
        let sum_percent = self
            .sum_proportion
            .iter()
            .map(|value| value * 100.0)
            .collect();
        vec![
            (Variable::Aesthetic(Aesthetic::X), self.x),
            (Variable::Computed(Computed::Count), self.count),
            (Variable::Computed(Computed::Density), self.density),
            (
                Variable::Computed(Computed::SumProportion),
                self.sum_proportion,
            ),
            (Variable::Computed(Computed::SumPercent), sum_percent),
        ]
    }
}

/// lets-plot `computeHistogramBins` over equal-width bins.
fn equal_bins(values: &[(f64, f64)], start: f64, count: usize, width: f64) -> Bins {
    let density_factor = if width > 0.0 { 1.0 / width } else { 1.0 };
    let mut total = 0.0;
    let mut count_by_bin: HashMap<i64, f64> = HashMap::new();
    for &(value, weight) in values {
        total += weight;
        // Kotlin `toInt()`: NaN is 0, out of range saturates (as Rust `as`).
        let bin = ((value - start) / width).floor() as i64;
        *count_by_bin.entry(bin).or_insert(0.0) += weight;
    }
    let mut bins = Bins::default();
    for index in 0..count {
        let count = count_by_bin.get(&(index as i64)).copied().unwrap_or(0.0);
        let proportion = count / total;
        bins.x.push(start + width / 2.0 + index as f64 * width);
        bins.count.push(count);
        bins.sum_proportion.push(proportion);
        bins.density.push(proportion * density_factor);
    }
    bins
}

/// The bin of `value` between sorted, distinct `breaks` (index of its lower
/// break), as lets-plot `bracketingIndicesOrNull`: bins are `(low, high]`, the
/// first one closed; `None` outside the breaks.
pub fn bracketing_index(breaks: &[f64], value: f64) -> Option<usize> {
    let first = *breaks.first()?;
    let below = breaks.partition_point(|edge| *edge < value);
    let upper = if below == 0 && value == first {
        1
    } else {
        below
    };
    (1..breaks.len()).contains(&upper).then(|| upper - 1)
}

/// lets-plot `computeHistogramBins` between `breaks` (sorted, distinct), with
/// bins as [`bracketing_index`].
fn bins_between_breaks(values: &[(f64, f64)], breaks: &[f64]) -> Bins {
    if breaks.len() < 2 {
        return Bins::default();
    }
    let (first, last) = (breaks[0], breaks[breaks.len() - 1]);
    let mut total = 0.0;
    let mut counts = vec![0.0; breaks.len() - 1];
    for &(value, weight) in values {
        if value < first || value > last {
            continue;
        }
        total += weight;
        if let Some(index) = bracketing_index(breaks, value) {
            counts[index] += weight;
        }
    }
    let normalizing: f64 = counts
        .iter()
        .zip(breaks.windows(2))
        .map(|(count, edges)| count.abs() * (edges[1] - edges[0]))
        .sum();
    let mut bins = Bins::default();
    for (count, edges) in counts.into_iter().zip(breaks.windows(2)) {
        bins.x.push(edges[0] + (edges[1] - edges[0]) / 2.0);
        bins.count.push(count);
        bins.density.push(count / normalizing);
        bins.sum_proportion.push(count / total);
    }
    bins
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Column;
    use crate::spec::Options;

    fn compute(values: &[f64], options: &str) -> LayerData {
        let mut data = LayerData::new(values.len());
        data.insert(
            Aesthetic::X,
            Column::Continuous(ContinuousColumn::from_values(values.to_vec())),
        )
        .unwrap();
        let options: Options = serde_json::from_str(options).unwrap();
        BinStat
            .compute(
                data,
                &Parameters::new(&options, "layers[0]"),
                &StatContext::default(),
            )
            .unwrap()
    }

    fn values(data: &LayerData, variable: impl Into<Variable>) -> Vec<f64> {
        let column = data.continuous(variable).unwrap();
        (0..column.len())
            .map(|row| column.get(row).unwrap_or(f64::NAN))
            .collect()
    }

    #[test]
    fn five_bins_match_lets_plot() {
        // Measured on lets-plot: bins=5 on 1..7.5 widen the range by 0.7 x 1.3 per side.
        let data = compute(&[1.0, 2.5, 3.0, 4.2, 4.9, 7.5], r#"{"bins": 5}"#);
        assert_eq!(values(&data, Computed::Count), [1.0, 2.0, 2.0, 0.0, 1.0]);
        let x = values(&data, Aesthetic::X);
        let width = (6.5 + 1.3 * 1.4) / 5.0;
        assert!(
            (x[0] - (1.0 - 1.3 * 0.7 + width / 2.0)).abs() < 1e-12,
            "{x:?}"
        );
        let density: f64 = values(&data, Computed::Density).iter().sum::<f64>() * width;
        assert!((density - 1.0).abs() < 1e-12);
    }

    #[test]
    fn boundary_aligns_a_bin_edge() {
        let data = compute(&[0.3, 1.7, 2.2, 3.9], r#"{"binwidth": 1, "boundary": 0}"#);
        let x = values(&data, Aesthetic::X);
        for center in x {
            let edge = center - 0.5;
            assert!((edge - edge.round()).abs() < 1e-9, "{center}");
        }
    }

    #[test]
    fn breaks_bin_between_them() {
        let data = compute(&[0.0, 1.0, 1.5, 3.0, 9.0], r#"{"breaks": [3, 0, 1]}"#);
        assert_eq!(values(&data, Aesthetic::X), [0.5, 2.0]);
        // 0.0 on the first break: first bin; 1.0 on an inner break: left bin.
        assert_eq!(values(&data, Computed::Count), [2.0, 2.0]);
        assert_eq!(bracketing_index(&[0.0, 1.0, 3.0], 3.0), Some(1));
        assert_eq!(bracketing_index(&[0.0, 1.0, 3.0], 3.5), None);
        assert_eq!(bracketing_index(&[0.0], 0.0), None);
    }

    #[test]
    fn threshold_drops_empty_end_bins() {
        let data = compute(
            &[1.5, 2.5],
            r#"{"breaks": [0, 1, 2, 3, 4], "threshold": 0}"#,
        );
        let count = values(&data, Computed::Count);
        assert!(count[0].is_nan() && count[3].is_nan(), "{count:?}");
        assert_eq!(&count[1..3], [1.0, 1.0]);
        // Two centers remain, so the dropped bins lose theirs.
        let x = values(&data, Aesthetic::X);
        assert!(x[0].is_nan() && x[3].is_nan() && x[1] == 1.5, "{x:?}");
    }
}
