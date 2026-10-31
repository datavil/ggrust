//! `stat="sina"`: every data point with the violin density at its y, as
//! lets-plot's `SinaStat`.
//!
//! Per group and distinct x, the violin density grid of [`super::ydensity`] is
//! computed; each finite point (sorted by y within its x) then gets `..density..`,
//! `..count..` and `..scaled..` interpolated linearly on the grid and the
//! `..quantile..` band of the next grid point at or above it. `..violinwidth..`
//! follows `scale` (`area`, `count`, `width`), normalised by the grid of the whole
//! layer. Output rows keep their input row, so per-point aesthetics (colour,
//! size, tooltips) stay with their point.
//!
//! What it does not do: place points; `geom_sina` spreads them across the width.

use std::collections::HashMap;

use super::density::{DensityOptions, estimate_bins};
use super::groups::rows_by_group;
use super::ydensity::{ViolinScale, quantiles};
use super::{Stat, StatContext};
use crate::Result;
use crate::data::{Column, Computed, ContinuousColumn, LayerData, SourceRows, Variable};
use crate::parameters::Parameters;
use crate::spec::Aesthetic;

/// Violin densities at each data point.
pub struct SinaStat;

impl Stat for SinaStat {
    fn name(&self) -> &'static str {
        "sina"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &[
            "scale",
            "trim",
            "tails_cutoff",
            "bw",
            "adjust",
            "kernel",
            "n",
            "fs_max",
            "quantiles",
        ]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Weight]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Y]
    }

    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        &[
            (Aesthetic::Violinwidth, Computed::ViolinWidth),
            (Aesthetic::Quantile, Computed::Quantile),
        ]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        context: &StatContext,
    ) -> Result<LayerData> {
        let scale = ViolinScale::from_parameters(parameters)?;
        let options = DensityOptions::from_parameters(parameters, true, Some(3.0))?;
        let quantiles = quantiles(parameters)?;
        let overall = context.y_range.unwrap_or((-0.5, 0.5));
        let empty = ContinuousColumn::from_values(Vec::new());
        let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
        let x = data.continuous(Aesthetic::X);
        let weight = data.continuous(Aesthetic::Weight);

        let mut points = Points::default();
        // The grid of every violin, for the layer-wide width normalisation.
        let mut grid_density: Vec<f64> = Vec::new();
        let mut grid_count: Vec<f64> = Vec::new();
        let mut group_index: Vec<u32> = Vec::new();
        for (group, rows) in rows_by_group(&data).into_iter().enumerate() {
            for (location, mut bin) in bins(&rows, x, y, weight) {
                bin.sort_by(|left, right| left.0.total_cmp(&right.0));
                let pairs = bin
                    .iter()
                    .map(|&(value, weight, _)| (value, weight))
                    .collect();
                let [_, grid, density, count, scaled, bands] =
                    estimate_bins(vec![(location, pairs)], &options, &quantiles, overall);
                for &(value, _, row) in &bin {
                    points.x.push(location);
                    points.y.push(value);
                    points.density.push(linear(&grid, &density, value));
                    points.count.push(linear(&grid, &count, value));
                    points.scaled.push(linear(&grid, &scaled, value));
                    points.quantile.push(ceiling(&grid, &bands, value));
                    points.rows.push(row as u32);
                    group_index.push(group as u32);
                }
                grid_density.extend(density);
                grid_count.extend(count);
            }
        }
        let widths = violin_widths(scale, &points, &grid_density, &grid_count);

        let mut result = LayerData::new(points.rows.len());
        for aesthetic in data.aesthetics().collect::<Vec<_>>() {
            if let Some(column) = data.get(aesthetic) {
                result.insert(aesthetic, column.take(&points.rows))?;
            }
        }
        let continuous =
            |values: Vec<f64>| Column::Continuous(ContinuousColumn::from_values(values));
        let columns: [(Variable, Vec<f64>); 7] = [
            (Variable::Aesthetic(Aesthetic::X), points.x),
            (Variable::Aesthetic(Aesthetic::Y), points.y),
            (Variable::Computed(Computed::Density), points.density),
            (Variable::Computed(Computed::Count), points.count),
            (Variable::Computed(Computed::Scaled), points.scaled),
            (Variable::Computed(Computed::Quantile), points.quantile),
            (Variable::Computed(Computed::ViolinWidth), widths),
        ];
        for (variable, values) in columns {
            result.insert(variable, continuous(values))?;
        }
        result.set_group_index(Some(group_index))?;
        let sources = points
            .rows
            .iter()
            .map(|&row| {
                data.source_row(row as usize)
                    .map_or(row, |source| source as u32)
            })
            .collect();
        result.set_source_rows(SourceRows::Selected(sources))?;
        Ok(result)
    }
}

