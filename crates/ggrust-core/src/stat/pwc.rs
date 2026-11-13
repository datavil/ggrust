//! `stat="pwc"`: pairwise comparisons of y between the levels along a discrete x,
//! as the R package ggpubr's `geom_pwc` (tests from rstatix) computes and places
//! them, one bracket row per comparison for `geom_bracket`.
//!
//! Comparisons run in families:
//! - one group per x: one family, the x categories;
//! - several groups per x (color, fill, shape or linetype groups, the "legend
//!   groups"), `group_by="x.var"` (default): one family per x, its legend
//!   groups, dodged by `dodge` as rstatix `add_x_position` (0.95, the dodge of
//!   ggrust's boxplots and violins; ggpubr's 0.8 fits its own `ggboxplot`);
//! - `group_by="legend.var"`: one family per legend group, its x categories at
//!   the group's dodged place; the brackets keep the group's color.
//!
//! In a family: every pair (rstatix order `(1, 2), (1, 3), (2, 3)`), or the pairs
//! with `ref_group` (an x category name or 1-based position; a 1-based legend
//! group rank with `group_by="x.var"`), or each level against all the family's
//! values (`"all"`, labels only: `xmin = xmax`).
//!
//! p-values follow rstatix: `signif(p, 3)`; a family of more than two levels is
//! adjusted with `p_adjust_method` (default `"holm"`) and both p and `p.adj`
//! rounded by `p_round` (3 decimals, 3 significant digits below 0.001); when no
//! family has more than two levels, ggpubr adjusts over the panel (unrounded),
//! as it does for every family with `p_adjust_by="panel"`.
//!
//! Labels: `label` names a value (`p`, `p.adj`, `p.format`, `p.adj.format`,
//! `p.signif`, `p.adj.signif`, `method`, `n1`, `n2`) or is a template with them
//! in braces (`"p = {p.adj.format}"`); `"= <"` becomes `"<"`, as ggpubr.
//! `hide_ns` drops comparisons with `p.adj > 0.05` (`true`, `"p.adj"`) or
//! `p > 0.05` (`"p"`) before stacking.
//!
//! Places (ggpubr `stat_pwc`): the i-th bracket of a stack (per x with
//! `group_by="x.var"`; all at one height for `"all"`) at the top of the y range
//! plus `bracket_nudge_y + step_increase (i - 1)` of the range (from `y_position`
//! when it is one number; a list gives each bracket's height), tips
//! `tip_length` of the range.
//!
//! What it does not do: other rstatix tests (`sign_test`, `dunn_test`,
//! `tukey_hsd`, ...), `method.args` (paired tests), `stack`, `remove.bracket`,
//! `symnum.args`, plotmath labels.

use super::compare_means::level_position;
use super::hypothesis::{Adjustment, Method};
use super::pubr_labels::{
    label_column, p_format_rstatix, p_signif, round_value, signif, significant,
};
use super::{Stat, StatContext};
use crate::data::{Column, ContinuousColumn, LayerData, SourceRows};
use crate::parameters::Parameters;
use crate::spec::{Aesthetic, AestheticKind, Value};
use crate::style::ValueKind;
use crate::{Error, Result};

/// ggpubr `geom_pwc` comparisons.
pub struct PwcStat;

/// The dodge width of lets-plot / ggrust boxplots and violins, so brackets end on
/// the boxes (ggpubr's default 0.8 matches its `ggboxplot`, not `geom_boxplot`).
const DODGE: f64 = 0.95;
/// ggpubr `step.increase`.
const STEP_INCREASE: f64 = 0.12;
/// ggpubr `tip.length`.
const TIP_LENGTH: f64 = 0.03;
/// ggpubr `bracket.nudge.y`.
const BRACKET_NUDGE_Y: f64 = 0.05;
/// Values a `label` may name.
const LABEL_VALUES: [&str; 9] = [
    "p",
    "p.adj",
    "p.format",
    "p.adj.format",
    "p.signif",
    "p.adj.signif",
    "method",
    "n1",
    "n2",
];
/// rstatix tests ggpubr accepts that ggrust does not compute.
const OTHER_METHODS: [&str; 7] = [
    "sign_test",
    "dunn_test",
    "emmeans_test",
    "emmeans",
    "tukey_hsd",
    "games_howell_test",
    "games_howell",
];

