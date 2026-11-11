//! Facet strip labellers (ggplot2 `labeller=`: `label_value`, `label_both`,
//! `label_context`, `as_labeller`).
//!
//! What it does: turns the formatted levels of a panel's faceting variables into
//! strip labels. An optional per-value step runs first (`as_labeller` with a lookup
//! table or a Python function, called once per variable over all its levels), then
//! the base labeller: the value alone, `variable<sep>value`, or `label_context`
//! (both when a strip group has several variables). `multi_line=false` joins the
//! variables of a group into one strip, as ggplot2 (`"a, b: 1, x"`), which `facet`
//! wraps at the first `labwidth`.
//!
//! What it does not do: per-variable labellers (ggplot2 `labeller(var = ...)`),
//! plotmath. Formatting the levels (`format=`) and wrapping (`labwidth=`) are
//! `facet`'s.

use std::collections::HashMap;
use std::sync::Arc;

use crate::scale::format::Label;
use crate::scale::secondary::{AxisFunction, AxisFunctions};
use crate::spec::Value;
use crate::{Error, Result};

/// The base labeller: how variable names and values combine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Base {
    /// `label_value`: the value alone.
    Value,
    /// `label_both`: `variable<sep>value`.
    Both,
    /// `label_context`: value alone for one variable, both for several.
    Context,
}

/// The per-value step of `as_labeller`.
#[derive(Clone)]
enum Transform {
    /// A lookup table; values not in it keep their text.
    Lookup(HashMap<String, String>),
    /// A function called with every level text of a variable.
    Function(Arc<dyn AxisFunction>),
}

impl std::fmt::Debug for Transform {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Transform::Lookup(lookup) => write!(formatter, "Lookup({} entries)", lookup.len()),
            Transform::Function(_) => write!(formatter, "Function"),
        }
    }
}

/// A parsed `labeller=` option.
#[derive(Debug, Clone)]
pub struct Labeller {
    base: Base,
    multi_line: bool,
    sep: String,
    transform: Option<Transform>,
    /// Spec path, for errors from the function.
    path: String,
}

impl Default for Labeller {
    fn default() -> Self {
        Labeller {
            base: Base::Value,
            multi_line: true,
            sep: ": ".to_owned(),
            transform: None,
            path: "facet.labeller".to_owned(),
        }
    }
}

const KNOWN: [&str; 6] = ["base", "multi_line", "sep", "lookup", "function", "default"];
const NAMES: [&str; 3] = ["label_value", "label_both", "label_context"];

impl Labeller {
    /// Parse `labeller` at `path`: absent or null is `label_value`; a name
    /// (`"label_both"`); or `{"base": "value" | "both" | "context", "multi_line",
    /// "sep", "lookup": {text: text}, "function": index}` where `function` indexes
    /// `functions`.
    pub fn parse(value: Option<&Value>, functions: &AxisFunctions, path: &str) -> Result<Self> {
        let mut labeller = Labeller {
            path: path.to_owned(),
            ..Labeller::default()
        };
        let options = match value {
            None | Some(Value::Null) => return Ok(labeller),
            Some(Value::String(name)) => {
                labeller.base = base_from_name(name, path)?;
                return Ok(labeller);
            }
            Some(Value::Object(options)) => options,
            Some(other) => {
                return Err(Error::spec(
                    path,
                    format!(
                        "expected a labeller such as label_both or as_labeller({{...}}), \
                         got {other}"
                    ),
                ));
            }
        };
        let at = |key: &str| format!("{path}.{key}");
        if let Some(key) = options.keys().find(|key| !KNOWN.contains(&key.as_str())) {
            return Err(Error::spec(
                at(key),
                format!(
                    "unknown labeller option '{key}'; use one of {}{}",
                    KNOWN.join(", "),
                    crate::suggest::did_you_mean(key, KNOWN)
                ),
            ));
        }
        // `default` (as_labeller's base labeller) is the same shape without a step.
        let base_options = match options.get("default") {
            Some(Value::Object(inner)) => Some(inner),
            None | Some(Value::Null) => None,
            Some(other) => {
                return Err(Error::spec(
                    at("default"),
                    format!("expected a labeller such as label_both, got {other}"),
                ));
            }
        };
        let kind_source = base_options
            .and_then(|inner| inner.get("base"))
            .or(options.get("base"));
        if let Some(kind) = kind_source {
            let Value::String(name) = kind else {
                return Err(Error::spec(
                    at("base"),
                    format!("expected a name, got {kind}"),
                ));
            };
            labeller.base = base_from_name(name, &at("base"))?;
        }
        let sep = base_options
            .and_then(|inner| inner.get("sep"))
            .or(options.get("sep"));
        match sep {
            None | Some(Value::Null) => {}
            Some(Value::String(sep)) => labeller.sep = sep.clone(),
            Some(other) => {
                return Err(Error::spec(
                    at("sep"),
                    format!("expected a string, got {other}"),
                ));
            }
        }
        // as_labeller's own multi_line wins over the base labeller's.
        let multi_line = options
            .get("multi_line")
            .or(base_options.and_then(|inner| inner.get("multi_line")));
        match multi_line {
            None | Some(Value::Null) => {}
            Some(Value::Bool(flag)) => labeller.multi_line = *flag,
            Some(other) => {
                return Err(Error::spec(
                    at("multi_line"),
                    format!("expected true or false, got {other}"),
                ));
            }
        }
        match (options.get("lookup"), options.get("function")) {
            (Some(_), Some(_)) => {
                return Err(Error::spec(
                    path,
                    "give either a lookup table or a function to as_labeller, not both",
                ));
            }
            (Some(Value::Object(table)), None) => {
                let mut lookup = HashMap::with_capacity(table.len());
                for (key, text) in table {
                    let Value::String(text) = text else {
                        return Err(Error::spec(
                            format!("{path}.lookup.{key}"),
                            format!("expected a label text, got {text}"),
                        ));
                    };
                    lookup.insert(key.clone(), text.clone());
                }
                labeller.transform = Some(Transform::Lookup(lookup));
            }
            (Some(other), None) => {
                return Err(Error::spec(
                    at("lookup"),
                    format!("expected a table of level to label, got {other}"),
                ));
            }
            (None, Some(index)) => {
                let function = index
                    .as_u64()
                    .and_then(|index| functions.0.get(usize::try_from(index).ok()?))
                    .ok_or_else(|| {
                        Error::spec(
                            at("function"),
                            format!(
                                "no labeller function {index} was passed with the spec; \
                                 render through ggrust's Python API"
                            ),
                        )
                    })?;
                labeller.transform = Some(Transform::Function(Arc::clone(function)));
            }
            (None, None) => {}
        }
        Ok(labeller)
    }

