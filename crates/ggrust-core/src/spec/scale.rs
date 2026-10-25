//! A scale: how data values map to an aesthetic, plus axis/legend options.

use serde::Deserialize;

use super::{Aesthetic, GuideSpec, OneOrMany, Options, Value};

/// One `scale_*()` call.
///
/// Mapper-specific options (`palette`, `low`, `values`, `range`, ...) land in
/// `parameters` and are validated by the scale implementation.
#[derive(Debug, Clone, Deserialize)]
pub struct ScaleSpec {
    /// Target aesthetic(s).
    pub aesthetic: OneOrMany<Aesthetic>,
    /// Axis or legend title.
    #[serde(default)]
    pub name: Option<String>,
    /// Break positions; numbers or category values.
    #[serde(default)]
    pub breaks: Option<Vec<Value>>,
    /// Break labels: a list, or a map from break to label.
    #[serde(default)]
    pub labels: Option<Value>,
    /// Limits: `[min, max]` (either may be null) or the category list.
    #[serde(default)]
    pub limits: Option<Vec<Value>>,
    /// Expansion `[multiplicative, additive]`.
    #[serde(default)]
    pub expand: Option<Vec<f64>>,
    /// Transform: `identity`, `log10`, `log2`, `sqrt`, `reverse`.
    #[serde(default)]
    pub trans: Option<String>,
    /// Label format, e.g. `.1f`, `{d}%`.
    #[serde(default)]
    pub format: Option<String>,
    /// Color/value for missing data.
    #[serde(default)]
    pub na_value: Option<Value>,
    /// Guide override for this scale.
    #[serde(default)]
    pub guide: Option<GuideSpec>,
    /// Force a discrete scale.
    #[serde(default)]
    pub discrete: Option<bool>,
    /// Reverse category order (discrete scales).
    #[serde(default)]
    pub reverse: Option<bool>,
    /// Axis position, e.g. `left`, `top`.
    #[serde(default)]
    pub position: Option<String>,
    /// Which mapper builds colors/values: `color_brewer`, `color_cmap`, `color_hue`, ...
    #[serde(default)]
    pub scale_mapper_kind: Option<String>,
    /// Mapper-specific options. Validated by the scale implementation.
    #[serde(flatten)]
    pub parameters: Options,
}
