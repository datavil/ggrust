//! Cell velocities averaged onto a regular grid: a port of scVelo's
//! `compute_velocity_on_grid` (scvelo 0.3.4, `scvelo/plotting/velocity_embedding_grid.py`).
//!
//! Each grid point takes the Gaussian-weighted mean velocity of its `n_neighbors`
//! nearest cells (exact kNN, Euclidean, as scikit-learn's `NearestNeighbors`).
//! The stream variant (`adjust_for_stream`) then hides grid points with little
//! velocity mass or short neighbour velocities, as scVelo prepares the field for
//! matplotlib's `streamplot` (see [`crate::streamplot`]).
//!
//! What it does not do: more than two dimensions, scVelo's quiver `autoscale`
//! (it needs matplotlib), or float32 arithmetic (scVelo computes in the input
//! dtype; ggrust always uses float64). Distances are always exact kd-tree style;
//! scikit-learn switches to brute force (`|x|^2 - 2xy + |y|^2`, less accurate) when
//! `n_neighbors >= n_points // 2`, so results differ there (never with the default).

use rayon::prelude::*;

use crate::{Error, Result};

/// The options of scVelo's `compute_velocity_on_grid`, with scVelo's defaults.
#[derive(Debug, Clone, PartialEq)]
pub struct VelocityGridOptions {
    /// Grid points per dimension: `int(50 * density)`.
    pub density: f64,
    /// Gaussian kernel width, in units of the mean grid step.
    pub smooth: f64,
    /// Neighbours per grid point; `None` takes `int(n_points / 50)`.
    pub n_neighbors: Option<usize>,
    /// Mass threshold: `10 ** (min_mass - 6)` for streams, a fraction of the
    /// 99th mass percentile in percent otherwise.
    pub min_mass: f64,
    /// Stream variant only: grid points whose neighbour speed is below this
    /// percentile are hidden.
    pub cutoff_percentile: f64,
    /// Hide points as scVelo does for streamplot (`true`) or for quiver arrows.
    pub adjust_for_stream: bool,
}

impl Default for VelocityGridOptions {
    fn default() -> Self {
        VelocityGridOptions {
            density: 1.0,
            smooth: 0.5,
            n_neighbors: None,
            min_mass: 1.0,
            cutoff_percentile: 5.0,
            adjust_for_stream: true,
        }
    }
}

/// Velocities on a regular grid, laid out for [`crate::streamplot::streamplot`]:
/// `u[row][column]` is the x component at `(x[column], y[row])`. Hidden grid
/// points are NaN: in `u` only for streams (as scVelo), in `u` and `v` otherwise.
#[derive(Debug, Clone, PartialEq)]
pub struct VelocityGrid {
    /// Grid x coordinates, increasing (constant if the points do not spread in x,
    /// accepted for quiver grids only).
    pub x: Vec<f64>,
    /// Grid y coordinates, increasing (or constant, as `x`).
    pub y: Vec<f64>,
    /// x component of the velocity, one row per `y`.
    pub u: Vec<Vec<f64>>,
    /// y component of the velocity, one row per `y`.
    pub v: Vec<Vec<f64>>,
}

/// Largest grid accepted (`density` about 63): each grid point scans every cell.
const MAX_GRID_POINTS: usize = 10_000_000;

/// What one grid point gathers from its neighbours.
struct GridPoint {
    u: f64,
    v: f64,
    mass: f64,
    length: f64,
}