impl Stat for PwcStat {
    fn name(&self) -> &'static str {
        "pwc"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &[
            "method",
            "ref_group",
            "label",
            "y_position",
            "group_by",
            "dodge",
            "step_increase",
            "tip_length",
            "bracket_nudge_y",
            "p_adjust_method",
            "p_adjust_by",
            "hide_ns",
        ]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        // Fill only splits the legend groups (geom_bracket does not draw it).
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Fill]
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
        let rows: Vec<usize> = (0..data.row_count())
            .filter(|&row| x.get(row).is_some_and(f64::is_finite))
            .collect();
        let legend = legend_ranks(&data);
        let legend_count = legend.iter().copied().max().map_or(0, |rank| rank + 1);
        let positions = distinct(rows.iter().filter_map(|&row| x.get(row)));
        let grouped = positions.iter().any(|&position| {
            let ranks = distinct(
                rows.iter()
                    .filter(|&&row| x.get(row) == Some(position))
                    .map(|&row| legend[row] as f64),
            );
            ranks.len() > 1
        });
        let mode = match (grouped, options.group_by) {
            (false, _) => Mode::Ungrouped,
            (true, GroupBy::XVar) => Mode::WithinX,
            (true, GroupBy::LegendVar) => Mode::WithinLegend,
        };
        let dodged = |position: f64, rank: usize| {
            position - options.dodge / 2.0
                + options.dodge * (rank as f64 + 0.5) / legend_count as f64
        };
        let families = families(mode, &rows, x, &legend, &positions, dodged);
        let reference = options.reference(parameters, context, mode)?;
        let value_of = |row: usize| y.get(row).unwrap_or(f64::NAN);
        let mut comparisons: Vec<Comparison> = Vec::new();
        let mut pairwise_families = 0;
        for (family_index, family) in families.iter().enumerate() {
            let pairs: Vec<(Option<usize>, usize)> = match reference {
                Reference::None => (0..family.members.len())
                    .flat_map(|first| {
                        (first + 1..family.members.len()).map(move |second| (Some(first), second))
                    })
                    .collect(),
                Reference::Key(key) => {
                    let Some(at) = family.members.iter().position(|m| m.key == key) else {
                        continue;
                    };
                    (0..family.members.len())
                        .filter(|&other| other != at)
                        .map(|other| (Some(at), other))
                        .collect()
                }
                Reference::All => (0..family.members.len()).map(|m| (None, m)).collect(),
            };
            let pairwise = family.members.len() > 2 || reference == Reference::All;
            let start = comparisons.len();
            let all: Vec<f64> = family
                .members
                .iter()
                .flat_map(|member| member.rows.iter().map(|&row| value_of(row)))
                .collect();
            for (first, second) in pairs {
                let first_values: Vec<f64> = match first {
                    Some(at) => family.members[at]
                        .rows
                        .iter()
                        .map(|&r| value_of(r))
                        .collect(),
                    None => all.clone(),
                };
                let second_member = &family.members[second];
                let second_values: Vec<f64> =
                    second_member.rows.iter().map(|&r| value_of(r)).collect();
                let count = |values: &[f64]| values.iter().filter(|v| !v.is_nan()).count();
                let (n1, n2) = (count(&first_values), count(&second_values));
                if n1 == 0 || n2 == 0 {
                    continue;
                }
                let p = options
                    .method
                    .test()
                    .p_value(&[&first_values, &second_values], false)
                    .map_err(|reason| {
                        let name = |member: Option<usize>| match member {
                            Some(at) => family.members[at].name.clone(),
                            None => "all values".to_owned(),
                        };
                        Error::spec(
                            parameters.path_of("method"),
                            format!(
                                "cannot test {} vs {}: {reason}",
                                name(first),
                                name(Some(second))
                            ),
                        )
                    })?;
                let xmin = first.map_or(second_member.position, |at| family.members[at].position);
                comparisons.push(Comparison {
                    family: family_index,
                    xmin: xmin.min(second_member.position),
                    xmax: xmin.max(second_member.position),
                    p: signif(p, 3),
                    p_adjusted: f64::NAN,
                    n1,
                    n2,
                    representative: family.representative,
                });
            }
            let family_comparisons = &mut comparisons[start..];
            if pairwise {
                pairwise_families += 1;
                let p: Vec<f64> = family_comparisons.iter().map(|c| c.p).collect();
                for (comparison, adjusted) in family_comparisons
                    .iter_mut()
                    .zip(options.adjustment.adjust(&p))
                {
                    comparison.p = round_value(comparison.p, 3);
                    comparison.p_adjusted = round_value(adjusted, 3);
                }
            } else {
                for comparison in family_comparisons.iter_mut() {
                    comparison.p_adjusted = comparison.p;
                }
            }
        }
        if pairwise_families == 0 || options.adjust_by_panel {
            let p: Vec<f64> = comparisons.iter().map(|c| c.p).collect();
            for (comparison, adjusted) in comparisons.iter_mut().zip(options.adjustment.adjust(&p))
            {
                comparison.p_adjusted = adjusted;
            }
        }
        comparisons.retain(|comparison| match options.hide_ns {
            HideNs::No => true,
            HideNs::ByP => comparison.p <= 0.05,
            HideNs::ByPAdjusted => comparison.p_adjusted <= 0.05,
        });
        let range = context
            .y_range
            .or_else(|| super::pubr_labels::column_range(Some(y), &rows))
            .unwrap_or((0.0, 1.0));
        let span = range.1 - range.0;
        let mut steps: Vec<usize> = Vec::with_capacity(comparisons.len());
        for (index, comparison) in comparisons.iter().enumerate() {
            steps.push(match (reference, mode) {
                (Reference::All, _) => 0,
                (_, Mode::WithinX) => match index {
                    0 => 0,
                    _ if comparisons[index - 1].family == comparison.family => steps[index - 1] + 1,
                    _ => 0,
                },
                _ => index,
            });
        }
        let heights: Vec<f64> = steps
            .iter()
            .enumerate()
            .map(|(index, &step)| {
                let lift = span * (options.bracket_nudge_y + options.step_increase * step as f64);
                match &options.y_position {
                    None => range.1 + lift,
                    Some(values) if values.len() == 1 => values[0] + lift,
                    Some(values) => values[index.min(values.len() - 1)],
                }
            })
            .collect();
        let texts: Vec<String> = comparisons
            .iter()
            .map(|comparison| options.label.text(comparison, options.method))
            .collect();
        let mut result = match mode {
            Mode::WithinLegend => {
                let representatives: Vec<u32> = comparisons
                    .iter()
                    .map(|c| c.representative as u32)
                    .collect();
                let mut taken = data.take(&representatives);
                let positional: Vec<Aesthetic> = taken
                    .aesthetics()
                    .filter(|a| a.is_x_position() || a.is_y_position() || *a == Aesthetic::Label)
                    .collect();
                for aesthetic in positional {
                    taken.remove(aesthetic);
                }
                taken
            }
            _ => LayerData::new(comparisons.len()),
        };
        let tips = vec![span * options.tip_length; comparisons.len()];
        let columns = [
            (
                Aesthetic::Xmin,
                comparisons.iter().map(|c| c.xmin).collect(),
            ),
            (
                Aesthetic::Xmax,
                comparisons.iter().map(|c| c.xmax).collect(),
            ),
            (Aesthetic::Y, heights),
            (Aesthetic::Lenstart, tips.clone()),
            (Aesthetic::Lenend, tips),
        ];
        for (aesthetic, values) in columns {
            result.insert(
                aesthetic,
                Column::Continuous(ContinuousColumn::from_values(values)),
            )?;
        }
        result.insert(Aesthetic::Label, label_column(&texts))?;
        result.set_source_rows(SourceRows::Aggregated)?;
        Ok(result)
    }
}

