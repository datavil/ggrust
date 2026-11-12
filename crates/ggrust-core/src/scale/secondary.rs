//! Secondary axes: `sec_axis()` / `dup_axis()` on a continuous position scale
//! (ggplot2 `AxisSecondary`; lets-plot has none).
//!
//! What it does: the transform is a function of the primary axis's data values,
//! given by the caller as an [`AxisFunction`] (Python passes the user's callable).
//! As ggplot2, it is sampled at [`DETAIL`] points over the panel range: the
//! samples must be strictly monotonic over the trained range, and they make a
//! lookup table. Secondary breaks are made over the secondary range (the
//! transformed panel range) by a scale of their own, then placed on the primary
//! axis by interpolating the table back (`stats::approx`). Labels come from those
//! breaks, the user's `labels` or `format`.
//!
//! What it does not do: discrete or date-time primary axes (they warn), label
//! functions, or guides other than `"axis"` and `"none"`.

use std::fmt;
use std::sync::Arc;

use super::label_pattern::LabelPattern;
use super::{ContinuousPositionScale, Transform};
use crate::spec::Value;
use crate::{Error, Result};

/// Samples of the transform over the panel range (ggplot2's `detail`).
pub const DETAIL: usize = 1000;

/// A secondary axis transform: primary data values to secondary values.
pub trait AxisFunction: Send + Sync {
    /// The secondary value of every primary value, in order. `Err` carries the
    /// reason (for example the Python exception), shown in the plot's error.
    fn call(&self, values: &[f64]) -> std::result::Result<Vec<f64>, String>;

    /// The new text of every text, in order: the same table also carries facet
    /// labeller functions (`as_labeller(function)`), which map strip labels.
    /// Numeric transforms keep the default, which refuses.
    fn call_text(&self, values: &[String]) -> std::result::Result<Vec<String>, String> {
        let _ = values;
        Err("this function maps numbers, not labels".to_owned())
    }

    /// A color scale's `rescaler` (ggplot2 `function(x, to, from)`): every
    /// value of `values` rescaled from `from` to `to`, in order, in one call.
    /// Other functions keep the default, which refuses.
    fn rescale(
        &self,
        values: &[f64],
        to: (f64, f64),
        from: (f64, f64),
    ) -> std::result::Result<Vec<f64>, String> {
        let _ = (values, to, from);
        Err("this function is not a rescaler".to_owned())
    }
}

/// The transform functions of a figure, indexed by `sec_axis.transform` in the
/// spec (attached with [`crate::spec::Figure::attach_functions`]).
#[derive(Clone, Default)]
pub struct AxisFunctions(pub Vec<Arc<dyn AxisFunction>>);

impl fmt::Debug for AxisFunctions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "AxisFunctions({})", self.0.len())
    }
}

/// An option that is unset, copied from the primary axis (`derive()`), or given.
#[derive(Debug, Clone, PartialEq)]
pub enum Derived<T> {
    /// Not given: the secondary axis's own default.
    Default,
    /// `derive()`: the primary axis's value.
    Derive,
    /// Given.
    Given(T),
}

/// Labels of a secondary axis.
#[derive(Debug, Clone, PartialEq)]
pub enum SecondaryLabels {
    /// One text per break.
    Texts(Vec<String>),
    /// A number format (`"{.1f} °F"`).
    Format(LabelPattern),
}

/// A secondary axis of a continuous position scale.
#[derive(Clone)]
pub struct SecondaryAxis {
    /// The transform; `None` is the identity (`dup_axis()`).
    function: Option<Arc<dyn AxisFunction>>,
    /// Axis title; [`Derived::Default`] draws none (ggplot2 `waiver()`).
    pub name: Derived<String>,
    /// Break values in secondary units (`None` items are skipped).
    pub breaks: Derived<Vec<Option<f64>>>,
    /// Labels.
    pub labels: Derived<SecondaryLabels>,
    /// `guide="none"`: the axis is not drawn.
    pub hidden: bool,
    /// Spec path of the `sec_axis` option, for errors.
    path: String,
    /// The sampled transform over the panel range, once resolved.
    table: Option<Arc<Table>>,
}

