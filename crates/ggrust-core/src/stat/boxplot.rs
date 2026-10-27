//! `stat="boxplot"` and `stat="boxplot_outlier"`: five-number summaries of y per
//! x, as lets-plot's `BoxplotStat` / `BoxplotOutlierStat`.
//!
//! Per group and distinct x (first appearance): `lower`, `middle`, `upper` (the
//! quartiles), `ymin`/`ymax` (whiskers: the most extreme values within `coef` x
//! IQR of the box, default 1.5). `varwidth=True` sets `width` to
//! sqrt(n) / sqrt(largest n). The outlier stat gives one row per value beyond the
//! fences, and a placeholder row (y missing) for an x without outliers, so groups
//! keep their dodge slots next to the boxes.
//!
//! Quartiles interpolate linearly between order statistics, as ggplot2 (R type 7)
//! does; lets-plot averages the two neighbours instead (a deliberate difference).
//! Quantiles are taken in axis units (after a scale transform), as lets-plot.
//!
//! What it does not do: `orientation="y"` (boxes along y).

use std::collections::HashMap;

use super::groups::per_group;
use super::{Stat, StatContext};
use crate::Result;
use crate::data::{Column, Computed, ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::spec::Aesthetic;

/// lets-plot `BoxplotStat.DEF_WHISKER_IQR_RATIO`.
const DEFAULT_COEF: f64 = 1.5;

/// Box statistics per x.
pub struct BoxplotStat;

impl Stat for BoxplotStat {
    fn name(&self) -> &'static str {
        "boxplot"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["coef", "varwidth"]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Y]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        let coef = parameters.number("coef")?.unwrap_or(DEFAULT_COEF);
        let varwidth = parameters.boolean("varwidth")?.unwrap_or(false);
        let mut result = per_group(&data, |rows| {
            let bins = bins(&data, rows);
            let mut columns: [Vec<f64>; 7] = Default::default();
            for (x, values) in &bins {
                let summary = Summary::of(values, coef);
                let row = [
                    *x,
                    summary.lower_whisker,
                    summary.lower,
                    summary.middle,
                    summary.upper,
                    summary.upper_whisker,
                    values.len() as f64,
                ];
                for (column, value) in columns.iter_mut().zip(row) {
                    column.push(value);
                }
            }
            let [x, ymin, lower, middle, upper, ymax, count] = columns;
            Ok(vec![
                (Variable::Aesthetic(Aesthetic::X), x),
                (Variable::Aesthetic(Aesthetic::Ymin), ymin),
                (Variable::Aesthetic(Aesthetic::Lower), lower),
                (Variable::Aesthetic(Aesthetic::Middle), middle),
                (Variable::Aesthetic(Aesthetic::Upper), upper),
                (Variable::Aesthetic(Aesthetic::Ymax), ymax),
                (Variable::Computed(Computed::Count), count),
            ])
        })?;
        if varwidth && let Some(counts) = result.continuous(Computed::Count).cloned() {
            let largest = (0..counts.len())
                .filter_map(|row| counts.get(row))
                .fold(0.0, f64::max);
            let norm = largest.sqrt();
            let widths = counts.map_values(|count| Some(count.sqrt() / norm));
            result.insert(Aesthetic::Width, Column::Continuous(widths))?;
        }
        Ok(result)
    }
}

/// Values beyond the whisker fences, one row each.
pub struct BoxplotOutlierStat;

impl Stat for BoxplotOutlierStat {
    fn name(&self) -> &'static str {
        "boxplot_outlier"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["coef"]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Y]
    }

    fn default_position(&self) -> Option<crate::position::DefaultPosition> {
        // lets-plot `GeomProto`: in step with the boxes' default position.
        Some(crate::position::DefaultPosition::dodge(0.95))
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        let coef = parameters.number("coef")?.unwrap_or(DEFAULT_COEF);
        per_group(&data, |rows| {
            let mut x_column = Vec::new();
            let mut y_column = Vec::new();
            for (x, values) in bins(&data, rows) {
                let summary = Summary::of(&values, coef);
                let outliers: Vec<f64> = values
                    .iter()
                    .copied()
                    .filter(|value| *value < summary.lower_fence || *value > summary.upper_fence)
                    .collect();
                // A placeholder keeps the group (and its dodge slot) when there are none.
                let outliers = if outliers.is_empty() {
                    vec![f64::NAN]
                } else {
                    outliers
                };
                for y in outliers {
                    x_column.push(x);
                    y_column.push(y);
                }
            }
            Ok(vec![
                (Variable::Aesthetic(Aesthetic::X), x_column),
                (Variable::Aesthetic(Aesthetic::Y), y_column),
            ])
        })
    }
}

/// Finite y values of `rows` by finite x (0 without an x), x in first appearance.
fn bins(data: &LayerData, rows: &[usize]) -> Vec<(f64, Vec<f64>)> {
    let empty = ContinuousColumn::from_values(Vec::new());
    let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
    let x = data.continuous(Aesthetic::X);
    let mut bins: Vec<(f64, Vec<f64>)> = Vec::new();
    let mut slot_by_x: HashMap<u64, usize> = HashMap::new();
    for &row in rows {
        let location = match x {
            Some(x) => x.get(row).unwrap_or(f64::NAN),
            None => 0.0,
        };
        let Some(value) = y.get(row) else { continue };
        if !(location.is_finite() && value.is_finite()) {
            continue;
        }
        // Kotlin's boxed doubles compare by bits.
        let slot = *slot_by_x.entry(location.to_bits()).or_insert_with(|| {
            bins.push((location, Vec::new()));
            bins.len() - 1
        });
        bins[slot].1.push(value);
    }
    bins
}