/// Per-point stat values, in output order.
#[derive(Default)]
struct Points {
    x: Vec<f64>,
    y: Vec<f64>,
    density: Vec<f64>,
    count: Vec<f64>,
    scaled: Vec<f64>,
    quantile: Vec<f64>,
    rows: Vec<u32>,
}

/// One x and its finite (value, weight, row) triples.
type Bin = (f64, Vec<(f64, f64, usize)>);

/// Finite (value, weight, row) triples of `rows` per distinct x, in order of
/// first appearance (lets-plot `handleBinnedData`).
fn bins(
    rows: &[usize],
    x: Option<&ContinuousColumn>,
    y: &ContinuousColumn,
    weight: Option<&ContinuousColumn>,
) -> Vec<Bin> {
    let mut bins: Vec<Bin> = Vec::new();
    let mut slot_of_bin: HashMap<u64, usize> = HashMap::new();
    for &row in rows {
        let location = x.map_or(Some(0.0), |x| x.get(row));
        let weight = weight.map_or(Some(1.0), |weight| weight.get(row));
        let (Some(location), Some(value), Some(weight)) = (location, y.get(row), weight) else {
            continue;
        };
        if !(location.is_finite() && value.is_finite() && weight.is_finite()) {
            continue;
        }
        let slot = *slot_of_bin.entry(location.to_bits()).or_insert_with(|| {
            bins.push((location, Vec::new()));
            bins.len() - 1
        });
        bins[slot].1.push((value, weight, row));
    }
    bins
}

/// lets-plot `pwLinInterp`: linear between the grid points around `t`, the end
/// values outside the grid.
fn linear(grid: &[f64], values: &[f64], t: f64) -> f64 {
    match grid.iter().position(|point| *point >= t) {
        None => values.last().copied().unwrap_or(f64::NAN),
        Some(0) => values[0],
        Some(index) => {
            let slope = (values[index] - values[index - 1]) / (grid[index] - grid[index - 1]);
            values[index - 1] + slope * (t - grid[index - 1])
        }
    }
}

/// lets-plot `SinaStat.pwCeilInterp`: the value of the first grid point at or
/// above `t`.
fn ceiling(grid: &[f64], values: &[f64], t: f64) -> f64 {
    match grid.iter().position(|point| *point >= t) {
        None => values.last().copied().unwrap_or(f64::NAN),
        Some(index) => values[index],
    }
}

/// lets-plot `SinaStat.normalize`: point widths, normalised by the layer's grid.
fn violin_widths(
    scale: ViolinScale,
    points: &Points,
    grid_density: &[f64],
    grid_count: &[f64],
) -> Vec<f64> {
    if grid_density.is_empty() {
        return vec![0.0; points.rows.len()];
    }
    let largest = grid_density
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    match scale {
        ViolinScale::Area => points.density.iter().map(|value| value / largest).collect(),
        ViolinScale::Width => points.scaled.clone(),
        ViolinScale::Count => {
            let weight = grid_count
                .iter()
                .zip(grid_density)
                .filter(|(_, density)| **density > 0.0)
                .map(|(count, density)| count / density)
                .fold(f64::NEG_INFINITY, f64::max);
            points
                .count
                .iter()
                .map(|value| value / (largest * weight))
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Options;

    #[test]
    fn interpolation_is_linear_inside_and_flat_outside() {
        let grid = [0.0, 1.0, 1.0, 2.0];
        let values = [0.0, 2.0, 3.0, 5.0];
        assert_eq!(linear(&grid, &values, -1.0), 0.0);
        assert_eq!(linear(&grid, &values, 0.5), 1.0);
        assert_eq!(linear(&grid, &values, 1.5), 4.0);
        assert_eq!(linear(&grid, &values, 3.0), 5.0);
        assert_eq!(ceiling(&grid, &values, 0.5), 2.0);
        assert_eq!(ceiling(&grid, &values, 9.0), 5.0);
    }

    #[test]
    fn points_keep_their_rows_sorted_by_y_within_x() {
        let mut data = LayerData::new(4);
        let column =
            |values: &[f64]| Column::Continuous(ContinuousColumn::from_values(values.to_vec()));
        data.insert(Aesthetic::X, column(&[0.0, 0.0, 0.0, 1.0]))
            .unwrap();
        data.insert(Aesthetic::Y, column(&[3.0, 1.0, 2.0, 5.0]))
            .unwrap();
        let options = Options::new();
        let result = SinaStat
            .compute(
                data,
                &Parameters::new(&options, ""),
                &StatContext::default(),
            )
            .unwrap();
        assert_eq!(
            result.continuous(Aesthetic::Y).unwrap().values(),
            [1.0, 2.0, 3.0, 5.0]
        );
        assert_eq!(
            result.source_rows(),
            &SourceRows::Selected(vec![1, 2, 0, 3])
        );
        let widths = result.continuous(Computed::ViolinWidth).unwrap();
        assert!(
            widths
                .values()
                .iter()
                .all(|width| (0.0..=1.0).contains(width))
        );
    }
}
