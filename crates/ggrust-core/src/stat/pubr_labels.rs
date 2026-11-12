//! Number formatting of the ggpubr / ggpmisc statistic labels, as R prints them.
//!
//! - [`significant`]: R's `format(x, digits = d)` of one number: rounded to `d`
//!   significant digits, trailing zeros dropped, fixed notation unless the
//!   scientific one is narrower (R's default `scipen = 0`), exponents as `e-05`.
//! - [`p_format`]: R's `format.pval(p, digits = 2)` of one p-value (ggpubr
//!   `p.format`): `"<2e-16"` below machine epsilon.
//! - [`p_equals`]: `"p = 0.012"` / `"p < 2e-16"` (ggpubr `create_p_label`).
//! - [`p_signif`]: significance stars, `(0, 1e-4]` `****` ... `(0.05, 1]` `ns`.
//! - [`p_format_rstatix`]: rstatix `p_format(p)` (geom_pwc's `p.format`):
//!   `"<0.0001"` below 1e-4, else 2 significant digits.
//! - [`signif`], [`round_value`]: R's `signif(x, d)` and rstatix's `p_round`.
//!
//! - [`label_column`]: label texts as a discrete `label` column.
//! - [`labels_per_group`]: one label per group at ggpubr's `.label_params`
//!   place (`stat_cor`, `stat_poly_eq`), stacked 1.4 em apart.
//!
//! What it does not do: ggpubr's other `p.format.style`s, decimal marks.

use super::StatContext;
use super::groups::{per_group, rows_by_group};
use crate::data::{Column, ContinuousColumn, DiscreteColumn, LayerData, Variable};
use crate::parameters::Parameters;
use crate::spec::{Aesthetic, Value};
use crate::{Error, Result};

/// `format(x, digits = digits)` in R, for one finite number (`"NA"` for NaN).
pub fn significant(x: f64, digits: usize) -> String {
    if x.is_nan() {
        return "NA".to_owned();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Inf" } else { "-Inf" }.to_owned();
    }
    if x == 0.0 {
        return "0".to_owned();
    }
    let digits = digits.max(1) as i32;
    // R's `scientific()`: the exponent and significant digits from the number
    // scaled to `digits` digits and rounded half to even (so 0.995 gives 1).
    let mut exponent = x.abs().log10().floor() as i32;
    let mut scaled = (x.abs() / 10_f64.powi(exponent) * 10_f64.powi(digits - 1)).round_ties_even();
    if scaled >= 10_f64.powi(digits) {
        exponent += 1;
        scaled = (scaled / 10.0).round_ties_even();
    }
    let mut significant_digits = digits;
    while significant_digits > 1 && scaled % 10.0 == 0.0 {
        scaled /= 10.0;
        significant_digits -= 1;
    }
    let negative = usize::from(x < 0.0);
    let decimals = (significant_digits - 1 - exponent).max(0) as usize;
    let fixed_width = negative
        + if exponent >= 0 {
            exponent as usize + 1
        } else {
            1
        }
        + if decimals > 0 { decimals + 1 } else { 0 };
    let scientific_width = negative
        + if significant_digits > 1 {
            significant_digits as usize + 1
        } else {
            1
        }
        + if exponent.abs() >= 100 { 5 } else { 4 };
    if fixed_width <= scientific_width {
        format!("{:.*}", decimals, x)
    } else {
        let printed = format!("{:.*e}", significant_digits as usize - 1, x);
        let (mantissa, printed_exponent) = printed.split_once('e').unwrap_or((&printed, "0"));
        let printed_exponent: i32 = printed_exponent.parse().unwrap_or(exponent);
        let sign = if printed_exponent < 0 { '-' } else { '+' };
        format!("{mantissa}e{sign}{:02}", printed_exponent.abs())
    }
}

/// R's `format.pval(p, digits = 2)` for one p-value.
pub(super) fn p_format(p: f64) -> String {
    if p.is_nan() {
        return "NA".to_owned();
    }
    if p < f64::EPSILON {
        return format!("<{}", significant(f64::EPSILON, 1));
    }
    significant(p, 2)
}

