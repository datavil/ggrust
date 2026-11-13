//! ggpubr's `compare_means()` table: the p-values of tests between the levels of
//! a grouping variable, for Python's `ggrust.compare_means`.
//!
//! The caller splits the data into partitions (ggpubr's `group.by`, one partition
//! without it) and each partition into the values of every level (the same level
//! list for every partition; a level may be empty). Rows follow ggpubr:
//!
//! - `t.test` / `wilcox.test`: every pair of non-empty levels, ordered as R's
//!   `pairwise.*.test` matrix read row by row: `(1, 2), (1, 3), (2, 3), (1, 4)...`.
//!   With a reference level, the pairs with it (`group1` = the reference, the
//!   others in level order); with [`Reference::All`], each level against all the
//!   partition's values (`group1` = `.all.`).
//! - `anova` / `kruskal.test`: one row per partition (no groups).
//!
//! `p.adj` is R's `p.adjust` over every row (ggpubr adjusts per response, and
//! there is one), rounded to 2 significant digits as ggpubr; `p.format` is
//! `format.pval(p, digits = 2)` of each p on its own (ggpubr formats the column at
//! once, see deviations.md); `p.signif` comes from the unadjusted p, as ggpubr.
//!
//! What it does not do: several responses (`c(a, b) ~ g`), one-sample tests
//! (`y ~ 1`), `symnum.args`, other `method` options (`method.args`).

use super::hypothesis::{Adjustment, Method};
use super::pubr_labels::{p_format, p_signif, signif};
use crate::{Error, Result};

/// The reference of the comparisons (ggpubr `ref.group`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reference {
    /// Every pair of levels.
    None,
    /// The pairs with this level (an index into the level list).
    Level(usize),
    /// Each level against all values (`".all."`).
    All,
}

/// The `compare_means()` table, one entry per row.
#[derive(Debug, Clone, PartialEq)]
pub struct CompareTable {
    /// The partition of each row.
    pub partition: Vec<usize>,
    /// First level of each comparison (`None`: `.all.`); empty for
    /// `anova` / `kruskal.test`.
    pub group1: Vec<Option<usize>>,
    /// Second level of each comparison; empty for `anova` / `kruskal.test`.
    pub group2: Vec<usize>,
    /// The p-value of each row.
    pub p: Vec<f64>,
    /// `p.adj`.
    pub p_adjusted: Vec<f64>,
    /// `p.format`.
    pub p_format: Vec<String>,
    /// `p.signif`.
    pub p_signif: Vec<String>,
    /// ggpubr's method name (`"Wilcoxon"`, `"T-test"`, `"Anova"`, `"Kruskal-Wallis"`).
    pub method: &'static str,
}

