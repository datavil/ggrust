//! `stat="pvalue_manual"`: brackets from a table of comparisons with p-value
//! labels, as the R package ggpubr's `stat_pvalue_manual` (its `geom_bracket`
//! stat) places them.
//!
//! Each row is a bracket from `xmin` to `xmax` at height `y` with its `label`
//! (columns and labels prepared by Python's `stat_pvalue_manual`). The stat lifts
//! the i-th row by `step_increase (i - 1)` of the y range plus `bracket_nudge_y`
//! (data units), counting i per group when `step_group_by` is true (the layer's
//! group mapping), and sets both tips to `tip_length` of the y range.
//!
//! What it does not do: tip lengths per bracket, label placement (the geom's).

use super::{Stat, StatContext};
use crate::data::{Column, ContinuousColumn, LayerData};
use crate::parameters::Parameters;
use crate::spec::Aesthetic;
use crate::{Error, Result};

/// Bracket heights and tips for a table of comparisons.
pub struct PvalueManualStat;

/// ggpubr `tip.length`.
const TIP_LENGTH: f64 = 0.03;

impl Stat for PvalueManualStat {
    fn name(&self) -> &'static str {
        "pvalue_manual"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &[
            "step_increase",
            "step_group_by",
            "tip_length",
            "bracket_nudge_y",
        ]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Xmin, Aesthetic::Xmax, Aesthetic::Y]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Xmin, Aesthetic::Xmax, Aesthetic::Y]
    }

    fn compute(
        &self,
        mut data: LayerData,
        parameters: &Parameters,
        context: &StatContext,
    ) -> Result<LayerData> {
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
        let step_increase = non_negative("step_increase", 0.0)?;
        let tip_length = non_negative("tip_length", TIP_LENGTH)?;
        let nudge = parameters.number("bracket_nudge_y")?.unwrap_or(0.0);
        let by_group = parameters.boolean("step_group_by")?.unwrap_or(false);
        let Some(y) = data.continuous(Aesthetic::Y) else {
            return Ok(data);
        };
        let rows: Vec<usize> = (0..data.row_count()).collect();
        let range = context
            .y_range
            .or_else(|| super::pubr_labels::column_range(Some(y), &rows))
            .unwrap_or((0.0, 1.0));
        let span = range.1 - range.0;
        let mut seen_per_group: Vec<usize> = vec![0; data.group_count().max(1)];
        let heights: Vec<f64> = rows
            .iter()
            .map(|&row| {
                let step = match by_group {
                    true => {
                        let slot = &mut seen_per_group[data.group_of(row) as usize];
                        *slot += 1;
                        *slot - 1
                    }
                    false => row,
                };
                y.get(row).unwrap_or(f64::NAN) + span * step_increase * step as f64 + nudge
            })
            .collect();
        let tips = vec![span * tip_length; data.row_count()];
        data.insert(
            Aesthetic::Y,
            Column::Continuous(ContinuousColumn::from_values(heights)),
        )?;
        for aesthetic in [Aesthetic::Lenstart, Aesthetic::Lenend] {
            data.insert(
                aesthetic,
                Column::Continuous(ContinuousColumn::from_values(tips.clone())),
            )?;
        }
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Options;

    fn compute(groups: Option<Vec<u32>>, options: &str) -> Result<LayerData> {
        let mut data = LayerData::new(3);
        for (aesthetic, values) in [
            (Aesthetic::Xmin, vec![0.0, 0.0, 1.0]),
            (Aesthetic::Xmax, vec![1.0, 2.0, 2.0]),
            (Aesthetic::Y, vec![35.0, 35.0, 35.0]),
        ] {
            data.insert(
                aesthetic,
                Column::Continuous(ContinuousColumn::from_values(values)),
            )?;
        }
        data.set_group_index(groups)?;
        let options: Options = serde_json::from_str(options).unwrap();
        let context = StatContext {
            y_range: Some((4.2, 35.0)),
            ..StatContext::default()
        };
        PvalueManualStat.compute(data, &Parameters::new(&options, "layers[1]"), &context)
    }

    fn values(result: &LayerData, aesthetic: Aesthetic) -> Vec<f64> {
        result.continuous(aesthetic).unwrap().values().to_vec()
    }

    // The ggpubr numbers are in tests/fixtures/pubr_helpers_reference.json
    // (`pvalue_manual`), checked end to end in tests/test_pubr_helpers.py.
    #[test]
    fn steps_and_tips_follow_ggpubr() {
        let result = compute(None, r#"{"step_increase": 0.1}"#).unwrap();
        let heights = values(&result, Aesthetic::Y);
        for (height, expected) in heights.iter().zip([35.0, 38.08, 41.16]) {
            assert!((height - expected).abs() < 1e-9, "{heights:?}");
        }
        assert!((values(&result, Aesthetic::Lenstart)[0] - 0.924).abs() < 1e-9);
        let grouped = compute(
            Some(vec![0, 0, 1]),
            r#"{"step_increase": 0.1, "step_group_by": true, "bracket_nudge_y": 1}"#,
        )
        .unwrap();
        let heights = values(&grouped, Aesthetic::Y);
        for (height, expected) in heights.iter().zip([36.0, 39.08, 36.0]) {
            assert!((height - expected).abs() < 1e-9, "{heights:?}");
        }
        let error = compute(None, r#"{"tip_length": -0.1}"#)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("tip_length") && error.contains(">= 0"),
            "{error}"
        );
    }
}
