//! `stat="quasirandom"`: points spread sideways within the density of their
//! group, as the R package ggbeeswarm's `position_quasirandom` (offsets from
//! vipor's `offsetSingleGroup`).
//!
//! Points are spread per x (per x and group with `dodge_width`, after dodging;
//! see [`super::beeswarm::Swarms`]). Each point of a swarm of `n` moves by
//! `(u - 0.5) x 2 x d x width`, where `d` is R's `density()` of the swarm
//! (`adjust = bandwidth`, `nbins` points) at the point, scaled to a maximum of 1,
//! and `u` in [0, 1] depends on the method:
//! - `quasirandom`: the van der Corput sequence (base 2) at the point's rank.
//! - `pseudorandom`: uniform, from the seeded Kotlin generator (not R's).
//! - `smiley` / `maxout`, `frowney` / `minout`: within each density bin, points
//!   alternate to the outside, low values out (smiley) or high values out.
//!
//! `width` defaults to 0.4 times the x resolution (of the dodged x, as ggplot2). `varwidth` scales each swarm
//! by `sqrt(n / largest n)`; the largest is the panel's.
//!
//! What it does not do: vipor's `tukey` methods (they need R's generator).

use super::beeswarm::Swarms;
use super::r_density::RDensity;
use super::{Stat, StatContext};
use crate::data::LayerData;
use crate::parameters::Parameters;
use crate::position::random::{DEFAULT_SEED, KotlinRandom};
use crate::spec::Aesthetic;
use crate::suggest::did_you_mean;
use crate::{Error, Result};

/// Points spread within their density.
pub struct QuasirandomStat;

impl Stat for QuasirandomStat {
    fn name(&self) -> &'static str {
        "quasirandom"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &[
            "method",
            "width",
            "varwidth",
            "bandwidth",
            "nbins",
            "dodge_width",
            "seed",
        ]
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
        let method = Method::from_parameters(parameters)?;
        let positive = |name: &str| -> Result<Option<f64>> {
            match parameters.number(name)? {
                Some(value) if value <= 0.0 => Err(Error::spec(
                    parameters.path_of(name),
                    format!("{name} must be a positive number, got {value}"),
                )),
                value => Ok(value),
            }
        };
        let width = positive("width")?;
        let bandwidth = positive("bandwidth")?.unwrap_or(0.5);
        let bins = match positive("nbins")? {
            Some(value) if value < 2.0 || value.fract() != 0.0 => {
                return Err(Error::spec(
                    parameters.path_of("nbins"),
                    format!("nbins must be a whole number of at least 2, got {value}"),
                ));
            }
            value => value.map(|value| value as usize),
        };
        let varwidth = parameters.boolean("varwidth")?.unwrap_or(false);
        let seed = parameters
            .number("seed")?
            .map_or(DEFAULT_SEED, |seed| seed as i64);

        let swarms = Swarms::of(data, parameters)?;
        // ggplot2's resolution after the dodge: dodged x is no longer discrete, so
        // each group's spread fits its dodge slot.
        let resolution = match (swarms.dodged, swarms.data.continuous(Aesthetic::X)) {
            (true, Some(x)) => crate::data::resolution(x),
            _ => swarms.data.resolution(Aesthetic::X),
        };
        let width = width.unwrap_or(resolution * 0.4);
        let largest = swarms.rows.iter().map(Vec::len).max().unwrap_or(1) as f64;
        let mut random = KotlinRandom::new(seed);
        let mut x = swarms.x.clone();
        for rows in &swarms.rows {
            let values: Vec<f64> = rows.iter().map(|&row| swarms.y[row]).collect();
            let share = match varwidth {
                true => (values.len() as f64 / largest).sqrt(),
                false => 1.0,
            };
            let offsets = offsets(&values, method, bins, bandwidth, &mut random);
            for (row, offset) in rows.iter().zip(offsets) {
                x[*row] += offset * share * width;
            }
        }
        swarms.finish(x, None)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Method {
    Quasirandom,
    Pseudorandom,
    Smiley,
    Frowney,
}

const METHODS: &str = "'quasirandom', 'pseudorandom', 'smiley' ('maxout'), 'frowney' ('minout')";

impl Method {
    fn from_parameters(parameters: &Parameters) -> Result<Method> {
        match parameters.string("method")? {
            None | Some("quasirandom") => Ok(Method::Quasirandom),
            Some("pseudorandom") => Ok(Method::Pseudorandom),
            Some("smiley" | "maxout") => Ok(Method::Smiley),
            Some("frowney" | "minout") => Ok(Method::Frowney),
            Some(other @ ("tukey" | "tukeyDense")) => Err(Error::spec(
                parameters.path_of("method"),
                format!(
                    "method '{other}' is not supported (it needs R's random generator); \
                     use one of {METHODS}"
                ),
            )),
            Some(other) => Err(Error::spec(
                parameters.path_of("method"),
                format!(
                    "unknown quasirandom method '{other}'; use one of {METHODS}{}",
                    did_you_mean(other, crate::suggest::quoted(METHODS))
                ),
            )),
        }
    }
}

/// vipor `offsetSingleGroup`: offsets in [-1, 1] of one swarm's values.
fn offsets(
    values: &[f64],
    method: Method,
    bins: Option<usize>,
    bandwidth: f64,
    random: &mut KotlinRandom,
) -> Vec<f64> {
    let bins = bins.unwrap_or(match method {
        Method::Quasirandom | Method::Pseudorandom => 1024,
        _ => (values.len().div_ceil(5)).max(2),
    });
    let Some(estimate) = RDensity::estimate(values, bins, bandwidth) else {
        return vec![0.0; values.len()];
    };
    let largest = estimate.density.iter().copied().fold(0.0, f64::max);
    let spread: Vec<f64> = match method {
        Method::Quasirandom => ranks(values)
            .into_iter()
            .map(|rank| van_der_corput(rank + 1))
            .collect(),
        Method::Pseudorandom => values.iter().map(|_| random.next_double()).collect(),
        Method::Smiley => outside_in(values, &estimate.grid, false),
        Method::Frowney => outside_in(values, &estimate.grid, true),
    };
    values
        .iter()
        .zip(spread)
        .map(|(value, spread)| {
            let density = match largest > 0.0 {
                true => estimate.at(*value) / largest,
                false => 0.0,
            };
            (spread - 0.5) * 2.0 * density
        })
        .collect()
}

/// 0-based ranks, ties in row order (R `rank(ties.method = "first")`).
fn ranks(values: &[f64]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|a, b| values[*a].total_cmp(&values[*b]));
    let mut ranks = vec![0; values.len()];
    for (rank, index) in order.into_iter().enumerate() {
        ranks[index] = rank;
    }
    ranks
}

