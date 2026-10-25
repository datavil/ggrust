//! Position scales from `scale_x_*()` / `scale_y_*()` / `xlim()` calls.
//!
//! Several calls for one aesthetic merge key by key, later ones winning, as in
//! lets-plot (`PlotConfigUtil.mergeScaleOptions`).

use super::{ContinuousPositionScale, Transform};
use crate::spec::{Aesthetic, PlotSpec, ScaleSpec, Value};
use crate::{Error, Result};

/// The x and y scales of a plot (untrained) and their `name`s.
#[derive(Debug, Clone, Default)]
pub struct PositionScales {
    /// Horizontal scale.
    pub x: ContinuousPositionScale,
    /// Vertical scale.
    pub y: ContinuousPositionScale,
    /// `scale_x_*(name=...)`.
    pub x_name: Option<String>,
    /// `scale_y_*(name=...)`.
    pub y_name: Option<String>,
}

/// One merged option with the path of the call that set it.
type Set<T> = Option<(T, String)>;

#[derive(Default)]
struct Merged {
    name: Option<String>,
    trans: Set<String>,
    limits: Set<Vec<Value>>,
    expand: Set<Vec<f64>>,
}

/// Read the x and y scales; options not built yet are reported in `warnings`.
pub fn position_scales(spec: &PlotSpec, warnings: &mut Vec<String>) -> Result<PositionScales> {
    let mut merged = [Merged::default(), Merged::default()];
    for (index, scale) in spec.scales.iter().enumerate() {
        let path = format!("scales[{index}]");
        for aesthetic in scale.aesthetic.as_slice() {
            let target = match aesthetic {
                Aesthetic::X => &mut merged[0],
                Aesthetic::Y => &mut merged[1],
                other => {
                    warnings.push(format!(
                        "at '{path}': scales for '{other}' are not supported yet; ignored"
                    ));
                    continue;
                }
            };
            merge(target, scale, &path);
            report_unbuilt(scale, &path, warnings);
        }
    }
    let [x, y] = merged;
    Ok(PositionScales {
        x_name: x.name.clone(),
        y_name: y.name.clone(),
        x: build(x)?,
        y: build(y)?,
    })
}

fn merge(target: &mut Merged, scale: &ScaleSpec, path: &str) {
    let at = |key: &str| format!("{path}.{key}");
    if let Some(name) = &scale.name {
        target.name = Some(name.clone());
    }
    if let Some(trans) = &scale.trans {
        target.trans = Some((trans.clone(), at("trans")));
    }
    if let Some(limits) = &scale.limits {
        target.limits = Some((limits.clone(), at("limits")));
    }
    if let Some(expand) = &scale.expand {
        target.expand = Some((expand.clone(), at("expand")));
    }
}

fn build(merged: Merged) -> Result<ContinuousPositionScale> {
    let mut scale = ContinuousPositionScale::default();
    if let Some((name, path)) = &merged.trans {
        scale.transform = Transform::parse(name, path)?;
    }
    if let Some((expand, path)) = &merged.expand {
        if expand.len() > 2 || expand.iter().any(|value| !value.is_finite()) {
            return Err(Error::InvalidSpec(format!(
                "at '{path}': expected [multiplicative] or [multiplicative, additive] \
                 finite numbers, got {expand:?}"
            )));
        }
        if let Some(multiplicative) = expand.first() {
            scale.multiplicative_expand = *multiplicative;
        }
        if let Some(additive) = expand.get(1) {
            scale.additive_expand = *additive;
        }
    }
    if let Some((limits, path)) = &merged.limits {
        scale.limits = transformed_limits(limits, scale.transform, path)?;
    }
    Ok(scale)
}

/// `[low, high]` in data units (either may be null) -> limits in transformed units.
fn transformed_limits(
    limits: &[Value],
    transform: Transform,
    path: &str,
) -> Result<[Option<f64>; 2]> {
    let invalid = |detail: String| Error::InvalidSpec(format!("at '{path}': {detail}"));
    let [low, high] = limits else {
        return Err(invalid(format!(
            "expected [low, high] (numbers or null), got {} values",
            limits.len()
        )));
    };
    let read = |value: &Value| -> Result<Option<f64>> {
        if value.is_null() {
            return Ok(None);
        }
        let number = value
            .as_f64()
            .ok_or_else(|| invalid(format!("expected a number or null, got {value}")))?;
        transform.apply(number).map(Some).ok_or_else(|| {
            invalid(format!(
                "limit {number} is outside the domain of the '{}' transform",
                transform.as_str()
            ))
        })
    };
    let (low, high) = (read(low)?, read(high)?);
    if let (Some(low), Some(high)) = (low, high)
        && low > high
        && transform != Transform::Reverse
    {
        return Err(invalid(format!(
            "lower limit is above the upper limit: {limits:?}"
        )));
    }
    // Reverse runs the other way: the lower data limit becomes the upper one.
    Ok(match transform {
        Transform::Reverse => [high, low],
        _ => [low, high],
    })
}