/// How comparisons split into families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// One group per x: the x categories.
    Ungrouped,
    /// `group_by="x.var"`: the legend groups at each x.
    WithinX,
    /// `group_by="legend.var"`: the x categories of each legend group.
    WithinLegend,
}

/// `group_by`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupBy {
    XVar,
    LegendVar,
}

/// `hide_ns`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HideNs {
    No,
    ByP,
    ByPAdjusted,
}

/// The reference of the comparisons, resolved to a member key (an x position,
/// or a legend rank as a number with `group_by="x.var"`).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Reference {
    None,
    Key(f64),
    All,
}

/// rstatix's tests that ggrust computes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PwcMethod {
    Wilcox,
    T,
}

impl PwcMethod {
    const NAMES: [&'static str; 2] = ["wilcox_test", "t_test"];

    /// ggpubr's method names: dots become underscores, `"wilcoxon"` is
    /// `"wilcox_test"`.
    fn parse(name: &str, parameters: &Parameters) -> Result<PwcMethod> {
        let normalised = name.replace('.', "_");
        match normalised.as_str() {
            "wilcox_test" | "wilcoxon" => Ok(PwcMethod::Wilcox),
            "t_test" => Ok(PwcMethod::T),
            other if OTHER_METHODS.contains(&other) => Err(Error::spec(
                parameters.path_of("method"),
                format!(
                    "method '{name}' is not available in ggrust yet; use one of: {}",
                    Self::NAMES.join(", ")
                ),
            )),
            _ => Err(Error::spec(
                parameters.path_of("method"),
                format!(
                    "unknown method '{name}'{}; use one of: {}",
                    crate::suggest::did_you_mean(&normalised, Self::NAMES),
                    Self::NAMES.join(", ")
                ),
            )),
        }
    }

    fn test(self) -> Method {
        match self {
            PwcMethod::Wilcox => Method::WilcoxTest,
            PwcMethod::T => Method::TTest,
        }
    }

    /// rstatix `get_description`.
    fn description(self) -> &'static str {
        match self {
            PwcMethod::Wilcox => "Wilcoxon test",
            PwcMethod::T => "T test",
        }
    }
}

