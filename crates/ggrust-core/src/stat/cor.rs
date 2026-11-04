//! `stat="cor"`: a correlation coefficient and its p-value per group, as the R
//! package ggpubr's `stat_cor` labels them: `"R = -0.87, p = 1.3e-10"`.
//!
//! `method` (`"pearson"` default, `"spearman"`, `"kendall"`; R's `cor.test`,
//! see `stat/hypothesis.rs`), `cor_coef_name` (`"R"`, or `"rho"`, `"tau"`),
//! `r_digits` / `p_digits` (significant digits, 2), `label_sep` (`", "`).
//! Places: `label_x`/`label_y` (data units, one or one per group), else
//! `label_x_npc` (`"left"`) / `label_y_npc` (`"top"`) of the plot's data range,
//! group labels stacked 1.4 em apart (`stat/pubr_labels.rs`).
//!
//! Groups with fewer than two distinct x (or too few pairs for the test) get no
//! label. A recorded deviation: ggpubr 1.0's text labels print the raw p-value
//! (`p = 1.29395870135052e-10`); ggrust prints `p_digits` significant digits, as
//! earlier ggpubr and its plotmath labels do.
//!
//! What it does not do: one-sided `alternative`, `output.type`, `r.accuracy`,
//! `p.accuracy`, confidence intervals, `rr.label`.

use super::hypothesis::Correlation;
use super::pubr_labels::{labels_per_group, significant};
use super::{Stat, StatContext};
use crate::data::LayerData;
use crate::parameters::Parameters;
use crate::spec::Aesthetic;
use crate::{Error, Result};

/// Correlation labels.
pub struct CorStat;

impl Stat for CorStat {
    fn name(&self) -> &'static str {
        "cor"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &[
            "method",
            "cor_coef_name",
            "r_digits",
            "p_digits",
            "label_sep",
            "label_x",
            "label_y",
            "label_x_npc",
            "label_y_npc",
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
        let method = match parameters.string("method")? {
            None => Correlation::Pearson,
            Some(name) => Correlation::parse(name).ok_or_else(|| {
                Error::spec(
                    parameters.path_of("method"),
                    format!(
                        "unknown method '{name}'{}; use one of: {}",
                        crate::suggest::did_you_mean(name, Correlation::NAMES),
                        Correlation::NAMES.join(", ")
                    ),
                )
            })?,
        };
        let digits = |name: &str| -> Result<usize> {
            match parameters.number(name)? {
                None => Ok(2),
                Some(value) if value >= 1.0 && value.fract() == 0.0 && value <= 15.0 => {
                    Ok(value as usize)
                }
                Some(value) => Err(Error::spec(
                    parameters.path_of(name),
                    format!("expected a whole number of digits from 1 to 15, got {value}"),
                )),
            }
        };
        let (r_digits, p_digits) = (digits("r_digits")?, digits("p_digits")?);
        let name = parameters.string("cor_coef_name")?.unwrap_or("R");
        let separator = parameters.string("label_sep")?.unwrap_or(", ");
        let (Some(x), Some(y)) = (data.continuous(Aesthetic::X), data.continuous(Aesthetic::Y))
        else {
            return Ok(LayerData::new(0));
        };
        labels_per_group(&data, parameters, context, |rows| {
            let xs: Vec<f64> = rows
                .iter()
                .map(|&row| x.get(row).unwrap_or(f64::NAN))
                .collect();
            let ys: Vec<f64> = rows
                .iter()
                .map(|&row| y.get(row).unwrap_or(f64::NAN))
                .collect();
            let mut distinct: Vec<f64> = xs.iter().copied().filter(|v| v.is_finite()).collect();
            distinct.sort_by(f64::total_cmp);
            distinct.dedup();
            if distinct.len() < 2 {
                return Ok(None);
            }
            let Ok((estimate, p)) = method.test(&xs, &ys) else {
                return Ok(None);
            };
            let p_text = match p {
                p if p < 2.2e-16 => "p < 2.2e-16".to_owned(),
                p => format!("p = {}", significant(p, p_digits)),
            };
            Ok(Some(format!(
                "{name} = {}{separator}{p_text}",
                significant(estimate, r_digits)
            )))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Column, ContinuousColumn};
    use crate::spec::Options;

    #[test]
    fn labels_and_places_match_ggpubr() {
        let all: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/pubr_reference.json"
        ))
        .unwrap();
        let layers = &all["layers"];
        let numbers = |name: &str| -> Vec<f64> {
            layers["cars"][name]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect()
        };
        // R numbers groups by sorted factor level: cyl 4, 6, 8.
        let groups: Vec<u32> = layers["cars"]["cyl"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| match v.as_str().unwrap() {
                "4" => 0,
                "6" => 1,
                _ => 2,
            })
            .collect();
        let (wt, mpg) = (numbers("wt"), numbers("mpg"));
        let context = StatContext {
            x_range: Some((1.513, 5.424)),
            y_range: Some((10.4, 33.9)),
            x_levels: None,
        };
        for (case, grouped) in [("cor", false), ("cor_groups", true)] {
            let mut data = LayerData::new(wt.len());
            let column =
                |values: &[f64]| Column::Continuous(ContinuousColumn::from_values(values.to_vec()));
            data.insert(Aesthetic::X, column(&wt)).unwrap();
            data.insert(Aesthetic::Y, column(&mpg)).unwrap();
            if grouped {
                data.set_group_index(Some(groups.clone())).unwrap();
            }
            let options: Options = serde_json::from_str("{}").unwrap();
            let result = CorStat
                .compute(data, &Parameters::new(&options, "layers[0]"), &context)
                .unwrap();
            let rows = layers[case].as_array().unwrap();
            assert_eq!(result.row_count(), rows.len());
            let Some(Column::Discrete(labels)) = result.get(Aesthetic::Label) else {
                panic!("labels");
            };
            // Rows in group order (ggrust visits groups in first appearance, R by id).
            let value = |aesthetic: Aesthetic, row: usize| {
                result.continuous(aesthetic).unwrap().get(row).unwrap()
            };
            let mut order: Vec<usize> = (0..result.row_count()).collect();
            order.sort_by(|&a, &b| {
                value(Aesthetic::Vjust, a).total_cmp(&value(Aesthetic::Vjust, b))
            });
            for (&row, expected) in order.iter().zip(rows) {
                for (aesthetic, key) in [
                    (Aesthetic::X, "x"),
                    (Aesthetic::Y, "y"),
                    (Aesthetic::Hjust, "hjust"),
                    (Aesthetic::Vjust, "vjust"),
                ] {
                    let actual = value(aesthetic, row);
                    let mut expected = expected[key].as_f64().unwrap();
                    if key == "vjust" {
                        // ggpubr steps 1.4 text heights; ggrust 1.4 em (`LABEL_STEP`).
                        expected *= super::super::pubr_labels::LABEL_STEP / 1.4;
                    }
                    assert!((actual - expected).abs() < 1e-9, "{case} {key}");
                }
                // ggpubr 1.0 prints the raw p-value; ggrust 2 significant digits.
                let text = labels.label(row).unwrap();
                let r_text = expected["label"].as_str().unwrap();
                let (coefficient, p) = r_text.split_once(", p = ").unwrap();
                let p: f64 = p.parse().unwrap();
                assert_eq!(
                    text,
                    format!("{coefficient}, p = {}", significant(p, 2)),
                    "{case}"
                );
            }
        }
    }
}