/// `p = <p_format>`, or `p < 2e-16` (ggpubr `create_p_label`).
pub(super) fn p_equals(p: f64) -> String {
    let formatted = p_format(p);
    match formatted.strip_prefix('<') {
        Some(bound) => format!("p < {bound}"),
        None => format!("p = {formatted}"),
    }
}

/// rstatix's `p_format(p, accuracy = 1e-4, digits = 2)` for one p-value
/// (`format.pval` with `eps = 1e-4`, `"<1e-04"` written `"<0.0001"`).
pub(super) fn p_format_rstatix(p: f64) -> String {
    if p.is_nan() {
        return "NA".to_owned();
    }
    if p < 1e-4 {
        return "<0.0001".to_owned();
    }
    significant(p, 2)
}

/// R's `signif(x, digits)`.
pub(super) fn signif(x: f64, digits: i32) -> f64 {
    if !x.is_finite() || x == 0.0 {
        return x;
    }
    let exponent = digits - 1 - x.abs().log10().floor() as i32;
    if exponent > 300 {
        return x;
    }
    let scale = 10_f64.powi(exponent);
    (x * scale).round_ties_even() / scale
}

/// rstatix `p_round(x, digits)`: `round(x, digits)` above `10^-digits`, else
/// `signif(x, digits)`.
pub(super) fn round_value(x: f64, digits: i32) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x.abs() > 10_f64.powi(-digits) {
        let scale = 10_f64.powi(digits);
        (x * scale).round_ties_even() / scale
    } else {
        signif(x, digits)
    }
}

/// Significance stars of a p-value (ggpubr's default `symnum.args`).
pub(super) fn p_signif(p: f64) -> &'static str {
    match p {
        p if p.is_nan() => "NA",
        p if p <= 1e-4 => "****",
        p if p <= 1e-3 => "***",
        p if p <= 1e-2 => "**",
        p if p <= 0.05 => "*",
        _ => "ns",
    }
}

/// Label texts as a discrete column (distinct levels in first appearance).
pub(super) fn label_column(labels: &[String]) -> Column {
    let mut levels: Vec<String> = Vec::new();
    let codes: Vec<Option<u32>> = labels
        .iter()
        .map(|label| {
            let code = match levels.iter().position(|level| level == label) {
                Some(code) => code,
                None => {
                    levels.push(label.clone());
                    levels.len() - 1
                }
            };
            Some(code as u32)
        })
        .collect();
    Column::Discrete(DiscreteColumn::from_parts(levels, codes.into()))
}

/// ggpubr `label.y.step` (1.4 text heights between stacked group labels) in
/// ggrust `vjust` units: lets-plot's `vjust` spans 0.7 em, not a text height,
/// so 1.4 / 0.7 keeps the labels 1.4 em apart, as they look in ggpubr.
pub(super) const LABEL_STEP: f64 = 1.4 / 0.7;

