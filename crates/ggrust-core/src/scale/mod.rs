//! Scales: map data values to positions (0..1 within the panel) or to visual values.
//!
//! Phase 0.5 has continuous linear position scales. Discrete, transformed and
//! color/size scales are added in Phase 1 and later (see plan.md).

pub mod breaks;
mod continuous;
pub mod format;

pub use continuous::ContinuousPositionScale;
