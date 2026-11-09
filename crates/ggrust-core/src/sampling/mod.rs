//! Layer sampling (`sampling=`): keep a subset of a layer's rows.
//!
//! Sampling runs on the stat output of each facet panel, before the position
//! adjustment, as lets-plot's does (a histogram samples its bins). A chain
//! (`sampling_random(...) + sampling_systematic(...)`) applies in order, each step
//! on the rows the previous one kept. Kept rows stay in their original order.
//! Rows with missing positions are dropped later, when drawn, so fewer than `n`
//! may show (lets-plot too).
//!
//! What it does not do: sample by default (ggrust draws every row), or pick the
//! same rows as lets-plot (the picks are ggrust's own). lets-plot's stratified,
//! group and vertex samplers ([`NOT_SUPPORTED`]) are reported as unsupported
//! options and skipped. One file per sampler; add it to [`SAMPLERS`].

mod fraction;
mod none;
mod random;
mod systematic;

use std::fmt;

use serde_json::Map;

use crate::spec::Value;
use crate::suggest::did_you_mean;
use crate::{Error, Result};

/// Seed of random samplers that give none, so renders are reproducible.
pub const DEFAULT_SEED: i64 = 0;

/// One sampling step.
pub trait Sampler: Send + Sync + fmt::Debug {
    /// The rows to keep out of `row_count`, ascending; `None` keeps every row.
    fn keep(&self, row_count: usize) -> Option<Vec<u32>>;
}

/// Parse a sampler's options (`n`, `seed`, ...); the path is the sampler's spec.
type Parse = fn(&Map<String, Value>, &str) -> Result<Box<dyn Sampler>>;

/// Every sampler the engine applies, by spec name: lookup and names derive from it.
pub static SAMPLERS: &[(&str, Parse)] = &[
    ("none", none::parse),
    ("random", random::parse),
    ("systematic", systematic::parse),
    ("fraction", fraction::parse),
];

/// lets-plot samplers the Python API builds but the engine skips with a warning.
/// Move a name to [`SAMPLERS`] when it is implemented.
pub static NOT_SUPPORTED: &[&str] = &[
    "random_stratified",
    "pick",
    "group_random",
    "group_systematic",
    "vertex_vw",
    "vertex_dp",
];

/// A layer's sampling: its applied steps, in order.
#[derive(Debug, Default)]
pub struct Sampling {
    steps: Vec<Box<dyn Sampler>>,
}

impl Sampling {
    /// Parse a layer's `sampling` value at `path` (e.g. `layers[0].sampling`):
    /// `"none"`, one `sampling_*()` spec, or a `feature-list` chain of them.
    /// Samplers in [`NOT_SUPPORTED`] are skipped (see [`unsupported_paths`]).
    pub fn parse(value: &Value, path: &str) -> Result<Sampling> {
        let mut steps = Vec::new();
        for (entry_path, options) in entries(value, path)? {
            let name = sampler_name(options, &entry_path)?;
            if NOT_SUPPORTED.contains(&name) {
                continue;
            }
            let Some((_, parse)) = SAMPLERS.iter().find(|(known, _)| *known == name) else {
                let names = known_names();
                return Err(Error::spec(
                    join(&entry_path, "name"),
                    format!(
                        "unknown sampling '{name}'; accepted: {}{}",
                        names.join(", "),
                        did_you_mean(name, names.iter().copied())
                    ),
                ));
            };
            steps.push(parse(options, &entry_path)?);
        }
        Ok(Sampling { steps })
    }

    /// The rows to keep out of `row_count`, ascending; `None` keeps every row.
    pub fn keep(&self, row_count: usize) -> Option<Vec<u32>> {
        let mut kept: Option<Vec<u32>> = None;
        for step in &self.steps {
            let count = kept.as_ref().map_or(row_count, Vec::len);
            if let Some(rows) = step.keep(count) {
                kept = Some(match kept {
                    None => rows,
                    Some(previous) => rows.iter().map(|&row| previous[row as usize]).collect(),
                });
            }
        }
        kept
    }
}

