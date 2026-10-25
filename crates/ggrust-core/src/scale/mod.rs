//! Scales: map data values to positions (0..1 within the panel) or to visual values.
//!
//! Position scales are continuous (linear or transformed) or discrete
//! ([`PositionScale`]). Color/size scales are added in Phase 1 (see plan.md).

pub mod breaks;
mod continuous;
mod discrete;
pub mod format;
mod position;
mod spec;
mod transform;

pub use continuous::{ContinuousPositionScale, ScaleBreaks};
pub use discrete::DiscretePositionScale;
pub use position::PositionScale;
pub use spec::{AxisScaleSpec, PositionScales, position_scales};
pub use transform::Transform;
