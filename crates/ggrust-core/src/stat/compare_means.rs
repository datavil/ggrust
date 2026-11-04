//! `stat="compare_means"`: p-values of tests between the groups of y along x,
//! as the R package ggpubr's `stat_compare_means` places and labels them.
//!
//! Three layouts, by the options:
//! - `comparisons` (pairs of x levels, by name or 1-based position): one bracket
//!   row per pair (`xmin`, `xmax`, `y`, `label`, tip lengths `lenstart` /
//!   `lenend`) for `geom_bracket`, as ggsignif: the i-th bracket at the top of
//!   the y range + `(0.05 + step_increase (i - 1))` of the range (from `label_y`
//!   when given), tips `tip_length` (0.03) of the range. `step_increase` is
//!   0.12 unless `label_y` is set. Labels: `p.format` (default) or `p.signif`.
//! - `ref_group` (an x level or `".all."`): each other x level (every level for
//!   `".all."`) against the reference, a label over each x at the top of the y
//!   range.
//! - otherwise, one global test across x (a label at the left of the x range,
//!   `hjust` 0.2); when other aesthetics split x into several groups, a test per
//!   x between those groups (a label over each x).
//!
//! Label rows carry `x`, `y`, `label`, `hjust`, `vjust` (0) for `geom_text`;
//! constant `hjust`/`vjust` options win. Labels are `"<method>, p = <p>"` by
//! default, or `label="p.format"` / `"p.signif"`; `hide_ns` blanks `ns` stars.
//!
//! `method` (`t.test`, `wilcox.test`, `anova`, `kruskal.test`; default
//! `wilcox.test` for two groups, `kruskal.test` for more), `paired` (t and
//! Wilcoxon tests, values paired in row order), `p_adjust_method` (R
//! `p.adjust`, over the layer panel's tests; default `"none"`, as ggpubr).
//! The tests are `stat/hypothesis.rs`.
//!
//! What it does not do: `label.x.npc`/`label.y.npc`, `method.args`, `symnum.args`,
//! `id`, other `p.format.style`s, `comparisons` together with color groups.

use super::hypothesis::{Adjustment, Method};
use super::pubr_labels::{column_range, label_column, p_equals, p_format, p_signif};
use super::{Stat, StatContext};
use crate::data::{Column, ContinuousColumn, LayerData, SourceRows};
use crate::parameters::Parameters;
use crate::spec::{Aesthetic, Value};
use crate::{Error, Result};

/// Group comparisons with p-value labels.
pub struct CompareMeansStat;

/// ggsignif `margin_top`: the first bracket's height above the y range.
const MARGIN_TOP: f64 = 0.05;
/// ggpubr's `step.increase` without `label.y`.
const STEP_INCREASE: f64 = 0.12;
/// ggpubr `tip.length`.
const TIP_LENGTH: f64 = 0.03;
/// ggpubr's `hjust` for the global label.
const GLOBAL_HJUST: f64 = 0.2;

