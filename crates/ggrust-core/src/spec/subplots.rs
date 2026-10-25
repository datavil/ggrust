//! A grid of figures: `gggrid([...])`.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::{
    CaptionSpec, Figure, GuideSpec, Options, SizeSpec, ThemeSpec, TitleSpec, ToolbarSpec, count,
    guide_map, join_path, report_unsupported,
};

/// Everything `SupPlotsSpec.as_dict()` produces.
#[derive(Debug, Clone, Deserialize)]
pub struct SubplotsSpec {
    /// Figure kind, always `subplots`. Consumed by [`super::Figure`] dispatch.
    pub kind: String,
    /// Grid layout options.
    pub layout: GridLayoutSpec,
    /// Cells in row-major order; `None` leaves a cell empty.
    pub figures: Vec<Option<Figure>>,
    /// Theme applied to the whole grid.
    #[serde(default)]
    pub theme: Option<ThemeSpec>,
    /// Guides applied to the whole grid.
    #[serde(default, deserialize_with = "guide_map")]
    pub guides: BTreeMap<String, GuideSpec>,
    /// `ggsize(width, height)` of the whole grid.
    #[serde(default)]
    pub ggsize: Option<SizeSpec>,
    /// Grid title.
    #[serde(default)]
    pub ggtitle: Option<TitleSpec>,
    /// Grid caption.
    #[serde(default)]
    pub caption: Option<CaptionSpec>,
    /// Toolbar for the whole grid.
    #[serde(default)]
    pub ggtoolbar: Option<ToolbarSpec>,
    /// Keys no component consumes.
    #[serde(flatten)]
    pub unsupported: Options,
}

impl SubplotsSpec {
    pub(super) fn collect_unsupported(&self, parent: &str, found: &mut Vec<String>) {
        report_unsupported(parent, &self.unsupported, found);
        report_unsupported(
            &join_path(parent, "layout"),
            &self.layout.unsupported,
            found,
        );
        for (index, figure) in self.figures.iter().enumerate() {
            let path = join_path(parent, &format!("figures[{index}]"));
            match figure {
                Some(Figure::Plot(plot)) => plot.collect_unsupported(&path, found),
                Some(Figure::Subplots(subplots)) => subplots.collect_unsupported(&path, found),
                None => {}
            }
        }
        if let Some(theme) = &self.theme {
            theme.collect_unsupported(&join_path(parent, "theme"), found);
        }
        for (aesthetic, guide) in &self.guides {
            guide.collect_unsupported(&join_path(parent, &format!("guides.{aesthetic}")), found);
        }
        if let Some(toolbar) = &self.ggtoolbar {
            report_unsupported(&join_path(parent, "ggtoolbar"), &toolbar.unsupported, found);
        }
    }
}

/// `gggrid(ncol=..., widths=..., align=..., guides=...)` layout.
#[derive(Debug, Clone, Deserialize)]
pub struct GridLayoutSpec {
    /// Layout kind; lets-plot emits `grid`.
    pub name: String,
    /// Number of columns.
    #[serde(deserialize_with = "count")]
    pub ncol: u32,
    /// Number of rows.
    #[serde(deserialize_with = "count")]
    pub nrow: u32,
    /// Relative column widths.
    #[serde(default)]
    pub widths: Option<Vec<f64>>,
    /// Relative row heights.
    #[serde(default)]
    pub heights: Option<Vec<f64>>,
    /// Horizontal gap between cells in pixels.
    #[serde(default)]
    pub hspace: Option<f64>,
    /// Vertical gap between cells in pixels.
    #[serde(default)]
    pub vspace: Option<f64>,
    /// Keep each plot's aspect ratio inside its cell.
    #[serde(default)]
    pub fit: Option<bool>,
    /// Align inner plot areas across cells.
    #[serde(default)]
    pub align: Option<bool>,
    /// `auto` or `collect`: share legends across cells.
    #[serde(default)]
    pub guides: Option<String>,
    /// Keys no component consumes.
    #[serde(flatten)]
    pub unsupported: Options,
}
