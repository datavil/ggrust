//! `stat="centroid"` and `"summary_xy"`: one row per group at a summary of its x
//! and y (ggpp `stat_centroid`, `stat_summary_xy`), e.g. a cluster's name at the
//! cluster's centre with `geom_text(stat="centroid")`.
//!
//! Per group (and facet panel): x and y summarised over the rows where both are
//! finite, by `fun` (`centroid`, both axes) or `fun_x` / `fun_y` (`summary_xy`):
//! `"mean"` (default) or `"median"` (R's: the mean of the two middle values for an
//! even count). A group without such rows gives no row. Other columns follow the
//! shared per-group rule (`groups.rs`: mean, or first value of a discrete column).
//!
//! What it does not do: ggpp's arbitrary R functions and their `.fun.args`, the
//! `mean_se` default's xmin/xmax/ymin/ymax, or setting a non-constant label to NA.

use super::groups::per_group;
use super::{Stat, StatContext};
use crate::data::{ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::spec::Aesthetic;
use crate::{Error, Result};

/// `stat="centroid"`: one summary function for x and y.
pub struct CentroidStat;

/// `stat="summary_xy"`: a summary function per axis.
pub struct SummaryXyStat;

impl Stat for CentroidStat {
    fn name(&self) -> &'static str {
        "centroid"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["fun"]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        let fun = Summary::from_parameter(parameters, "fun")?;
        compute(&data, fun, fun)
    }
}

impl Stat for SummaryXyStat {
    fn name(&self) -> &'static str {
        "summary_xy"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["fun_x", "fun_y"]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        CentroidStat.aesthetics()
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        CentroidStat.required_aesthetics()
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        let fun_x = Summary::from_parameter(parameters, "fun_x")?;
        let fun_y = Summary::from_parameter(parameters, "fun_y")?;
        compute(&data, fun_x, fun_y)
    }
}

/// A summary of one axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Summary {
    Mean,
    Median,
}

impl Summary {
    const NAMES: [&'static str; 2] = ["mean", "median"];

    fn from_parameter(parameters: &Parameters, name: &str) -> Result<Self> {
        let Some(value) = parameters.value(name) else {
            return Ok(Summary::Mean);
        };
        match value.as_str() {
            Some("mean") => Ok(Summary::Mean),
            Some("median") => Ok(Summary::Median),
            other => Err(Error::spec(
                parameters.path_of(name),
                format!(
                    "expected 'mean' or 'median', got {value}{}",
                    suggestion(other)
                ),
            )),
        }
    }

    /// The summary of `values` (finite, not empty); reorders them.
    fn of(self, values: &mut [f64]) -> f64 {
        match self {
            Summary::Mean => values.iter().sum::<f64>() / values.len() as f64,
            Summary::Median => {
                values.sort_by(f64::total_cmp);
                let middle = values.len() / 2;
                if values.len() % 2 == 1 {
                    values[middle]
                } else {
                    (values[middle - 1] + values[middle]) / 2.0
                }
            }
        }
    }
}

fn suggestion(word: Option<&str>) -> String {
    word.map(|word| crate::suggest::did_you_mean(word, Summary::NAMES))
        .unwrap_or_default()
}

fn compute(data: &LayerData, fun_x: Summary, fun_y: Summary) -> Result<LayerData> {
    let empty = ContinuousColumn::from_values(Vec::new());
    let x = data.continuous(Aesthetic::X).unwrap_or(&empty);
    let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
    per_group(data, |rows| {
        let (mut xs, mut ys): (Vec<f64>, Vec<f64>) = rows
            .iter()
            .filter_map(|&row| {
                let (x, y) = (x.get(row)?, y.get(row)?);
                (x.is_finite() && y.is_finite()).then_some((x, y))
            })
            .unzip();
        let (x, y) = if xs.is_empty() {
            (Vec::new(), Vec::new())
        } else {
            (vec![fun_x.of(&mut xs)], vec![fun_y.of(&mut ys)])
        };
        Ok(vec![
            (Variable::Aesthetic(Aesthetic::X), x),
            (Variable::Aesthetic(Aesthetic::Y), y),
        ])
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Column;
    use crate::spec::Options;

    fn column(values: &[f64]) -> Column {
        Column::Continuous(ContinuousColumn::from_values(values.to_vec()))
    }

    fn values(data: &LayerData, aesthetic: Aesthetic) -> Vec<f64> {
        let column = data.continuous(aesthetic).unwrap();
        (0..column.len())
            .map(|row| column.get(row).unwrap())
            .collect()
    }

    fn run(stat: &dyn Stat, options: &str) -> Result<LayerData> {
        // ggpp's check: groups a = (1, 1), (2, 1), (3, 4); b = (10, 5), (11, 7),
        // plus a row with a missing y that is left out.
        let mut data = LayerData::new(6);
        data.insert(Aesthetic::X, column(&[1.0, 2.0, 3.0, 10.0, 11.0, 5.0]))
            .unwrap();
        data.insert(Aesthetic::Y, column(&[1.0, 1.0, 4.0, 5.0, 7.0, f64::NAN]))
            .unwrap();
        data.set_group_index(Some(vec![0, 0, 0, 1, 1, 0])).unwrap();
        let options: Options = serde_json::from_str(options).unwrap();
        stat.compute(
            data,
            &Parameters::new(&options, "layers[0]"),
            &StatContext::default(),
        )
    }

    #[test]
    fn centroid_is_one_row_per_group_as_ggpp() {
        let mean = run(&CentroidStat, "{}").unwrap();
        assert_eq!(values(&mean, Aesthetic::X), vec![2.0, 10.5]);
        assert_eq!(values(&mean, Aesthetic::Y), vec![2.0, 6.0]);
        let median = run(&CentroidStat, r#"{"fun": "median"}"#).unwrap();
        assert_eq!(values(&median, Aesthetic::X), vec![2.0, 10.5]);
        assert_eq!(values(&median, Aesthetic::Y), vec![1.0, 6.0]);
    }

    #[test]
    fn summary_xy_summarises_each_axis_on_its_own() {
        let data = run(&SummaryXyStat, r#"{"fun_x": "median", "fun_y": "mean"}"#).unwrap();
        assert_eq!(values(&data, Aesthetic::X), vec![2.0, 10.5]);
        assert_eq!(values(&data, Aesthetic::Y), vec![2.0, 6.0]);
        let data = run(&SummaryXyStat, r#"{"fun_y": "median"}"#).unwrap();
        assert_eq!(values(&data, Aesthetic::Y), vec![1.0, 6.0]);
    }

    #[test]
    fn median_follows_r() {
        assert_eq!(Summary::Median.of(&mut [4.0, 1.0, 3.0, 2.0]), 2.5);
        assert_eq!(Summary::Median.of(&mut [5.0]), 5.0);
    }

    #[test]
    fn bad_functions_name_the_parameter() {
        let error = run(&CentroidStat, r#"{"fun": "medain"}"#)
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("layers[0].fun"), "{error}");
        assert!(error.contains("did you mean 'median'?"), "{error}");
        let error = run(&SummaryXyStat, r#"{"fun_y": 2}"#)
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("layers[0].fun_y"), "{error}");
        assert!(error.contains("'mean' or 'median'"), "{error}");
    }
}
