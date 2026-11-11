//! `stat="summary2d"`: `z` summarised over a grid of equal rectangles, as
//! ggplot2's `stat_summary_2d` (lets-plot has none).
//!
//! The grid is `stat="bin2d"`'s (lets-plot rules: `bins`, `binwidth`, the range
//! widened by half a bin). Per group and bin, `..value..` is `fun` (`mean` by
//! default; `count`, `sum`, `median`, `min`, `max`) of the bin's finite `z`; fill
//! is mapped to it. `drop` (default true) leaves out bins without values; with
//! `drop=False` they are reported with a missing value.
//!
//! What it does not do: Python callables for `fun`, `breaks`.

use super::bin2d::{Bin2dOptions, rectangle_axis};
use super::groups::per_group;
use super::summary::{Aggregate, PLAIN_FUNCTION_NAMES};
use super::{Stat, StatContext};
use crate::data::{Computed, ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::spec::Aesthetic;
use crate::{Error, Result};

/// `z` summarised per 2D bin.
pub struct Summary2dStat;

impl Stat for Summary2dStat {
    fn name(&self) -> &'static str {
        "summary2d"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["bins", "binwidth", "drop", "fun"]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Z]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Z]
    }

    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        &[(Aesthetic::Fill, Computed::Value)]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        context: &StatContext,
    ) -> Result<LayerData> {
        let options = Bin2dOptions::from_parameters(parameters, 1.0)?;
        let aggregate = match parameters.string("fun")? {
            None => Aggregate::Mean,
            Some(name) => Aggregate::named(name).ok_or_else(|| {
                Error::spec(
                    parameters.path_of("fun"),
                    format!(
                        "unknown summary function '{name}'; expected one of {}{}",
                        PLAIN_FUNCTION_NAMES.join(", "),
                        crate::suggest::did_you_mean(name, PLAIN_FUNCTION_NAMES)
                    ),
                )
            })?,
        };
        let empty = ContinuousColumn::from_values(Vec::new());
        let x = data.continuous(Aesthetic::X).unwrap_or(&empty);
        let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
        let z = data.continuous(Aesthetic::Z).unwrap_or(&empty);
        let ranges = context
            .x_range
            .or_else(|| super::bin::finite_range(x))
            .zip(context.y_range.or_else(|| super::bin::finite_range(y)));
        per_group(&data, |rows| {
            let Some((x_range, y_range)) = ranges.filter(|_| data.row_count() > 0) else {
                return Ok(Vec::new());
            };
            let x_axis = rectangle_axis(x_range, options.x);
            let y_axis = rectangle_axis(y_range, options.y);
            let mut cells: std::collections::BTreeMap<(usize, usize), Vec<f64>> =
                std::collections::BTreeMap::new();
            for &row in rows {
                let (Some(x_value), Some(y_value), Some(z_value)) =
                    (x.get(row), y.get(row), z.get(row))
                else {
                    continue;
                };
                if !(x_value.is_finite() && y_value.is_finite() && z_value.is_finite()) {
                    continue;
                }
                let column = ((x_value - x_axis.start) / x_axis.width).floor();
                let grid_row = ((y_value - y_axis.start) / y_axis.width).floor();
                if column >= 0.0
                    && grid_row >= 0.0
                    && column < x_axis.count as f64
                    && grid_row < y_axis.count as f64
                {
                    cells
                        .entry((column as usize, grid_row as usize))
                        .or_default()
                        .push(z_value);
                }
            }
            if !options.drop {
                for column in 0..x_axis.count {
                    for grid_row in 0..y_axis.count {
                        cells.entry((column, grid_row)).or_default();
                    }
                }
            }
            let (mut xs, mut ys, mut values) = (Vec::new(), Vec::new(), Vec::new());
            for ((column, grid_row), mut bin) in cells {
                bin.sort_by(f64::total_cmp);
                xs.push(x_axis.start + (column as f64 + 0.5) * x_axis.width);
                ys.push(y_axis.start + (grid_row as f64 + 0.5) * y_axis.width);
                values.push(if bin.is_empty() {
                    f64::NAN
                } else {
                    aggregate.apply(&bin)
                });
            }
            let n = xs.len();
            Ok(vec![
                (Variable::Aesthetic(Aesthetic::X), xs),
                (Variable::Aesthetic(Aesthetic::Y), ys),
                (Variable::Computed(Computed::Value), values),
                (
                    Variable::Computed(Computed::BinWidth),
                    vec![x_axis.width; n],
                ),
                (
                    Variable::Computed(Computed::BinHeight),
                    vec![y_axis.width; n],
                ),
            ])
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Column;
    use crate::spec::Options;

    #[test]
    fn mean_and_max_per_bin() {
        let mut data = LayerData::new(4);
        let column =
            |values: &[f64]| Column::Continuous(ContinuousColumn::from_values(values.to_vec()));
        data.insert(Aesthetic::X, column(&[0.0, 0.1, 1.0, 1.0]))
            .unwrap();
        data.insert(Aesthetic::Y, column(&[0.0, 0.1, 1.0, 1.0]))
            .unwrap();
        data.insert(Aesthetic::Z, column(&[1.0, 3.0, 10.0, 20.0]))
            .unwrap();
        let run = |options: &str| {
            let options: Options = serde_json::from_str(options).unwrap();
            let result = Summary2dStat
                .compute(
                    data.clone(),
                    &Parameters::new(&options, ""),
                    &StatContext::default(),
                )
                .unwrap();
            let values = result.continuous(Computed::Value).unwrap();
            (0..values.len())
                .map(|row| values.get(row).unwrap())
                .collect::<Vec<f64>>()
        };
        assert_eq!(run(r#"{"bins": 2}"#), [2.0, 15.0]);
        assert_eq!(run(r#"{"bins": 2, "fun": "max"}"#), [3.0, 20.0]);
        assert_eq!(run(r#"{"bins": 2, "drop": false}"#).len(), 4);
    }
}
