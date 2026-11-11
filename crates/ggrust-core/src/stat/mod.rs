//! Stats: transform a layer's data before drawing (counts, densities, boxplot
//! statistics, ...). One file per stat; add it to [`STATS`].

mod beeswarm;
mod bin;
mod bin2d;
mod binhex;
mod boxplot;
mod centroid;
mod compare_means;
mod contour;
mod cor;
mod count;
mod density;
mod density2d;
mod density_1d;
mod densityridges;
pub(crate) mod distribution;
mod dotplot;
mod ecdf;
mod ellipse;
mod ggdist_density;
pub(crate) mod groups;
mod hull;
mod hypothesis;
mod identity;
mod pointdensity;
mod pointinterval;
mod poly_eq;
mod pubr_labels;
mod qq;
mod quantile;
mod quasirandom;
mod r_density;
mod sina;
mod slab;
mod smooth;
mod streamline;
mod sum;
mod summary;
mod summary_2d;
mod unique;
mod ydensity;

pub use beeswarm::BeeswarmStat;
pub use bin::{BinStat, bracketing_index};
pub use bin2d::Bin2dStat;
pub use binhex::BinHexStat;
pub use boxplot::{BoxplotOutlierStat, BoxplotStat};
pub use centroid::{CentroidStat, SummaryXyStat};
pub use compare_means::CompareMeansStat;
pub use contour::{ContourStat, ContourfStat};
pub use cor::CorStat;
pub use count::CountStat;
pub use density_1d::DensityStat;
pub use density2d::{Density2dStat, Density2dfStat};
pub use densityridges::DensityRidgesStat;
pub use dotplot::{DotplotStat, YDotplotStat};
pub use ecdf::EcdfStat;
pub use ellipse::EllipseStat;
pub use hull::HullStat;
pub use identity::IdentityStat;
pub use pointdensity::PointDensityStat;
pub use pointinterval::{IntervalStat, PointIntervalStat};
pub use poly_eq::PolyEqStat;
pub use qq::{Qq2LineStat, Qq2Stat, QqLineStat, QqStat};
pub use quantile::QuantileStat;
pub use quasirandom::QuasirandomStat;
pub use sina::SinaStat;
pub use slab::SlabStat;
pub use smooth::SmoothStat;
pub use streamline::StreamlineStat;
pub use sum::SumStat;
pub use summary::{SummaryBinStat, SummaryStat};
pub use summary_2d::Summary2dStat;
pub use unique::UniqueStat;
pub use ydensity::{YDensityStat, quantiles as ydensity_quantiles};

use crate::Result;
use crate::data::{Computed, LayerData};
use crate::parameters::Parameters;
use crate::spec::Aesthetic;

/// What a stat may know beyond its own layer (lets-plot `StatContext`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StatContext<'a> {
    /// x range over every layer's data, in axis units, with explicit scale
    /// limits replacing its ends (lets-plot `ConfiguredStatContext.overallXRange`).
    pub x_range: Option<(f64, f64)>,
    /// The same for y (`overallYRange`).
    pub y_range: Option<(f64, f64)>,
    /// Categories of a discrete x axis in axis order (position 0 first), so
    /// stats can resolve level names given as options (`ref_group`).
    pub x_levels: Option<&'a [&'a str]>,
}

/// A statistical transformation.
pub trait Stat: Sync {
    /// lets-plot name, e.g. `identity`, `count`.
    fn name(&self) -> &'static str;

    /// Layer options this stat reads (e.g. `bins`).
    fn parameters(&self) -> &'static [&'static str] {
        &[]
    }

    /// Aesthetics the stat reads besides the geom's (e.g. `weight`).
    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[]
    }

    /// Aesthetics that must be mapped (or constant) for the stat to run.
    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[]
    }

    /// Computed variables drawn by default when the layer does not map the
    /// aesthetic itself (lets-plot `Stat.getDefaultMapping`), e.g. y = `..count..`.
    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        &[]
    }

    /// Position for layers that name none, over the geom's (lets-plot gives the
    /// outlier points of a boxplot the boxes' dodge).
    fn default_position(&self) -> Option<crate::position::DefaultPosition> {
        None
    }

    /// Transform the layer data. Positions are in axis units (transformed numbers,
    /// category indices).
    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        context: &StatContext,
    ) -> Result<LayerData>;
}

/// Every stat, in one place: lookup and names derive from it.
pub static STATS: &[&dyn Stat] = &[
    &IdentityStat,
    &CountStat,
    &BinStat,
    &DotplotStat,
    &YDotplotStat,
    &BoxplotStat,
    &BoxplotOutlierStat,
    &DensityStat,
    &YDensityStat,
    &DensityRidgesStat,
    &SinaStat,
    &BeeswarmStat,
    &QuasirandomStat,
    &SmoothStat,
    &SummaryStat,
    &SummaryBinStat,
    &EcdfStat,
    &QuantileStat,
    &EllipseStat,
    &SumStat,
    &Summary2dStat,
    &UniqueStat,
    &HullStat,
    &Bin2dStat,
    &BinHexStat,
    &ContourStat,
    &ContourfStat,
    &Density2dStat,
    &Density2dfStat,
    &PointDensityStat,
    &SlabStat,
    &PointIntervalStat,
    &IntervalStat,
    &CompareMeansStat,
    &CorStat,
    &PolyEqStat,
    &StreamlineStat,
    &CentroidStat,
    &SummaryXyStat,
    &QqStat,
    &QqLineStat,
    &Qq2Stat,
    &Qq2LineStat,
];

/// lets-plot stats the Python API builds but the engine does not compute yet
/// (plan.md). Remove a name when its stat joins [`STATS`].
pub static NOT_COMPUTED_YET: &[&str] = &[];

/// Look up a stat by its lets-plot name.
pub fn stat(name: &str) -> Option<&'static dyn Stat> {
    STATS.iter().copied().find(|item| item.name() == name)
}

/// Names of all stats, for error messages.
pub fn names() -> Vec<&'static str> {
    STATS.iter().map(|item| item.name()).collect()
}

#[cfg(test)]
mod registry_tests {
    use super::*;

    #[test]
    fn registry_names_round_trip() {
        for name in names() {
            assert!(!NOT_COMPUTED_YET.contains(&name), "{name} is computed");
            assert_eq!(stat(name).map(|item| item.name()), Some(name));
        }
    }
}
