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
    /// Level order of `x` (`facet_grid`): `1` ascending (default), `-1`
    /// descending, `0` data order.
    #[serde(default)]
    pub x_order: Option<i64>,
    /// Level order of `y` (`facet_grid`), as `x_order`.
    #[serde(default)]
    pub y_order: Option<i64>,
    /// Strip label format of `x` (`facet_grid`).
    #[serde(default)]
    pub x_format: Option<String>,
    /// Strip label format of `y` (`facet_grid`).
    #[serde(default)]
    pub y_format: Option<String>,
    /// Strip label wrap width of `x` in characters (`facet_grid`); not positive:
    /// no wrap.
    #[serde(default)]
    pub x_labwidth: Option<i64>,
    /// Strip label wrap width of `y` (`facet_grid`), as `x_labwidth`.
    #[serde(default)]
    pub y_labwidth: Option<i64>,
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
    /// Strip label wrap width(s) in characters (`facet_wrap`), matched to the
    /// faceting columns; null or not positive: no wrap.
    #[serde(default)]
    pub labwidth: Option<OneOrMany<Option<i64>>>,
    /// Keys no component consumes.
    #[serde(flatten)]
    pub unsupported: Options,
}
