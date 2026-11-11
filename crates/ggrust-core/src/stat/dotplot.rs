//! `stat="dotplot"` and `stat="ydotplot"`: the stacks of a dot plot (lets-plot
//! `DotplotStat`, `YDotplotStat`, `BinStatUtil`). `method="dotdensity"` (default) grows
//! a stack from the smallest value until a value is a bin width away and centres it
//! on its values; `method="histodot"` uses the equal bins of `stat="bin"`.
//!
//! `ydotplot` bins y separately at each x. Computed: `..count..`, `..density..`,
//! `..binwidth..`.

use super::bin::{BinOptions, binning_parameters, equal_bins, finite_range};
use super::count::row_weight;
use super::groups::{GroupColumns, per_group};
use super::{Stat, StatContext};
use crate::data::{Computed, ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::spec::Aesthetic;
use crate::{Error, Result};

const OPTIONS: &[&str] = &["bins", "binwidth", "center", "boundary", "method"];
const MAPPING: &[(Aesthetic, Computed)] = &[
    (Aesthetic::Stacksize, Computed::Count),
    (Aesthetic::Binwidth, Computed::BinWidth),
];

/// Whether `method` asks for equal histogram bins.
fn histodot(parameters: &Parameters) -> Result<bool> {
    match parameters
        .string("method")?
        .map(str::to_lowercase)
        .as_deref()
    {
        None | Some("dotdensity") => Ok(false),
        Some("histodot") => Ok(true),
        Some(other) => Err(Error::spec(
            parameters.path_of("method"),
            format!("expected 'dotdensity' or 'histodot', got '{other}'"),
        )),
    }
}

/// The stacks of one run of values.
#[derive(Default, Debug, PartialEq)]
struct Stacks {
    position: Vec<f64>,
    count: Vec<f64>,
    density: Vec<f64>,
    bin_width: Vec<f64>,
}

impl Stacks {
    fn extend(&mut self, other: Stacks) {
        self.position.extend(other.position);
        self.count.extend(other.count);
        self.density.extend(other.density);
        self.bin_width.extend(other.bin_width);
    }

    fn computed(self) -> Vec<(Variable, Vec<f64>)> {
        vec![
            (Variable::Computed(Computed::Count), self.count),
            (Variable::Computed(Computed::Density), self.density),
            (Variable::Computed(Computed::BinWidth), self.bin_width),
        ]
    }
}

/// lets-plot `computeDotdensityBins`.
fn dot_density(values: &[f64], width: f64) -> Stacks {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let total = sorted.len() as f64;
    let mut stacks = Stacks::default();
    let mut start = 0;
    for index in 1..=sorted.len() {
        if index < sorted.len() && sorted[index] - sorted[start] < width {
            continue;
        }
        let size = (index - start) as f64;
        stacks
            .position
            .push(sorted[start] + (sorted[index - 1] - sorted[start]) / 2.0);
        stacks.count.push(size);
        stacks.density.push(size / total);
        stacks.bin_width.push(width);
        start = index;
    }
    stacks
}

/// The stacks of weighted, finite `values` over the overall `range`.
fn stacks(
    values: &[(f64, f64)],
    range: (f64, f64),
    options: &BinOptions,
    histogram: bool,
) -> Stacks {
    if histogram {
        let (count, width, start) = binning_parameters(range, options);
        let bins = equal_bins(values, start, count, width);
        return Stacks {
            bin_width: vec![width; bins.x.len()],
            position: bins.x,
            count: bins.count,
            density: bins.density,
        };
    }
    let span = range.1 - range.0;
    let width = if span > 0.0 {
        options.count_and_width(span).1
    } else {
        // One stack over all the data.
        1.0
    };
    let plain: Vec<f64> = values.iter().map(|&(value, _)| value).collect();
    dot_density(&plain, width)
}

/// Stacks of x.
pub struct DotplotStat;

impl Stat for DotplotStat {
    fn name(&self) -> &'static str {
        "dotplot"
    }

    fn parameters(&self) -> &'static [&'static str] {
        OPTIONS
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Weight]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X]
    }

    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        MAPPING
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        context: &StatContext,
    ) -> Result<LayerData> {
        let options = BinOptions::from_parameters(parameters)?;
        let histogram = histodot(parameters)?;
        let empty = ContinuousColumn::from_values(Vec::new());
        let x = data.continuous(Aesthetic::X).unwrap_or(&empty);
        let weight = data.continuous(Aesthetic::Weight);
        let range = context.x_range.or_else(|| finite_range(x));
        per_group(&data, |rows| {
            let values: Vec<(f64, f64)> = rows
                .iter()
                .filter_map(|&row| {
                    let value = x.get(row).filter(|value| value.is_finite())?;
                    Some((value, row_weight(weight, row)))
                })
                .collect();
            let mut found = match range {
                Some(range) if data.row_count() > 0 => stacks(&values, range, &options, histogram),
                _ => Stacks::default(),
            };
            let position = std::mem::take(&mut found.position);
            let mut columns: GroupColumns = vec![(Variable::Aesthetic(Aesthetic::X), position)];
            columns.extend(found.computed());
            Ok(columns)
        })
    }
}