/// Averages the velocities `vectors` of the cells at `points` onto a grid.
/// Rows with a non-finite coordinate or velocity are dropped first.
pub fn velocity_grid(
    points: &[[f64; 2]],
    vectors: &[[f64; 2]],
    options: &VelocityGridOptions,
) -> Result<VelocityGrid> {
    if points.len() != vectors.len() {
        return Err(Error::spec(
            "vectors",
            format!(
                "expected one velocity per point ({} points), got {} velocities",
                points.len(),
                vectors.len()
            ),
        ));
    }
    validate(options)?;
    let (points, vectors): (Vec<[f64; 2]>, Vec<[f64; 2]>) = points
        .iter()
        .zip(vectors)
        .filter(|(point, vector)| {
            // np.isfinite(X.sum(1) + V.sum(1)): also drops rows whose sum overflows.
            (point[0] + point[1] + (vector[0] + vector[1])).is_finite()
        })
        .map(|(point, vector)| (*point, *vector))
        .unzip();
    let n_points = points.len();
    let n_neighbors = match options.n_neighbors {
        Some(n) => n,
        None => n_points / 50,
    };
    if n_neighbors == 0 || n_neighbors > n_points {
        let default_note = if options.n_neighbors.is_none() {
            format!(" (the default is int(n_points / 50) = {n_neighbors})")
        } else {
            String::new()
        };
        return Err(Error::spec(
            "n_neighbors",
            format!(
                "n_neighbors must be between 1 and the number of finite points ({n_points}), \
                 got {n_neighbors}{default_note}; pass n_neighbors in that range"
            ),
        ));
    }

    let size = (50.0 * options.density) as usize;
    let x = grid_axis(&points, 0, size);
    let y = grid_axis(&points, 1, size);
    for (name, axis) in [("x", &x), ("y", &y)] {
        let spread = axis[1] - axis[0];
        // Streams need an increasing grid (scVelo fails on a flat axis there too).
        if !spread.is_finite() || (options.adjust_for_stream && spread <= 0.0) {
            return Err(Error::spec(
                "points",
                format!(
                    "the points have no usable spread in {name} (grid from {} to {}); \
                     a stream grid needs at least two distinct finite {name} values",
                    axis[0],
                    axis[size - 1]
                ),
            ));
        }
    }
    let scale = ((x[1] - x[0]) + (y[1] - y[0])) / 2.0 * options.smooth;
    if !(scale > 0.0 && scale.is_finite()) {
        return Err(Error::spec(
            "points",
            format!(
                "the kernel width (mean grid step x smooth) is {scale}; the points need \
                 a finite spread in x or y"
            ),
        ));
    }

    // Row-major over the meshgrid: grid point `row * size + column` is (x[column], y[row]).
    let grid: Vec<GridPoint> = (0..size * size)
        .into_par_iter()
        .map_init(
            || Vec::with_capacity(n_points),
            |buffer, index| {
                let query = [x[index % size], y[index / size]];
                grid_point(query, &points, &vectors, n_neighbors, scale, buffer)
            },
        )
        .collect();

    let mut u: Vec<f64> = grid.iter().map(|point| point.u).collect();
    let mut v: Vec<f64> = grid.iter().map(|point| point.v).collect();
    if options.adjust_for_stream {
        let speed: Vec<f64> = u
            .iter()
            .zip(&v)
            .map(|(u, v)| (u * u + v * v).sqrt())
            .collect();
        let max_speed = speed.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        // np.clip(10 ** (min_mass - 6), None, 0.9 * max): a NaN bound wins.
        let bound = max_speed * 0.9;
        let raw = 10f64.powf(options.min_mass - 6.0);
        let min_speed = if bound.is_nan() || raw > bound {
            bound
        } else {
            raw
        };
        let lengths: Vec<f64> = grid.iter().map(|point| point.length).collect();
        let min_length = percentile(&lengths, options.cutoff_percentile);
        for (index, point) in grid.iter().enumerate() {
            if speed[index] < min_speed || point.length < min_length {
                u[index] = f64::NAN;
            }
        }
    } else {
        let masses: Vec<f64> = grid.iter().map(|point| point.mass).collect();
        let threshold = options.min_mass * (percentile(&masses, 99.0) / 100.0);
        for (index, point) in grid.iter().enumerate() {
            // scVelo keeps `p_mass > min_mass`: anything else (NaN too) is dropped.
            if point.mass.partial_cmp(&threshold) != Some(std::cmp::Ordering::Greater) {
                u[index] = f64::NAN;
                v[index] = f64::NAN;
            }
        }
    }
    Ok(VelocityGrid {
        x,
        y,
        u: u.chunks(size).map(<[f64]>::to_vec).collect(),
        v: v.chunks(size).map(<[f64]>::to_vec).collect(),
    })
}

fn validate(options: &VelocityGridOptions) -> Result<()> {
    if !(options.density.is_finite() && 50.0 * options.density >= 2.0) {
        return Err(Error::spec(
            "density",
            format!(
                "density must be a finite number of at least 0.04 (the grid has \
                 int(50 * density) points per axis, at least 2), got {}",
                options.density
            ),
        ));
    }
    if !(options.smooth.is_finite() && options.smooth > 0.0) {
        return Err(Error::spec(
            "smooth",
            format!(
                "smooth must be a positive finite number, got {}",
                options.smooth
            ),
        ));
    }
    let size = (50.0 * options.density) as usize;
    if size.saturating_mul(size) > MAX_GRID_POINTS {
        return Err(Error::spec(
            "density",
            format!(
                "density {} gives a {size} x {size} grid, more than {MAX_GRID_POINTS} grid \
                 points; pass a density of at most 63",
                options.density
            ),
        ));
    }
    if !options.min_mass.is_finite() {
        return Err(Error::spec(
            "min_mass",
            format!("min_mass must be a finite number, got {}", options.min_mass),
        ));
    }
    if !(0.0..=100.0).contains(&options.cutoff_percentile) {
        return Err(Error::spec(
            "cutoff_percentile",
            format!(
                "cutoff_percentile must be between 0 and 100, got {}",
                options.cutoff_percentile
            ),
        ));
    }
    Ok(())
}

