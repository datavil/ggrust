//! Faceting: `facet_wrap` (and later `facet_grid`).

use serde::Deserialize;

use super::{OneOrMany, Options};

/// One `facet_*()` call.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FacetSpec {
    /// `wrap` or `grid`.
    pub name: String,
    /// Faceting columns (`facet_wrap`).
    #[serde(default)]
    pub facets: Option<OneOrMany<String>>,
    /// Column faceting variable (`facet_grid`).
    #[serde(default)]
    pub x: Option<String>,
    /// Row faceting variable (`facet_grid`).
    #[serde(default)]
    pub y: Option<String>,
    /// Number of columns.
    #[serde(default)]
    pub ncol: Option<u32>,
    /// Number of rows.
    #[serde(default)]
    pub nrow: Option<u32>,
    /// `fixed`, `free`, `free_x`, `free_y`.
    #[serde(default)]
    pub scales: Option<String>,
    /// Level order per faceting column: `1` ascending, `-1` descending.
    #[serde(default)]
    pub order: Option<OneOrMany<i64>>,
    /// Fill direction: `h` or `v`.
    #[serde(default)]
    pub dir: Option<String>,
    /// Strip label format(s).
    #[serde(default)]
    pub format: Option<OneOrMany<String>>,
    /// Keys no component consumes.
    #[serde(flatten)]
    pub unsupported: Options,
}
