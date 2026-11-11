//! `stat="beeswarm"`: points moved sideways so none overlap, as the R package
//! ggbeeswarm's `position_beeswarm` (with the beeswarm package's swarm layouts).
//!
//! Points are swarmed per x (per x and group with `dodge_width`, after dodging).
//! Sizes are ggbeeswarm's, in data units: a point is 1/100 of the x and y scale
//! ranges across (`cex` times that), a discrete range counting its levels. The
//! ranges are the stat context's (every layer, scale limits applied), as
//! ggbeeswarm reads the trained scales.
//!
//! Methods:
//! - `swarm`: each point, in `priority` order, takes the free spot nearest the
//!   centre line touching earlier points (beeswarm `swarmx`).
//! - `compactswarm`: repeatedly places the point that can sit nearest the centre.
//! - `center`, `square`, `hex`: y is binned into rows one point high (hex rows
//!   `sqrt(3)/2`), points in a row sit side by side; y moves to the row middle.
//!
//! `side` 1 or -1 swarms to the right or left only. `priority="random"` uses the
//! seeded Kotlin generator, not R's, so its layouts differ from R's.
//!
//! What it does not do: ggbeeswarm's `corral` options, log axes in data units.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use super::r_density::RDensity;
use super::{Stat, StatContext};
use crate::data::{Column, ContinuousColumn, LayerData};
use crate::parameters::Parameters;
use crate::position::dodge::dodge;
use crate::position::random::{DEFAULT_SEED, KotlinRandom};
use crate::spec::Aesthetic;
use crate::suggest::did_you_mean;
use crate::{Error, Result};

/// Non-overlapping points.
pub struct BeeswarmStat;

impl Stat for BeeswarmStat {
    fn name(&self) -> &'static str {
        "beeswarm"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["method", "cex", "side", "priority", "dodge_width", "seed"]
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
        context: &StatContext,
    ) -> Result<LayerData> {
        let method = Method::from_parameters(parameters)?;
        let priority = Priority::from_parameters(parameters)?;
        let side = side(parameters)?;
        let cex = parameters.number("cex")?.unwrap_or(1.0);
        if cex <= 0.0 {
            return Err(Error::spec(
                parameters.path_of("cex"),
                format!("cex must be a positive number, got {cex}; use e.g. cex=1.5"),
            ));
        }
        let seed = parameters
            .number("seed")?
            .map_or(DEFAULT_SEED, |seed| seed as i64);
        let swarms = Swarms::of(data, parameters)?;
        let (x_discrete, y_discrete) = swarms.data.discrete_axes();
        let x_size = scale_size(context.x_range, x_discrete) / 100.0 * cex;
        let y_size = scale_size(context.y_range, y_discrete) / 100.0 * cex;
        let mut x = swarms.x.clone();
        let mut y = swarms.y.clone();
        let mut random = KotlinRandom::new(seed);
        for rows in &swarms.rows {
            let values: Vec<f64> = rows.iter().map(|&row| swarms.y[row]).collect();
            match method {
                Method::Swarm | Method::Compact => {
                    let scaled: Vec<f64> = values.iter().map(|value| value / y_size).collect();
                    let order = priority.order(&scaled, &mut random);
                    let offsets = match method {
                        Method::Swarm => swarm(&scaled, &order, side),
                        _ => compact_swarm(&scaled, &order, side),
                    };
                    for (row, offset) in rows.iter().zip(offsets) {
                        x[*row] += offset * x_size;
                    }
                }
                Method::Center | Method::Square | Method::Hex => {
                    let row_height = match method {
                        Method::Hex => y_size * 3f64.sqrt() / 2.0,
                        _ => y_size,
                    };
                    let range = context.y_range.unwrap_or((0.0, 0.0));
                    let (bins, middles) = grid_rows(&values, range, row_height);
                    let offsets = row_positions(&bins, method, side);
                    for ((row, offset), bin) in rows.iter().zip(offsets).zip(&bins) {
                        x[*row] += offset * x_size;
                        y[*row] = middles[*bin];
                    }
                }
            }
        }
        swarms.finish(x, Some(y))
    }
}