/// `np.linspace(m, M, size)` over the points' range on `dimension`, widened by
/// 1 percent on each side as scVelo does (`M` widens from the widened `m`).
fn grid_axis(points: &[[f64; 2]], dimension: usize, size: usize) -> Vec<f64> {
    let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
    for point in points {
        low = low.min(point[dimension]);
        high = high.max(point[dimension]);
    }
    let low = low - 0.01 * (high - low).abs();
    let high = high + 0.01 * (high - low).abs();
    let delta = high - low;
    let divisor = (size - 1) as f64;
    let step = delta / divisor;
    let mut axis: Vec<f64> = if step == 0.0 {
        // numpy's zero-step branch: `i / div * delta`.
        (0..size)
            .map(|index| index as f64 / divisor * delta + low)
            .collect()
    } else {
        (0..size).map(|index| index as f64 * step + low).collect()
    };
    axis[size - 1] = high;
    axis
}

/// The kernel-weighted velocity of the `n_neighbors` cells nearest to `query`.
fn grid_point(
    query: [f64; 2],
    points: &[[f64; 2]],
    vectors: &[[f64; 2]],
    n_neighbors: usize,
    scale: f64,
    buffer: &mut Vec<(f64, u32)>,
) -> GridPoint {
    buffer.clear();
    // Squared distances (as scikit-learn's kd-tree ranks them), ties by index.
    buffer.extend(points.iter().zip(0u32..).map(|(point, index)| {
        let dx = query[0] - point[0];
        let dy = query[1] - point[1];
        (dx * dx + dy * dy, index)
    }));
    let order = |a: &(f64, u32), b: &(f64, u32)| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1));
    if n_neighbors < buffer.len() {
        buffer.select_nth_unstable_by(n_neighbors - 1, order);
    }
    let neighbours = &mut buffer[..n_neighbors];
    neighbours.sort_unstable_by(order);

    let mut weights = Vec::with_capacity(n_neighbors);
    let (mut u, mut v, mut length_u, mut length_v) = (0.0, 0.0, 0.0, 0.0);
    for &(_, index) in neighbours.iter() {
        let point = points[index as usize];
        let dx = query[0] - point[0];
        let dy = query[1] - point[1];
        // scikit-learn's kd-tree distance, contracted to a fused multiply-add as
        // clang compiles it on arm64 (the ranking above skips the FMA for speed).
        let distance = dy.mul_add(dy, dx * dx).sqrt();
        // scipy.stats.norm.pdf(distance, scale=scale)
        let z = distance / scale;
        let weight = (-(z * z) / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt() / scale;
        weights.push(weight);
        let vector = vectors[index as usize];
        u += vector[0] * weight;
        v += vector[1] * weight;
        length_u += vector[0].abs();
        length_v += vector[1].abs();
    }
    let mass = pairwise_sum(&weights);
    let divisor = mass.max(1.0);
    let count = n_neighbors as f64;
    GridPoint {
        u: u / divisor,
        v: v / divisor,
        mass,
        length: length_u / count + length_v / count,
    }
}

/// numpy's `add.reduce` over a contiguous axis: a pairwise sum (blocks of 8
/// up to 128 values, sequential below 8).
fn pairwise_sum(values: &[f64]) -> f64 {
    fn pairwise(values: &[f64]) -> f64 {
        let n = values.len();
        if n < 8 {
            values.iter().fold(0.0, |sum, value| sum + value)
        } else if n <= 128 {
            let mut partial = [0.0; 8];
            partial.copy_from_slice(&values[..8]);
            let blocks = n - n % 8;
            for block in values[8..blocks].chunks_exact(8) {
                for (sum, value) in partial.iter_mut().zip(block) {
                    *sum += value;
                }
            }
            let mut sum = ((partial[0] + partial[1]) + (partial[2] + partial[3]))
                + ((partial[4] + partial[5]) + (partial[6] + partial[7]));
            for value in &values[blocks..] {
                sum += value;
            }
            sum
        } else {
            let half = n / 2;
            let half = half - half % 8;
            pairwise(&values[..half]) + pairwise(&values[half..])
        }
    }
    pairwise(values)
}