/// Warn about scale options that are parsed but not built yet.
fn report_unbuilt(scale: &ScaleSpec, path: &str, warnings: &mut Vec<String>) {
    let present = [
        ("breaks", scale.breaks.is_some()),
        ("labels", scale.labels.is_some()),
        ("format", scale.format.is_some()),
        ("na_value", scale.na_value.is_some()),
        ("guide", scale.guide.is_some()),
        ("discrete", scale.discrete == Some(true)),
        ("reverse", scale.reverse == Some(true)),
        ("position", scale.position.is_some()),
        ("scale_mapper_kind", scale.scale_mapper_kind.is_some()),
    ];
    for (key, _) in present.iter().filter(|(_, present)| *present) {
        warnings.push(format!("at '{path}.{key}': not supported yet; ignored"));
    }
    for key in scale.parameters.keys() {
        warnings.push(format!("at '{path}.{key}': not supported yet; ignored"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Figure;

    fn scales(json: &str) -> Result<(PositionScales, Vec<String>)> {
        let Figure::Plot(plot) = Figure::from_json(json).unwrap() else {
            panic!("expected a plot");
        };
        let mut warnings = Vec::new();
        position_scales(&plot, &mut warnings).map(|scales| (scales, warnings))
    }

    #[test]
    fn calls_merge_key_by_key_later_wins() {
        let (scales, warnings) = scales(
            r#"{"kind": "plot", "scales": [
                {"aesthetic": "x", "trans": "log10", "name": "first"},
                {"aesthetic": "x", "limits": [1, 1000]},
                {"aesthetic": "x", "name": "second", "expand": [0.1]}]}"#,
        )
        .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(scales.x.transform, Transform::Log10);
        assert_eq!(scales.x_name.as_deref(), Some("second"));
        let [low, high] = scales.x.limits;
        assert!((low.unwrap() - 0.0).abs() < 1e-12 && (high.unwrap() - 3.0).abs() < 1e-12);
        assert_eq!(scales.x.multiplicative_expand, 0.1);
        assert_eq!(scales.y, ContinuousPositionScale::default());
    }

    #[test]
    fn reverse_limits_swap_sides() {
        let (scales, _) = scales(
            r#"{"kind": "plot", "scales": [{"aesthetic": "y", "trans": "reverse", "limits": [2, null]}]}"#,
        )
        .unwrap();
        assert_eq!(scales.y.limits, [None, Some(-2.0)]);
    }

    #[test]
    fn errors_name_the_path() {
        for (json, expected) in [
            (
                r#"{"aesthetic": "x", "trans": "log"}"#,
                "at 'scales[0].trans': unknown transform",
            ),
            (
                r#"{"aesthetic": "x", "trans": "log10", "limits": [0, 10]}"#,
                "at 'scales[0].limits': limit 0 is outside the domain of the 'log10' transform",
            ),
            (
                r#"{"aesthetic": "x", "limits": [5, 1]}"#,
                "lower limit is above the upper",
            ),
            (
                r#"{"aesthetic": "y", "limits": [1]}"#,
                "expected [low, high]",
            ),
            (
                r#"{"aesthetic": "y", "expand": [0, 1, 2]}"#,
                "at 'scales[0].expand'",
            ),
        ] {
            let error = scales(&format!(r#"{{"kind": "plot", "scales": [{json}]}}"#))
                .unwrap_err()
                .to_string();
            assert!(error.contains(expected), "{error}");
        }
    }

    #[test]
    fn unbuilt_options_and_other_aesthetics_warn() {
        let (_, warnings) = scales(
            r#"{"kind": "plot", "scales": [
                {"aesthetic": "x", "breaks": [1, 2], "discrete": true, "reverse": false},
                {"aesthetic": "color", "palette": "Set1"}]}"#,
        )
        .unwrap();
        assert_eq!(
            warnings,
            [
                "at 'scales[0].breaks': not supported yet; ignored",
                "at 'scales[0].discrete': not supported yet; ignored",
                "at 'scales[1]': scales for 'color' are not supported yet; ignored",
            ]
        );
    }
}
