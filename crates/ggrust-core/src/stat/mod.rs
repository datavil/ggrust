//! Stats: transform a layer's data before drawing (counts, densities, boxplot
//! statistics, ...). One file per stat; add it to [`STATS`].

mod bin;
mod boxplot;
mod count;
mod density;
pub(crate) mod groups;
mod identity;
mod ydensity;

pub use bin::{BinStat, bracketing_index};
pub use boxplot::{BoxplotOutlierStat, BoxplotStat};
pub use count::CountStat;
pub use identity::IdentityStat;
pub use ydensity::{YDensityStat, quantiles as ydensity_quantiles};

use crate::Result;
use crate::data::{Computed, LayerData};
use crate::parameters::Parameters;
use crate::spec::Aesthetic;

/// What a stat may know beyond its own layer (lets-plot `StatContext`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StatContext {
    /// x range over every layer's data, in axis units, with explicit scale
    /// limits replacing its ends (lets-plot `ConfiguredStatContext.overallXRange`).
    pub x_range: Option<(f64, f64)>,
    /// The same for y (`overallYRange`).
    pub y_range: Option<(f64, f64)>,
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
    &BoxplotStat,
    &BoxplotOutlierStat,
    &YDensityStat,
];

/// lets-plot stats the Python API builds but the engine does not compute yet
/// (plan.md). Remove a name when its stat joins [`STATS`].
pub static NOT_COMPUTED_YET: &[&str] = &["densityridges", "sina"];

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