/// A `label`: one value, or a template of literal text and values.
#[derive(Debug, Clone, PartialEq)]
struct Label {
    /// Literal text and value names, in order (`Ok`: text, `Err`: a value name).
    parts: Vec<std::result::Result<String, &'static str>>,
}

impl Label {
    fn parse(text: &str, parameters: &Parameters) -> Result<Label> {
        let value = |name: &str| -> Result<&'static str> {
            LABEL_VALUES
                .iter()
                .copied()
                .find(|known| *known == name)
                .ok_or_else(|| {
                    Error::spec(
                        parameters.path_of("label"),
                        format!(
                            "unknown label value '{name}'{}; use one of: {}, or a template \
                             such as \"p = {{p.adj.format}}\"",
                            crate::suggest::did_you_mean(name, LABEL_VALUES),
                            LABEL_VALUES.join(", ")
                        ),
                    )
                })
        };
        if !text.contains('{') {
            return Ok(Label {
                parts: vec![Err(value(text.trim())?)],
            });
        }
        let mut parts = Vec::new();
        let mut rest = text;
        while let Some(open) = rest.find('{') {
            if open > 0 {
                parts.push(Ok(rest[..open].to_owned()));
            }
            let Some(close) = rest[open..].find('}') else {
                return Err(Error::spec(
                    parameters.path_of("label"),
                    format!("unclosed '{{' in the label template \"{text}\""),
                ));
            };
            parts.push(Err(value(rest[open + 1..open + close].trim())?));
            rest = &rest[open + close + 1..];
        }
        if !rest.is_empty() {
            parts.push(Ok(rest.to_owned()));
        }
        Ok(Label { parts })
    }

    fn text(&self, comparison: &Comparison, method: PwcMethod) -> String {
        let mut text = String::new();
        for part in &self.parts {
            match part {
                Ok(literal) => text.push_str(literal),
                Err(name) => text.push_str(&match *name {
                    "p" => significant(comparison.p, 15),
                    "p.adj" => significant(comparison.p_adjusted, 15),
                    "p.format" => p_format_rstatix(comparison.p),
                    "p.adj.format" => p_format_rstatix(comparison.p_adjusted),
                    "p.signif" => p_signif(comparison.p).to_owned(),
                    "p.adj.signif" => p_signif(comparison.p_adjusted).to_owned(),
                    "method" => method.description().to_owned(),
                    "n1" => comparison.n1.to_string(),
                    _ => comparison.n2.to_string(),
                }),
            }
        }
        equals_before_less(&text)
    }
}

/// ggpubr's `gsub("=+(\\s+)?<", "<\\1", label)`: `"p = <0.0001"` reads
/// `"p < 0.0001"`.
fn equals_before_less(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('=') {
        let after_equals = rest[at..].trim_start_matches('=');
        let spaces = after_equals.len() - after_equals.trim_start().len();
        if after_equals[spaces..].starts_with('<') {
            result.push_str(&rest[..at]);
            result.push('<');
            result.push_str(&after_equals[..spaces]);
            rest = &after_equals[spaces + 1..];
        } else {
            let next = rest.len() - after_equals.len();
            result.push_str(&rest[..next]);
            rest = after_equals;
        }
    }
    result.push_str(rest);
    result
}

/// Parsed options.
struct Options {
    method: PwcMethod,
    label: Label,
    y_position: Option<Vec<f64>>,
    group_by: GroupBy,
    dodge: f64,
    step_increase: f64,
    tip_length: f64,
    bracket_nudge_y: f64,
    adjustment: Adjustment,
    adjust_by_panel: bool,
    hide_ns: HideNs,
}

