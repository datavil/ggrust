//! `stat="contour_filled"` (ggplot2 `stat_contour_filled`): the bands between
//! the isolines of z, each labelled by its interval as an ordered discrete level,
//! so `geom_contour_filled` fills by band with a discrete legend.
//!
//! As ggplot2 4.0.1:
//! - breaks: `breaks` as given; else `pretty(range(z), 10)` (R's `pretty()`),
//!   or, with `bins`, the z range widened to round steps and cut into
//!   `bins` steps; or `binwidth` steps (`scales::fullseq`), see [`breaks`];
//! - one band between each two consecutive breaks, nothing outside them;
//! - `..level..` is `"(low, high]"`, the two ends formatted as R's
//!   `format(digits = d)` over all bands (one number of decimals; `d` from 3 up
//!   until every break reads differently), levels in
//!   band order; only bands that hold a polygon are levels (ggplot2 keeps empty
//!   levels in the factor, and its default scale drops them).
//!
//! The band polygons are the shared isobands of [`super::contour`].
//!
//! What it does not do: `level_low`, `level_high`, `level_mid` and `nlevel`
//! (ggplot2's extra band variables), or breaks over the whole layer when faceted
//! (each panel takes its own z range).

use super::contour::{Grid, Piece, contour_columns};
use super::groups::per_group_pieces;
use super::{Stat, StatContext};
use crate::data::{Column, Computed, ContinuousColumn, DiscreteColumn, LayerData};
use crate::parameters::Parameters;
use crate::spec::{Aesthetic, Value};
use crate::{Error, Result};

/// `stat="contour_filled"`.
pub struct ContourFilledStat;

/// Most bands (as the other contour stats' level cap).
const MAX_BANDS: usize = super::bin::MAX_BIN_COUNT;

impl Stat for ContourFilledStat {
    fn name(&self) -> &'static str {
        "contour_filled"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["bins", "binwidth", "breaks"]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Z]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Z]
    }

    /// ggplot2: `fill = after_stat(level)`.
    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        &[(Aesthetic::Fill, Computed::Level)]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        let options = BreakOptions::from_parameters(parameters)?;
        let empty = ContinuousColumn::from_values(Vec::new());
        if data.get(Aesthetic::Z).is_some() && data.continuous(Aesthetic::Z).is_none() {
            return Err(Error::spec(
                parameters.path_of("z"),
                "z must be numeric (the height of each grid node), got a discrete column; \
                 map z to a numeric column, or use geom_tile(aes(fill=...)) for categories",
            ));
        }
        let x = data.continuous(Aesthetic::X).unwrap_or(&empty);
        let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
        let z = data.continuous(Aesthetic::Z).unwrap_or(&empty);
        // ggplot2 takes the breaks from the z range of the whole layer.
        let z_range = (0..z.len())
            .filter_map(|row| z.get(row))
            .filter(|value| value.is_finite())
            .fold(None, |range: Option<(f64, f64)>, value| {
                Some(range.map_or((value, value), |(low, high)| {
                    (low.min(value), high.max(value))
                }))
            });
        let breaks = match z_range {
            Some(range) => options.breaks(range, parameters)?,
            None => Vec::new(),
        };
        let mut result = per_group_pieces(&data, |rows| {
            let value = |column: &ContinuousColumn, row: usize| column.get(row).unwrap_or(f64::NAN);
            let nodes: Vec<(f64, f64, f64)> = rows
                .iter()
                .map(|&row| (value(x, row), value(y, row), value(z, row)))
                .collect();
            let pieces: Vec<Piece> = match Grid::from_nodes(&nodes)? {
                Some(grid) => grid.bands_between(&breaks),
                None => Vec::new(),
            };
            Ok(contour_columns(pieces))
        })?;
        // `..level..` holds band indices here; make it the ordered band labels.
        let labels = band_labels(&breaks);
        let indices: Vec<Option<usize>> = match result.continuous(Computed::Level) {
            Some(column) => (0..column.len())
                .map(|row| column.get(row).map(|index| index as usize))
                .collect(),
            None => Vec::new(),
        };
        let mut used = vec![false; labels.len()];
        for index in indices.iter().flatten() {
            if let Some(slot) = used.get_mut(*index) {
                *slot = true;
            }
        }
        let mut code_of = vec![None; labels.len()];
        let mut levels = Vec::new();
        for (index, label) in labels.into_iter().enumerate() {
            if used[index] {
                code_of[index] = Some(levels.len() as u32);
                levels.push(label);
            }
        }
        let codes: Vec<Option<u32>> = indices
            .iter()
            .map(|index| index.and_then(|index| code_of.get(index).copied().flatten()))
            .collect();
        result.insert(
            Computed::Level,
            Column::Discrete(DiscreteColumn::from_parts(levels, codes.into())),
        )?;
        Ok(result)
    }
}

