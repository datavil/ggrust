//! Position scales from `scale_x_*()` / `scale_y_*()` / `xlim()` calls.
//!
//! Several calls for one aesthetic merge key by key, later ones winning, as in
//! lets-plot (`PlotConfigUtil.mergeScaleOptions`).

use super::{ContinuousPositionScale, DiscretePositionScale, PositionScale, Transform};
use crate::spec::{Aesthetic, PlotSpec, ScaleSpec, Value};
use crate::{Error, Result};

/// The merged x and y scale options of a plot. Whether an axis is discrete depends
/// on the data too, so the scales are built later with [`AxisScaleSpec::build`].
#[derive(Debug, Clone, Default)]
pub struct PositionScales {
    /// Horizontal axis.
    pub x: AxisScaleSpec,
    /// Vertical axis.
    pub y: AxisScaleSpec,
}

/// One merged option with the path of the call that set it.
type Set<T> = Option<(T, String)>;

/// Merged `scale_x_*()` (or `y`) options of one axis.
#[derive(Debug, Clone, Default)]
pub struct AxisScaleSpec {
    /// `name=...`: the axis title (after `labs()`).
    pub name: Option<String>,
    trans: Set<String>,
    limits: Set<Vec<Value>>,
    expand: Set<Vec<f64>>,
    discrete: Set<bool>,
    reverse: Set<bool>,
}

impl AxisScaleSpec {
    /// Whether the user asked for a discrete scale (`scale_x_discrete()`).
    pub fn requests_discrete(&self) -> bool {
        self.discrete
            .as_ref()
            .is_some_and(|(discrete, _)| *discrete)
    }

    /// Build the (untrained) scale; `discrete` when the axis shows categories.
    pub fn build(&self, discrete: bool, warnings: &mut Vec<String>) -> Result<PositionScale> {
        if discrete {
            return self.build_discrete(warnings).map(PositionScale::Discrete);
        }
        if let Some((true, path)) = &self.reverse {
            warnings.push(format!(
                "at '{path}': 'reverse' applies to discrete scales; use scale_*_reverse() for \
                 a continuous axis; ignored"
            ));
        }
        self.build_continuous().map(PositionScale::Continuous)
    }

    fn build_continuous(&self) -> Result<ContinuousPositionScale> {
        let mut scale = ContinuousPositionScale::default();
        if let Some((name, path)) = &self.trans {
            scale.transform = Transform::parse(name, path)?;
        }
        if let Some([multiplicative, additive]) = self.expansion()? {
            scale.multiplicative_expand = multiplicative.unwrap_or(scale.multiplicative_expand);
            scale.additive_expand = additive.unwrap_or(scale.additive_expand);
        }
        if let Some((limits, path)) = &self.limits {
            scale.limits = transformed_limits(limits, scale.transform, path)?;
        }
        Ok(scale)
    }

    fn build_discrete(&self, warnings: &mut Vec<String>) -> Result<DiscretePositionScale> {
        let mut scale = DiscretePositionScale::default();
        // lets-plot ignores a transform on a discrete axis.
        if let Some((name, path)) = &self.trans
            && Transform::parse(name, path)? != Transform::Identity
        {
            warnings.push(format!(
                "at '{path}': transform '{name}' applies to continuous axes, but this axis is \
                 discrete (a discrete column or scale_*_discrete() is mapped to it); ignored"
            ));
        }
        if let Some([multiplicative, additive]) = self.expansion()? {
            scale.multiplicative_expand = multiplicative.unwrap_or(scale.multiplicative_expand);
            scale.additive_expand = additive.unwrap_or(scale.additive_expand);
        }
        if let Some((limits, path)) = &self.limits {
            // Nulls are skipped (`xlim("a", None)`).
            let categories = limits
                .iter()
                .filter(|value| !value.is_null())
                .map(|value| match value {
                    Value::String(text) => Ok(text.clone()),
                    other => Err(Error::InvalidSpec(format!(
                        "at '{path}': discrete limits are category names (strings), got {other}"
                    ))),
                })
                .collect::<Result<Vec<_>>>()?;
            scale.set_limits(categories);
        }
        scale.reverse = self.reverse.as_ref().is_some_and(|(reverse, _)| *reverse);
        Ok(scale)
    }