/// The `index`-th (1-based) term of the base 2 van der Corput sequence: the
/// binary digits of `index` mirrored behind the point (1 -> 0.5, 2 -> 0.25, 3 -> 0.75).
fn van_der_corput(mut index: usize) -> f64 {
    let mut value = 0.0;
    let mut scale = 0.5;
    while index > 0 {
        if index & 1 == 1 {
            value += scale;
        }
        index >>= 1;
        scale /= 2.0;
    }
    value
}

/// vipor's `maxout` / `minout`: points are binned by the density grid (R `cut`,
/// right-closed); in a bin of `k`, ranks 1, 3, 5, .. go to the low end and 2, 4,
/// .. to the high end, as shares of [0, 1]. `frowney` ranks by descending value.
fn outside_in(values: &[f64], grid: &[f64], frowney: bool) -> Vec<f64> {
    let bin_of = |value: f64| grid.partition_point(|point| *point < value);
    let mut members: std::collections::BTreeMap<usize, Vec<usize>> = Default::default();
    for (index, value) in values.iter().enumerate() {
        members.entry(bin_of(*value)).or_default().push(index);
    }
    let mut spread = vec![0.5; values.len()];
    for indices in members.values() {
        if indices.len() == 1 {
            continue;
        }
        let keyed: Vec<f64> = indices
            .iter()
            .map(|index| match frowney {
                true => -values[*index],
                false => values[*index],
            })
            .collect();
        // Odd ranks (1-based) become negative, so they sort first, largest first.
        let keys: Vec<i64> = ranks(&keyed)
            .into_iter()
            .map(|rank| rank as i64 + 1)
            .map(|rank| if rank % 2 == 1 { -rank } else { rank })
            .collect();
        let final_ranks = ranks(&keys.iter().map(|key| *key as f64).collect::<Vec<_>>());
        let last = (indices.len() - 1) as f64;
        for (index, rank) in indices.iter().zip(final_ranks) {
            spread[*index] = rank as f64 / last;
        }
    }
    spread
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn van_der_corput_matches_vipor() {
        // R: vipor::vanDerCorput(7)
        let expected = [0.5, 0.25, 0.75, 0.125, 0.625, 0.375, 0.875];
        for (index, expected) in expected.into_iter().enumerate() {
            assert_eq!(van_der_corput(index + 1), expected);
        }
    }

    #[test]
    fn outside_in_matches_vipor_top_bottom() {
        // R: vipor:::topBottomDistribute(c(5, 1, 4, 2, 3)) and frowney = TRUE.
        let values = [5.0, 1.0, 4.0, 2.0, 3.0];
        let grid = [0.0, 10.0];
        assert_eq!(
            outside_in(&values, &grid, false),
            [0.0, 0.5, 1.0, 0.75, 0.25]
        );
        assert_eq!(
            outside_in(&values, &grid, true),
            [0.5, 0.0, 0.75, 1.0, 0.25]
        );
    }

    #[test]
    fn single_points_stay_on_the_line() {
        let mut random = KotlinRandom::new(0);
        assert_eq!(
            offsets(&[3.0], Method::Quasirandom, None, 0.5, &mut random),
            [0.0]
        );
    }
}
