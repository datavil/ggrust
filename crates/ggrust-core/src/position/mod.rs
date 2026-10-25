//! Position adjustments: move layer data to avoid overlap (dodge, jitter, stack).
//! One file per position; register it in [`position`].

mod identity;

pub use identity::IdentityPosition;

use crate::Result;
use crate::data::LayerData;
use crate::parameters::Parameters;

/// A position adjustment.
pub trait Position: Sync {
    /// lets-plot name, e.g. `identity`, `dodge`.
    fn name(&self) -> &'static str;

    /// Options this position reads (e.g. `width`, `seed`).
    fn parameters(&self) -> &'static [&'static str] {
        &[]
    }

    /// Adjust positions.
    fn adjust(&self, data: LayerData, parameters: &Parameters) -> Result<LayerData>;
}

/// Look up a position by its lets-plot name.
pub fn position(name: &str) -> Option<&'static dyn Position> {
    match name {
        "identity" => Some(&IdentityPosition),
        _ => None,
    }
}

/// Names [`position`] accepts, for error messages.
pub const NAMES: &[&str] = &["identity"];