impl fmt::Debug for SecondaryAxis {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SecondaryAxis")
            .field("identity", &self.function.is_none())
            .field("name", &self.name)
            .field("breaks", &self.breaks)
            .field("labels", &self.labels)
            .field("hidden", &self.hidden)
            .field("table", &self.table.as_ref().map(|table| table.range()))
            .finish()
    }
}

impl PartialEq for SecondaryAxis {
    fn eq(&self, other: &Self) -> bool {
        let same_function = match (&self.function, &other.function) {
            (None, None) => true,
            (Some(first), Some(second)) => Arc::ptr_eq(first, second),
            _ => false,
        };
        same_function
            && self.name == other.name
            && self.breaks == other.breaks
            && self.labels == other.labels
            && self.hidden == other.hidden
            && self.table == other.table
    }
}

/// Primary data values and their secondary values, strictly increasing in the
/// secondary values (the ones looked up).
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    secondary: Vec<f64>,
    primary: Vec<f64>,
}

impl Table {
    /// Secondary range (low, high).
    pub fn range(&self) -> (f64, f64) {
        (self.secondary[0], self.secondary[self.secondary.len() - 1])
    }

    /// The primary data value of a secondary value by linear interpolation;
    /// `None` outside the table (ggplot2's `approx` gives NA there).
    pub fn primary_of(&self, value: f64) -> Option<f64> {
        let (low, high) = self.range();
        if !value.is_finite() || value < low || value > high {
            return None;
        }
        let index = self.secondary.partition_point(|known| *known < value);
        if index == 0 {
            return Some(self.primary[0]);
        }
        let (x0, x1) = (self.secondary[index - 1], self.secondary[index]);
        let (y0, y1) = (self.primary[index - 1], self.primary[index]);
        Some(y0 + (y1 - y0) * (value - x0) / (x1 - x0))
    }

    /// Pairs (primary data value, secondary value) in primary order, for the HTML
    /// runtime.
    pub fn pairs(&self) -> impl Iterator<Item = (f64, f64)> + '_ {
        self.primary
            .iter()
            .copied()
            .zip(self.secondary.iter().copied())
    }
}