/// Where a label goes along one axis: `label_x`/`label_y` (data units, one or
/// one per group), else `label_x_npc`/`label_y_npc` (`"left"`, `"center"`,
/// `"right"` / `"bottom"`, `"center"`, `"top"`, or a share of the range).
#[derive(Debug, Clone, PartialEq)]
enum Place {
    Data(Vec<f64>),
    Named(&'static str),
    Share(f64),
}

impl Place {
    fn parse(parameters: &Parameters, axis: char, default: &'static str) -> Result<Place> {
        let data = format!("label_{axis}");
        if let Some(value) = parameters.value(&data).filter(|value| !value.is_null()) {
            let invalid = || {
                Error::spec(
                    parameters.path_of(&data),
                    format!("expected a number or a list of numbers (one per group), got {value}"),
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
            return Ok(Place::Data(values));
        }
        let npc = format!("label_{axis}_npc");
        let names: [&'static str; 4] = match axis {
            'x' => ["left", "center", "centre", "right"],
            _ => ["bottom", "center", "centre", "top"],
        };
        match parameters.value(&npc) {
            None | Some(Value::Null) => Ok(Place::Named(default)),
            Some(Value::Number(number)) => match number.as_f64() {
                Some(share) if (0.0..=1.0).contains(&share) => Ok(Place::Share(share)),
                _ => Err(Error::spec(
                    parameters.path_of(&npc),
                    format!("expected a share of the range in [0, 1], got {number}"),
                )),
            },
            Some(Value::String(name)) => match names.iter().find(|known| *known == name) {
                Some(&"centre") => Ok(Place::Named("center")),
                Some(known) => Ok(Place::Named(known)),
                None => Err(Error::spec(
                    parameters.path_of(&npc),
                    format!(
                        "unknown {npc} '{name}'{}; use one of: {}, or a number in [0, 1]",
                        crate::suggest::did_you_mean(name, names),
                        [names[0], names[1], names[3]].join(", ")
                    ),
                )),
            },
            Some(value) => Err(Error::spec(
                parameters.path_of(&npc),
                format!("expected a name or a number in [0, 1], got {value}"),
            )),
        }
    }

    /// Position and justification for the group numbered `group` (from 1).
    fn locate(&self, range: (f64, f64), group: usize, is_x: bool) -> (f64, f64) {
        match self {
            Place::Data(values) => {
                let value = values[(group - 1).min(values.len() - 1)];
                (value, if is_x { 0.0 } else { 0.5 })
            }
            Place::Share(share) => {
                let position = range.0 + share * (range.1 - range.0);
                let just = match is_x {
                    true => 0.0,
                    false => LABEL_STEP * group as f64 - LABEL_STEP / 2.0,
                };
                (position, just)
            }
            Place::Named(name) => match (*name, is_x) {
                ("left", true) => (range.0, 0.0),
                ("center", true) => ((range.0 + range.1) / 2.0, 0.5),
                ("right", true) => (range.1, 1.0),
                ("bottom", false) => (range.0, -LABEL_STEP * group as f64),
                ("center", false) => (
                    (range.0 + range.1) / 2.0,
                    LABEL_STEP * group as f64 - LABEL_STEP / 2.0,
                ),
                _ => (range.1, LABEL_STEP * group as f64),
            },
        }
    }
}

/// The finite range of a column over some rows.
pub(super) fn column_range(
    column: Option<&ContinuousColumn>,
    rows: &[usize],
) -> Option<(f64, f64)> {
    let column = column?;
    rows.iter()
        .filter_map(|&row| column.get(row))
        .filter(|value| value.is_finite())
        .fold(None, |range, value| {
            Some(range.map_or((value, value), |(low, high): (f64, f64)| {
                (low.min(value), high.max(value))
            }))
        })
}

/// One label per group (`label` returns `None` to leave a group out), placed
/// as ggpubr's `.label_params` places `stat_cor` labels: top left of the
/// plot's data range by default, group `g` (from 1) `1.4 g` lines below the
/// top. A constant `hjust`/`vjust` option replaces the computed one. Other
/// columns are carried per group (a label's color is its group's).
pub(super) fn labels_per_group(
    data: &LayerData,
    parameters: &Parameters,
    context: &StatContext,
    label: impl Fn(&[usize]) -> Result<Option<String>> + Sync,
) -> Result<LayerData> {
    let place_x = Place::parse(parameters, 'x', "left")?;
    let place_y = Place::parse(parameters, 'y', "top")?;
    let all_rows: Vec<usize> = (0..data.row_count()).collect();
    let x_range = context
        .x_range
        .or_else(|| column_range(data.continuous(Aesthetic::X), &all_rows))
        .unwrap_or((0.0, 1.0));
    let y_range = context
        .y_range
        .or_else(|| column_range(data.continuous(Aesthetic::Y), &all_rows))
        .unwrap_or((0.0, 1.0));
    let groups = rows_by_group(data);
    let mut labels: Vec<Option<String>> = Vec::with_capacity(groups.len());
    for rows in &groups {
        labels.push(label(rows)?);
    }
    let slot_of = |rows: &[usize]| {
        rows.first()
            .and_then(|first| groups.iter().position(|group| group.first() == Some(first)))
    };
    let mut result = per_group(data, |rows| {
        let Some(slot) = slot_of(rows).filter(|slot| labels[*slot].is_some()) else {
            return Ok(empty_label_columns());
        };
        let group = data.group_of(rows[0]) as usize + 1;
        let (x, hjust) = place_x.locate(x_range, group, true);
        let (y, vjust) = place_y.locate(y_range, group, false);
        Ok(vec![
            (Variable::Aesthetic(Aesthetic::X), vec![x]),
            (Variable::Aesthetic(Aesthetic::Y), vec![y]),
            (Variable::Aesthetic(Aesthetic::Hjust), vec![hjust]),
            (Variable::Aesthetic(Aesthetic::Vjust), vec![vjust]),
            (Variable::Aesthetic(Aesthetic::Label), vec![slot as f64]),
        ])
    })?;
    let texts: Vec<String> = result
        .continuous(Aesthetic::Label)
        .map(|column| {
            (0..column.len())
                .map(|row| {
                    let slot = column.get(row).unwrap_or(0.0) as usize;
                    labels[slot].clone().unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default();
    result.insert(Aesthetic::Label, label_column(&texts))?;
    for aesthetic in [Aesthetic::Hjust, Aesthetic::Vjust] {
        if parameters.value(aesthetic.as_str()).is_some() {
            result.remove(aesthetic);
        }
    }
    Ok(result)
}

fn empty_label_columns() -> Vec<(Variable, Vec<f64>)> {
    [
        Aesthetic::X,
        Aesthetic::Y,
        Aesthetic::Hjust,
        Aesthetic::Vjust,
        Aesthetic::Label,
    ]
    .into_iter()
    .map(|aesthetic| (Variable::Aesthetic(aesthetic), Vec::new()))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_print_as_r() {
        for (x, digits, r) in [
            (-0.8677, 2, "-0.87"),
            (0.5, 2, "0.5"),
            (1.0, 2, "1"),
            (12345.0, 2, "12345"),
            (123456.0, 3, "123456"),
            (1234567.0, 2, "1234567"),
            (123456789.0, 2, "1.2e+08"),
            (99.96, 3, "100"),
            (0.1 + 0.2, 15, "0.3"),
            (4.5876e-05, 3, "4.59e-05"),
            (0.000689, 3, "0.000689"),
            (-13.42, 3, "-13.4"),
            (1e-100, 2, "1e-100"),
        ] {
            assert_eq!(significant(x, digits), r, "{x}");
        }
        assert_eq!(p_equals(1e-20), "p < 2e-16");
        assert_eq!(p_equals(0.0645), "p = 0.065");
        assert_eq!(p_signif(0.05), "*");
        assert_eq!(p_signif(0.0001), "****");
    }

    #[test]
    fn rstatix_rounding_and_formats() {
        // R: signif(0.0147630147630148, 3), rstatix:::round_value(c(...), 3).
        assert_eq!(signif(0.014_763_014_763_014_8, 3), 0.0148);
        assert_eq!(signif(1.268_300_720_173_85e-7, 3), 1.27e-7);
        assert_eq!(round_value(0.0148, 3), 0.015);
        assert_eq!(round_value(0.000_181_5, 3), 0.000_182);
        assert_eq!(round_value(0.001, 3), 0.001);
        for (p, text) in [
            (0.000_152, "0.00015"),
            (9.9e-5, "<0.0001"),
            (1e-4, "1e-04"),
            (0.0232, "0.023"),
            (1.0, "1"),
            (0.004_03, "0.004"),
        ] {
            assert_eq!(p_format_rstatix(p), text, "{p}");
        }
    }
}