/// The rows of each swarm, after the optional dodge.
pub(super) struct Swarms {
    /// The layer, dodged by `dodge_width` when given.
    pub data: LayerData,
    /// Rows of each swarm (finite x and y), in row order; swarms in order of first
    /// appearance.
    pub rows: Vec<Vec<usize>>,
    /// x of every row (0 without an x column), after dodging.
    pub x: Vec<f64>,
    /// y of every row.
    pub y: Vec<f64>,
    /// Whether `dodge_width` was given.
    pub dodged: bool,
}

impl Swarms {
    /// Dodge by `dodge_width` (as `position_dodge`, so swarms line up with dodged
    /// violins and boxplots), then split rows by x, and by group when dodged.
    pub fn of(data: LayerData, parameters: &Parameters) -> Result<Swarms> {
        let dodge_width = parameters.number("dodge_width")?;
        let data = match dodge_width {
            Some(width) => dodge(data, Some(width), None, false)?,
            None => data,
        };
        let column = |aesthetic: Aesthetic, default: f64| -> Vec<f64> {
            data.continuous(aesthetic).map_or_else(
                || vec![default; data.row_count()],
                |column| {
                    (0..data.row_count())
                        .map(|row| column.get(row).unwrap_or(f64::NAN))
                        .collect()
                },
            )
        };
        let x = column(Aesthetic::X, 0.0);
        let y = column(Aesthetic::Y, f64::NAN);
        let mut rows: Vec<Vec<usize>> = Vec::new();
        let mut slot_of_key: HashMap<(u64, u32), usize> = HashMap::new();
        for row in 0..data.row_count() {
            if !(x[row].is_finite() && y[row].is_finite()) {
                continue;
            }
            let group = match dodge_width {
                Some(_) => data.group_of(row),
                None => 0,
            };
            let slot = *slot_of_key
                .entry(((x[row] + 0.0).to_bits(), group))
                .or_insert_with(|| {
                    rows.push(Vec::new());
                    rows.len() - 1
                });
            rows[slot].push(row);
        }
        Ok(Swarms {
            data,
            rows,
            x,
            y,
            dodged: dodge_width.is_some(),
        })
    }

    /// The layer with the new x (and y), every row kept.
    pub fn finish(self, x: Vec<f64>, y: Option<Vec<f64>>) -> Result<LayerData> {
        let mut data = self.data;
        let continuous = |values| Column::Continuous(ContinuousColumn::from_values(values));
        if data.contains(Aesthetic::X) || !self.rows.is_empty() {
            data.insert(Aesthetic::X, continuous(x))?;
        }
        if let Some(y) = y {
            data.insert(Aesthetic::Y, continuous(y))?;
        }
        Ok(data)
    }
}

