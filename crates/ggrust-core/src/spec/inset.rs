//! `inset_element(plot, left, bottom, right, top, ...)`: a figure drawn inside
//! another figure's area (patchwork `inset_element`).
//!
//! What it does: parse the inset options. Placement and drawing: `grid/inset.rs`.

use serde::Deserialize;

use super::{Figure, Options, join_path, report_unsupported};
use crate::{Error, Result};

/// One inset of a plot or grid.
#[derive(Debug, Clone, Deserialize)]
pub struct InsetSpec {
    /// The figure drawn in the inset.
    pub figure: Figure,
    /// Left edge as a fraction (0..1) of the area `align_to` names.
    pub left: f64,
    /// Bottom edge, 0 = bottom of the area.
    pub bottom: f64,
    /// Right edge.
    pub right: f64,
    /// Top edge, 1 = top of the area.
    pub top: f64,
    /// `panel` (default), `plot` or `full`.
    #[serde(default)]
    pub align_to: Option<String>,
    /// Draw over the host (default) or under it, above its background.
    #[serde(default)]
    pub on_top: Option<bool>,
    /// Clip the inset to its area (default).
    #[serde(default)]
    pub clip: Option<bool>,
    /// Leave the inset out of automatic tags.
    #[serde(default)]
    pub ignore_tag: Option<bool>,
    /// Keys no component consumes.
    #[serde(flatten)]
    pub unsupported: Options,
}

/// The area an inset's fractions refer to (patchwork `align_to`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsetAlign {
    /// The data panels (all panels of a faceted plot).
    Panel,
    /// Panels, axes and axis titles: the plot without titles, caption, legends,
    /// tag and margin.
    Plot,
    /// The whole area of the figure.
    Full,
}

impl InsetSpec {
    /// The validated `align_to` and edges `(left, bottom, right, top)`.
    pub fn placement(&self) -> Result<(InsetAlign, [f64; 4])> {
        let align = match self.align_to.as_deref() {
            None | Some("panel") => InsetAlign::Panel,
            Some("plot") => InsetAlign::Plot,
            Some("full") => InsetAlign::Full,
            Some(other) => {
                return Err(Error::spec(
                    "align_to",
                    format!(
                        "expected 'panel', 'plot' or 'full', got '{other}'{}",
                        crate::suggest::did_you_mean(other, ["panel", "plot", "full"])
                    ),
                ));
            }
        };
        let edges = [self.left, self.bottom, self.right, self.top];
        for (name, value) in ["left", "bottom", "right", "top"].iter().zip(edges) {
            if !(0.0..=1.0).contains(&value) {
                return Err(Error::spec(
                    *name,
                    format!(
                        "expected a fraction of the area between 0 and 1, got {value}; \
                         0 is the left or bottom edge, 1 the right or top edge"
                    ),
                ));
            }
        }
        if self.left >= self.right || self.bottom >= self.top {
            return Err(Error::spec(
                "",
                format!(
                    "the inset is empty: expected left < right and bottom < top, got \
                     left={}, right={}, bottom={}, top={}",
                    self.left, self.right, self.bottom, self.top
                ),
            ));
        }
        Ok((align, edges))
    }

    pub(super) fn collect_unsupported(&self, parent: &str, found: &mut Vec<String>) {
        report_unsupported(parent, &self.unsupported, found);
        let path = join_path(parent, "figure");
        match &self.figure {
            Figure::Plot(plot) => plot.collect_unsupported(&path, found),
            Figure::Subplots(subplots) => subplots.collect_unsupported(&path, found),
        }
    }
}
