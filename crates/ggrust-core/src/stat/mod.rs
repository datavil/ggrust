//! Stats: transform a layer's data before drawing (counts, densities, boxplot
//! statistics, ...). One file per stat; add it to [`STATS`].

mod identity;

pub use identity::IdentityStat;

use crate::Result;
use crate::data::LayerData;
use crate::parameters::Parameters;

/// A statistical transformation.
pub trait Stat: Sync {
    /// lets-plot name, e.g. `identity`, `count`.
    fn name(&self) -> &'static str;

    /// Layer options this stat reads (e.g. `bins`).
    fn parameters(&self) -> &'static [&'static str] {
        &[]
    }

    /// Transform the layer data.
    fn compute(&self, data: LayerData, parameters: &Parameters) -> Result<LayerData>;
}

/// Every stat, in one place: lookup and names derive from it.
pub static STATS: &[&dyn Stat] = &[&IdentityStat];

/// lets-plot stats the Python API builds but the engine does not compute yet
/// (plan.md). Remove a name when its stat joins [`STATS`].
pub static NOT_COMPUTED_YET: &[&str] = &[
    "bin",
    "boxplot",
    "boxplot_outlier",
    "count",
    "densityridges",
    "sina",
    "ydensity",
];

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