impl Stat for CompareMeansStat {
    fn name(&self) -> &'static str {
        "compare_means"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &[
            "method",
            "paired",
            "comparisons",
            "ref_group",
            "hide_ns",
            "label",
            "label_sep",
            "label_x",
            "label_y",
            "step_increase",
            "tip_length",
            "p_adjust_method",
        ]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        context: &StatContext,
    ) -> Result<LayerData> {
        let options = Options::parse(parameters)?;
        let (Some(x), Some(y)) = (data.continuous(Aesthetic::X), data.continuous(Aesthetic::Y))
        else {
            return Ok(LayerData::new(0));
        };
        let rows: Vec<usize> = (0..data.row_count()).collect();
        let y_range = context
            .y_range
            .or_else(|| column_range(Some(y), &rows))
            .unwrap_or((0.0, 1.0));
        let positions = distinct_positions(x);
        let values_at = |position: f64| -> Vec<f64> {
            rows.iter()
                .filter(|&&row| x.get(row) == Some(position))
                .map(|&row| y.get(row).unwrap_or(f64::NAN))
                .collect()
        };
        if let Some(comparisons) = parameters.value("comparisons").filter(|v| !v.is_null()) {
            let pairs = level_pairs(comparisons, parameters, context)?;
            return brackets(&pairs, &values_at, &options, parameters, context, y_range);
        }
        let mut labels = Vec::new();
        if let Some(reference) = parameters.value("ref_group").filter(|v| !v.is_null()) {
            let method = options.method.unwrap_or(Method::WilcoxTest);
            let (reference_values, others) = match reference.as_str() {
                Some(".all.") => {
                    let all: Vec<f64> = rows
                        .iter()
                        .map(|&row| y.get(row).unwrap_or(f64::NAN))
                        .collect();
                    (all, positions.clone())
                }
                _ => {
                    let at = level_position(reference, "ref_group", parameters, context)?;
                    let others = positions.iter().copied().filter(|p| *p != at).collect();
                    (values_at(at), others)
                }
            };
            for position in others {
                let group = values_at(position);
                let what = format!("{} against the reference", level_name(context, position));
                let tested = run(
                    method,
                    &[&reference_values, &group],
                    &what,
                    &options,
                    parameters,
                )?;
                if let Some(p) = tested {
                    labels.push((position, 0.5, method, p));
                }
            }
        } else if let Some(per_x) = groups_within_x(&data, x, &positions) {
            let group_count = {
                let mut seen: Vec<u32> = (0..data.row_count())
                    .map(|row| data.group_of(row))
                    .collect();
                seen.sort_unstable();
                seen.dedup();
                seen.len()
            };
            let method = options.method.unwrap_or(match group_count {
                2 => Method::WilcoxTest,
                _ => Method::KruskalTest,
            });
            for (position, groups) in per_x {
                let values: Vec<Vec<f64>> = groups
                    .iter()
                    .map(|rows| {
                        rows.iter()
                            .map(|&row| y.get(row).unwrap_or(f64::NAN))
                            .collect()
                    })
                    .collect();
                if values.len() < 2 {
                    continue;
                }
                let slices: Vec<&[f64]> = values.iter().map(Vec::as_slice).collect();
                let what = format!("the groups at {}", level_name(context, position));
                if let Some(p) = run(method, &slices, &what, &options, parameters)? {
                    labels.push((position, 0.5, method, p));
                }
            }
        } else {
            let method = options.method.unwrap_or(match positions.len() {
                2 => Method::WilcoxTest,
                _ => Method::KruskalTest,
            });
            if positions.len() >= 2 {
                let values: Vec<Vec<f64>> = positions.iter().map(|p| values_at(*p)).collect();
                let slices: Vec<&[f64]> = values.iter().map(Vec::as_slice).collect();
                let x_low = context.x_range.map(|range| range.0).unwrap_or(positions[0]);
                if let Some(p) = run(method, &slices, "the x categories", &options, parameters)? {
                    labels.push((x_low, GLOBAL_HJUST, method, p));
                }
            }
        }
        let adjusted = options
            .adjustment
            .adjust(&labels.iter().map(|l| l.3).collect::<Vec<_>>());
        let texts: Vec<String> = labels
            .iter()
            .zip(&adjusted)
            .map(|((_, _, method, _), p)| options.text(*method, *p, false))
            .collect();
        let label_y = |index: usize| match &options.label_y {
            Some(values) => values[index.min(values.len() - 1)],
            None => y_range.1,
        };
        let mut result = LayerData::new(labels.len());
        let x_values: Vec<f64> = match options.label_x {
            Some(x) if labels.len() == 1 => vec![x],
            _ => labels.iter().map(|label| label.0).collect(),
        };
        let hjust: Vec<f64> = match options.label_x {
            Some(_) if labels.len() == 1 => vec![0.5],
            _ => labels.iter().map(|label| label.1).collect(),
        };
        let columns = [
            (Aesthetic::X, x_values),
            (Aesthetic::Y, (0..labels.len()).map(label_y).collect()),
            (Aesthetic::Hjust, hjust),
            (Aesthetic::Vjust, vec![0.0; labels.len()]),
        ];
        for (aesthetic, values) in columns {
            if matches!(aesthetic, Aesthetic::Hjust | Aesthetic::Vjust)
                && parameters.value(aesthetic.as_str()).is_some()
            {
                continue;
            }
            result.insert(aesthetic, continuous(values))?;
        }
        result.insert(Aesthetic::Label, label_column(&texts))?;
        result.set_source_rows(SourceRows::Aggregated)?;
        Ok(result)
    }
}