    /// The labels of one variable's levels after the per-value step (lookup or
    /// function; the function is called once with every level, numbers as their
    /// plain decimal text). Other labels with a superscript go as plain text.
    ///
    /// A lookup key matches the label text, or for a number its plain decimal
    /// text (`1000` matches the label `1,000`); `raw[i]` is that text of level `i`.
    pub fn transform_levels(
        &self,
        levels: Vec<Label>,
        raw: &[Option<String>],
    ) -> Result<Vec<Label>> {
        match &self.transform {
            None => Ok(levels),
            Some(Transform::Lookup(lookup)) => Ok(levels
                .into_iter()
                .enumerate()
                .map(|(index, label)| {
                    let found = lookup.get(&plain_text(&label)).or_else(|| {
                        raw.get(index)
                            .and_then(Option::as_ref)
                            .and_then(|text| lookup.get(text))
                    });
                    match found {
                        Some(text) => Label::plain(text.clone()),
                        None => label,
                    }
                })
                .collect()),
            Some(Transform::Function(function)) => {
                // Numbers go as their plain value (ggplot2 `as.character`).
                let texts: Vec<String> = levels
                    .iter()
                    .enumerate()
                    .map(
                        |(index, label)| match raw.get(index).and_then(Option::as_ref) {
                            Some(text) => text.clone(),
                            None => plain_text(label),
                        },
                    )
                    .collect();
                let mapped = function.call_text(&texts).map_err(|reason| {
                    Error::spec(
                        &self.path,
                        format!("the labeller function failed: {reason}"),
                    )
                })?;
                if mapped.len() != texts.len() {
                    return Err(Error::spec(
                        &self.path,
                        format!(
                            "the labeller function returned {} labels for {} levels",
                            mapped.len(),
                            texts.len()
                        ),
                    ));
                }
                Ok(mapped.into_iter().map(Label::plain).collect())
            }
        }
    }

    /// Strip labels of one strip group (the variables of a `facet_wrap`, or one
    /// margin of a `facet_grid`): one label per variable, or one joined label with
    /// `multi_line=false`. `values[i]` is variable `names[i]`'s transformed label.
    pub fn strips(&self, names: &[&str], values: Vec<Label>) -> Vec<Label> {
        let both = match self.base {
            Base::Value => false,
            Base::Both => true,
            Base::Context => names.len() > 1,
        };
        if self.multi_line || values.len() < 2 {
            return values
                .into_iter()
                .zip(names)
                .map(|(mut label, name)| {
                    if both {
                        label.text = format!("{name}{}{}", self.sep, label.text);
                    }
                    label
                })
                .collect();
        }
        let value = values.iter().map(plain_text).collect::<Vec<_>>().join(", ");
        let text = if both {
            format!("{}{}{value}", names.join(", "), self.sep)
        } else {
            value
        };
        vec![Label::plain(text)]
    }
}

