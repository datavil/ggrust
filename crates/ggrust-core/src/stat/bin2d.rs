//! `stat="bin2d"`: counts of (x, y) in a grid of equal rectangles, as lets-plot's
//! `Bin2dStat` (`geom_bin2d`).
//!
//! The grid spans every layer's x and y (scale limits replace the ends), widened
//! by half a bin on each side, and is filled per group. Options: `bins` (default
//! `[30, 30]`; one number is used for both axes, as ggplot2), `binwidth` (per
//! axis, wins over `bins`), `drop` (default true: empty bins are left out).
//!
//! Computed: `..count..` (fill by default), `..density..` (integrates to 1 over
//! the grid), and the bin sizes `..binwidth..` / `..binheight..`, which size the
//! tiles. lets-plot sizes them from the data resolution instead, which is wrong
//! with one bin along an axis (#1290; deviation).
//!
//! What it does not do: bins on a discrete axis (the codes are binned as numbers).

use super::bin::MAX_BIN_COUNT;
use super::count::row_weight;
use super::groups::per_group;
use super::{Stat, StatContext};
use crate::data::{Computed, ContinuousColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::scale::breaks::is_degenerate_range;
use crate::spec::{Aesthetic, Value};
use crate::{Error, Result};

/// lets-plot `Bin2dStat.DEF_BINS` (and `BinHexStat.DEF_BINS`).
const DEFAULT_BIN_COUNT: usize = 30;

/// Grids with more cells than this count into a hash map instead of a dense array.
const DENSE_CELL_LIMIT: usize = 1 << 22;

/// Counts (x, y) per rectangle.
pub struct Bin2dStat;

impl Stat for Bin2dStat {
    fn name(&self) -> &'static str {
        "bin2d"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["bins", "binwidth", "drop"]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Weight]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        &[(Aesthetic::Fill, Computed::Count)]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        context: &StatContext,
    ) -> Result<LayerData> {
        let options = Bin2dOptions::from_parameters(parameters, 1.0)?;
        let empty = ContinuousColumn::from_values(Vec::new());
        let x = data.continuous(Aesthetic::X).unwrap_or(&empty);
        let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
        let weight = data.continuous(Aesthetic::Weight);
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
            Ok(count_rectangles(
                rows,
                x,
                y,
                weight,
                &x_axis,
                &y_axis,
                options.drop,
            ))
        })
    }
}

/// Bin options of one axis: lets-plot `BinStatUtil.BinOptions`.
#[derive(Clone, Copy, Debug)]
pub(super) struct AxisOptions {
    count: usize,
    /// Positive, or `None`.
    width: Option<f64>,
}

impl AxisOptions {
    /// lets-plot `binCountAndWidth` with a `count -> width` rule.
    pub(super) fn count_and_width(
        self,
        span: f64,
        count_to_width: impl Fn(f64, usize) -> f64,
    ) -> (usize, f64) {
        match self.width {
            Some(width) => {
                let count = (span / width).min(MAX_BIN_COUNT as f64).ceil();
                (count.max(0.0) as usize, width)
            }
            None => (self.count, count_to_width(span, self.count)),
        }
    }

    pub(super) fn has_width(self) -> bool {
        self.width.is_some()
    }
}

/// `bins`, `binwidth` and `drop` of a 2D bin stat.
#[derive(Clone, Copy, Debug)]
pub(super) struct Bin2dOptions {
    pub(super) x: AxisOptions,
    pub(super) y: AxisOptions,
    pub(super) drop: bool,
}