/// `bins`, `binwidth` and `breaks` as ggplot2 reads them (none set: pretty breaks).
struct BreakOptions {
    bins: Option<usize>,
    /// Positive.
    width: Option<f64>,
    /// Ascending and distinct.
    breaks: Option<Vec<f64>>,
}

impl BreakOptions {
    fn from_parameters(parameters: &Parameters) -> Result<Self> {
        let bins = parameters
            .number("bins")?
            .map(|count| {
                if !count.is_finite() || count < 1.0 {
                    return Err(Error::spec(
                        parameters.path_of("bins"),
                        format!("bins is a number of bands of 1 or more, got {count}"),
                    ));
                }
                Ok((count.trunc() as usize).min(MAX_BANDS))
            })
            .transpose()?;
        let width = parameters
            .number("binwidth")?
            .map(|width| {
                if !width.is_finite() || width <= 0.0 {
                    return Err(Error::spec(
                        parameters.path_of("binwidth"),
                        format!("binwidth is a positive number (the band height), got {width}"),
                    ));
                }
                Ok(width)
            })
            .transpose()?;
        let breaks = parameters
            .value("breaks")
            .map(|value| {
                let invalid = || {
                    Error::spec(
                        parameters.path_of("breaks"),
                        format!(
                            "expected a list of at least two finite numbers (the band edges), \
                             got {value}"
                        ),
                    )
                };
                let items = match value {
                    Value::Array(items) if items.len() >= 2 => items,
                    _ => return Err(invalid()),
                };
                let mut breaks = items
                    .iter()
                    .map(|item| item.as_f64().filter(|number| number.is_finite()))
                    .collect::<Option<Vec<f64>>>()
                    .ok_or_else(invalid)?;
                breaks.sort_by(f64::total_cmp);
                breaks.dedup();
                if breaks.len() < 2 {
                    return Err(invalid());
                }
                if breaks.len() > MAX_BANDS + 1 {
                    return Err(Error::spec(
                        parameters.path_of("breaks"),
                        format!(
                            "at most {} band edges, got {}; pass fewer breaks",
                            MAX_BANDS + 1,
                            breaks.len()
                        ),
                    ));
                }
                Ok(breaks)
            })
            .transpose()?;
        Ok(BreakOptions {
            bins,
            width,
            breaks,
        })
    }

    /// ggplot2 `contour_breaks(z_range, bins, binwidth, breaks)`.
    fn breaks(&self, z_range: (f64, f64), parameters: &Parameters) -> Result<Vec<f64>> {
        if let Some(breaks) = &self.breaks {
            return Ok(breaks.clone());
        }
        let breaks = match (self.bins, self.width) {
            (None, None) => pretty(z_range, 10),
            (Some(bins), _) => {
                // Widen the range to round steps so the bins span it nicely.
                let accuracy = signif(z_range.1 - z_range.0, 1) / 10.0;
                let mut range = z_range;
                if accuracy > 0.0 && accuracy.is_finite() {
                    range = (
                        (range.0 / accuracy).floor() * accuracy,
                        (range.1 / accuracy).ceil() * accuracy,
                    );
                }
                if bins == 1 {
                    vec![range.0, range.1]
                } else {
                    let width = (range.1 - range.0) / (bins - 1) as f64;
                    let breaks = fullseq(range, width);
                    // Sometimes the sequence above yields one bin too few.
                    if breaks.len() < bins + 1 {
                        fullseq(range, (range.1 - range.0) / bins as f64)
                    } else {
                        breaks
                    }
                }
            }
            (None, Some(width)) => fullseq(z_range, width),
        };
        if breaks.len() > MAX_BANDS + 1 {
            return Err(Error::spec(
                parameters.path_of("binwidth"),
                format!(
                    "the bands would need {} edges, at most {} are drawn; pass a larger \
                     binwidth or fewer bins",
                    breaks.len(),
                    MAX_BANDS + 1
                ),
            ));
        }
        Ok(breaks)
    }
}