/// Spec paths of the steps of `value` that ggrust skips ([`NOT_SUPPORTED`]).
/// Malformed values give none here: [`Sampling::parse`] reports them.
pub fn unsupported_paths(value: &Value, path: &str) -> Vec<String> {
    let Ok(entries) = entries(value, path) else {
        return Vec::new();
    };
    entries
        .into_iter()
        .filter(|(_, options)| {
            options
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|name| NOT_SUPPORTED.contains(&name))
        })
        .map(|(entry_path, _)| entry_path)
        .collect()
}

/// Names for messages: applied samplers, then the skipped lets-plot ones.
fn known_names() -> Vec<&'static str> {
    SAMPLERS
        .iter()
        .map(|(name, _)| *name)
        .chain(NOT_SUPPORTED.iter().copied())
        .collect()
}

const ACCEPTED: &str = "accepted: \"none\", a sampling_*() spec such as \
    sampling_random(5000), or several joined with +";

/// The sampler specs of `value` with their paths, flattening `feature-list` chains.
fn entries<'a>(value: &'a Value, path: &str) -> Result<Vec<(String, &'a Map<String, Value>)>> {
    match value {
        Value::String(text) if text == "none" => Ok(Vec::new()),
        Value::Object(object) => match object.get("feature-list") {
            Some(Value::Array(elements)) => {
                let mut found = Vec::new();
                for (index, element) in elements.iter().enumerate() {
                    let element_path = format!("{}[{index}]", join(path, "feature-list"));
                    let Some(inner) = element.get("sampling") else {
                        return Err(Error::spec(
                            element_path,
                            format!("only sampling specs can be joined with +; {ACCEPTED}"),
                        ));
                    };
                    found.extend(entries(inner, &join(&element_path, "sampling"))?);
                }
                Ok(found)
            }
            Some(_) => Err(Error::spec(
                join(path, "feature-list"),
                format!("expected a list of sampling specs; {ACCEPTED}"),
            )),
            None => Ok(vec![(path.to_owned(), object)]),
        },
        other => Err(Error::spec(
            path,
            format!("{} is not a sampling; {ACCEPTED}", describe(other)),
        )),
    }
}

fn sampler_name<'a>(options: &'a Map<String, Value>, path: &str) -> Result<&'a str> {
    options.get("name").and_then(Value::as_str).ok_or_else(|| {
        Error::spec(
            join(path, "name"),
            format!("missing sampler name; {ACCEPTED}"),
        )
    })
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_owned()
    } else {
        format!("{path}.{key}")
    }
}

fn describe(value: &Value) -> String {
    match value {
        Value::String(text) => format!("'{text}'"),
        other => other.to_string(),
    }
}

/// Reject options other than `name` and `accepted` (typos such as `seeds`).
fn check_options(
    options: &Map<String, Value>,
    accepted: &[&str],
    function: &str,
    path: &str,
) -> Result<()> {
    for key in options.keys().filter(|key| *key != "name") {
        if !accepted.contains(&key.as_str()) {
            return Err(Error::spec(
                join(path, key),
                format!(
                    "{function}() has no option '{key}'; accepted: {}{}",
                    accepted.join(", "),
                    did_you_mean(key, accepted.iter().copied())
                ),
            ));
        }
    }
    Ok(())
}

