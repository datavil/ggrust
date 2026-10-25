//! Stats: transform a layer's data before drawing (counts, densities, boxplot
//! statistics, ...). One file per stat; register it in [`stat`].

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

/// Look up a stat by its lets-plot name.
pub fn stat(name: &str) -> Option<&'static dyn Stat> {
    match name {
        "identity" => Some(&IdentityStat),
        _ => None,
    }
}

/// Names [`stat`] accepts, for error messages.
pub const NAMES: &[&str] = &["identity"];