/// `"(low, high]"` for each band between consecutive `breaks` (distinct); the
/// lows and the highs each formatted as R's `format(digits = d)` of the whole
/// vector, `d` from 3 up until all breaks format apart (ggplot2).
fn band_labels(breaks: &[f64]) -> Vec<String> {
    if breaks.len() < 2 {
        return Vec::new();
    }
    // ggplot2 `pretty_isoband_levels`: more digits until every break reads apart.
    let mut digits = 3;
    while digits < 15 {
        let mut texts = format_common(breaks, digits);
        texts.sort();
        texts.dedup();
        if texts.len() == breaks.len() {
            break;
        }
        digits += 1;
    }
    let lows = format_common(&breaks[..breaks.len() - 1], digits);
    let highs = format_common(&breaks[1..], digits);
    lows.iter()
        .zip(&highs)
        .map(|(low, high)| format!("({low}, {high}]"))
        .collect()
}

/// R's `format(x, digits = digits, trim = TRUE)` of a numeric vector: every
/// number with the same decimals (enough for the one needing most, at `digits`
/// significant digits), or all in scientific notation when that is narrower.
fn format_common(values: &[f64], digits: usize) -> Vec<String> {
    let digits = digits.max(1) as i32;
    // (exponent, significant digits) of each finite non-zero value, as R's
    // `scientific()`.
    let parts: Vec<(i32, i32)> = values
        .iter()
        .map(|&value| {
            if value == 0.0 || !value.is_finite() {
                return (0, 1);
            }
            let mut exponent = value.abs().log10().floor() as i32;
            let mut scaled =
                (value.abs() / 10_f64.powi(exponent) * 10_f64.powi(digits - 1)).round_ties_even();
            if scaled >= 10_f64.powi(digits) {
                exponent += 1;
                scaled = (scaled / 10.0).round_ties_even();
            }
            let mut significant = digits;
            while significant > 1 && scaled % 10.0 == 0.0 {
                scaled /= 10.0;
                significant -= 1;
            }
            (exponent, significant)
        })
        .collect();
    let negative = usize::from(values.iter().any(|value| *value < 0.0));
    let decimals = parts
        .iter()
        .map(|(exponent, significant)| (significant - 1 - exponent).max(0))
        .max()
        .unwrap_or(0) as usize;
    let left = parts
        .iter()
        .map(|(exponent, _)| (*exponent + 1).max(1) as usize)
        .max()
        .unwrap_or(1);
    let fixed_width = negative + left + if decimals > 0 { decimals + 1 } else { 0 };
    let max_significant = parts.iter().map(|(_, significant)| *significant).max();
    let max_significant = max_significant.unwrap_or(1) as usize;
    let wide_exponent = parts.iter().any(|(exponent, _)| exponent.abs() >= 100);
    let scientific_width = negative
        + if max_significant > 1 {
            max_significant + 1
        } else {
            1
        }
        + if wide_exponent { 5 } else { 4 };
    values
        .iter()
        .map(|&value| {
            if fixed_width <= scientific_width {
                format!("{value:.decimals$}")
            } else {
                let printed = format!("{:.*e}", max_significant - 1, value);
                let (mantissa, exponent) = printed.split_once('e').unwrap_or((&printed, "0"));
                let exponent: i32 = exponent.parse().unwrap_or(0);
                let sign = if exponent < 0 { '-' } else { '+' };
                format!("{mantissa}e{sign}{:02}", exponent.abs())
            }
        })
        .collect()
}