impl Options {
    fn parse(parameters: &Parameters) -> Result<Options> {
        let choice = |name: &str, accepted: &[&str]| -> Result<Option<usize>> {
            match parameters.string(name)? {
                None => Ok(None),
                Some(value) => accepted
                    .iter()
                    .position(|known| *known == value)
                    .map(Some)
                    .ok_or_else(|| {
                        Error::spec(
                            parameters.path_of(name),
                            format!(
                                "unknown {name} '{value}'{}; use one of: {}",
                                crate::suggest::did_you_mean(value, accepted.iter().copied()),
                                accepted.join(", ")
                            ),
                        )
                    }),
            }
        };
        let adjustment = match parameters.string("p_adjust_method")? {
            None => Adjustment::Holm,
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
        let hide_ns = match parameters.value("hide_ns") {
            None | Some(Value::Bool(false)) => HideNs::No,
            Some(Value::Bool(true)) => HideNs::ByPAdjusted,
            Some(Value::String(column)) if column == "p.adj" => HideNs::ByPAdjusted,
            Some(Value::String(column)) if column == "p" => HideNs::ByP,
            Some(other) => {
                return Err(Error::spec(
                    parameters.path_of("hide_ns"),
                    format!("expected true, false, \"p\" or \"p.adj\", got {other}"),
                ));
            }
        };
        let y_position = match parameters.value("y_position") {
            None => None,
            Some(value) => {
                let invalid = || {
                    Error::spec(
                        parameters.path_of("y_position"),
                        format!("expected a number or a non-empty list of numbers, got {value}"),
                    )
                };
                let finite = |item: &Value| item.as_f64().filter(|v| v.is_finite());
                Some(match value {
                    Value::Array(items) if !items.is_empty() => items
                        .iter()
                        .map(|item| finite(item).ok_or_else(invalid))
                        .collect::<Result<Vec<f64>>>()?,
                    other => vec![finite(other).ok_or_else(invalid)?],
                })
            }
        };
        let non_negative = |name: &str, default: f64| -> Result<f64> {
            match parameters.number(name)? {
                Some(value) if value < 0.0 => Err(Error::spec(
                    parameters.path_of(name),
                    format!("expected a number >= 0, got {value}"),
                )),
                Some(value) => Ok(value),
                None => Ok(default),
            }
        };
        let dodge = non_negative("dodge", DODGE)?;
        Ok(Options {
            method: match parameters.string("method")? {
                None => PwcMethod::Wilcox,
                Some(name) => PwcMethod::parse(name, parameters)?,
            },
            label: Label::parse(
                parameters.string("label")?.unwrap_or("p.format"),
                parameters,
            )?,
            y_position,
            group_by: match choice("group_by", &["x.var", "legend.var"])? {
                Some(1) => GroupBy::LegendVar,
                _ => GroupBy::XVar,
            },
            dodge,
            step_increase: non_negative("step_increase", STEP_INCREASE)?,
            tip_length: non_negative("tip_length", TIP_LENGTH)?,
            bracket_nudge_y: parameters
                .number("bracket_nudge_y")?
                .unwrap_or(BRACKET_NUDGE_Y),
            adjustment,
            adjust_by_panel: choice("p_adjust_by", &["group", "panel"])? == Some(1),
            hide_ns,
        })
    }

    /// `ref_group` as a member key for `mode`.
    fn reference(
        &self,
        parameters: &Parameters,
        context: &StatContext,
        mode: Mode,
    ) -> Result<Reference> {
        let Some(value) = parameters.value("ref_group") else {
            return Ok(Reference::None);
        };
        if matches!(value.as_str(), Some("all" | ".all.")) {
            return Ok(Reference::All);
        }
        if mode != Mode::WithinX {
            return level_position(value, "ref_group", parameters, context).map(Reference::Key);
        }
        match value.as_f64() {
            Some(rank) if rank.fract() == 0.0 && rank >= 1.0 => Ok(Reference::Key(rank - 1.0)),
            _ => Err(Error::spec(
                parameters.path_of("ref_group"),
                format!(
                    "with group_by='x.var' the comparisons are between the legend groups at \
                     each x, so ref_group is the 1-based rank of a legend group (e.g. 1 for \
                     the first), got {value}"
                ),
            )),
        }
    }
}

/// One tested comparison.
#[derive(Debug, Clone, PartialEq)]
struct Comparison {
    family: usize,
    xmin: f64,
    xmax: f64,
    p: f64,
    p_adjusted: f64,
    n1: usize,
    n2: usize,
    /// An input row of the family (its legend group's columns, for colors).
    representative: usize,
}

/// A level in a family: a key (x position or legend rank), its bracket end and rows.
struct Member {
    key: f64,
    name: String,
    position: f64,
    rows: Vec<usize>,
}

/// Levels compared with each other.
struct Family {
    members: Vec<Member>,
    representative: usize,
}

/// Families of comparisons (see [`Mode`]), members in key order.
fn families(
    mode: Mode,
    rows: &[usize],
    x: &ContinuousColumn,
    legend: &[usize],
    positions: &[f64],
    dodged: impl Fn(f64, usize) -> f64,
) -> Vec<Family> {
    let x_of = |row: usize| x.get(row).unwrap_or(f64::NAN);
    let member = |key: f64, name: String, position: f64, keep: &dyn Fn(usize) -> bool| Member {
        key,
        name,
        position,
        rows: rows.iter().copied().filter(|&row| keep(row)).collect(),
    };
    let position_name = |position: f64| format!("x = {position}");
    match mode {
        Mode::Ungrouped => vec![Family {
            members: positions
                .iter()
                .map(|&p| member(p, position_name(p), p, &|row| x_of(row) == p))
                .collect(),
            representative: rows.first().copied().unwrap_or(0),
        }],
        Mode::WithinX => positions
            .iter()
            .map(|&p| {
                let ranks = distinct(
                    rows.iter()
                        .filter(|&&row| x_of(row) == p)
                        .map(|&row| legend[row] as f64),
                );
                Family {
                    members: ranks
                        .iter()
                        .map(|&rank| {
                            member(
                                rank,
                                format!("legend group {} at x = {p}", rank + 1.0),
                                dodged(p, rank as usize),
                                &|row| x_of(row) == p && legend[row] as f64 == rank,
                            )
                        })
                        .collect(),
                    representative: rows
                        .iter()
                        .copied()
                        .find(|&row| x_of(row) == p)
                        .unwrap_or(0),
                }
            })
            .collect(),
        Mode::WithinLegend => {
            let ranks = distinct(rows.iter().map(|&row| legend[row] as f64));
            ranks
                .iter()
                .map(|&rank| {
                    let in_group = |row: usize| legend[row] as f64 == rank;
                    let group_positions =
                        distinct(rows.iter().filter(|&&row| in_group(row)).map(|&r| x_of(r)));
                    Family {
                        members: group_positions
                            .iter()
                            .map(|&p| {
                                member(
                                    p,
                                    format!("x = {p} of legend group {}", rank + 1.0),
                                    dodged(p, rank as usize),
                                    &|row| in_group(row) && x_of(row) == p,
                                )
                            })
                            .collect(),
                        representative: rows
                            .iter()
                            .copied()
                            .find(|&row| in_group(row))
                            .unwrap_or(0),
                    }
                })
                .collect()
        }
    }
}

/// The legend group rank of every row: rows sharing their discrete color, fill,
/// shape and linetype values (or their group, without such columns) share a
/// group, ranked by their smallest group index (the dodge order).
fn legend_ranks(data: &LayerData) -> Vec<usize> {
    let discrete: Vec<&crate::data::DiscreteColumn> = data
        .columns()
        .filter_map(|(variable, column)| match (variable, column) {
            (crate::data::Variable::Aesthetic(aesthetic), Column::Discrete(column))
                if matches!(
                    aesthetic.kind(),
                    AestheticKind::Style(ValueKind::Color | ValueKind::Shape | ValueKind::Linetype)
                ) =>
            {
                Some(column)
            }
            _ => None,
        })
        .collect();
    let key_of = |row: usize| -> Vec<Option<u32>> {
        match discrete.is_empty() {
            true => vec![Some(data.group_of(row))],
            false => discrete.iter().map(|column| column.code(row)).collect(),
        }
    };
    let mut keys: Vec<(Vec<Option<u32>>, u32)> = Vec::new();
    let mut key_index: Vec<usize> = Vec::with_capacity(data.row_count());
    for row in 0..data.row_count() {
        let key = key_of(row);
        let group = data.group_of(row);
        let index = match keys.iter().position(|(known, _)| *known == key) {
            Some(index) => {
                keys[index].1 = keys[index].1.min(group);
                index
            }
            None => {
                keys.push((key, group));
                keys.len() - 1
            }
        };
        key_index.push(index);
    }
    let mut order: Vec<usize> = (0..keys.len()).collect();
    order.sort_by_key(|&index| (keys[index].1, index));
    let mut rank_of_key = vec![0; keys.len()];
    for (rank, index) in order.into_iter().enumerate() {
        rank_of_key[index] = rank;
    }
    key_index
        .into_iter()
        .map(|index| rank_of_key[index])
        .collect()
}

/// Distinct finite values, sorted.
fn distinct(values: impl Iterator<Item = f64>) -> Vec<f64> {
    let mut values: Vec<f64> = values.filter(|value| value.is_finite()).collect();
    values.sort_by(f64::total_cmp);
    values.dedup();
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::DiscreteColumn;
    use crate::spec::Options as SpecOptions;
    use serde_json::Value as Json;

    fn reference() -> Json {
        serde_json::from_str(include_str!(
            "../../../../tests/fixtures/pubr_helpers_reference.json"
        ))
        .unwrap()
    }

    fn column(data: &Json, name: &str) -> Vec<Json> {
        data.as_array()
            .map(|rows| rows.iter().map(|row| row[name].clone()).collect())
            .unwrap_or_else(|| data[name].as_array().unwrap().clone())
    }

    fn text(value: &Json) -> String {
        value
            .as_str()
            .map_or_else(|| value.to_string(), str::to_owned)
    }

    fn leak(levels: Vec<String>) -> &'static [&'static str] {
        Box::leak(
            levels
                .into_iter()
                .map(|level| &*Box::leak(level.into_boxed_str()))
                .collect::<Vec<&str>>()
                .into_boxed_slice(),
        )
    }