/// Stacks of y at each x.
pub struct YDotplotStat;

impl Stat for YDotplotStat {
    fn name(&self) -> &'static str {
        "ydotplot"
    }

    fn parameters(&self) -> &'static [&'static str] {
        OPTIONS
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Weight]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Y]
    }

    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        MAPPING
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        context: &StatContext,
    ) -> Result<LayerData> {
        let options = BinOptions::from_parameters(parameters)?;
        let histogram = histodot(parameters)?;
        let empty = ContinuousColumn::from_values(Vec::new());
        let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
        let x = data.continuous(Aesthetic::X);
        let weight = data.continuous(Aesthetic::Weight);
        let range = context.y_range.or_else(|| finite_range(y));
        per_group(&data, |rows| {
            // Each x gets its own stacks, in order of first appearance.
            let mut by_x: Vec<(f64, Vec<(f64, f64)>)> = Vec::new();
            for &row in rows {
                let at = x.map_or(Some(0.0), |x| x.get(row));
                let (Some(at), Some(value)) = (at.filter(|at| at.is_finite()), y.get(row)) else {
                    continue;
                };
                if !value.is_finite() {
                    continue;
                }
                let entry = (value, row_weight(weight, row));
                match by_x.iter_mut().find(|(known, _)| *known == at) {
                    Some((_, values)) => values.push(entry),
                    None => by_x.push((at, vec![entry])),
                }
            }
            let mut xs = Vec::new();
            let mut all = Stacks::default();
            if let Some(range) = range.filter(|_| data.row_count() > 0) {
                for (at, values) in &by_x {
                    let found = stacks(values, range, &options, histogram);
                    xs.extend(std::iter::repeat_n(*at, found.position.len()));
                    all.extend(found);
                }
            }
            let position = std::mem::take(&mut all.position);
            let mut columns: GroupColumns = vec![
                (Variable::Aesthetic(Aesthetic::X), xs),
                (Variable::Aesthetic(Aesthetic::Y), position),
            ];
            columns.extend(all.computed());
            Ok(columns)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stack_grows_until_a_value_is_a_bin_away() {
        let stacks = dot_density(&[1.0, 1.4, 1.5, 3.0], 0.5);
        assert_eq!(stacks.count, [2.0, 1.0, 1.0]);
        assert_eq!(stacks.position, [1.2, 1.5, 3.0]);
        assert_eq!(stacks.density, [0.5, 0.25, 0.25]);
        assert_eq!(stacks.bin_width, [0.5; 3]);
    }

    #[test]
    fn no_values_have_no_stacks() {
        assert_eq!(dot_density(&[], 1.0), Stacks::default());
    }
}