/// R's `signif(x, digits)`.
fn signif(x: f64, digits: i32) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let exponent = x.abs().log10().floor() as i32;
    let scale = 10_f64.powi(digits - 1 - exponent);
    (x * scale).round_ties_even() / scale
}

/// `scales::fullseq(range, size)`: multiples of `size` covering `range`.
fn fullseq(range: (f64, f64), size: f64) -> Vec<f64> {
    let (low, high) = range;
    let tolerance = 1000.0 * f64::EPSILON;
    let zero_range = low == high
        || (low.abs().max(high.abs()) > 0.0
            && ((high - low) / low.abs().max(high.abs())).abs() < tolerance);
    if zero_range {
        return vec![low - size / 2.0, high + size / 2.0];
    }
    let from = (low / size).floor() * size;
    let to = (high / size).ceil() * size;
    r_seq_by(from, to, size)
}

/// R's `seq(from, to, by = by)` for `by > 0`.
fn r_seq_by(from: f64, to: f64, by: f64) -> Vec<f64> {
    let span = to - from;
    if span.abs() / from.abs().max(to.abs()) < 100.0 * f64::EPSILON {
        return vec![from];
    }
    let count = (span / by + 1e-10) as usize;
    (0..=count)
        .map(|index| (from + index as f64 * by).min(to))
        .collect()
}