fn continuous(values: Vec<f64>) -> Column {
    Column::Continuous(ContinuousColumn::from_values(values))
}

/// What a label shows (`label`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LabelKind {
    /// `"<method>, p = <p>"` (brackets: the p-value).
    Default,
    Format,
    Signif,
}

/// Parsed options.
struct Options {
    method: Option<Method>,
    paired: bool,
    hide_ns: bool,
    label: LabelKind,
    separator: String,
    label_x: Option<f64>,
    label_y: Option<Vec<f64>>,
    step_increase: Option<f64>,
    tip_length: f64,
    adjustment: Adjustment,
}

impl Options {
    fn parse(parameters: &Parameters) -> Result<Self> {
        let method = match parameters.string("method")? {
            None => None,
            Some(name) => Some(Method::parse(name).ok_or_else(|| {
                Error::spec(
                    parameters.path_of("method"),
                    format!(
                        "unknown method '{name}'{}; use one of: {}",
                        crate::suggest::did_you_mean(name, Method::ALIASES),
                        Method::NAMES.join(", ")
                    ),
                )
            })?),
        };
        let label = match parameters.string("label")? {
            None => LabelKind::Default,
            Some("p.format") => LabelKind::Format,
            Some("p.signif") => LabelKind::Signif,
            Some(other) => {
                const LABELS: [&str; 2] = ["p.format", "p.signif"];
                return Err(Error::spec(
                    parameters.path_of("label"),
                    format!(
                        "unknown label '{other}'{}; use one of: {}",
                        crate::suggest::did_you_mean(other, LABELS),
                        LABELS.join(", ")
                    ),
                ));
            }
        };
        let adjustment = match parameters.string("p_adjust_method")? {
            None => Adjustment::None,
            Some(name) => Adjustment::parse(name).ok_or_else(|| {
                Error::spec(
                    parameters.path_of("p_adjust_method"),
                    format!(
                        "unknown p_adjust_method '{name}'{}; use one of: {}",
                        crate::suggest::did_you_mean(name, Adjustment::NAMES),
                        Adjustment::NAMES.join(", ")
                    ),
                )
            })?,
        };
        let label_y = match parameters.value("label_y") {
            None | Some(Value::Null) => None,
            Some(value) => {
                let invalid = || {
                    Error::spec(
                        parameters.path_of("label_y"),
                        format!("expected a number or a non-empty list of numbers, got {value}"),
                    )
                };
                let values: Vec<f64> = match value {
                    Value::Array(items) if !items.is_empty() => items
                        .iter()
                        .map(|item| item.as_f64().filter(|v| v.is_finite()).ok_or_else(invalid))
                        .collect::<Result<_>>()?,
                    Value::Number(number) => vec![number.as_f64().ok_or_else(invalid)?],
                    _ => return Err(invalid()),
                };
                Some(values)
            }
        };
        let non_negative = |name: &str| -> Result<Option<f64>> {
            match parameters.number(name)? {
                Some(value) if value < 0.0 => Err(Error::spec(
                    parameters.path_of(name),
                    format!("expected a number >= 0, got {value}"),
                )),
                other => Ok(other),
            }
        };
        Ok(Options {
            method,
            paired: parameters.boolean("paired")?.unwrap_or(false),
            hide_ns: parameters.boolean("hide_ns")?.unwrap_or(false),
            label,
            separator: parameters.string("label_sep")?.unwrap_or(", ").to_owned(),
            label_x: parameters.number("label_x")?,
            label_y,
            step_increase: non_negative("step_increase")?,
            tip_length: non_negative("tip_length")?.unwrap_or(TIP_LENGTH),
            adjustment,
        })
    }

    /// A label's text (`bracket`: a `comparisons` bracket, which shows only the p-value).
    fn text(&self, method: Method, p: f64, bracket: bool) -> String {
        match self.label {
            LabelKind::Signif => match p_signif(p) {
                "ns" if self.hide_ns => if bracket { "" } else { " " }.to_owned(),
                stars => stars.to_owned(),
            },
            LabelKind::Format => p_format(p),
            LabelKind::Default if bracket => p_format(p),
            LabelKind::Default => format!("{}{}{}", method.label(), self.separator, p_equals(p)),
        }
    }
}

