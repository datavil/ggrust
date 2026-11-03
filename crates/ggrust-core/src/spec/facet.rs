//! Faceting: `facet_wrap` and `facet_grid`.

use serde::Deserialize;

use super::{OneOrMany, Options, optional_count};

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
    /// Level order of `x` (`facet_grid`): `1` ascending, `-1` descending, `0` data
    /// order. Only the default `1` is built yet.
    #[serde(default)]
    pub x_order: Option<i64>,
    /// Level order of `y` (`facet_grid`), as `x_order`.
    #[serde(default)]
    pub y_order: Option<i64>,
    /// Number of columns.
    #[serde(default, deserialize_with = "optional_count")]
    pub ncol: Option<u32>,
    /// Number of rows.
    #[serde(default, deserialize_with = "optional_count")]
    pub nrow: Option<u32>,
    /// `fixed`, `free`, `free_x`, `free_y`.
    #[serde(default)]
    pub scales: Option<String>,
    /// Level order per faceting column: `1` ascending, `-1` descending, null default.
    #[serde(default)]
    pub order: Option<OneOrMany<Option<i64>>>,
    /// Whether level combinations without data get no panel (default true).
    #[serde(default)]
    pub drop: Option<bool>,
    /// Fill direction: `h` or `v`.
    #[serde(default)]
    pub dir: Option<String>,
    /// Strip label format(s); null keeps the default for that column.
    #[serde(default)]
    pub format: Option<OneOrMany<Option<String>>>,
    /// Keys no component consumes.
    #[serde(flatten)]
    pub unsupported: Options,
}