/// Test the levels of every partition (see the module docs). `levels` names the
/// levels (for messages); `method` and `p_adjust_method` take ggpubr's names.
///
/// # Errors
///
/// An unknown method or adjustment, a reference outside the levels, or a test R
/// rejects (constant data for a t test, unequal pairs for a paired test).
pub fn compare_means(
    partitions: &[Vec<Vec<f64>>],
    levels: &[String],
    method: &str,
    paired: bool,
    reference: Reference,
    p_adjust_method: &str,
) -> Result<CompareTable> {
    let method_value = Method::parse(method).ok_or_else(|| {
        Error::spec(
            "method",
            format!(
                "unknown method '{method}'{}; use one of: {}",
                crate::suggest::did_you_mean(method, Method::ALIASES),
                Method::NAMES.join(", ")
            ),
        )
    })?;
    let adjustment = Adjustment::parse(p_adjust_method).ok_or_else(|| {
        Error::spec(
            "p_adjust_method",
            format!(
                "unknown p_adjust_method '{p_adjust_method}'{}; use one of: {}",
                crate::suggest::did_you_mean(p_adjust_method, Adjustment::NAMES),
                Adjustment::NAMES.join(", ")
            ),
        )
    })?;
    if let Reference::Level(index) = reference
        && index >= levels.len()
    {
        return Err(Error::spec(
            "ref_group",
            format!(
                "reference level {index} is out of range; there are {} levels",
                levels.len()
            ),
        ));
    }
    if let Some(partition) = partitions.iter().find(|p| p.len() != levels.len()) {
        return Err(Error::spec(
            "data",
            format!(
                "a partition has {} groups of values, expected one per level ({})",
                partition.len(),
                levels.len()
            ),
        ));
    }
    let present = |values: &[f64]| values.iter().any(|value| !value.is_nan());
    let fail = |what: String, reason: String| {
        Error::spec("method", format!("cannot test {what}: {reason}"))
    };
    let mut table = CompareTable {
        partition: Vec::new(),
        group1: Vec::new(),
        group2: Vec::new(),
        p: Vec::new(),
        p_adjusted: Vec::new(),
        p_format: Vec::new(),
        p_signif: Vec::new(),
        method: method_value.label(),
    };
    for (index, groups) in partitions.iter().enumerate() {
        if !method_value.is_two_sample() {
            let slices: Vec<&[f64]> = groups
                .iter()
                .map(Vec::as_slice)
                .filter(|values| present(values))
                .collect();
            if slices.len() < 2 {
                continue;
            }
            let p = method_value
                .p_value(&slices, false)
                .map_err(|reason| fail("the levels".to_owned(), reason))?;
            table.partition.push(index);
            table.p.push(p);
            continue;
        }
        let pairs: Vec<(Option<usize>, usize)> = match reference {
            Reference::None => (1..levels.len())
                .flat_map(|second| (0..second).map(move |first| (Some(first), second)))
                .collect(),
            Reference::Level(at) => (0..levels.len())
                .filter(|&other| other != at)
                .map(|other| (Some(at), other))
                .collect(),
            Reference::All => (0..levels.len()).map(|level| (None, level)).collect(),
        };
        let all: Vec<f64> = groups.iter().flatten().copied().collect();
        for (first, second) in pairs {
            let first_values = first.map_or(all.as_slice(), |at| groups[at].as_slice());
            let second_values = groups[second].as_slice();
            if !present(first_values) || !present(second_values) {
                continue;
            }
            let p = method_value
                .p_value(&[first_values, second_values], paired)
                .map_err(|reason| {
                    let name = |level: Option<usize>| match level {
                        Some(at) => format!("'{}'", levels[at]),
                        None => "'.all.'".to_owned(),
                    };
                    fail(format!("{} vs {}", name(first), name(Some(second))), reason)
                })?;
            table.partition.push(index);
            table.group1.push(first);
            table.group2.push(second);
            table.p.push(p);
        }
    }
    table.p_adjusted = adjustment
        .adjust(&table.p)
        .into_iter()
        .map(|p| signif(p, 2))
        .collect();
    table.p_format = table.p.iter().map(|p| p_format(*p)).collect();
    table.p_signif = table.p.iter().map(|p| p_signif(*p).to_owned()).collect();
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// tests/fixtures/pubr_helpers_reference.json, from
    /// generate_pubr_helpers_reference.R.
    fn reference() -> Value {
        serde_json::from_str(include_str!(
            "../../../../tests/fixtures/pubr_helpers_reference.json"
        ))
        .unwrap()
    }

    fn column<'a>(data: &'a Value, name: &str) -> Vec<&'a Value> {
        match &data[name] {
            Value::Array(items) => items.iter().collect(),
            _ => data
                .as_array()
                .unwrap()
                .iter()
                .map(|row| &row[name])
                .collect(),
        }
    }

    fn text(value: &Value) -> String {
        match value {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        }
    }

    /// Levels in first appearance, as ggpubr's `factor(levels = unique(x))`.
    fn unique(values: &[String]) -> Vec<String> {
        let mut seen: Vec<String> = Vec::new();
        for value in values {
            if !seen.contains(value) {
                seen.push(value.clone());
            }
        }
        seen
    }

    #[test]
    fn tables_match_ggpubr() {
        let all = reference();
        for case in all["compare_means"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let formula = case["formula"].as_str().unwrap();
            let (response, grouping) = formula.split_once(" ~ ").unwrap();
            let data = if formula.starts_with("v ~") {
                &all["data"]["four"]
            } else {
                &all["data"]["tooth"]
            };
            let y: Vec<f64> = column(data, response)
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect();
            let groups: Vec<String> = column(data, grouping).iter().map(|v| text(v)).collect();
            // ToothGrowth's supp is a factor with levels OJ, VC.
            let levels = match grouping {
                "supp" => vec!["OJ".to_owned(), "VC".to_owned()],
                _ => unique(&groups),
            };
            let options = &case["options"];
            let by: Option<Vec<String>> = options["group.by"]
                .as_str()
                .map(|by| column(data, by).iter().map(|v| text(v)).collect());
            // ggpubr keeps the group.by keys in first appearance (ToothGrowth: VC, OJ).
            let keys: Vec<String> = by.clone().map(|by| unique(&by)).unwrap_or_default();
            let partition_of = |row: usize| match &by {
                Some(by) => keys.iter().position(|key| *key == by[row]).unwrap(),
                None => 0,
            };
            let mut partitions = vec![vec![Vec::new(); levels.len()]; keys.len().max(1)];
            for row in 0..y.len() {
                let level = levels.iter().position(|l| *l == groups[row]).unwrap();
                partitions[partition_of(row)][level].push(y[row]);
            }
            let reference = match options["ref.group"].as_str() {
                None => Reference::None,
                Some(".all.") => Reference::All,
                Some(level) => Reference::Level(levels.iter().position(|l| l == level).unwrap()),
            };
            let table = compare_means(
                &partitions,
                &levels,
                options["method"].as_str().unwrap_or("wilcox.test"),
                options["paired"].as_bool().unwrap_or(false),
                reference,
                options["p.adjust.method"].as_str().unwrap_or("holm"),
            )
            .unwrap();
            let rows = case["rows"].as_array().unwrap();
            assert_eq!(table.p.len(), rows.len(), "{name}");
            for (index, row) in rows.iter().enumerate() {
                let expected = row["p"].as_f64().unwrap();
                assert!(
                    (table.p[index] - expected).abs() <= 1e-6 * expected.max(1e-300),
                    "{name} row {index}: p {} vs {expected}",
                    table.p[index]
                );
                assert_eq!(
                    table.p_adjusted[index],
                    row["p.adj"].as_f64().unwrap(),
                    "{name}"
                );
                assert_eq!(table.p_format[index], text(&row["p.format.each"]), "{name}");
                assert_eq!(table.p_signif[index], text(&row["p.signif"]), "{name}");
                assert_eq!(table.method, text(&row["method"]), "{name}");
                if let Some(group1) = row["group1"].as_str() {
                    let first = match table.group1[index] {
                        Some(at) => levels[at].as_str(),
                        None => ".all.",
                    };
                    assert_eq!(first, group1, "{name} row {index}");
                    assert_eq!(levels[table.group2[index]], text(&row["group2"]), "{name}");
                }
                if let Some(by) = options["group.by"].as_str() {
                    assert_eq!(keys[table.partition[index]], text(&row[by]), "{name}");
                }
            }
        }
    }

    #[test]
    fn errors_name_the_problem() {
        let levels = vec!["a".to_owned(), "b".to_owned()];
        let partitions = vec![vec![vec![1.0, 1.0, 1.0], vec![1.0, 1.0, 1.0]]];
        let message = |method: &str, adjust: &str, reference| {
            compare_means(&partitions, &levels, method, false, reference, adjust)
                .unwrap_err()
                .to_string()
        };
        assert!(message("t.tset", "holm", Reference::None).contains("did you mean 't.test'"));
        assert!(message("t.test", "holmm", Reference::None).contains("did you mean 'holm'"));
        assert!(message("t.test", "holm", Reference::Level(5)).contains("out of range"));
        let constant = message("t.test", "holm", Reference::None);
        assert!(constant.contains("cannot test 'a' vs 'b'"), "{constant}");
    }
}