impl Bin2dOptions {
    /// Read the options; the y bin width is multiplied by `y_width_factor` (hexagons
    /// space their rows closer than their height).
    pub(super) fn from_parameters(parameters: &Parameters, y_width_factor: f64) -> Result<Self> {
        let (x_count, y_count) = number_pair(parameters, "bins")?;
        let count = |value: Option<f64>| -> Result<usize> {
            let value = value.unwrap_or(DEFAULT_BIN_COUNT as f64);
            // lets-plot truncates to an integer; fewer than one bin cannot bin.
            if value.trunc() < 1.0 {
                return Err(Error::spec(
                    parameters.path_of("bins"),
                    format!(
                        "bins must be at least 1, got {value}; \
                         pass e.g. bins=[30, 30] (x, y), or one number for both axes"
                    ),
                ));
            }
            Ok(value.trunc().min(u32::MAX as f64) as usize)
        };
        let (x_width, y_width) = number_pair(parameters, "binwidth")?;
        let positive = |width: Option<f64>| width.filter(|width| *width > 0.0);
        Ok(Bin2dOptions {
            x: AxisOptions {
                count: count(x_count)?,
                width: positive(x_width),
            },
            y: AxisOptions {
                count: count(y_count)?,
                width: positive(y_width).map(|width| width * y_width_factor),
            },
            drop: parameters.boolean("drop")?.unwrap_or(true),
        })
    }
}

/// An `[x, y]` option: a list of two numbers (or nulls), or one number for both.
pub(super) fn number_pair(
    parameters: &Parameters,
    name: &str,
) -> Result<(Option<f64>, Option<f64>)> {
    let invalid = |value: &Value| {
        Error::spec(
            parameters.path_of(name),
            format!(
                "{name} must be a number or a list of two numbers [x, y], got {value}; \
                 pass e.g. {name}=[30, 30]"
            ),
        )
    };
    let number = |value: &Value| -> Result<Option<f64>> {
        match value {
            Value::Null => Ok(None),
            value => match value.as_f64() {
                Some(number) if number.is_finite() => Ok(Some(number)),
                _ => Err(invalid(value)),
            },
        }
    };
    match parameters.value(name) {
        None => Ok((None, None)),
        Some(Value::Array(items)) => match items.as_slice() {
            [x, y] => Ok((number(x)?, number(y)?)),
            _ => Err(invalid(&Value::Array(items.clone()))),
        },
        Some(value) => {
            let both = number(value)?;
            Ok((both, both))
        }
    }
}

/// lets-plot `ensureApplicableRange`: a zero-width range becomes 1 wide.
pub(super) fn applicable_range((low, high): (f64, f64)) -> (f64, f64) {
    if is_degenerate_range(low, high) {
        (low - 0.5, low + 0.5)
    } else {
        (low, high)
    }
}

/// The equal bins along one axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Axis {
    /// Lower edge of the first bin.
    pub(super) start: f64,
    pub(super) count: usize,
    pub(super) width: f64,
}

/// lets-plot `Bin2dStat` bins of one axis: the range widened by half a
/// (preliminary) bin on each side, so no point lies on the outer edges.
pub(super) fn rectangle_axis(range: (f64, f64), options: AxisOptions) -> Axis {
    let by_count = |span: f64, count: usize| span / count as f64;
    let (low, high) = applicable_range(range);
    let (_, initial_width) = options.count_and_width(high - low, by_count);
    let (low, high) = if is_degenerate_range(range.0, range.1) {
        (range.0 - 0.5, range.1 + 0.5)
    } else {
        (range.0 - initial_width / 2.0, range.1 + initial_width / 2.0)
    };
    let (count, width) = options.count_and_width(high - low, by_count);
    Axis {
        start: low,
        count,
        width,
    }
}

/// Summed weights per cell of a `columns` x `rows` grid, dense when it is small.
pub(super) enum CellCounts {
    Dense { columns: usize, counts: Vec<f64> },
    Sparse(std::collections::HashMap<(usize, usize), f64>),
}

impl CellCounts {
    pub(super) fn new(columns: usize, rows: usize) -> Self {
        match columns.checked_mul(rows) {
            Some(cells) if cells <= DENSE_CELL_LIMIT => CellCounts::Dense {
                columns,
                counts: vec![0.0; cells],
            },
            _ => CellCounts::Sparse(std::collections::HashMap::new()),
        }
    }