/// `np.percentile(values, percent)` with numpy's default linear method.
fn percentile(values: &[f64], percent: f64) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable_by(f64::total_cmp);
    let n = sorted.len();
    let virtual_index = (n - 1) as f64 * (percent / 100.0);
    if virtual_index >= (n - 1) as f64 {
        return sorted[n - 1];
    }
    let previous = virtual_index.floor();
    let gamma = virtual_index - previous;
    let a = sorted[previous as usize];
    let b = sorted[previous as usize + 1];
    // numpy's _lerp: from the nearer end.
    let difference = b - a;
    if gamma >= 0.5 {
        b - difference * (1.0 - gamma)
    } else {
        a + difference * gamma
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linspace_matches_numpy_formula() {
        let points = [[0.0, 0.0], [1.0, 3.0]];
        let axis = grid_axis(&points, 0, 50);
        assert_eq!(axis[0], -0.01);
        assert_eq!(axis[49], 1.0 + 0.01 * 1.01);
        assert_eq!(axis[1], (axis[49] - axis[0]) / 49.0 + axis[0]);
    }

    #[test]
    fn percentile_matches_numpy() {
        let values = [5.0, 1.0, 4.0, 2.0, 3.0];
        assert_eq!(percentile(&values, 50.0), 3.0);
        assert_eq!(percentile(&values, 0.0), 1.0);
        assert_eq!(percentile(&values, 100.0), 5.0);
        assert_eq!(percentile(&values, 10.0), 1.4);
    }

    #[test]
    fn pairwise_sum_matches_sequential_on_small_inputs() {
        let values: Vec<f64> = (0..300).map(|index| index as f64 * 0.5).collect();
        assert_eq!(pairwise_sum(&values), values.iter().sum::<f64>());
        assert_eq!(pairwise_sum(&[]), 0.0);
    }

    fn uniform_field(n: usize) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
        let points: Vec<[f64; 2]> = (0..n)
            .map(|index| [(index % 20) as f64, (index / 20) as f64 * 0.7])
            .collect();
        let vectors = vec![[1.0, -2.0]; n];
        (points, vectors)
    }

    #[test]
    fn uniform_velocities_stay_uniform() {
        let (points, vectors) = uniform_field(400);
        let grid = velocity_grid(&points, &vectors, &VelocityGridOptions::default()).unwrap();
        assert_eq!(grid.x.len(), 50);
        assert_eq!(grid.u.len(), 50);
        for (row_u, row_v) in grid.u.iter().zip(&grid.v) {
            for (&u, &v) in row_u.iter().zip(row_v) {
                // the kernel mass may be below 1, which shrinks the mean
                assert!(u.is_nan() || (u > 0.0 && u <= 1.0 + 1e-12));
                assert!((-2.0 - 1e-12..0.0).contains(&v));
            }
        }
    }

    #[test]
    fn non_finite_rows_are_dropped() {
        let (mut points, mut vectors) = uniform_field(400);
        let clean = velocity_grid(&points, &vectors, &VelocityGridOptions::default()).unwrap();
        points.push([f64::NAN, 1.0]);
        vectors.push([0.0, 0.0]);
        points.push([1.0, 1.0]);
        vectors.push([f64::INFINITY, 0.0]);
        let options = VelocityGridOptions {
            n_neighbors: Some(8),
            ..VelocityGridOptions::default()
        };
        let dirty = velocity_grid(&points, &vectors, &options).unwrap();
        assert_eq!(dirty.x, clean.x);
        assert_eq!(dirty.y, clean.y);
    }

    #[test]
    fn errors_name_the_parameter() {
        let (points, vectors) = uniform_field(40);
        let path = |options: VelocityGridOptions| match velocity_grid(&points, &vectors, &options) {
            Err(Error::InvalidSpec { path, .. }) => path,
            other => panic!("expected a spec error, got {other:?}"),
        };
        let default = VelocityGridOptions::default;
        assert_eq!(path(default()), "n_neighbors");
        let with_neighbors = || VelocityGridOptions {
            n_neighbors: Some(4),
            ..default()
        };
        assert_eq!(
            path(VelocityGridOptions {
                density: 0.01,
                ..with_neighbors()
            }),
            "density"
        );
        assert_eq!(
            path(VelocityGridOptions {
                smooth: 0.0,
                ..with_neighbors()
            }),
            "smooth"
        );
        assert_eq!(
            path(VelocityGridOptions {
                cutoff_percentile: 101.0,
                ..with_neighbors()
            }),
            "cutoff_percentile"
        );
        assert!(matches!(
            velocity_grid(&points, &vectors[1..], &with_neighbors()),
            Err(Error::InvalidSpec { path, .. }) if path == "vectors"
        ));
        let flat: Vec<[f64; 2]> = points.iter().map(|point| [point[0], 1.0]).collect();
        assert!(matches!(
            velocity_grid(&flat, &vectors, &with_neighbors()),
            Err(Error::InvalidSpec { path, .. }) if path == "points"
        ));
        assert_eq!(
            path(VelocityGridOptions {
                density: 1e9,
                ..with_neighbors()
            }),
            "density"
        );
        // quiver mode accepts a flat axis, as scVelo
        let quiver = VelocityGridOptions {
            adjust_for_stream: false,
            ..with_neighbors()
        };
        let grid = velocity_grid(&flat, &vectors, &quiver).unwrap();
        assert!(grid.y.iter().all(|&y| y == 1.0));
    }
}
