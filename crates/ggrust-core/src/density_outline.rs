//! Outlines of a 2D point cloud: where its kernel density falls to a fraction of
//! the peak.
//!
//! The density is `scipy.stats.gaussian_kde` (full covariance, see [`crate::kde`])
//! evaluated on a `grid_size x grid_size` grid spanning the points' bounding box
//! widened by `padding` (data units) on every side (`np.linspace` axes). The
//! outline is the contour at `level * max(density)`, traced by marching squares
//! as `skimage.measure.find_contours` (see [`crate::marching_squares`]), and its
//! grid-index vertices are mapped back to data coordinates as `np.interp` over
//! the axes. This reproduces the scipy + scikit-image recipe, to about 1e-15.
//!
//! What it does not do: groups (call it once per point set), weights, several
//! levels, or filled bands (`stat="density2df"` draws those, with lets-plot's
//! bandwidths and contour engine).

use crate::kde::{BandwidthMethod, GaussianKde};
use crate::marching_squares::find_contours;
use crate::numeric::{interp_index, linspace};
use crate::{Error, Result};

/// The largest `grid_size * grid_size`.
const MAX_GRID_POINTS: usize = 10_000_000;

/// Options of [`density_outline`].
#[derive(Debug, Clone, PartialEq)]
pub struct DensityOutlineOptions {
    /// Outline level as a fraction of the peak density, `0 < level < 1`.
    pub level: f64,
    /// Data units added to each side of the bounding box before gridding.
    pub padding: f64,
    /// Grid points per axis (at least 2).
    pub grid_size: usize,
    /// Kernel bandwidth rule.
    pub bandwidth: BandwidthMethod,
}

impl Default for DensityOutlineOptions {
    fn default() -> Self {
        DensityOutlineOptions {
            level: 0.1,
            padding: 1.0,
            grid_size: 200,
            bandwidth: BandwidthMethod::Scott,
        }
    }
}

/// Outline paths: one row per vertex, `path` numbers the contours from 0 in
/// scikit-image's order. Closed outlines repeat their first vertex.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DensityOutline {
    /// Vertex x coordinates (data units).
    pub x: Vec<f64>,
    /// Vertex y coordinates (data units).
    pub y: Vec<f64>,
    /// Contour number of each vertex.
    pub path: Vec<u32>,
}

/// The outline of `points` at `options.level` of the peak density. Rows with a
/// non-finite coordinate are dropped first.
pub fn density_outline(
    points: &[[f64; 2]],
    options: &DensityOutlineOptions,
) -> Result<DensityOutline> {
    validate(options)?;
    let points: Vec<[f64; 2]> = points
        .iter()
        .filter(|point| point[0].is_finite() && point[1].is_finite())
        .copied()
        .collect();
    let kde = GaussianKde::new(&points, options.bandwidth, "points")?;

    let axis = |dimension: usize| {
        let (low, high) = points
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), point| {
                (low.min(point[dimension]), high.max(point[dimension]))
            });
        linspace(
            low - options.padding,
            high + options.padding,
            options.grid_size,
        )
    };
    let x_axis = axis(0);
    let y_axis = axis(1);
    if x_axis.iter().chain(&y_axis).any(|value| !value.is_finite()) {
        return Err(Error::spec(
            "padding",
            "the padded grid overflows a float (points or padding too large); \
             rescale the points or pass a smaller padding",
        ));
    }
    // np.meshgrid(x, y), flattened row-major: rows follow y.
    let queries: Vec<[f64; 2]> = y_axis
        .iter()
        .flat_map(|&y| x_axis.iter().map(move |&x| [x, y]))
        .collect();
    let density = kde.evaluate(&queries);
    let peak = density.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let threshold = peak * options.level;
    let contours = find_contours(&density, options.grid_size, options.grid_size, threshold);

    let mut outline = DensityOutline::default();
    let total: usize = contours.iter().map(Vec::len).sum();
    outline.x.reserve_exact(total);
    outline.y.reserve_exact(total);
    outline.path.reserve_exact(total);
    for (number, contour) in (0u32..).zip(&contours) {
        for &[row, column] in contour {
            outline.x.push(interp_index(column, &x_axis));
            outline.y.push(interp_index(row, &y_axis));
            outline.path.push(number);
        }
    }
    Ok(outline)
}

fn validate(options: &DensityOutlineOptions) -> Result<()> {
    if !(options.level > 0.0 && options.level < 1.0) {
        return Err(Error::spec(
            "level",
            format!(
                "level is a fraction of the peak density and must be between 0 and 1 \
                 (exclusive), got {}",
                options.level
            ),
        ));
    }
    if !(options.padding.is_finite() && options.padding >= 0.0) {
        return Err(Error::spec(
            "padding",
            format!(
                "padding must be a finite number of at least 0 (data units), got {}",
                options.padding
            ),
        ));
    }
    let size = options.grid_size;
    if size < 2 || size.saturating_mul(size) > MAX_GRID_POINTS {
        return Err(Error::spec(
            "grid_size",
            format!("grid_size must be between 2 and 3162 (grid points per axis), got {size}"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cloud(n: usize) -> Vec<[f64; 2]> {
        (0..n)
            .map(|i| {
                let angle = i as f64 * 2.399_963;
                let radius = (i as f64 / n as f64).sqrt();
                [radius * angle.cos() * 2.0 + 5.0, radius * angle.sin() - 3.0]
            })
            .collect()
    }

    #[test]
    fn a_round_cloud_has_one_closed_outline_around_it() {
        let options = DensityOutlineOptions {
            grid_size: 60,
            ..DensityOutlineOptions::default()
        };
        let outline = density_outline(&cloud(300), &options).unwrap();
        assert!(outline.path.iter().all(|&path| path == 0));
        let last = outline.x.len() - 1;
        assert_eq!(
            (outline.x[0], outline.y[0]),
            (outline.x[last], outline.y[last])
        );
        let x_low = outline.x.iter().copied().fold(f64::INFINITY, f64::min);
        let x_high = outline.x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        assert!(x_low < 3.0 && x_high > 7.0, "{x_low} {x_high}");
    }

    #[test]
    fn non_finite_rows_are_dropped() {
        let mut points = cloud(100);
        let options = DensityOutlineOptions {
            grid_size: 30,
            ..DensityOutlineOptions::default()
        };
        let clean = density_outline(&points, &options).unwrap();
        points.push([f64::NAN, 1.0]);
        points.insert(3, [f64::INFINITY, 0.0]);
        assert_eq!(density_outline(&points, &options).unwrap(), clean);
    }

    #[test]
    fn errors_name_the_parameter() {
        let points = cloud(20);
        for (options, path) in [
            (
                DensityOutlineOptions {
                    level: 1.0,
                    ..DensityOutlineOptions::default()
                },
                "level",
            ),
            (
                DensityOutlineOptions {
                    padding: -1.0,
                    ..DensityOutlineOptions::default()
                },
                "padding",
            ),
            (
                DensityOutlineOptions {
                    grid_size: 1,
                    ..DensityOutlineOptions::default()
                },
                "grid_size",
            ),
        ] {
            let error = density_outline(&points, &options).unwrap_err();
            assert!(error.to_string().contains(path), "{error}");
        }
    }
}