/// ggbeeswarm `get_range`: the extent of a scale range, its level count on a
/// discrete axis, 1 when empty or zero.
pub(super) fn scale_size(range: Option<(f64, f64)>, discrete: bool) -> f64 {
    let Some((low, high)) = range else {
        return 1.0;
    };
    let size = if discrete {
        high - low + 1.0
    } else {
        high - low
    };
    if size > 0.0 && size.is_finite() {
        size
    } else {
        1.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Method {
    Swarm,
    Compact,
    Center,
    Square,
    Hex,
}

const METHODS: &str = "'swarm', 'compactswarm', 'center', 'square', 'hex'";

impl Method {
    fn from_parameters(parameters: &Parameters) -> Result<Method> {
        match parameters.string("method")? {
            None | Some("swarm") => Ok(Method::Swarm),
            Some("compactswarm") => Ok(Method::Compact),
            Some("center" | "centre") => Ok(Method::Center),
            Some("square") => Ok(Method::Square),
            Some("hex") => Ok(Method::Hex),
            Some(other) => Err(Error::spec(
                parameters.path_of("method"),
                format!(
                    "unknown beeswarm method '{other}'; use one of {METHODS}{}",
                    did_you_mean(other, crate::suggest::quoted(METHODS))
                ),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Priority {
    Ascending,
    Descending,
    Density,
    Random,
    None,
}

const PRIORITIES: &str = "'ascending', 'descending', 'density', 'random', 'none'";

impl Priority {
    fn from_parameters(parameters: &Parameters) -> Result<Priority> {
        match parameters.string("priority")? {
            None | Some("ascending") => Ok(Priority::Ascending),
            Some("descending") => Ok(Priority::Descending),
            Some("density") => Ok(Priority::Density),
            Some("random") => Ok(Priority::Random),
            Some("none") => Ok(Priority::None),
            Some(other) => Err(Error::spec(
                parameters.path_of("priority"),
                format!(
                    "unknown priority '{other}'; use one of {PRIORITIES}{}",
                    did_you_mean(other, crate::suggest::quoted(PRIORITIES))
                ),
            )),
        }
    }

    /// Indices of `values` in placement order (stable, as R's `order`).
    fn order(self, values: &[f64], random: &mut KotlinRandom) -> Vec<usize> {
        let mut order: Vec<usize> = (0..values.len()).collect();
        match self {
            Priority::Ascending => order.sort_by(|a, b| values[*a].total_cmp(&values[*b])),
            Priority::Descending => order.sort_by(|a, b| values[*b].total_cmp(&values[*a])),
            Priority::None => {}
            Priority::Density => {
                if let Some(estimate) = RDensity::estimate(values, 512, 1.0) {
                    let density: Vec<f64> =
                        values.iter().map(|value| estimate.at(*value)).collect();
                    order.sort_by(|a, b| density[*b].total_cmp(&density[*a]));
                }
            }
            Priority::Random => {
                for index in (1..order.len()).rev() {
                    let other = random.next_int_until(index as i32 + 1) as usize;
                    order.swap(index, other);
                }
            }
        }
        order
    }
}

/// `side`: -1 (left), 0 (both) or 1 (right).
pub(super) fn side(parameters: &Parameters) -> Result<i8> {
    match parameters.number("side")? {
        None => Ok(0),
        Some(value) if value == -1.0 || value == 0.0 || value == 1.0 => Ok(value as i8),
        Some(value) => Err(Error::spec(
            parameters.path_of("side"),
            format!("side must be -1 (left), 0 (both sides) or 1 (right), got {value}"),
        )),
    }
}

/// A placed point of a swarm: data position in point diameters, offset, and
/// placement number.
#[derive(Clone, Copy)]
struct Placed {
    value: f64,
    offset: f64,
    number: usize,
}

/// Placed points sorted by value; neighbours closer than one diameter.
struct Neighbourhood {
    placed: Vec<Placed>,
}

impl Neighbourhood {
    fn add(&mut self, point: Placed) {
        let index = self
            .placed
            .partition_point(|other| other.value <= point.value);
        self.placed.insert(index, point);
    }

    /// Placed points within one diameter of `value` (exclusive), in placement order.
    fn near(&self, value: f64) -> Vec<Placed> {
        let start = self
            .placed
            .partition_point(|other| other.value <= value - 1.0);
        let mut near: Vec<Placed> = self.placed[start..]
            .iter()
            .take_while(|other| other.value < value + 1.0)
            .filter(|other| (value - other.value).abs() < 1.0)
            .copied()
            .collect();
        near.sort_by_key(|other| other.number);
        near
    }
}

/// beeswarm's swarm: in `order`, each point takes the candidate offset nearest
/// zero (zero, or touching an earlier neighbour above or below, per `side`)
/// that overlaps no earlier neighbour. Values and offsets are in diameters.
fn swarm(values: &[f64], order: &[usize], side: i8) -> Vec<f64> {
    let mut offsets = vec![0.0; values.len()];
    let mut placed = Neighbourhood {
        placed: Vec::with_capacity(values.len()),
    };
    for (number, &index) in order.iter().enumerate() {
        let value = values[index];
        let near = placed.near(value);
        let reach = |other: &Placed| (1.0 - (value - other.value).powi(2)).sqrt();
        let mut candidates = vec![0.0];
        if side >= 0 {
            candidates.extend(near.iter().map(|other| other.offset + reach(other)));
        }
        if side <= 0 {
            candidates.extend(near.iter().map(|other| other.offset - reach(other)));
        }
        // Only neighbours within one diameter of a candidate's offset can overlap it.
        let mut by_offset: Vec<(f64, f64)> = near
            .iter()
            .map(|other| (other.offset, other.value))
            .collect();
        by_offset.sort_by(|left, right| left.0.total_cmp(&right.0));
        let free = |candidate: f64| {
            let start = by_offset.partition_point(|(offset, _)| *offset <= candidate - 1.0);
            by_offset[start..]
                .iter()
                .take_while(|(offset, _)| *offset < candidate + 1.0)
                .all(|(offset, other)| {
                    (value - other).powi(2) + (candidate - offset).powi(2) >= 0.999
                })
        };
        // R's which.min: the first of the smallest, in candidate order. Trying
        // candidates by |offset| (then that order) stops at the first free one.
        let mut ranked: Vec<(usize, f64)> = candidates.into_iter().enumerate().collect();
        ranked.sort_by(|left, right| {
            left.1
                .abs()
                .total_cmp(&right.1.abs())
                .then(left.0.cmp(&right.0))
        });
        let offset = ranked
            .into_iter()
            .map(|(_, candidate)| candidate)
            .find(|candidate| free(*candidate))
            .unwrap_or(0.0);
        offsets[index] = offset;
        placed.add(Placed {
            value,
            offset,
            number,
        });
    }
    offsets
}

/// A candidate of the compact swarm: |best offset|, then placement priority.
#[derive(PartialEq)]
struct Candidate {
    distance: f64,
    rank: usize,
}

impl Eq for Candidate {}

impl Ord for Candidate {
    // Reversed: the heap pops the smallest distance, then the earliest rank.
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .distance
            .total_cmp(&self.distance)
            .then(other.rank.cmp(&self.rank))
    }
}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// beeswarm's compact swarm: repeatedly place the unplaced point whose best
/// offset is nearest zero (ties: earlier in `order`), then raise the upper and
/// lower bounds of its unplaced neighbours.
fn compact_swarm(values: &[f64], order: &[usize], side: i8) -> Vec<f64> {
    let count = order.len();
    // Per rank (position in `order`).
    let value: Vec<f64> = order.iter().map(|&index| values[index]).collect();
    let mut high = vec![0.0f64; count];
    let mut low = vec![0.0f64; count];
    let mut best = vec![0.0f64; count];
    let mut placed = vec![false; count];
    let mut offsets = vec![0.0; values.len()];
    // Ranks sorted by value, to find neighbours.
    let mut by_value: Vec<usize> = (0..count).collect();
    by_value.sort_by(|a, b| value[*a].total_cmp(&value[*b]));
    let sorted: Vec<f64> = by_value.iter().map(|rank| value[*rank]).collect();
    let mut heap: BinaryHeap<Candidate> = (0..count)
        .map(|rank| Candidate {
            distance: 0.0,
            rank,
        })
        .collect();
    while let Some(Candidate { distance, rank }) = heap.pop() {
        if placed[rank] || distance.to_bits() != best[rank].abs().to_bits() {
            continue;
        }
        placed[rank] = true;
        let offset = best[rank];
        offsets[order[rank]] = offset;
        let start = sorted.partition_point(|other| *other <= value[rank] - 1.0);
        for &neighbour in &by_value[start..] {
            let gap = (value[rank] - value[neighbour]).abs();
            if value[neighbour] >= value[rank] + 1.0 {
                break;
            }
            if placed[neighbour] || gap >= 1.0 {
                continue;
            }
            let reach = (1.0 - gap * gap).sqrt();
            high[neighbour] = high[neighbour].max(offset + reach);
            low[neighbour] = low[neighbour].min(offset - reach);
            best[neighbour] = match side {
                0 if -low[neighbour] < high[neighbour] => low[neighbour],
                0 | 1 => high[neighbour],
                _ => low[neighbour],
            };
            heap.push(Candidate {
                distance: best[neighbour].abs(),
                rank: neighbour,
            });
        }
    }
    offsets
}

/// ggbeeswarm's row grid for `center`, `square` and `hex`: rows `height` high
/// from the y range extended by 1% each side (R `extendrange`). Returns each
/// value's row (0-based; R's `cut(include.lowest = TRUE)`) and the row middles.
fn grid_rows(values: &[f64], range: (f64, f64), height: f64) -> (Vec<usize>, Vec<f64>) {
    let extension = 0.01 * (range.1 - range.0);
    let low = range.0 - extension;
    let to = range.1 + extension + height;
    // R seq(from, to, by): steps while within to (1e-10 fuzz), the last clipped.
    let steps = ((to - low) / height + 1e-10).floor().max(0.0) as usize;
    let breaks: Vec<f64> = (0..=steps)
        .map(|step| (low + step as f64 * height).min(to))
        .collect();
    let middles: Vec<f64> = breaks
        .windows(2)
        .map(|pair| (pair[0] + pair[1]) / 2.0)
        .collect();
    let last = middles.len().saturating_sub(1);
    let bins = values
        .iter()
        .map(|value| {
            let first_at_or_above = breaks.partition_point(|point| point < value);
            first_at_or_above.saturating_sub(1).min(last)
        })
        .collect();
    (bins, middles)
}

/// ggbeeswarm `determine_pos`: offsets, in point widths, of the points of each
/// row, numbered 1, 2, .. in row order and shifted per method and side.
fn row_positions(bins: &[usize], method: Method, side: i8) -> Vec<f64> {
    let mut size_of_bin: HashMap<usize, usize> = HashMap::new();
    let numbers: Vec<f64> = bins
        .iter()
        .map(|bin| {
            let size = size_of_bin.entry(*bin).or_insert(0);
            *size += 1;
            *size as f64
        })
        .collect();
    bins.iter()
        .zip(numbers)
        .map(|(bin, number)| {
            let size = size_of_bin[bin] as f64;
            let mean = (size + 1.0) / 2.0;
            // R's row names are 1-based, so the first row is odd.
            let odd = bin % 2 == 0;
            match (method, side) {
                (Method::Hex, 0) if odd => number - mean.floor() - 0.25,
                (Method::Hex, 0) => number - mean.ceil() + 0.25,
                (Method::Hex, -1) if odd => number - size,
                (Method::Hex, -1) => number - size - 0.5,
                (Method::Hex, _) if odd => number - 1.0,
                (Method::Hex, _) => number - 0.5,
                (_, -1) => number - size,
                (_, 1) => number - 1.0,
                (Method::Center, _) => number - mean,
                _ => number - mean.floor(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swarm_matches_beeswarm_swarmx() {
        // R: beeswarm::swarmx(rep(0, 6), c(1, 1, 1, 1.5, 3, 1.2), xsize = 1,
        //   ysize = 1, cex = 1)$x
        let values = [1.0, 1.0, 1.0, 1.5, 3.0, 1.2];
        let order = Priority::Ascending.order(&values, &mut KotlinRandom::new(0));
        let offsets = swarm(&values, &order, 0);
        let expected = [
            0.0,
            1.0,
            -1.0,
            -1.866_025_403_784_439,
            0.0,
            1.979_795_897_113_271,
        ];
        for (actual, expected) in offsets.iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-12, "{offsets:?}");
        }
    }

    #[test]
    fn rows_number_points_per_method() {
        let bins = [0, 0, 0, 1, 1];
        assert_eq!(
            row_positions(&bins, Method::Center, 0),
            [-1.0, 0.0, 1.0, -0.5, 0.5]
        );
        assert_eq!(
            row_positions(&bins, Method::Square, 0),
            [-1.0, 0.0, 1.0, 0.0, 1.0]
        );
        assert_eq!(
            row_positions(&bins, Method::Hex, 0),
            [-1.25, -0.25, 0.75, -0.75, 0.25]
        );
        assert_eq!(
            row_positions(&bins, Method::Square, -1),
            [-2.0, -1.0, 0.0, -1.0, 0.0]
        );
    }

    #[test]
    fn scale_sizes_count_discrete_levels() {
        assert_eq!(scale_size(Some((0.0, 2.0)), true), 3.0);
        assert_eq!(scale_size(Some((0.0, 2.0)), false), 2.0);
        assert_eq!(scale_size(Some((1.0, 1.0)), false), 1.0);
        assert_eq!(scale_size(None, false), 1.0);
    }
}