/// The required row count `n`: an integer >= 1.
fn count(options: &Map<String, Value>, function: &str, path: &str) -> Result<usize> {
    let value = options.get("n").unwrap_or(&Value::Null);
    let parsed = match value {
        // Integers beyond u64 arrive as floats: saturate (they keep every row).
        Value::Number(number) => number.as_u64().or_else(|| {
            number
                .as_f64()
                .filter(|float| float.is_finite() && float.fract() == 0.0 && *float >= 0.0)
                .map(|float| float as u64)
        }),
        _ => None,
    };
    match parsed {
        Some(n) if n >= 1 => Ok(usize::try_from(n).unwrap_or(usize::MAX)),
        _ => Err(Error::spec(
            join(path, "n"),
            format!(
                "{function}() needs n, the number of rows to keep, as an integer >= 1; got {}",
                describe(value)
            ),
        )),
    }
}

/// The optional integer `seed` ([`DEFAULT_SEED`] when missing).
fn seed(options: &Map<String, Value>, function: &str, path: &str) -> Result<i64> {
    match options.get("seed") {
        None | Some(Value::Null) => Ok(DEFAULT_SEED),
        Some(Value::Number(number)) if number.as_i64().is_some() => {
            Ok(number.as_i64().unwrap_or(DEFAULT_SEED))
        }
        Some(other) => Err(Error::spec(
            join(path, "seed"),
            format!(
                "{function}() takes an integer seed (or none); got {}",
                describe(other)
            ),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(value: Value) -> Result<Sampling> {
        Sampling::parse(&value, "layers[0].sampling")
    }

    fn message(value: Value) -> String {
        parse(value).unwrap_err().to_string()
    }

    #[test]
    fn none_keeps_every_row() {
        assert_eq!(parse(json!("none")).unwrap().keep(10), None);
        assert_eq!(parse(json!({"name": "none"})).unwrap().keep(10), None);
    }

    #[test]
    fn chains_apply_in_order() {
        let sampling = parse(json!({"feature-list": [
            {"sampling": {"name": "systematic", "n": 50}},
            {"sampling": {"name": "systematic", "n": 10}},
        ]}))
        .unwrap();
        // Every 2nd of 100, then every 5th of those 50: every 10th row.
        let expected: Vec<u32> = (0..10).map(|row| row * 10).collect();
        assert_eq!(sampling.keep(100), Some(expected));
    }

    #[test]
    fn unsupported_samplers_are_skipped_and_reported() {
        let value = json!({"feature-list": [
            {"sampling": {"name": "pick", "n": 5}},
            {"sampling": {"name": "random", "n": 5}},
        ]});
        let kept = parse(value.clone()).unwrap().keep(100).unwrap();
        assert_eq!(kept.len(), 5);
        assert_eq!(
            unsupported_paths(&value, "layers[0].sampling"),
            ["layers[0].sampling.feature-list[0].sampling"]
        );
        assert!(unsupported_paths(&json!("none"), "s").is_empty());
    }

    #[test]
    fn errors_name_the_path_and_the_fix() {
        let error = message(json!({"name": "randon", "n": 5}));
        assert!(error.contains("layers[0].sampling.name"), "{error}");
        assert!(error.contains("did you mean 'random'?"), "{error}");
        let error = message(json!("all"));
        assert!(error.contains("'all' is not a sampling"), "{error}");
        let error = message(json!({"name": "random", "n": 0}));
        assert!(error.contains("integer >= 1; got 0"), "{error}");
        // Huge integers (Python sends them as floats) keep every row.
        assert_eq!(
            parse(json!({"name": "random", "n": 1e30}))
                .unwrap()
                .keep(10),
            None
        );
        let error = message(json!({"name": "random", "n": 2.5}));
        assert!(error.contains("got 2.5"), "{error}");
        let error = message(json!({"name": "random", "n": 5, "seeds": 1}));
        assert!(error.contains("no option 'seeds'"), "{error}");
        assert!(error.contains("did you mean 'seed'?"), "{error}");
        let error = message(json!({"name": "random", "n": 5, "seed": "a"}));
        assert!(error.contains("integer seed"), "{error}");
        let error = message(json!({"feature-list": [{"position": {"name": "dodge"}}]}));
        assert!(error.contains("feature-list[0]"), "{error}");
    }
}