    pub(super) fn add(&mut self, column: usize, row: usize, weight: f64) {
        match self {
            CellCounts::Dense { columns, counts } => counts[row * *columns + column] += weight,
            CellCounts::Sparse(map) => *map.entry((column, row)).or_insert(0.0) += weight,
        }
    }

    pub(super) fn get(&self, column: usize, row: usize) -> f64 {
        match self {
            CellCounts::Dense { columns, counts } => counts[row * *columns + column],
            CellCounts::Sparse(map) => map.get(&(column, row)).copied().unwrap_or(0.0),
        }
    }

    /// Occupied cells (any order), for grids too big to walk.
    pub(super) fn occupied(&self) -> Vec<(usize, usize)> {
        match self {
            CellCounts::Dense { columns, counts } => counts
                .iter()
                .enumerate()
                .filter(|(_, count)| **count != 0.0)
                .map(|(index, _)| (index % *columns, index / *columns))
                .collect(),
            CellCounts::Sparse(map) => map
                .iter()
                .filter(|(_, count)| **count != 0.0)
                .map(|(cell, _)| *cell)
                .collect(),
        }
    }
}

/// Cells to report, in lets-plot's order (`outer` index first, then `inner`):
/// every cell, or with `drop` the non-empty ones and the empty corner cells
/// (0, 0), (0, 1) and (1, 0), which lets-plot reports as NaN padding rows.
pub(super) fn reported_cells(
    counts: &CellCounts,
    columns: usize,
    rows: usize,
    drop: bool,
    column_major: bool,
) -> Vec<(usize, usize)> {
    let order = |&(column, row): &(usize, usize)| {
        if column_major {
            (column, row)
        } else {
            (row, column)
        }
    };
    let corner = |(column, row): (usize, usize)| matches!((column, row), (0, 0) | (0, 1) | (1, 0));
    let walk_all = !drop || matches!(counts, CellCounts::Dense { .. });
    if !walk_all {
        let mut cells = counts.occupied();
        for cell in [(0, 0), (0, 1), (1, 0)] {
            if cell.0 < columns && cell.1 < rows && counts.get(cell.0, cell.1) == 0.0 {
                cells.push(cell);
            }
        }
        cells.sort_unstable_by_key(order);
        return cells;
    }
    let (outer, inner) = if column_major {
        (columns, rows)
    } else {
        (rows, columns)
    };
    let mut cells = Vec::new();
    for outer_index in 0..outer {
        for inner_index in 0..inner {
            let cell = if column_major {
                (outer_index, inner_index)
            } else {
                (inner_index, outer_index)
            };
            if !drop || corner(cell) || counts.get(cell.0, cell.1) != 0.0 {
                cells.push(cell);
            }
        }
    }
    cells
}

/// lets-plot `densityNormalizingFactor`: one over the area of a bin.
pub(super) fn density_factor(x: &Axis, y: &Axis) -> f64 {
    let area = (x.count as f64 * x.width) * (y.count as f64 * y.width);
    (x.count * y.count) as f64 / area
}