/// A base labeller from its name: `label_value` or the short `value` (also
/// `both`, `context`).
fn base_from_name(name: &str, path: &str) -> Result<Base> {
    match name.strip_prefix("label_").unwrap_or(name) {
        "value" => Ok(Base::Value),
        "both" => Ok(Base::Both),
        "context" => Ok(Base::Context),
        _ => Err(Error::spec(
            path,
            format!(
                "unknown labeller '{name}'; use one of {}{}",
                NAMES.join(", "),
                crate::suggest::did_you_mean(name, NAMES)
            ),
        )),
    }
}

/// A label as one plain text: a superscript becomes `^exponent`.
fn plain_text(label: &Label) -> String {
    let mut text = label.text.clone();
    if let Some(superscript) = &label.superscript {
        text.push('^');
        text.push_str(superscript);
    }
    if let Some(suffix) = &label.suffix {
        text.push_str(suffix);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Upper;
    impl AxisFunction for Upper {
        fn call(&self, values: &[f64]) -> std::result::Result<Vec<f64>, String> {
            Ok(values.to_vec())
        }
        fn call_text(&self, values: &[String]) -> std::result::Result<Vec<String>, String> {
            Ok(values.iter().map(|value| value.to_uppercase()).collect())
        }
    }

    fn parse(json: &str) -> Result<Labeller> {
        let value: Value = serde_json::from_str(json).unwrap();
        let functions = AxisFunctions(vec![Arc::new(Upper)]);
        Labeller::parse(Some(&value), &functions, "facet.labeller")
    }

    fn texts(labels: &[Label]) -> Vec<&str> {
        labels.iter().map(|label| label.text.as_str()).collect()
    }

    fn labels(values: &[&str]) -> Vec<Label> {
        values
            .iter()
            .map(|value| Label::plain((*value).to_owned()))
            .collect()
    }

    #[test]
    fn value_both_and_context_follow_ggplot2() {
        let value = parse(r#""label_value""#).unwrap();
        assert_eq!(
            texts(&value.strips(&["a", "b"], labels(&["1", "x"]))),
            ["1", "x"]
        );
        let both = parse(r#""label_both""#).unwrap();
        assert_eq!(
            texts(&both.strips(&["a", "b"], labels(&["1", "x"]))),
            ["a: 1", "b: x"]
        );
        let context = parse(r#"{"base": "context"}"#).unwrap();
        assert_eq!(texts(&context.strips(&["a"], labels(&["1"]))), ["1"]);
        assert_eq!(
            texts(&context.strips(&["a", "b"], labels(&["1", "x"]))),
            ["a: 1", "b: x"]
        );
    }

    #[test]
    fn single_line_joins_names_and_values() {
        let both = parse(r#"{"base": "both", "multi_line": false, "sep": " = "}"#).unwrap();
        assert_eq!(
            texts(&both.strips(&["a", "b"], labels(&["1", "x"]))),
            ["a, b = 1, x"]
        );
        let value = parse(r#"{"base": "value", "multi_line": false}"#).unwrap();
        assert_eq!(
            texts(&value.strips(&["a", "b"], labels(&["1", "x"]))),
            ["1, x"]
        );
    }

    #[test]
    fn lookup_keeps_unknown_values_and_function_maps_all() {
        let lookup = parse(r#"{"lookup": {"4": "four"}, "default": {"base": "both"}}"#).unwrap();
        let mapped = lookup.transform_levels(labels(&["4", "6"]), &[]).unwrap();
        assert_eq!(texts(&mapped), ["four", "6"]);
        assert_eq!(texts(&lookup.strips(&["cyl"], mapped)), ["cyl: four"]);
        let function = parse(r#"{"function": 0}"#).unwrap();
        assert_eq!(
            texts(&function.transform_levels(labels(&["cat"]), &[]).unwrap()),
            ["CAT"]
        );
    }

    #[test]
    fn errors_name_the_option_and_suggest() {
        let error = parse(r#""label_bth""#).unwrap_err().to_string();
        assert!(error.contains("did you mean 'label_both'"), "{error}");
        let error = parse(r#"{"function": 3}"#).unwrap_err().to_string();
        assert!(error.contains("facet.labeller.function"), "{error}");
        let error = parse(r#"{"multiline": true}"#).unwrap_err().to_string();
        assert!(error.contains("did you mean 'multi_line'"), "{error}");
        let error = parse(r#"{"lookup": {"a": 1}}"#).unwrap_err().to_string();
        assert!(error.contains("facet.labeller.lookup.a"), "{error}");
    }
}