/// Box statistics of one bin.
struct Summary {
    lower: f64,
    middle: f64,
    upper: f64,
    lower_fence: f64,
    upper_fence: f64,
    lower_whisker: f64,
    upper_whisker: f64,
}

impl Summary {
    fn of(values: &[f64], coef: f64) -> Summary {
        let mut sorted = values.to_vec();
        sorted.sort_by(f64::total_cmp);
        let (lower, middle, upper) = (
            quantile(&sorted, 0.25),
            quantile(&sorted, 0.5),
            quantile(&sorted, 0.75),
        );
        let range = upper - lower;
        let (lower_fence, upper_fence) = (lower - range * coef, upper + range * coef);
        // Whiskers: the extremes within the fences (the fences themselves when
        // they are not finite or nothing lies within).
        let (mut lower_whisker, mut upper_whisker) = (lower_fence, upper_fence);
        if lower_fence.is_finite() && upper_fence.is_finite() {
            let inside = sorted
                .iter()
                .filter(|value| (lower_fence..=upper_fence).contains(*value));
            if let Some((low, high)) = inside.fold(None, |range: Option<(f64, f64)>, value| {
                Some(range.map_or((*value, *value), |(low, high)| {
                    (low.min(*value), high.max(*value))
                }))
            }) {
                (lower_whisker, upper_whisker) = (low, high);
            }
        }
        Summary {
            lower,
            middle,
            upper,
            lower_fence,
            upper_fence,
            lower_whisker,
            upper_whisker,
        }
    }
}

/// R type 7 quantile of sorted values: linear between the order statistics around
/// `p * (n - 1)`. NaN when empty.
fn quantile(sorted: &[f64], p: f64) -> f64 {
    let Some(last) = sorted.len().checked_sub(1) else {
        return f64::NAN;
    };
    let place = p * last as f64;
    let (below, above) = (place.floor() as usize, place.ceil() as usize);
    let fraction = place - below as f64;
    sorted[below] + (sorted[above.min(last)] - sorted[below]) * fraction
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Options;

    fn compute(stat: &dyn Stat, x: &[f64], y: &[f64], options: &str) -> LayerData {
        let mut data = LayerData::new(y.len());
        let column =
            |values: &[f64]| Column::Continuous(ContinuousColumn::from_values(values.to_vec()));
        data.insert(Aesthetic::X, column(x)).unwrap();
        data.insert(Aesthetic::Y, column(y)).unwrap();
        let options: Options = serde_json::from_str(options).unwrap();
        stat.compute(
            data,
            &Parameters::new(&options, "layers[0]"),
            &StatContext::default(),
        )
        .unwrap()
    }

    fn values(data: &LayerData, aesthetic: Aesthetic) -> Vec<f64> {
        let column = data.continuous(aesthetic).unwrap();
        (0..column.len())
            .map(|row| column.get(row).unwrap_or(f64::NAN))
            .collect()
    }

    #[test]
    fn quartiles_are_type_7() {
        assert_eq!(quantile(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 0.25), 2.25);
        assert_eq!(quantile(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 0.5), 3.5);
        assert_eq!(quantile(&[7.0], 0.75), 7.0);
        assert!(quantile(&[], 0.5).is_nan());
    }

    #[test]
    fn boxes_and_outliers_per_x() {
        let x = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let y = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 20.0, 2.0, 3.0, 4.0];
        let boxes = compute(&BoxplotStat, &x, &y, "{}");
        assert_eq!(values(&boxes, Aesthetic::X), [0.0, 1.0]);
        assert_eq!(values(&boxes, Aesthetic::Lower), [2.5, 2.5]);
        assert_eq!(values(&boxes, Aesthetic::Middle), [4.0, 3.0]);
        // Fences 2.5 - 4.5 and 5.5 + 4.5: whiskers 1 and 6; 20 is an outlier.
        assert_eq!(values(&boxes, Aesthetic::Ymin), [1.0, 2.0]);
        assert_eq!(values(&boxes, Aesthetic::Ymax), [6.0, 4.0]);
        assert!(!boxes.contains(Aesthetic::Width));
        let outliers = compute(&BoxplotOutlierStat, &x, &y, "{}");
        let y = values(&outliers, Aesthetic::Y);
        assert_eq!(values(&outliers, Aesthetic::X), [0.0, 1.0]);
        assert!(y[0] == 20.0 && y[1].is_nan());
    }

    #[test]
    fn coef_and_varwidth() {
        let x = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        let y = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 9.0, 1.0];
        let boxes = compute(&BoxplotStat, &x, &y, r#"{"coef": 0.5, "varwidth": true}"#);
        assert_eq!(values(&boxes, Aesthetic::Ymax)[0], 6.0);
        let width = values(&boxes, Aesthetic::Width);
        assert!((width[0] - 1.0).abs() < 1e-12 && (width[1] - 1.0 / 7_f64.sqrt()).abs() < 1e-12);
        let outliers = compute(&BoxplotOutlierStat, &x, &y, r#"{"coef": 0.5}"#);
        assert_eq!(values(&outliers, Aesthetic::Y)[0], 9.0);
    }
}