/// lets-plot `Bin2dStat.computeBins` for the rows of one group.
fn count_rectangles(
    rows: &[usize],
    x: &ContinuousColumn,
    y: &ContinuousColumn,
    weight: Option<&ContinuousColumn>,
    x_axis: &Axis,
    y_axis: &Axis,
    drop: bool,
) -> Vec<(Variable, Vec<f64>)> {
    let mut counts = CellCounts::new(x_axis.count, y_axis.count);
    let mut total = 0.0;
    for &row in rows {
        let (Some(x_value), Some(y_value)) = (
            x.get(row).filter(|value| value.is_finite()),
            y.get(row).filter(|value| value.is_finite()),
        ) else {
            continue;
        };
        let weight = row_weight(weight, row);
        total += weight;
        let column = ((x_value - x_axis.start) / x_axis.width).floor();
        let row_index = ((y_value - y_axis.start) / y_axis.width).floor();
        // Points outside the grid count towards the total only, as lets-plot.
        if column >= 0.0
            && row_index >= 0.0
            && column < x_axis.count as f64
            && row_index < y_axis.count as f64
        {
            counts.add(column as usize, row_index as usize, weight);
        }
    }
    let factor = density_factor(x_axis, y_axis);
    let cells = reported_cells(&counts, x_axis.count, y_axis.count, drop, true);
    let mut xs = Vec::with_capacity(cells.len());
    let mut ys = Vec::with_capacity(cells.len());
    let mut count_values = Vec::with_capacity(cells.len());
    let mut densities = Vec::with_capacity(cells.len());
    let x0 = x_axis.start + x_axis.width / 2.0;
    let y0 = y_axis.start + y_axis.width / 2.0;
    for (column, row) in cells {
        let count = counts.get(column, row);
        let (x, y) = (
            x0 + column as f64 * x_axis.width,
            y0 + row as f64 * y_axis.width,
        );
        if drop && count == 0.0 {
            // lets-plot's "resolution hack": an empty corner bin is kept as a row
            // with one position missing; its other position still trains the axis.
            let padding = if column == 0 {
                (f64::NAN, y)
            } else {
                (x, f64::NAN)
            };
            xs.push(padding.0);
            ys.push(padding.1);
            count_values.push(f64::NAN);
            densities.push(f64::NAN);
            continue;
        }
        xs.push(x);
        ys.push(y);
        count_values.push(count);
        densities.push(count / total * factor);
    }
    let n = xs.len();
    vec![
        (Variable::Aesthetic(Aesthetic::X), xs),
        (Variable::Aesthetic(Aesthetic::Y), ys),
        (Variable::Computed(Computed::Count), count_values),
        (Variable::Computed(Computed::Density), densities),
        (
            Variable::Computed(Computed::BinWidth),
            vec![x_axis.width; n],
        ),
        (
            Variable::Computed(Computed::BinHeight),
            vec![y_axis.width; n],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Column;
    use crate::spec::Options;

    fn column(values: &[f64]) -> Column {
        Column::Continuous(ContinuousColumn::from_values(values.to_vec()))
    }

    fn values(data: &LayerData, variable: impl Into<Variable>) -> Vec<f64> {
        let column = data.continuous(variable).unwrap();
        (0..column.len())
            .map(|row| column.get(row).unwrap())
            .collect()
    }

    /// Values of the bins with a count (padding rows left out).
    fn counted(data: &LayerData, variable: impl Into<Variable>) -> Vec<f64> {
        let counts = values(data, Computed::Count);
        values(data, variable)
            .into_iter()
            .zip(counts)
            .filter(|(_, count)| count.is_finite())
            .map(|(value, _)| value)
            .collect()
    }

    fn compute(x: &[f64], y: &[f64], options: &str) -> LayerData {
        let mut data = LayerData::new(x.len());
        data.insert(Aesthetic::X, column(x)).unwrap();
        data.insert(Aesthetic::Y, column(y)).unwrap();
        let options: Options = serde_json::from_str(options).unwrap();
        Bin2dStat
            .compute(
                data,
                &Parameters::new(&options, "layers[0]"),
                &StatContext::default(),
            )
            .unwrap()
    }

    #[test]
    fn grid_widens_by_half_a_bin_as_lets_plot() {
        // x over [0, 4] with 4 bins: preliminary width 1, so [-0.5, 4.5] in 4 bins.
        let data = compute(&[0.0, 4.0, 4.0], &[0.0, 0.0, 2.0], r#"{"bins": [4, 2]}"#);
        assert_eq!(values(&data, Computed::BinWidth)[0], 1.25);
        assert_eq!(values(&data, Computed::BinHeight)[0], 1.5);
        assert_eq!(counted(&data, Aesthetic::X), vec![0.125, 3.875, 3.875]);
        assert_eq!(counted(&data, Aesthetic::Y), vec![0.25, 0.25, 1.75]);
        assert_eq!(counted(&data, Computed::Count), vec![1.0, 1.0, 1.0]);
    }

    #[test]
    fn empty_corner_bins_pad_as_lets_plot() {
        // Bins (0, 1) and (1, 0) are empty: lets-plot keeps one position of each.
        let data = compute(&[0.0, 4.0, 4.0], &[0.0, 0.0, 2.0], r#"{"bins": [4, 2]}"#);
        let x = values(&data, Aesthetic::X);
        let y = values(&data, Aesthetic::Y);
        assert_eq!(data.row_count(), 5);
        assert!(x[1].is_nan() && y[1] == 1.75);
        assert!(x[2] == 1.375 && y[2].is_nan());
        assert!(values(&data, Computed::Count)[1].is_nan());
    }

    #[test]
    fn counts_sum_to_n_and_density_integrates_to_one() {
        let x: Vec<f64> = (0..100).map(|index| (index as f64 * 0.37).sin()).collect();
        let y: Vec<f64> = (0..100).map(|index| (index as f64 * 0.91).cos()).collect();
        let data = compute(&x, &y, r#"{"bins": [7, 5], "drop": false}"#);
        assert_eq!(data.row_count(), 35);
        let counts = values(&data, Computed::Count);
        assert_eq!(counts.iter().sum::<f64>(), 100.0);
        let area = values(&data, Computed::BinWidth)[0] * values(&data, Computed::BinHeight)[0];
        let integral: f64 = values(&data, Computed::Density).iter().sum::<f64>() * area;
        assert!((integral - 1.0).abs() < 1e-12, "{integral}");
    }

    #[test]
    fn drop_keeps_only_occupied_bins_in_column_order() {
        let data = compute(&[0.0, 1.0, 0.0], &[1.0, 0.0, 0.0], r#"{"bins": [2, 2]}"#);
        assert_eq!(values(&data, Computed::Count), vec![1.0, 1.0, 1.0]);
        assert_eq!(values(&data, Aesthetic::X), vec![0.125, 0.125, 0.875]);
        assert_eq!(values(&data, Aesthetic::Y), vec![0.125, 0.875, 0.125]);
    }

    #[test]
    fn one_bin_along_an_axis_keeps_its_size() {
        // lets-plot #1290: one y bin; the tile must still span the y range.
        let data = compute(&[0.0, 1.0, 2.0], &[0.0, 1.0, 3.0], r#"{"bins": [3, 1]}"#);
        assert_eq!(values(&data, Computed::BinHeight)[0], 6.0);
        assert_eq!(values(&data, Aesthetic::Y), vec![1.5; 3]);
    }

    #[test]
    fn binwidth_wins_and_one_number_sets_both_axes() {
        let data = compute(
            &[0.0, 10.0],
            &[0.0, 10.0],
            r#"{"binwidth": [2, null], "bins": 4}"#,
        );
        assert_eq!(values(&data, Computed::BinWidth)[0], 2.0);
        // y: 4 bins over [-1.25, 11.25].
        assert_eq!(values(&data, Computed::BinHeight)[0], 3.125);
    }

    #[test]
    fn bad_bins_are_reported() {
        let options: Options = serde_json::from_str(r#"{"bins": [0, 3]}"#).unwrap();
        let error = Bin2dOptions::from_parameters(&Parameters::new(&options, "layers[0]"), 1.0)
            .unwrap_err()
            .to_string();
        assert!(error.contains("bins must be at least 1"), "{error}");
        let options: Options = serde_json::from_str(r#"{"bins": [1, 2, 3]}"#).unwrap();
        let error = Bin2dOptions::from_parameters(&Parameters::new(&options, "layers[0]"), 1.0)
            .unwrap_err()
            .to_string();
        assert!(error.contains("list of two numbers"), "{error}");
    }
}