    /// A layer of `x` (positions of its sorted levels), `y` and an optional discrete
    /// `color` (groups = x and color, as ggrust groups the boxes).
    fn make_layer(
        data: &Json,
        x: &str,
        y: &str,
        color: Option<&str>,
    ) -> (LayerData, StatContext<'static>) {
        let xs: Vec<String> = column(data, x).iter().map(text).collect();
        let mut levels = xs.clone();
        levels.sort_by(|a, b| {
            a.parse::<f64>()
                .ok()
                .partial_cmp(&b.parse::<f64>().ok())
                .unwrap()
                .then(a.cmp(b))
        });
        levels.dedup();
        let ys: Vec<f64> = column(data, y)
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let mut layer = LayerData::new(ys.len());
        let positions: Vec<f64> = xs
            .iter()
            .map(|value| levels.iter().position(|l| l == value).unwrap() as f64)
            .collect();
        layer
            .insert(
                Aesthetic::X,
                Column::Continuous(ContinuousColumn::from_values(positions.clone())),
            )
            .unwrap();
        layer
            .insert(
                Aesthetic::Y,
                Column::Continuous(ContinuousColumn::from_values(ys.clone())),
            )
            .unwrap();
        if let Some(color) = color {
            let colors: Vec<String> = column(data, color).iter().map(text).collect();
            let mut color_levels = colors.clone();
            color_levels.sort();
            color_levels.dedup();
            let codes: Vec<Option<u32>> = colors
                .iter()
                .map(|c| Some(color_levels.iter().position(|l| l == c).unwrap() as u32))
                .collect();
            let groups: Vec<u32> = positions
                .iter()
                .zip(&codes)
                .map(|(p, c)| *p as u32 * color_levels.len() as u32 + c.unwrap())
                .collect();
            layer
                .insert(
                    Aesthetic::Color,
                    Column::Discrete(DiscreteColumn::from_parts(color_levels, codes.into())),
                )
                .unwrap();
            layer.set_group_index(Some(groups)).unwrap();
        }
        let range = ys
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(*v), hi.max(*v))
            });
        let levels = leak(levels);
        let context = StatContext {
            x_range: Some((0.0, levels.len() as f64 - 1.0)),
            y_range: Some(range),
            x_levels: Some(levels),
        };
        (layer, context)
    }

    fn compute(data: LayerData, context: &StatContext, options: &str) -> Result<LayerData> {
        let options: SpecOptions = serde_json::from_str(options).unwrap();
        PwcStat.compute(data, &Parameters::new(&options, "layers[1]"), context)
    }

    fn values(result: &LayerData, aesthetic: Aesthetic) -> Vec<f64> {
        result.continuous(aesthetic).unwrap().values().to_vec()
    }

    fn labels(result: &LayerData) -> Vec<String> {
        let Some(Column::Discrete(column)) = result.get(Aesthetic::Label) else {
            panic!("no labels");
        };
        (0..column.len())
            .map(|row| column.label(row).unwrap().to_owned())
            .collect()
    }

    /// ggpubr option names and values as ggrust's.
    fn options(r_options: &Json) -> String {
        let mut options = serde_json::Map::new();
        // jsonlite writes empty R lists as [].
        for (name, value) in r_options.as_object().into_iter().flatten() {
            options.insert(name.replace('.', "_"), value.clone());
        }
        // ggpubr's dodge (ggrust defaults to its boxplots' 0.95).
        options.entry("dodge").or_insert(Json::from(0.8));
        Json::Object(options).to_string()
    }

    #[test]
    fn brackets_match_ggpubr() {
        let all = reference();
        for case in all["pwc"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let (data, x, y, color) = match name {
                "four" => (&all["data"]["four"], "g", "v", None),
                "three_x_var" => (&all["data"]["three"], "x", "v", Some("color")),
                _ if name.starts_with("x_var") || name.starts_with("legend_var") => {
                    (&all["data"]["tooth"], "dose", "len", Some("supp"))
                }
                _ => (&all["data"]["tooth"], "dose", "len", None),
            };
            let (layer, context) = make_layer(data, x, y, color);
            let result = compute(layer, &context, &options(&case["options"])).unwrap();
            let rows = case["rows"].as_array().unwrap();
            assert_eq!(result.row_count(), rows.len(), "{name}");
            let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
            let (xmin, xmax, heights) = (
                values(&result, Aesthetic::Xmin),
                values(&result, Aesthetic::Xmax),
                values(&result, Aesthetic::Y),
            );
            let tips = values(&result, Aesthetic::Lenstart);
            let texts = labels(&result);
            for (index, row) in rows.iter().enumerate() {
                // R positions are 1-based.
                assert!(
                    close(xmin[index] + 1.0, row["xmin"].as_f64().unwrap()),
                    "{name} {index} xmin {}",
                    xmin[index]
                );
                assert!(
                    close(xmax[index] + 1.0, row["xmax"].as_f64().unwrap()),
                    "{name} {index} xmax"
                );
                assert!(
                    close(heights[index], row["y"].as_f64().unwrap()),
                    "{name} {index} y {} vs {}",
                    heights[index],
                    row["y"]
                );
                assert!(
                    close(tips[index], row["tip_left"].as_f64().unwrap()),
                    "{name} {index} tip"
                );
                assert_eq!(texts[index], text(&row["label"]), "{name} {index}");
            }
            if name.starts_with("legend_var") {
                let Some(Column::Discrete(colors)) = result.get(Aesthetic::Color) else {
                    panic!("{name}: legend groups keep their color");
                };
                // Rows of the first legend group (OJ, ggplot2's #F8766D) come first.
                let first: Vec<bool> = rows.iter().map(|row| row["colour"] == "#F8766D").collect();
                for (index, is_first) in first.iter().enumerate() {
                    assert_eq!(
                        colors.label(index) == Some("OJ"),
                        *is_first,
                        "{name} {index}"
                    );
                }
            }
        }
    }

    #[test]
    fn options_are_checked() {
        let all = reference();
        let (layer, context) = make_layer(&all["data"]["tooth"], "dose", "len", None);
        let message = |options: &str| {
            compute(layer.clone(), &context, options)
                .unwrap_err()
                .to_string()
        };
        assert!(message(r#"{"method": "wilcox_tset"}"#).contains("did you mean 'wilcox_test'"));
        assert!(message(r#"{"method": "dunn_test"}"#).contains("not available in ggrust yet"));
        assert!(message(r#"{"label": "p.adj.fromat"}"#).contains("did you mean 'p.adj.format'"));
        assert!(message(r#"{"label": "p = {p"}"#).contains("unclosed"));
        assert!(message(r#"{"group_by": "x"}"#).contains("x.var, legend.var"));
        assert!(message(r#"{"hide_ns": "q"}"#).contains("\"p.adj\""));
        assert!(message(r#"{"ref_group": "3"}"#).contains("not an x category"));
        assert!(message(r#"{"tip_length": -1}"#).contains(">= 0"));
        let (grouped, context) = make_layer(&all["data"]["tooth"], "dose", "len", Some("supp"));
        let error = compute(grouped, &context, r#"{"ref_group": "OJ"}"#)
            .unwrap_err()
            .to_string();
        assert!(error.contains("1-based rank of a legend group"), "{error}");
    }

    #[test]
    fn legend_reference_compares_against_that_group() {
        let all = reference();
        let (layer, context) = make_layer(&all["data"]["three"], "x", "v", Some("color"));
        let result = compute(layer, &context, r#"{"ref_group": 2}"#).unwrap();
        // Per x: the second group against the first and the third.
        let (xmin, xmax) = (
            values(&result, Aesthetic::Xmin),
            values(&result, Aesthetic::Xmax),
        );
        let third = 0.95 / 3.0;
        let expected = [
            (-third, 0.0),
            (0.0, third),
            (1.0 - third, 1.0),
            (1.0, 1.0 + third),
        ];
        assert_eq!(xmin.len(), 4);
        for (index, (low, high)) in expected.iter().enumerate() {
            assert!((xmin[index] - low).abs() < 1e-9 && (xmax[index] - high).abs() < 1e-9);
        }
    }

    #[test]
    fn equals_before_less_follows_ggpubr() {
        assert_eq!(equals_before_less("p = <0.0001"), "p < 0.0001");
        assert_eq!(equals_before_less("p=<0.0001, a = b"), "p<0.0001, a = b");
        assert_eq!(equals_before_less("p = 0.5"), "p = 0.5");
    }
}