impl SecondaryAxis {
    /// Parse a `sec_axis` spec value at `path`: `{"transform": index | null,
    /// "name", "breaks", "labels", "guide"}`, where `{"derive": true}` copies the
    /// primary axis's option. `functions` resolves the transform index.
    pub fn parse(value: &Value, functions: &AxisFunctions, path: &str) -> Result<Self> {
        let Value::Object(options) = value else {
            return Err(Error::spec(
                path,
                format!("expected sec_axis(...) or dup_axis(...), got {value}"),
            ));
        };
        let at = |key: &str| format!("{path}.{key}");
        let known = ["transform", "name", "breaks", "labels", "guide"];
        if let Some(key) = options.keys().find(|key| !known.contains(&key.as_str())) {
            return Err(Error::spec(
                at(key),
                format!(
                    "unknown sec_axis option '{key}'; use one of {}{}",
                    known.join(", "),
                    crate::suggest::did_you_mean(key, known)
                ),
            ));
        }
        let function = match options.get("transform") {
            None | Some(Value::Null) => None,
            Some(Value::Number(number)) => {
                let index = number.as_u64().ok_or_else(|| {
                    Error::spec(
                        at("transform"),
                        format!("expected a function index, got {number}"),
                    )
                })?;
                let function = functions.0.get(index as usize).cloned().ok_or_else(|| {
                    Error::spec(
                        at("transform"),
                        format!(
                            "function {index} was not passed with the spec ({} given); render \
                             through ggrust so the transform callable reaches the engine",
                            functions.0.len()
                        ),
                    )
                })?;
                Some(function)
            }
            Some(other) => {
                return Err(Error::spec(
                    at("transform"),
                    format!("expected a function index or null, got {other}"),
                ));
            }
        };
        let derived = |key: &str| -> Option<Derived<&Value>> {
            match options.get(key) {
                None | Some(Value::Null) => None,
                Some(Value::Object(object)) if object.get("derive") == Some(&Value::Bool(true)) => {
                    Some(Derived::Derive)
                }
                Some(value) => Some(Derived::Given(value)),
            }
        };
        let name = match derived("name") {
            None | Some(Derived::Default) => Derived::Default,
            Some(Derived::Derive) => Derived::Derive,
            Some(Derived::Given(Value::String(text))) => Derived::Given(text.clone()),
            Some(Derived::Given(other)) => {
                return Err(Error::spec(
                    at("name"),
                    format!("expected a text, got {other}"),
                ));
            }
        };
        let breaks = match derived("breaks") {
            None | Some(Derived::Default) => Derived::Default,
            Some(Derived::Derive) => Derived::Derive,
            Some(Derived::Given(Value::Array(items))) => Derived::Given(
                items
                    .iter()
                    .map(|item| match item {
                        Value::Null => Ok(None),
                        item => item
                            .as_f64()
                            .filter(|value| value.is_finite())
                            .map(Some)
                            .ok_or_else(|| {
                                Error::spec(
                                    at("breaks"),
                                    format!("breaks of a secondary axis are numbers, got {item}"),
                                )
                            }),
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
            Some(Derived::Given(other)) => {
                return Err(Error::spec(
                    at("breaks"),
                    format!("expected a list of numbers, got {other}"),
                ));
            }
        };
        let labels = match derived("labels") {
            None | Some(Derived::Default) => Derived::Default,
            Some(Derived::Derive) => Derived::Derive,
            Some(Derived::Given(Value::String(pattern))) => {
                Derived::Given(SecondaryLabels::Format(
                    LabelPattern::parse(pattern)
                        .map_err(|message| Error::spec(at("labels"), message))?,
                ))
            }
            Some(Derived::Given(Value::Array(items))) => Derived::Given(SecondaryLabels::Texts(
                items
                    .iter()
                    .map(|item| match item {
                        Value::String(text) => Ok(text.clone()),
                        Value::Number(number) => Ok(number.to_string()),
                        Value::Null => Ok(String::new()),
                        other => Err(Error::spec(
                            at("labels"),
                            format!("labels are texts, got {other}"),
                        )),
                    })
                    .collect::<Result<Vec<_>>>()?,
            )),
            Some(Derived::Given(other)) => {
                return Err(Error::spec(
                    at("labels"),
                    format!("expected a list of texts or a format text, got {other}"),
                ));
            }
        };
        if let (Derived::Given(SecondaryLabels::Texts(texts)), Derived::Given(values)) =
            (&labels, &breaks)
            && texts.len() != values.len()
        {
            return Err(Error::spec(
                at("labels"),
                format!(
                    "{} labels for {} breaks; give one label per break",
                    texts.len(),
                    values.len()
                ),
            ));
        }
        if let (Derived::Given(SecondaryLabels::Texts(_)), Derived::Default) = (&labels, &breaks) {
            return Err(Error::spec(
                at("labels"),
                "a list of labels needs 'breaks' (one label per break); give breaks, or a \
                 format text such as '{.1f}'",
            ));
        }
        let hidden = match derived("guide") {
            None | Some(Derived::Default) | Some(Derived::Derive) => false,
            Some(Derived::Given(Value::String(name))) if name == "axis" => false,
            Some(Derived::Given(Value::String(name))) if name == "none" => true,
            Some(Derived::Given(other)) => {
                return Err(Error::spec(
                    at("guide"),
                    format!("unsupported guide {other}; use 'axis' or 'none'"),
                ));
            }
        };
        Ok(SecondaryAxis {
            function,
            name,
            breaks,
            labels,
            hidden,
            path: path.to_owned(),
            table: None,
        })
    }

    /// `dup_axis()` with every option derived: the secondary side mirrors the
    /// primary breaks and labels.
    pub fn mirrors_primary(&self) -> bool {
        self.function.is_none() && self.breaks == Derived::Derive && self.labels == Derived::Derive
    }

    /// The sampled table, once [`Self::resolve`]d (`None` on an empty axis).
    pub fn table(&self) -> Option<&Table> {
        self.table.as_deref()
    }

    /// The axis title: the given name, or `primary` for `derive()`.
    pub fn title(&self, primary: Option<&String>) -> Option<String> {
        match &self.name {
            Derived::Default => None,
            Derived::Derive => primary.cloned(),
            Derived::Given(name) => Some(name.clone()),
        }
    }

    fn apply(&self, values: &[f64]) -> Result<Vec<f64>> {
        let Some(function) = &self.function else {
            return Ok(values.to_vec());
        };
        let result = function.call(values).map_err(|reason| {
            Error::spec(
                format!("{}.transform", self.path),
                format!("the transform failed: {reason}"),
            )
        })?;
        if result.len() != values.len() {
            return Err(Error::spec(
                format!("{}.transform", self.path),
                format!(
                    "the transform returned {} values for {} inputs; return one number per value",
                    result.len(),
                    values.len()
                ),
            ));
        }
        Ok(result)
    }

    /// Sample the transform over the panel range of `scale` (ggplot2
    /// `AxisSecondary$break_info`): an error unless strictly monotonic over the
    /// trained range. Call again after the scale's domain changes.
    pub fn resolve(&self, scale: &ContinuousPositionScale) -> Result<SecondaryAxis> {
        let mut resolved = self.clone();
        resolved.table = None;
        let Some((low, high)) = scale.trained_domain() else {
            return Ok(resolved);
        };
        let transform = scale.transform;
        let path = format!("{}.transform", self.path);
        // Monotonic over the trained range (ggplot2 `mono_test`).
        let along = sequence(low, high);
        let data: Vec<f64> = along
            .iter()
            .map(|&value| transform.inverse(value))
            .collect();
        let values = self.apply(&data)?;
        if let Some(index) = values.iter().position(|value| !value.is_finite()) {
            return Err(Error::spec(
                &path,
                format!(
                    "the transform returned {} at {}; it must give a finite number over the \
                     axis range ({} to {})",
                    values[index],
                    data[index],
                    data[0].min(data[DETAIL - 1]),
                    data[0].max(data[DETAIL - 1]),
                ),
            ));
        }
        if low < high {
            let increasing = values[DETAIL - 1] > values[0];
            let broken = values.windows(2).position(|pair| {
                if increasing {
                    pair[1] <= pair[0]
                } else {
                    pair[1] >= pair[0]
                }
            });
            if let Some(index) = broken {
                return Err(Error::spec(
                    &path,
                    format!(
                        "the transform of a secondary axis must be strictly monotonic over the \
                         axis range, but it is not near {}; use a function that only increases \
                         or only decreases there",
                        data[index + 1]
                    ),
                ));
            }
        }
        // The table over the expanded range: ends that are not finite or turn back
        // (a transform monotonic only on the data) are dropped.
        let (low, high) = scale.expanded_domain();
        let along = sequence(low, high);
        let data: Vec<f64> = along
            .iter()
            .map(|&value| transform.inverse(value))
            .collect();
        let values = self.apply(&data)?;
        let (trained_low, trained_high) = scale
            .trained_domain()
            .map(|(low, high)| (low.min(high), low.max(high)))
            .unwrap_or((low, high));
        let start = along
            .iter()
            .position(|value| *value >= trained_low)
            .unwrap_or(0);
        let mut pairs: Vec<(f64, f64)> = Vec::with_capacity(DETAIL);
        let direction = {
            // A one-value trained range (constant data) has no sample inside it:
            // take the step from `start` (or into it at the top end).
            let end = along
                .iter()
                .rposition(|value| *value <= trained_high)
                .unwrap_or(DETAIL - 1)
                .max(start);
            let (from, to) = if end > start {
                (start, end)
            } else if start + 1 < DETAIL {
                (start, start + 1)
            } else {
                (start - 1, start)
            };
            if values[to] >= values[from] {
                1.0
            } else {
                -1.0
            }
        };
        // Outward from the trained range, while the values keep the direction.
        let mut previous = values[start];
        if previous.is_finite() {
            pairs.push((data[start], previous));
        }
        for index in start + 1..DETAIL {
            let value = values[index];
            if !value.is_finite() || (value - previous) * direction <= 0.0 {
                break;
            }
            pairs.push((data[index], value));
            previous = value;
        }
        let mut previous = values[start];
        for index in (0..start).rev() {
            let value = values[index];
            if !value.is_finite() || (previous - value) * direction <= 0.0 {
                break;
            }
            pairs.push((data[index], value));
            previous = value;
        }
        if pairs.len() < 2 {
            return Ok(resolved);
        }
        pairs.sort_by(|first, second| first.1.total_cmp(&second.1));
        pairs.dedup_by(|second, first| second.1 == first.1);
        let (primary, secondary) = pairs.into_iter().unzip();
        resolved.table = Some(Arc::new(Table { secondary, primary }));
        Ok(resolved)
    }

    /// A scale over the secondary range that makes the secondary breaks and
    /// labels: linear, or the primary's log / sqrt transform (ggplot2 takes the
    /// primary transform's break function). Its positions go back to the primary
    /// axis with [`Self::primary_position`].
    pub fn break_scale(
        &self,
        primary: &ContinuousPositionScale,
    ) -> Option<ContinuousPositionScale> {
        let (low, high) = self.table()?.range();
        let mut scale = ContinuousPositionScale::default();
        scale.multiplicative_expand = 0.0;
        scale.additive_expand = 0.0;
        scale.exponent_format = primary.exponent_format;
        if !primary.transform.is_linear()
            && let (Some(low), Some(high)) =
                (primary.transform.apply(low), primary.transform.apply(high))
        {
            scale.transform = primary.transform;
            scale.train_range(low.min(high), low.max(high));
        } else {
            scale.train_range(low, high);
        }
        scale.fixed_breaks = match &self.breaks {
            Derived::Default => None,
            Derived::Derive => {
                scale.break_width = primary.break_width;
                primary.fixed_breaks.clone()
            }
            Derived::Given(values) => Some(values.clone()),
        };
        match &self.labels {
            Derived::Default => {}
            Derived::Derive => {
                scale.labels = primary.labels.clone();
                scale.format = primary.format.clone();
            }
            Derived::Given(SecondaryLabels::Texts(texts)) => scale.labels = Some(texts.clone()),
            Derived::Given(SecondaryLabels::Format(pattern)) => {
                scale.format = Some(pattern.with_exponent(primary.exponent_format));
            }
        }
        // Labels without breaks are ignored by the scale (lets-plot rule).
        if scale.fixed_breaks.is_none() {
            scale.labels = None;
        }
        Some(scale)
    }

    /// Where a break of [`Self::break_scale`] (its position in that scale's
    /// transformed units) sits on the primary axis, in the primary's transformed
    /// units; `None` when it falls outside the table.
    pub fn primary_position(
        &self,
        primary: Transform,
        break_scale: Transform,
        position: f64,
    ) -> Option<f64> {
        let secondary = break_scale.inverse(position);
        let data = self.table()?.primary_of(secondary)?;
        primary.apply(data)
    }
}

/// [`DETAIL`] evenly spaced values from `low` to `high` (R `seq(length.out=)`).
fn sequence(low: f64, high: f64) -> Vec<f64> {
    let step = (high - low) / (DETAIL - 1) as f64;
    (0..DETAIL)
        .map(|index| {
            if index == DETAIL - 1 {
                high
            } else {
                low + step * index as f64
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Linear(f64, f64);

    impl AxisFunction for Linear {
        fn call(&self, values: &[f64]) -> std::result::Result<Vec<f64>, String> {
            Ok(values.iter().map(|value| value * self.0 + self.1).collect())
        }
    }

    struct Square;

    impl AxisFunction for Square {
        fn call(&self, values: &[f64]) -> std::result::Result<Vec<f64>, String> {
            Ok(values.iter().map(|value| value * value).collect())
        }
    }

    fn functions(function: impl AxisFunction + 'static) -> AxisFunctions {
        AxisFunctions(vec![Arc::new(function)])
    }

    fn axis(json: &str, functions: &AxisFunctions) -> Result<SecondaryAxis> {
        let value: Value = serde_json::from_str(json).unwrap();
        SecondaryAxis::parse(&value, functions, "scales[0].sec_axis")
    }

    fn trained(low: f64, high: f64) -> ContinuousPositionScale {
        let mut scale = ContinuousPositionScale::default();
        scale.train_range(low, high);
        scale
    }

    #[test]
    fn a_linear_transform_maps_the_expanded_range() {
        let functions = functions(Linear(1.8, 32.0));
        let axis = axis(r#"{"transform": 0}"#, &functions).unwrap();
        let scale = trained(0.0, 100.0);
        let resolved = axis.resolve(&scale).unwrap();
        let (low, high) = resolved.table().unwrap().range();
        // 0..100 expanded by 5%: -5..105 C = 23..221 F.
        assert!(
            (low - 23.0).abs() < 1e-9 && (high - 221.0).abs() < 1e-9,
            "{low} {high}"
        );
        let at = resolved.table().unwrap().primary_of(212.0).unwrap();
        assert!((at - 100.0).abs() < 1e-9, "{at}");
        assert_eq!(resolved.table().unwrap().primary_of(300.0), None);
    }

    #[test]
    fn non_monotonic_transforms_fail_with_the_place() {
        let functions = functions(Square);
        let axis = axis(r#"{"transform": 0}"#, &functions).unwrap();
        let error = axis.resolve(&trained(-2.0, 3.0)).unwrap_err().to_string();
        assert!(
            error.contains("at 'scales[0].sec_axis.transform'")
                && error.contains("strictly monotonic"),
            "{error}"
        );
        // Monotonic on the data, turning back in the expansion only: kept, trimmed.
        let resolved = axis.resolve(&trained(0.0, 3.0)).unwrap();
        let (low, high) = resolved.table().unwrap().range();
        assert!(
            low >= 0.0 && (high - 3.15 * 3.15).abs() < 1e-9,
            "{low} {high}"
        );
    }

    #[test]
    fn a_one_value_range_still_gets_a_table() {
        for function in [Linear(2.0, 0.0), Linear(-2.0, 0.0)] {
            let functions = functions(function);
            let axis = axis(r#"{"transform": 0}"#, &functions).unwrap();
            let resolved = axis.resolve(&trained(2.0, 2.0)).unwrap();
            // 2 widened to 1.5..2.5, times +/-2.
            let (low, high) = resolved.table().unwrap().range();
            assert!((high - low - 2.0).abs() < 1e-9, "{low} {high}");
        }
    }

    #[test]
    fn options_parse_with_paths_and_did_you_mean() {
        let none = AxisFunctions::default();
        let error = axis(r#"{"transform": 0}"#, &none).unwrap_err().to_string();
        assert!(error.contains("function 0 was not passed"), "{error}");
        let error = axis(r#"{"brakes": [1]}"#, &none).unwrap_err().to_string();
        assert!(error.contains("did you mean 'breaks'"), "{error}");
        let error = axis(r#"{"breaks": [1, 2], "labels": ["a"]}"#, &none)
            .unwrap_err()
            .to_string();
        assert!(error.contains("1 labels for 2 breaks"), "{error}");
        let error = axis(r#"{"guide": "legend"}"#, &none)
            .unwrap_err()
            .to_string();
        assert!(error.contains("use 'axis' or 'none'"), "{error}");
        let duplicate = axis(
            r#"{"name": {"derive": true}, "breaks": {"derive": true}, "labels": {"derive": true}}"#,
            &none,
        )
        .unwrap();
        assert!(duplicate.mirrors_primary());
        assert_eq!(duplicate.title(Some(&"x".to_owned())), Some("x".to_owned()));
    }

    #[test]
    fn breaks_go_back_through_a_log_primary() {
        let functions = functions(Linear(10.0, 0.0));
        let axis = axis(r#"{"transform": 0}"#, &functions).unwrap();
        let mut scale = ContinuousPositionScale::default();
        scale.transform = Transform::Log10;
        scale.train_range(0.0, 3.0); // 1..1000
        let resolved = axis.resolve(&scale).unwrap();
        let break_scale = resolved.break_scale(&scale).unwrap();
        assert_eq!(break_scale.transform, Transform::Log10);
        // Secondary 100 is primary 10, at log10 position 1.
        let position = resolved
            .primary_position(Transform::Log10, Transform::Log10, 2.0)
            .unwrap();
        assert!((position - 1.0).abs() < 1e-9, "{position}");
    }
}
