//! Position adjustments: move layer data to avoid overlap (dodge, jitter, stack).
//! One file per position; add it to [`POSITIONS`].

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

/// Every position, in one place: lookup and names derive from it.
pub static POSITIONS: &[&dyn Position] = &[&IdentityPosition];

/// Look up a position by its lets-plot name.
pub fn position(name: &str) -> Option<&'static dyn Position> {
    POSITIONS.iter().copied().find(|item| item.name() == name)
}

/// Names of all positions, for error messages.
pub fn names() -> Vec<&'static str> {
    POSITIONS.iter().map(|item| item.name()).collect()
}

#[cfg(test)]
mod registry_tests {
    use super::*;

    #[test]
    fn registry_names_round_trip() {
        for name in names() {
            assert_eq!(position(name).map(|item| item.name()), Some(name));
        }
    }
}
