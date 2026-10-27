//! Position adjustments: move layer data to avoid overlap (dodge, jitter, stack).
//! One file per position; add it to [`POSITIONS`].

mod dodge;
mod identity;
mod shift;
mod stack;

pub use dodge::DodgePosition;
pub use identity::IdentityPosition;
pub use stack::StackPosition;

use crate::Result;
use crate::data::LayerData;
use crate::parameters::Parameters;
use crate::spec::{Options, Value};

/// The position a layer gets when it names none (lets-plot `GeomProto` defaults).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DefaultPosition {
    /// Position name, e.g. `stack`.
    pub name: &'static str,
    /// `width` option (dodge).
    pub width: Option<f64>,
}

impl DefaultPosition {
    /// No adjustment.
    pub const IDENTITY: DefaultPosition = DefaultPosition::named("identity");

    /// A position without options.
    pub const fn named(name: &'static str) -> Self {
        DefaultPosition { name, width: None }
    }

    /// `position_dodge(width)`.
    pub const fn dodge(width: f64) -> Self {
        DefaultPosition {
            name: "dodge",
            width: Some(width),
        }
    }

    /// The position's options.
    pub fn options(&self) -> Options {
        let mut options = Options::new();
        if let Some(width) = self.width {
            options.insert("width".to_owned(), Value::from(width));
        }
        options
    }
}

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
pub static POSITIONS: &[&dyn Position] = &[&IdentityPosition, &StackPosition, &DodgePosition];

/// lets-plot positions the Python API builds but the engine does not apply yet
/// (plan.md). Remove a name when its position joins [`POSITIONS`].
pub static NOT_APPLIED_YET: &[&str] = &["fill", "jitter", "jitterdodge", "nudge"];

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
            assert!(!NOT_APPLIED_YET.contains(&name), "{name} is applied");
            assert_eq!(position(name).map(|item| item.name()), Some(name));
        }
    }
}
