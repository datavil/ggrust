//! `sampling_fraction(fraction, seed)` (ggrust extension): keep a random share of
//! the rows, `round(fraction * row_count)` of them (at least one), chosen as
//! [`super::random`] does.

use serde_json::Map;

use super::{Sampler, check_options, join, random, seed};
use crate::spec::Value;
use crate::{Error, Result};

/// Keeps a random share of the rows.
#[derive(Debug)]
pub struct FractionSampler {
    fraction: f64,
    seed: i64,
}

impl Sampler for FractionSampler {
    fn keep(&self, row_count: usize) -> Option<Vec<u32>> {
        let n = ((self.fraction * row_count as f64).round() as usize).max(1);
        (n < row_count).then(|| random::pick(row_count, n, self.seed))
    }
}

pub(super) fn parse(options: &Map<String, Value>, path: &str) -> Result<Box<dyn Sampler>> {
    const FUNCTION: &str = "sampling_fraction";
    check_options(options, &["fraction", "seed"], FUNCTION, path)?;
    let value = options.get("fraction").unwrap_or(&Value::Null);
    let fraction = value.as_f64().filter(|fraction| fraction.is_finite());
    let fraction = match fraction {
        Some(fraction) if fraction > 0.0 && fraction <= 1.0 => fraction,
        Some(fraction) if fraction > 1.0 && fraction <= 100.0 => {
            return Err(Error::spec(
                join(path, "fraction"),
                format!(
                    "{FUNCTION}() takes a share of the rows in (0, 1], got {value}; \
                     for {value}% use fraction={}",
                    fraction / 100.0
                ),
            ));
        }
        _ => {
            return Err(Error::spec(
                join(path, "fraction"),
                format!(
                    "{FUNCTION}() takes a share of the rows in (0, 1], such as 0.2 \
                     for 20%; got {value}"
                ),
            ));
        }
    };
    Ok(Box::new(FractionSampler {
        fraction,
        seed: seed(options, FUNCTION, path)?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sampler(fraction: f64) -> FractionSampler {
        FractionSampler { fraction, seed: 0 }
    }

    #[test]
    fn keeps_the_share() {
        assert_eq!(sampler(0.2).keep(1000).map(|rows| rows.len()), Some(200));
        assert_eq!(sampler(0.25).keep(10).map(|rows| rows.len()), Some(3));
        assert_eq!(sampler(0.001).keep(10).map(|rows| rows.len()), Some(1));
        assert_eq!(sampler(1.0).keep(10), None);
        assert_eq!(sampler(0.99).keep(10), None);
    }

    #[test]
    fn percent_values_suggest_the_share() {
        let error = parse(json!({"fraction": 20}).as_object().unwrap(), "s")
            .unwrap_err()
            .to_string();
        assert!(error.contains("for 20% use fraction=0.2"), "{error}");
        for bad in [json!(0), json!(-0.5), json!(500), json!("a"), json!(null)] {
            let options = json!({"fraction": bad});
            let error = parse(options.as_object().unwrap(), "s").unwrap_err();
            assert!(error.to_string().contains("(0, 1]"), "{error}");
        }
    }
}