/// The p-value of one test of `what` (e.g. `'0.5' vs '1'`), or an error naming
/// it; `None` when a group has no values here (a facet panel without that
/// category): that test gets no label, as ggsignif leaves it out.
fn run(
    method: Method,
    groups: &[&[f64]],
    what: &str,
    options: &Options,
    parameters: &Parameters,
) -> Result<Option<f64>> {
    let present = |group: &&[f64]| group.iter().any(|value| !value.is_nan());
    let groups: Vec<&[f64]> = match method.is_two_sample() {
        true if !groups.iter().all(present) => return Ok(None),
        true => groups.to_vec(),
        false => groups.iter().copied().filter(present).collect(),
    };
    if groups.len() < 2 {
        return Ok(None);
    }
    if method.is_two_sample() && groups.len() != 2 {
        return Err(Error::spec(
            parameters.path_of("method"),
            format!(
                "{} compares 2 groups, but the layer has {}; use method='anova' or \
                 'kruskal.test', or compare pairs with comparisons= or ref_group=",
                Method::NAMES[method as usize],
                groups.len()
            ),
        ));
    }
    method
        .p_value(&groups, options.paired)
        .map(Some)
        .map_err(|reason| {
            Error::spec(
                parameters.path_of("method"),
                format!("cannot test {what}: {reason}"),
            )
        })
}

/// A category's name for messages.
fn level_name(context: &StatContext, position: f64) -> String {
    match context
        .x_levels
        .and_then(|levels| levels.get(position as usize))
    {
        Some(name) => format!("'{name}'"),
        None => format!("x = {position}"),
    }
}

/// Distinct x positions, sorted.
fn distinct_positions(x: &ContinuousColumn) -> Vec<f64> {
    let mut positions: Vec<f64> = (0..x.len())
        .filter_map(|row| x.get(row))
        .filter(|value| value.is_finite())
        .collect();
    positions.sort_by(f64::total_cmp);
    positions.dedup();
    positions
}

/// Rows of each group (by group index) at one x.
type GroupRows = Vec<(u32, Vec<usize>)>;

/// Rows of each group per x, when some x holds more than one group (color
/// groups within x); `None` when every x is a single group.
fn groups_within_x(
    data: &LayerData,
    x: &ContinuousColumn,
    positions: &[f64],
) -> Option<Vec<(f64, Vec<Vec<usize>>)>> {
    let mut per_x: Vec<(f64, GroupRows)> = positions.iter().map(|p| (*p, Vec::new())).collect();
    for row in 0..data.row_count() {
        let Some(position) = x.get(row) else {
            continue;
        };
        let Some(slot) = per_x.iter_mut().find(|(p, _)| *p == position) else {
            continue;
        };
        let group = data.group_of(row);
        match slot.1.iter_mut().find(|(known, _)| *known == group) {
            Some((_, rows)) => rows.push(row),
            None => slot.1.push((group, vec![row])),
        }
    }
    if per_x.iter().all(|(_, groups)| groups.len() <= 1) {
        return None;
    }
    Some(
        per_x
            .into_iter()
            .map(|(position, mut groups)| {
                groups.sort_by_key(|(group, _)| *group);
                (position, groups.into_iter().map(|(_, rows)| rows).collect())
            })
            .collect(),
    )
}

/// The x position of a level given by name or 1-based position.
fn level_position(
    level: &Value,
    option: &str,
    parameters: &Parameters,
    context: &StatContext,
) -> Result<f64> {
    let Some(levels) = context.x_levels else {
        return Err(Error::spec(
            parameters.path_of(option),
            format!(
                "{option} names x categories, but x is continuous; map x to a categorical \
                 column (e.g. a string column)"
            ),
        ));
    };
    match level {
        Value::String(name) => levels
            .iter()
            .position(|known| known == name)
            .map(|index| index as f64)
            .ok_or_else(|| {
                Error::spec(
                    parameters.path_of(option),
                    format!(
                        "'{name}' is not an x category{}; the categories are: {}",
                        crate::suggest::did_you_mean(name, levels.iter().copied()),
                        levels.join(", ")
                    ),
                )
            }),
        Value::Number(number) => match number.as_f64() {
            Some(index) if index.fract() == 0.0 && index >= 1.0 && index <= levels.len() as f64 => {
                Ok(index - 1.0)
            }
            _ => Err(Error::spec(
                parameters.path_of(option),
                format!(
                    "x category position {number} is out of range; use 1 to {} or a name",
                    levels.len()
                ),
            )),
        },
        other => Err(Error::spec(
            parameters.path_of(option),
            format!("expected an x category name or 1-based position, got {other}"),
        )),
    }
}