    /// `[multiplicative, additive]` from `expand`, each optional.
    fn expansion(&self) -> Result<Option<[Option<f64>; 2]>> {
        let Some((expand, path)) = &self.expand else {
            return Ok(None);
        };
        if expand.len() > 2 || expand.iter().any(|value| !value.is_finite()) {
            return Err(Error::InvalidSpec(format!(
                "at '{path}': expected [multiplicative] or [multiplicative, additive] \
                 finite numbers, got {expand:?}"
            )));
        }
        Ok(Some([expand.first().copied(), expand.get(1).copied()]))
    }
}

/// Read the x and y scale options; options not built yet are reported in `warnings`.
pub fn position_scales(spec: &PlotSpec, warnings: &mut Vec<String>) -> Result<PositionScales> {
    let mut merged = [AxisScaleSpec::default(), AxisScaleSpec::default()];
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
    Ok(PositionScales { x, y })
}

fn merge(target: &mut AxisScaleSpec, scale: &ScaleSpec, path: &str) {
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
    if let Some(discrete) = scale.discrete {
        target.discrete = Some((discrete, at("discrete")));
    }
    if let Some(reverse) = scale.reverse {
        target.reverse = Some((reverse, at("reverse")));
    }
}

/// `[low, high]` in data units -> limits in transformed units, as lenient as
/// lets-plot: null leaves a side open, the two ends may come in either order, values
/// after the second are ignored, an empty list means no limits, and a limit outside
/// the transform's domain (log of 0) leaves that side open.
fn transformed_limits(
    limits: &[Value],
    transform: Transform,
    path: &str,
) -> Result<[Option<f64>; 2]> {
    let read = |value: Option<&Value>| -> Result<Option<f64>> {
        match value {
            None | Some(Value::Null) => Ok(None),
            Some(value) => value.as_f64().map(Some).ok_or_else(|| {
                Error::InvalidSpec(format!(
                    "at '{path}': limits are numbers or null, got {value}"
                ))
            }),
        }
    };
    let (mut low, mut high) = (read(limits.first())?, read(limits.get(1))?);
    if let (Some(first), Some(second)) = (low, high)
        && first > second
    {
        (low, high) = (Some(second), Some(first));
    }
    let transformed = |limit: Option<f64>| limit.and_then(|value| transform.apply(value));
    let (low, high) = (transformed(low), transformed(high));
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

    fn built(axis: &AxisScaleSpec, discrete: bool) -> Result<PositionScale> {
        axis.build(discrete, &mut Vec::new())
    }

    fn continuous(axis: &AxisScaleSpec) -> ContinuousPositionScale {
        match built(axis, false).unwrap() {
            PositionScale::Continuous(scale) => scale,
            PositionScale::Discrete(_) => panic!("expected a continuous scale"),
        }
    }

    fn plot_with(scale: &str) -> String {
        format!(r#"{{"kind": "plot", "scales": [{scale}]}}"#)
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
        assert_eq!(scales.x.name.as_deref(), Some("second"));
        let x = continuous(&scales.x);
        assert_eq!(x.transform, Transform::Log10);
        let [low, high] = x.limits;
        assert!((low.unwrap() - 0.0).abs() < 1e-12 && (high.unwrap() - 3.0).abs() < 1e-12);
        assert_eq!(x.multiplicative_expand, 0.1);
        assert_eq!(continuous(&scales.y), ContinuousPositionScale::default());
    }

    #[test]
    fn reverse_limits_swap_sides() {
        let (scales, _) = scales(&plot_with(
            r#"{"aesthetic": "y", "trans": "reverse", "limits": [2, null]}"#,
        ))
        .unwrap();
        assert_eq!(continuous(&scales.y).limits, [None, Some(-2.0)]);
    }

    #[test]
    fn discrete_scales_take_category_limits_reverse_and_expand() {
        let (scales, warnings) = scales(&plot_with(
            r#"{"aesthetic": "x", "discrete": true, "reverse": true, "limits": ["b", "a"],
                "expand": [0, 0.5]}"#,
        ))
        .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(scales.x.requests_discrete());
        let PositionScale::Discrete(x) = built(&scales.x, true).unwrap() else {
            panic!("expected a discrete scale");
        };
        assert_eq!(x.effective_levels(), ["a", "b"]);
        assert_eq!(x.expanded_domain(), (-0.5, 1.5));
    }

    #[test]
    fn reverse_on_a_continuous_axis_warns() {
        let (scales, _) = scales(&plot_with(r#"{"aesthetic": "x", "reverse": true}"#)).unwrap();
        let mut warnings = Vec::new();
        scales.x.build(false, &mut warnings).unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].contains("at 'scales[0].reverse'"),
            "{warnings:?}"
        );
    }

    #[test]
    fn limits_are_as_lenient_as_lets_plot() {
        let limits = |scale: &str| {
            let (scales, _) = scales(&plot_with(scale)).unwrap();
            continuous(&scales.x).limits
        };
        // Either order; reverse keeps both points inside (bug hunt 0.10.2).
        assert_eq!(
            limits(r#"{"aesthetic": "x", "limits": [10, 1]}"#),
            [Some(1.0), Some(10.0)]
        );
        assert_eq!(
            limits(r#"{"aesthetic": "x", "trans": "reverse", "limits": [10, 1]}"#),
            [Some(-10.0), Some(-1.0)]
        );
        // Outside the log domain: that side stays open.
        let [low, high] = limits(r#"{"aesthetic": "x", "trans": "log10", "limits": [0, 100]}"#);
        assert_eq!(low, None);
        assert!((high.unwrap() - 2.0).abs() < 1e-12);
        assert_eq!(limits(r#"{"aesthetic": "x", "limits": []}"#), [None, None]);
        assert_eq!(
            limits(r#"{"aesthetic": "x", "limits": [1, 5, 9]}"#),
            [Some(1.0), Some(5.0)]
        );
    }

    #[test]
    fn discrete_axes_skip_null_limits_and_ignore_transforms_with_a_warning() {
        let (scales, _) = scales(&plot_with(
            r#"{"aesthetic": "x", "trans": "log10", "limits": ["a", null]}"#,
        ))
        .unwrap();
        let mut warnings = Vec::new();
        let PositionScale::Discrete(x) = scales.x.build(true, &mut warnings).unwrap() else {
            panic!("expected a discrete scale");
        };
        assert_eq!(x.effective_levels(), ["a"]);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("at 'scales[0].trans'"), "{warnings:?}");
    }

    #[test]
    fn errors_name_the_path() {
        for (json, discrete, expected) in [
            (
                r#"{"aesthetic": "x", "trans": "log"}"#,
                false,
                "at 'scales[0].trans': unknown transform",
            ),
            (
                r#"{"aesthetic": "y", "expand": [0, 1, 2]}"#,
                false,
                "at 'scales[0].expand'",
            ),
            (
                r#"{"aesthetic": "x", "limits": ["a", 2]}"#,
                true,
                "at 'scales[0].limits': discrete limits are category names",
            ),
            (
                r#"{"aesthetic": "x", "limits": ["a", "b"]}"#,
                false,
                "at 'scales[0].limits': limits are numbers or null",
            ),
        ] {
            let (scales, _) = scales(&plot_with(json)).unwrap();
            let axis = if json.contains(r#""x""#) {
                &scales.x
            } else {
                &scales.y
            };
            let error = built(axis, discrete).unwrap_err().to_string();
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
                "at 'scales[1]': scales for 'color' are not supported yet; ignored",
            ]
        );
    }
}
