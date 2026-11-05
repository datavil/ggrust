//! Breaks of a secondary axis (`sec_axis()` / `dup_axis()`) for the layout.
//!
//! What it does: picks the secondary breaks over the secondary range with the
//! same rules as a primary axis (target count from the axis length, label
//! overlap), then moves them to their place on the primary axis through the
//! scale's sampled transform (see [`crate::scale::secondary`]).
//!
//! What it does not do: draw anything; the axis side code draws these breaks
//! like any others.

use super::axis::{AxisBreaks, Orientation, choose_breaks};
use crate::scale::PositionScale;
use crate::theme::TextStyle;

/// The breaks of a secondary axis and the side they are drawn on.
#[derive(Debug, Clone, PartialEq)]
pub struct SecondaryBreaks {
    /// Positions in the primary scale's transformed units, with their labels.
    pub breaks: AxisBreaks,
    /// Drawn on the top (x) or right (y) side; else bottom / left.
    pub far_side: bool,
}

/// The secondary breaks of `scale` for an axis `length` px long, or `None`
/// without a secondary axis. `primary` are the axis's own breaks (a `dup_axis()`
/// mirrors them).
pub fn secondary_breaks(
    scale: &PositionScale,
    primary: &AxisBreaks,
    length: f64,
    orientation: Orientation,
    text: Option<TextStyle>,
    overflow: (f64, f64),
) -> Option<SecondaryBreaks> {
    let axis = scale.secondary_axis()?;
    let far_side = scale.secondary_on_far_side()?;
    let PositionScale::Continuous(primary_scale) = scale else {
        return None;
    };
    if axis.mirrors_primary() {
        return Some(SecondaryBreaks {
            breaks: primary.clone(),
            far_side,
        });
    }
    let Some(break_scale) = axis.break_scale(primary_scale) else {
        return Some(SecondaryBreaks {
            breaks: AxisBreaks::new(Vec::new(), Vec::new()),
            far_side,
        });
    };
    let transform = break_scale.transform;
    let chosen = choose_breaks(
        &PositionScale::Continuous(break_scale),
        length,
        orientation,
        text,
        overflow,
    );
    let (values, labels) = chosen
        .values
        .iter()
        .zip(chosen.labels)
        .filter_map(|(&value, label)| {
            let position = axis.primary_position(primary_scale.transform, transform, value)?;
            Some((position, label))
        })
        .unzip();
    let mut breaks = AxisBreaks::new(values, labels);
    breaks.arrangement = chosen.arrangement;
    Some(SecondaryBreaks { breaks, far_side })
}