/// `comparisons` as pairs of x positions.
fn level_pairs(
    comparisons: &Value,
    parameters: &Parameters,
    context: &StatContext,
) -> Result<Vec<(f64, f64)>> {
    let invalid = || {
        Error::spec(
            parameters.path_of("comparisons"),
            format!(
                "expected a list of pairs of x categories, e.g. [['a', 'b'], ['a', 'c']], got \
                 {comparisons}"
            ),
        )
    };
    let Value::Array(pairs) = comparisons else {
        return Err(invalid());
    };
    pairs
        .iter()
        .map(|pair| match pair {
            Value::Array(items) if items.len() == 2 => Ok((
                level_position(&items[0], "comparisons", parameters, context)?,
                level_position(&items[1], "comparisons", parameters, context)?,
            )),
            _ => Err(invalid()),
        })
        .collect()
}

/// Bracket rows of `comparisons`.
fn brackets(
    pairs: &[(f64, f64)],
    values_at: &dyn Fn(f64) -> Vec<f64>,
    options: &Options,
    parameters: &Parameters,
    context: &StatContext,
    y_range: (f64, f64),
) -> Result<LayerData> {
    let method = options.method.unwrap_or(Method::WilcoxTest);
    // Each tested pair keeps its index (its height), as ggsignif.
    let mut tested: Vec<(usize, f64)> = Vec::new();
    for (index, (first, second)) in pairs.iter().enumerate() {
        let (a, b) = (values_at(*first), values_at(*second));
        let what = format!(
            "{} vs {}",
            level_name(context, *first),
            level_name(context, *second)
        );
        if let Some(p) = run(method, &[&a, &b], &what, options, parameters)? {
            tested.push((index, p));
        }
    }
    let p_values = options
        .adjustment
        .adjust(&tested.iter().map(|(_, p)| *p).collect::<Vec<_>>());
    let span = y_range.1 - y_range.0;
    let step = options.step_increase.unwrap_or(match options.label_y {
        Some(_) => 0.0,
        None => STEP_INCREASE,
    });
    let n = tested.len();
    let mut columns: [Vec<f64>; 4] = Default::default();
    for &(index, _) in &tested {
        let (first, second) = &pairs[index];
        let base = match &options.label_y {
            Some(values) => values[index.min(values.len() - 1)],
            None => y_range.1,
        };
        let row = [
            first.min(*second),
            first.max(*second),
            base + span * (MARGIN_TOP + step * index as f64),
            span * options.tip_length,
        ];
        for (column, value) in columns.iter_mut().zip(row) {
            column.push(value);
        }
    }
    let [xmin, xmax, y, tips] = columns;
    let texts: Vec<String> = p_values
        .iter()
        .map(|p| options.text(method, *p, true))
        .collect();
    let mut result = LayerData::new(n);
    result.insert(Aesthetic::Xmin, continuous(xmin))?;
    result.insert(Aesthetic::Xmax, continuous(xmax))?;
    result.insert(Aesthetic::Y, continuous(y))?;
    result.insert(Aesthetic::Lenstart, continuous(tips.clone()))?;
    result.insert(Aesthetic::Lenend, continuous(tips))?;
    result.insert(Aesthetic::Label, label_column(&texts))?;
    result.set_source_rows(SourceRows::Aggregated)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Options;

    /// ggplot_build data of ggpubr layers (tests/fixtures/pubr_reference.json).
    fn reference() -> serde_json::Value {
        let all: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/pubr_reference.json"
        ))
        .unwrap();
        all["layers"].clone()
    }

    const LEVELS: [&str; 3] = ["0.5", "1", "2"];

    /// ToothGrowth: x = dose position, y = len, groups by supp when `grouped`.
    fn tooth(x_column: &str, grouped: bool) -> (LayerData, StatContext<'static>) {
        let layers = reference();
        let tooth = &layers["tooth"];
        let strings = |name: &str| -> Vec<String> {
            tooth[name]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_owned())
                .collect()
        };
        let len: Vec<f64> = tooth["len"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let (x, levels): (Vec<f64>, Vec<String>) = match x_column {
            "dose" => (
                strings("dose")
                    .iter()
                    .map(|d| LEVELS.iter().position(|l| l == d).unwrap() as f64)
                    .collect(),
                LEVELS.iter().map(|l| (*l).to_owned()).collect(),
            ),
            _ => (
                strings("supp")
                    .iter()
                    .map(|s| if s == "VC" { 0.0 } else { 1.0 })
                    .collect(),
                vec!["VC".to_owned(), "OJ".to_owned()],
            ),
        };
        let mut data = LayerData::new(len.len());
        data.insert(Aesthetic::X, continuous(x.clone())).unwrap();
        data.insert(Aesthetic::Y, continuous(len)).unwrap();
        if grouped {
            let groups = strings("supp")
                .iter()
                .map(|s| u32::from(s == "OJ"))
                .collect();
            data.set_group_index(Some(groups)).unwrap();
        }
        let levels: &'static [&'static str] = Box::leak(
            levels
                .into_iter()
                .map(|level| &*Box::leak(level.into_boxed_str()))
                .collect::<Vec<&str>>()
                .into_boxed_slice(),
        );
        let context = StatContext {
            x_range: Some((0.0, levels.len() as f64 - 1.0)),
            y_range: Some((4.2, 33.9)),
            x_levels: Some(levels),
        };
        (data, context)
    }

    fn compute(data: LayerData, context: &StatContext, options: &str) -> Result<LayerData> {
        let options: Options = serde_json::from_str(options).unwrap();
        CompareMeansStat.compute(data, &Parameters::new(&options, "layers[0]"), context)
    }

    fn labels(result: &LayerData) -> Vec<String> {
        let Some(Column::Discrete(column)) = result.get(Aesthetic::Label) else {
            panic!("no labels");
        };
        (0..column.len())
            .map(|row| column.label(row).unwrap().to_owned())
            .collect()
    }

    fn values(result: &LayerData, aesthetic: Aesthetic) -> Vec<f64> {
        let column = result.continuous(aesthetic).unwrap();
        (0..column.len())
            .map(|row| column.get(row).unwrap())
            .collect()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn brackets_match_ggpubr() {
        let comparisons = r#"{"comparisons": [["0.5", "1"], ["1", "2"], ["0.5", "2"]]"#;
        for (case, options) in [
            ("comparisons", format!("{comparisons}}}")),
            ("signif", format!(r#"{comparisons}, "label": "p.signif"}}"#)),
            ("t_test", format!(r#"{comparisons}, "method": "t.test"}}"#)),
        ] {
            let (data, context) = tooth("dose", false);
            let result = compute(data, &context, &options).unwrap();
            // ggsignif rows: left tip, bar, right tip per comparison (in data order).
            let rows = reference()[case].as_array().unwrap().clone();
            let texts = labels(&result);
            let (xmin, xmax, y, tips) = (
                values(&result, Aesthetic::Xmin),
                values(&result, Aesthetic::Xmax),
                values(&result, Aesthetic::Y),
                values(&result, Aesthetic::Lenstart),
            );
            for index in 0..3 {
                let bar = &rows[3 * index + 1];
                let tip = &rows[3 * index];
                // R data: the ggsignif rows are ordered by group name, find ours by xmin/xmax.
                let found = (0..3)
                    .find(|&i| {
                        close(xmin[i] + 1.0, bar["x"].as_f64().unwrap())
                            && close(xmax[i] + 1.0, bar["xend"].as_f64().unwrap())
                    })
                    .unwrap_or_else(|| panic!("{case}: no bracket like {bar}"));
                assert!(close(y[found], bar["y"].as_f64().unwrap()), "{case} y");
                let tip_length = bar["y"].as_f64().unwrap() - tip["y"].as_f64().unwrap();
                assert!(close(tips[found], tip_length), "{case} tip");
                assert_eq!(texts[found], bar["annotation"].as_str().unwrap(), "{case}");
            }
        }
    }

    #[test]
    fn text_labels_match_ggpubr() {
        for (case, x_column, grouped, options) in [
            ("global", "dose", false, "{}"),
            ("anova", "dose", false, r#"{"method": "anova"}"#),
            (
                "reference",
                "dose",
                false,
                r#"{"ref_group": "0.5", "label": "p.signif"}"#,
            ),
            (
                "reference_all",
                "dose",
                false,
                r#"{"ref_group": ".all.", "label": "p.signif"}"#,
            ),
            ("grouped", "dose", true, r#"{"label": "p.signif"}"#),
            (
                "paired",
                "supp",
                false,
                r#"{"method": "t.test", "paired": true}"#,
            ),
        ] {
            let (data, context) = tooth(x_column, grouped);
            let result = compute(data, &context, options).unwrap();
            let rows = reference()[case].as_array().unwrap().clone();
            assert_eq!(result.row_count(), rows.len(), "{case}");
            let (x, y, hjust, vjust) = (
                values(&result, Aesthetic::X),
                values(&result, Aesthetic::Y),
                values(&result, Aesthetic::Hjust),
                values(&result, Aesthetic::Vjust),
            );
            for (row, expected) in rows.iter().enumerate() {
                assert!(
                    close(x[row] + 1.0, expected["x"].as_f64().unwrap()),
                    "{case} x"
                );
                assert!(close(y[row], expected["y"].as_f64().unwrap()), "{case} y");
                assert!(
                    close(hjust[row], expected["hjust"].as_f64().unwrap()),
                    "{case}"
                );
                assert!(
                    close(vjust[row], expected["vjust"].as_f64().unwrap()),
                    "{case}"
                );
                assert_eq!(
                    labels(&result)[row],
                    expected["label"].as_str().unwrap(),
                    "{case}"
                );
            }
        }
    }

    #[test]
    fn comparisons_without_values_are_left_out() {
        // A facet panel without one category: that bracket is left out, the
        // others keep their heights (ggsignif).
        let (mut data, context) = tooth("dose", false);
        let x = values(&data, Aesthetic::X);
        let y: Vec<f64> = values(&data, Aesthetic::Y)
            .iter()
            .zip(&x)
            .map(|(y, x)| if *x == 0.0 { f64::NAN } else { *y })
            .collect();
        data.insert(Aesthetic::Y, continuous(y)).unwrap();
        let options = r#"{"comparisons": [["0.5", "1"], ["1", "2"]]}"#;
        let result = compute(data, &context, options).unwrap();
        assert_eq!(result.row_count(), 1);
        assert_eq!(values(&result, Aesthetic::Xmin), vec![1.0]);
        // The second bracket's height: 33.9 + 29.7 x (0.05 + 0.12).
        assert!(close(values(&result, Aesthetic::Y)[0], 33.9 + 29.7 * 0.17));
    }

    #[test]
    fn options_are_checked() {
        let (data, context) = tooth("dose", false);
        let error = compute(data, &context, r#"{"comparisons": [["0.5", "3"]]}"#)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("'3' is not an x category") && error.contains("0.5, 1, 2"),
            "{error}"
        );
        let (data, context) = tooth("dose", false);
        let error = compute(data, &context, r#"{"method": "t.test"}"#)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("compares 2 groups, but the layer has 3"),
            "{error}"
        );
        let (data, context) = tooth("dose", false);
        let error = compute(data, &context, r#"{"method": "kruskal"}"#).map(|_| ());
        assert!(error.is_ok(), "ggpubr alias");
        let (data, context) = tooth("dose", false);
        let error = compute(data, &context, r#"{"method": "kruskall"}"#)
            .unwrap_err()
            .to_string();
        assert!(error.contains("did you mean 'kruskal'"), "{error}");
        let (data, context) = tooth("dose", false);
        let adjusted = compute(
            data,
            &context,
            r#"{"comparisons": [["0.5", "1"], ["1", "2"]], "p_adjust_method": "bonferroni", "label": "p.format"}"#,
        )
        .unwrap();
        // format.pval(p.adjust(c(7.020855e-06, 1.772382e-04), "bonferroni"), digits = 2).
        assert_eq!(labels(&adjusted), vec!["1.4e-05", "0.00035"]);
    }
}