/// R's `pretty(range, n)` (R 4.x `R_pretty` with the defaults: `min.n = n %/% 3`,
/// `shrink.sml = 0.75`, `high.u.bias = 1.5`, `u5.bias = 2.75`).
fn pretty((low, high): (f64, f64), n: usize) -> Vec<f64> {
    const ROUNDING_EPS: f64 = 1e-10;
    const H: f64 = 1.5;
    const H5: f64 = 0.5 + 1.5 * H;
    let min_n = (n / 3) as f64;
    let ndiv = n.max(1) as f64;
    let dx = high - low;
    let (mut cell, small) = if dx == 0.0 && high == 0.0 {
        (1.0, true)
    } else {
        let cell = low.abs().max(high.abs());
        let unit =
            (1.0 + if H5 >= 1.5 * H + 0.5 {
                1.0 / (1.0 + H)
            } else {
                1.5 / (1.0 + H5)
            }) * ndiv
                * f64::EPSILON;
        (cell, dx < cell * unit * 3.0)
    };
    if small {
        if cell > 10.0 {
            cell = 9.0 + cell / 10.0;
        }
        cell *= 0.75;
        if min_n > 1.0 {
            cell /= min_n;
        }
    } else {
        cell = dx;
        if ndiv > 1.0 {
            cell /= ndiv;
        }
    }
    cell = cell.max(f64::MIN_POSITIVE);
    let base = 10_f64.powf(cell.log10().floor());
    let mut unit = base;
    if 2.0 * base - cell < H * (cell - unit) {
        unit = 2.0 * base;
        if 5.0 * base - cell < H5 * (cell - unit) {
            unit = 5.0 * base;
            if 10.0 * base - cell < H * (cell - unit) {
                unit = 10.0 * base;
            }
        }
    }
    let mut start = (low / unit + ROUNDING_EPS).floor();
    let mut end = (high / unit - ROUNDING_EPS).ceil();
    while start * unit > low + ROUNDING_EPS * unit {
        start -= 1.0;
    }
    while end * unit < high - ROUNDING_EPS * unit {
        end += 1.0;
    }
    let mut k = (0.5 + end - start) as i64;
    let min_count = min_n as i64;
    if k < min_count {
        k = min_count - k;
        if start >= 0.0 {
            end += (k / 2) as f64;
            start -= (k / 2 + k % 2) as f64;
        } else {
            start -= (k / 2) as f64;
            end += (k / 2 + k % 2) as f64;
        }
        k = min_count;
    }
    let from = (start * unit).min(low);
    let to = (end * unit).max(high);
    let count = k.max(0) as usize;
    if count == 0 {
        return vec![from];
    }
    // `seq.int(from, to, length.out = count + 1)`: the ends exact, steps between.
    let by = (to - from) / count as f64;
    let mut values: Vec<f64> = (0..=count)
        .map(|index| match index {
            0 => from,
            last if last == count => to,
            index => from + index as f64 * by,
        })
        .collect();
    // R zaps rounding noise around zero.
    let delta = (to - from) / count as f64;
    for value in &mut values {
        if value.abs() < 1e-14 * delta {
            *value = 0.0;
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_matches_r() {
        // R 4.4: pretty(c(lo, hi), 10).
        assert_eq!(
            pretty((0.0, 1.0), 10),
            [
                0.0,
                0.1,
                0.2,
                0.30000000000000004,
                0.4,
                0.5,
                0.6000000000000001,
                0.7000000000000001,
                0.8,
                0.9,
                1.0
            ]
        );
        assert_eq!(
            pretty((0.002, 0.037), 10),
            [0.0, 0.005, 0.01, 0.015, 0.02, 0.025, 0.03, 0.035, 0.04]
        );
        assert_eq!(
            pretty((-3.2, 47.9), 10),
            [
                -5.0, 0.0, 5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0, 45.0, 50.0
            ]
        );
        assert_eq!(
            pretty((95.0, 195.0), 10),
            [
                90.0, 100.0, 110.0, 120.0, 130.0, 140.0, 150.0, 160.0, 170.0, 180.0, 190.0, 200.0
            ]
        );
    }

    #[test]
    fn labels_share_decimals_as_r_format() {
        assert_eq!(
            band_labels(&[0.0, 0.005, 0.01, 0.015]),
            ["(0.000, 0.005]", "(0.005, 0.010]", "(0.010, 0.015]"]
        );
        assert_eq!(
            band_labels(&[90.0, 100.0, 110.0]),
            ["(90, 100]", "(100, 110]"]
        );
        assert_eq!(band_labels(&[-5.0, 0.0, 5.0]), ["(-5, 0]", "(0, 5]"]);
        // Narrow ranges get more digits (R: "(100.0, 100.1]", "(100.1, 100.2]").
        assert_eq!(
            band_labels(&[100.0, 100.1, 100.2]),
            ["(100.0, 100.1]", "(100.1, 100.2]"]
        );
        // R: pretty_isoband_levels(c("0.00012:0.00014", "1e-4:12345")).
        assert_eq!(format_common(&[0.00012, 1e-4], 3), ["0.00012", "0.00010"]);
        assert_eq!(
            format_common(&[1.4e-4, 12345.0], 3),
            ["1.40e-04", "1.23e+04"]
        );
    }

    #[test]
    fn bins_and_binwidth_follow_ggplot2() {
        let options = |bins: Option<usize>, width: Option<f64>| BreakOptions {
            bins,
            width,
            breaks: None,
        };
        let empty = crate::spec::Options::new();
        let parameters = Parameters::new(&empty, "layers[0]");
        // R: ggplot2:::contour_breaks(c(0.3, 9.7), bins = 5)
        let breaks = options(Some(5), None)
            .breaks((0.3, 9.7), &parameters)
            .unwrap();
        let expected = [0.0, 1.98, 3.96, 5.94, 7.92, 9.9];
        assert_eq!(breaks.len(), expected.len());
        for (got, want) in breaks.iter().zip(expected) {
            assert!((got - want).abs() < 1e-12, "{breaks:?}");
        }
        // R: ggplot2:::contour_breaks(c(0.3, 9.7), binwidth = 2)
        assert_eq!(
            options(None, Some(2.0))
                .breaks((0.3, 9.7), &parameters)
                .unwrap(),
            [0.0, 2.0, 4.0, 6.0, 8.0, 10.0]
        );
    }
}
