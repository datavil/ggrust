//! Position scales from `scale_x_*()` / `scale_y_*()` / `xlim()` calls.
//!
//! Several calls for one aesthetic merge key by key, later ones winning, as in
//! lets-plot (`PlotConfigUtil.mergeScaleOptions`).

use super::datetime::{DatePattern, TemporalAxis, TemporalKind, TimeInterval};
use super::label_pattern::LabelPattern;
use super::secondary::SecondaryAxis;
use super::{AxisSides, ContinuousPositionScale, DiscretePositionScale, PositionScale, Transform};
use crate::cluster::{DendrogramType, Tree};
use crate::layout::axis_mark::AxisMark;
use crate::layout::dendrogram::{AxisDendrogram, DEFAULT_SPACE, DendrogramSide};
use crate::spec::{Aesthetic, GuideSpec, Options, PlotSpec, ScaleSpec, Value};
use crate::{Error, Result};

/// The merged x and y scale options of a plot. Whether an axis is discrete depends
/// on the data too, so the scales are built later with [`AxisScaleSpec::build`].
#[derive(Debug, Clone, Default)]
pub struct PositionScales {
    /// Horizontal axis.
    pub x: AxisScaleSpec,
    /// Vertical axis.
    pub y: AxisScaleSpec,
    /// Depth axis of `coord_3d` (`scale_z_continuous()`).
    pub z: AxisScaleSpec,
    /// The x side panel's own y axis (ggside `scale_xsidey_*()`).
    pub xside_y: AxisScaleSpec,
    /// The y side panel's own x axis (ggside `scale_ysidex_*()`).
    pub yside_x: AxisScaleSpec,
    /// Side band scales that name their band (`side_pos=`, `side_band=`), one per
    /// distinct target, merged key by key. They replace `xside_y` / `yside_x` for
    /// the bands they target.
    pub targeted: Vec<TargetedSideScale>,
}

/// The side bands a `scale_xsidey_*()` / `scale_ysidex_*()` call with `side_pos=`
/// or `side_band=` sets: every band on its axis matching each key it gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideTarget {
    /// An x side band scale (`scale_xsidey_*()`), else a y one.
    pub shares_x: bool,
    /// `side_pos=`: the far side (top, right) or not; `None`: either side.
    pub far: Option<bool>,
    /// `side_band=`: the band's name; `None`: any band.
    pub name: Option<String>,
}

impl SideTarget {
    /// Whether it targets the band on axis `shares_x`, side `far`, named `name`.
    pub fn matches(&self, shares_x: bool, far: bool, name: Option<&str>) -> bool {
        self.shares_x == shares_x
            && self.far.is_none_or(|own| own == far)
            && self.name.as_deref().is_none_or(|own| Some(own) == name)
    }

    /// How specific it is: both keys, then a name, then a side.
    fn rank(&self) -> u8 {
        u8::from(self.name.is_some()) * 2 + u8::from(self.far.is_some())
    }
}

/// The merged options of the scale calls with one [`SideTarget`].
#[derive(Debug, Clone)]
pub struct TargetedSideScale {
    /// The bands it sets.
    pub target: SideTarget,
    /// The merged options.
    pub spec: AxisScaleSpec,
}

impl PositionScales {
    /// The options of the most specific targeted scale of the band on axis
    /// `shares_x`, side `far`, named `name` (`None`: no targeted scale; the band
    /// takes `xside_y` / `yside_x`).
    pub fn side_spec(
        &self,
        shares_x: bool,
        far: bool,
        name: Option<&str>,
    ) -> Option<&AxisScaleSpec> {
        self.targeted
            .iter()
            .filter(|scale| scale.target.matches(shares_x, far, name))
            .max_by_key(|scale| scale.target.rank())
            .map(|scale| &scale.spec)
    }
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
    breaks: Set<Vec<Value>>,
    labels: Set<Value>,
    format: Set<String>,
    /// `scale_*_datetime()` (date-time) or `scale_*_time()` (duration).
    temporal: Set<TemporalKind>,
    /// `break_width=...`: a time interval (`"2 weeks"`) on date-time axes, a
    /// number elsewhere.
    break_width: Set<Value>,
    /// `na_value=...`: where missing positions are drawn (continuous axes).
    na_value: Set<Value>,
    /// `lablim=...`: most characters of a category label (discrete axes).
    label_limit: Set<Value>,
    /// `scale_*_dendro(tree)`: the tree as given in the spec.
    dendrogram: Set<Value>,
    /// `scale_*_dendro(leaf_levels=)`: `{leaf: [levels]}`.
    leaf_levels: Set<Value>,
    /// `guide_axis_dendro(...)` options (`space`, `side`, `type`).
    dendrogram_guide: Set<Options>,
    /// `guide_axis_mark(...)` options (`spread`, `link_width`, `padding`).
    mark_guide: Set<Options>,
    /// `guide_axis_nested(...)` options (`key`, `type`, `bracket`, ...).
    nested_guide: Set<Options>,
    /// `position=...`: the sides the axis is drawn on.
    sides: Set<AxisSides>,
    /// `sec_axis=...` / `dup_axis()`, parsed, with its path.
    secondary: Set<SecondaryAxis>,
    /// A lets-plot margin below or left of the panel: values grow away from the
    /// panel (reversed axis), whatever the axis kind.
    away_from_panel: bool,
}

impl AxisScaleSpec {
    /// The sides the axis is drawn on (`position=`; bottom / left by default).
    pub fn sides(&self) -> AxisSides {
        self.sides
            .as_ref()
            .map(|(sides, _)| *sides)
            .unwrap_or_default()
    }

    /// The cross axis of a lets-plot margin below or left of the panel, where
    /// values grow away from the panel (`ggmarginal("b")`, `ggmarginal("l")`).
    pub fn away_from_panel() -> AxisScaleSpec {
        AxisScaleSpec {
            away_from_panel: true,
            ..AxisScaleSpec::default()
        }
    }

    /// Whether the user asked for a discrete scale (`scale_x_discrete()`).
    pub fn requests_discrete(&self) -> bool {
        self.discrete
            .as_ref()
            .is_some_and(|(discrete, _)| *discrete)
    }

    /// Build the (untrained) scale; `discrete` when the axis shows categories.
    pub fn build(&self, discrete: bool, warnings: &mut Vec<String>) -> Result<PositionScale> {
        self.build_with(discrete, None, warnings)
    }

    /// [`Self::build`] for an axis whose mapped columns are of `column_kind`
    /// (date-time columns get a date-time axis without `scale_*_datetime()`).
    pub fn build_with(
        &self,
        discrete: bool,
        column_kind: Option<TemporalKind>,
        warnings: &mut Vec<String>,
    ) -> Result<PositionScale> {
        if discrete {
            return self.build_discrete(warnings).map(PositionScale::Discrete);
        }
        if let Some((_, path)) = &self.dendrogram {
            return Err(Error::spec(
                path,
                "scale_*_dendro() needs a discrete axis, but only numbers are mapped to it; \
                 map the column whose values are the tree's leaves",
            ));
        }
        if let Some((true, path)) = &self.reverse {
            warnings.push(format!(
                "at '{path}': 'reverse' applies to discrete scales; use scale_*_reverse() for \
                 a continuous axis; ignored"
            ));
        }
        self.build_continuous(column_kind, warnings)
            .map(PositionScale::Continuous)
    }

    fn build_continuous(
        &self,
        column_kind: Option<TemporalKind>,
        warnings: &mut Vec<String>,
    ) -> Result<ContinuousPositionScale> {
        let mut scale = ContinuousPositionScale::default();
        scale.sides = self.sides();
        scale.mark = self.axis_mark()?;
        scale.nested = self.axis_nested(false)?;
        let labels = self.label_texts()?;
        scale.temporal = self.temporal_axis(column_kind, warnings)?;
        if scale.temporal.is_none() {
            scale.break_width = self.number_break_width()?;
        }
        // lets-plot ignores `format` too when `labels` come without `breaks`.
        if self.breaks.is_some() || labels.is_none() {
            match (&mut scale.temporal, &self.format) {
                (Some(temporal), Some((pattern, _)))
                    if temporal.kind != TemporalKind::Duration
                        && DatePattern::is_datetime(pattern) =>
                {
                    temporal.format = Some(DatePattern::parse(pattern));
                }
                _ => scale.format = self.label_format(warnings),
            }
        }
        match (&self.breaks, labels) {
            (Some((breaks, path)), labels) => {
                let values = breaks
                    .iter()
                    .map(|value| match value {
                        Value::Null => Ok(None),
                        value => value
                            .as_f64()
                            .filter(|value| value.is_finite())
                            .map(Some)
                            .ok_or_else(|| {
                                Error::spec(
                                    path,
                                    format!("breaks of a continuous axis are numbers, got {value}"),
                                )
                            }),
                    })
                    .collect::<Result<Vec<_>>>()?;
                if let Some((labels, labels_path)) = &labels
                    && labels.len() != values.len()
                {
                    // lets-plot fails the plot here too.
                    return Err(Error::spec(
                        labels_path,
                        format!(
                            "{} labels for {} breaks; give one label per break",
                            labels.len(),
                            values.len()
                        ),
                    ));
                }
                scale.fixed_breaks = Some(values);
                scale.labels = labels.map(|(labels, _)| labels);
            }
            // ggplot2: the labels name the generated breaks (lets-plot ignores
            // them); see `ContinuousPositionScale::attach_labels`.
            (None, Some((labels, path))) => {
                scale.labels = Some(labels);
                scale.labels_without_breaks = Some(path);
            }
            (None, None) => {}
        }
        if let Some((name, path)) = &self.trans {
            scale.transform = Transform::parse(name, path)?;
        } else if self.away_from_panel {
            scale.transform = Transform::parse("reverse", "margin_side")?;
        }
        if let Some((axis, path)) = &self.secondary {
            if scale.temporal.is_some() {
                warnings.push(format!(
                    "at '{path}': secondary axes on date-time axes are not supported yet; ignored"
                ));
            } else {
                scale.secondary = Some(axis.clone());
            }
        }
        if let Some([multiplicative, additive]) = self.expansion()? {
            scale.multiplicative_expand = multiplicative.unwrap_or(scale.multiplicative_expand);
            scale.additive_expand = additive.unwrap_or(scale.additive_expand);
        }
        if let Some((limits, path)) = &self.limits {
            scale.limits = transformed_limits(limits, scale.transform, path)?;
        }
        if let Some((value, path)) = &self.na_value {
            scale.na_value = Some(
                value
                    .as_f64()
                    .filter(|value| value.is_finite())
                    .and_then(|value| scale.transform.apply(value))
                    .ok_or_else(|| {
                        Error::spec(
                            path,
                            format!(
                                "na_value of a continuous axis is the position missing values \
                                 are drawn at: a finite number inside the '{}' transform's \
                                 domain, got {value}",
                                scale.transform.as_str()
                            ),
                        )
                    })?,
            );
        }
        Ok(scale)
    }

    fn build_discrete(&self, warnings: &mut Vec<String>) -> Result<DiscretePositionScale> {
        let mut scale = DiscretePositionScale::default();
        scale.sides = self.sides();
        scale.mark = self.axis_mark()?;
        scale.nested = self.axis_nested(true)?;
        if let Some((_, path)) = &self.na_value {
            // ggplot2 ignores it too (missing categories get their own level there).
            warnings.push(format!(
                "at '{path}': na_value applies to continuous axes; missing categories are \
                 dropped; ignored"
            ));
        }
        if let Some((_, path)) = &self.secondary {
            warnings.push(format!(
                "at '{path}': a secondary axis needs a continuous axis, but this one shows \
                 categories; ignored"
            ));
        }
        scale.format = self.label_format(warnings);
        scale.labels = self.label_texts()?.map(|(labels, _)| labels);
        scale.label_limit = self.label_limit(warnings);
        if let Some((breaks, path)) = &self.breaks {
            scale.set_breaks(
                breaks
                    .iter()
                    .filter(|value| !value.is_null())
                    .map(|value| category(value, path, "breaks"))
                    .collect::<Result<_>>()?,
            );
        }
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
                    other => Err(Error::spec(
                        path,
                        format!("discrete limits are category names (strings), got {other}"),
                    )),
                })
                .collect::<Result<Vec<_>>>()?;
            if let Some((_, dendrogram_path)) = &self.dendrogram {
                return Err(Error::spec(
                    path,
                    format!(
                        "limits conflict with the dendrogram at '{dendrogram_path}', which \
                         orders this axis; drop the limits (the tree's leaves are the levels)"
                    ),
                ));
            }
            scale.set_limits(categories);
        }
        if let Some(dendrogram) = self.axis_dendrogram()? {
            scale.set_limits(dendrogram.ordered_levels());
            scale.dendrogram = Some(std::sync::Arc::new(dendrogram));
        }
        scale.reverse =
            self.reverse.as_ref().is_some_and(|(reverse, _)| *reverse) || self.away_from_panel;
        Ok(scale)
    }

    /// The `guide_axis_mark()` options, checked.
    fn axis_mark(&self) -> Result<Option<AxisMark>> {
        self.mark_guide
            .as_ref()
            .map(|(options, path)| AxisMark::parse(options, path))
            .transpose()
    }

    /// The `guide_axis_nested()` options, checked for a discrete or continuous axis.
    fn axis_nested(
        &self,
        discrete: bool,
    ) -> Result<Option<std::sync::Arc<crate::layout::axis_nested::AxisNested>>> {
        self.nested_guide
            .as_ref()
            .map(|(options, path)| {
                crate::layout::axis_nested::AxisNested::parse(options, path, discrete)
                    .map(std::sync::Arc::new)
            })
            .transpose()
    }

    /// The tree of `scale_*_dendro()` with its guide options, checked.
    fn axis_dendrogram(&self) -> Result<Option<AxisDendrogram>> {
        let Some((value, path)) = &self.dendrogram else {
            return Ok(None);
        };
        let tree = tree_from_value(value, path)?;
        let leaf_levels = self
            .leaf_levels
            .as_ref()
            .map(|(runs, runs_path)| leaf_levels_from_value(runs, runs_path, &tree))
            .transpose()?;
        let mut dendrogram = AxisDendrogram {
            tree,
            side: DendrogramSide::Axis,
            space: DEFAULT_SPACE,
            kind: DendrogramType::Rectangle,
            leaf_levels,
        };
        let Some((options, guide_path)) = &self.dendrogram_guide else {
            return Ok(Some(dendrogram));
        };
        let at = |key: &str| format!("{guide_path}.{key}");
        for (key, value) in options {
            match key.as_str() {
                "space" => match value
                    .as_f64()
                    .filter(|space| space.is_finite() && *space >= 0.0)
                {
                    Some(space) => dendrogram.space = space,
                    None => {
                        return Err(Error::spec(
                            at(key),
                            format!("space is the tree's size in pixels (0 or more), got {value}"),
                        ));
                    }
                },
                "side" => {
                    dendrogram.side = match value.as_str() {
                        Some("axis") => DendrogramSide::Axis,
                        Some("opposite") => DendrogramSide::Opposite,
                        _ => {
                            return Err(Error::spec(
                                at(key),
                                format!("side is 'axis' or 'opposite', got {value}"),
                            ));
                        }
                    }
                }
                "type" => {
                    dendrogram.kind = match value.as_str() {
                        Some("rectangle") => DendrogramType::Rectangle,
                        Some("triangle") => DendrogramType::Triangle,
                        _ => {
                            return Err(Error::spec(
                                at(key),
                                format!("type is 'rectangle' or 'triangle', got {value}"),
                            ));
                        }
                    }
                }
                other => {
                    let known = ["space", "side", "type"];
                    return Err(Error::spec(
                        at(other),
                        format!(
                            "guide_axis_dendro() has no option '{other}'; accepted: {}{}",
                            known.join(", "),
                            crate::suggest::did_you_mean(other, known)
                        ),
                    ));
                }
            }
        }
        Ok(Some(dendrogram))
    }

    /// `lablim=...` of a discrete axis (lets-plot: fractions truncate, 0 and below
    /// mean no limit; continuous axes ignore it, as lets-plot).
    fn label_limit(&self, warnings: &mut Vec<String>) -> Option<usize> {
        let (value, path) = self.label_limit.as_ref()?;
        match value.as_f64().filter(|limit| limit.is_finite()) {
            Some(limit) => (limit.trunc() >= 1.0).then_some(limit.trunc() as usize),
            None => {
                warnings.push(format!(
                    "at '{path}': expected a number of characters (e.g. lablim=10), got \
                     {value}; ignored"
                ));
                None
            }
        }
    }

    /// `break_width=<number>` of a number axis (a time interval string is handled
    /// by [`Self::temporal_axis`]). Zero, negative and non-number widths fail
    /// (lets-plot draws no breaks for them).
    fn number_break_width(&self) -> Result<Option<f64>> {
        match &self.break_width {
            None | Some((Value::String(_), _)) => Ok(None),
            Some((value, path)) => value
                .as_f64()
                .filter(|width| width.is_finite() && *width > 0.0)
                .map(Some)
                .ok_or_else(|| {
                    Error::spec(
                        path,
                        format!(
                            "break_width is the distance between breaks, a positive number \
                             (e.g. break_width=5), got {value}"
                        ),
                    )
                }),
        }
    }

    /// The date-time axis: `scale_*_time()` gives durations, `scale_*_datetime()`
    /// or date-time columns give date-times (dates and times of day keep their
    /// column's kind). `break_width` elsewhere is reported and ignored.
    fn temporal_axis(
        &self,
        column_kind: Option<TemporalKind>,
        warnings: &mut Vec<String>,
    ) -> Result<Option<TemporalAxis>> {
        let kind = match (&self.temporal, column_kind) {
            (Some((TemporalKind::Duration, _)), _) => Some(TemporalKind::Duration),
            (_, Some(kind)) => Some(kind),
            (Some((kind, _)), None) => Some(*kind),
            (None, None) => None,
        };
        let Some(kind) = kind else {
            if let Some((Value::String(_), path)) = &self.break_width {
                warnings.push(format!(
                    "at '{path}': a time interval break_width applies to \
                     scale_*_datetime() and scale_*_time(); ignored"
                ));
            }
            return Ok(None);
        };
        let break_width = match &self.break_width {
            Some((Value::String(width), path)) => {
                Some(TimeInterval::parse(width).map_err(|message| Error::spec(path, message))?)
            }
            Some((other, path)) => {
                return Err(Error::spec(
                    path,
                    format!(
                        "break_width of a date-time axis is a time interval such as \
                         '2 weeks' or '6 hours', got {other}"
                    ),
                ));
            }
            None => None,
        };
        Ok(Some(TemporalAxis {
            kind,
            break_width,
            format: None,
        }))
    }

    /// `format=...`; a pattern ggrust cannot format (datetime) is reported and ignored.
    fn label_format(&self, warnings: &mut Vec<String>) -> Option<LabelPattern> {
        let (pattern, path) = self.format.as_ref()?;
        match LabelPattern::parse(pattern) {
            Ok(pattern) => Some(pattern),
            Err(message) => {
                warnings.push(format!("at '{path}': {message}; ignored"));
                None
            }
        }
    }

    /// `labels=[...]` as texts (numbers are written as given), with their path.
    fn label_texts(&self) -> Result<Option<(Vec<String>, String)>> {
        let Some((labels, path)) = &self.labels else {
            return Ok(None);
        };
        let Value::Array(items) = labels else {
            return Err(Error::spec(
                path,
                format!("expected a list of labels, got {labels}"),
            ));
        };
        let texts = items
            .iter()
            .map(|item| match item {
                Value::String(text) => Ok(text.clone()),
                Value::Number(number) => Ok(number.to_string()),
                Value::Null => Ok(String::new()),
                other => Err(Error::spec(path, format!("labels are texts, got {other}"))),
            })
            .collect::<Result<_>>()?;
        Ok(Some((texts, path.clone())))
    }

    /// `[multiplicative, additive]` from `expand`, each optional.
    fn expansion(&self) -> Result<Option<[Option<f64>; 2]>> {
        let Some((expand, path)) = &self.expand else {
            return Ok(None);
        };
        if expand.len() > 2 || expand.iter().any(|value| !value.is_finite()) {
            return Err(Error::spec(
                path,
                format!(
                    "expected [multiplicative] or [multiplicative, additive] \
                 finite numbers, got {expand:?}"
                ),
            ));
        }
        Ok(Some([expand.first().copied(), expand.get(1).copied()]))
    }
}

/// Read the x and y scale options; options not built yet are reported in `warnings`.
pub fn position_scales(spec: &PlotSpec, warnings: &mut Vec<String>) -> Result<PositionScales> {
    let mut merged: [AxisScaleSpec; 5] = Default::default();
    let mut targeted: Vec<TargetedSideScale> = Vec::new();
    for (index, scale) in spec.scales.iter().enumerate() {
        let path = format!("scales[{index}]");
        let side = side_of_scale(scale, &path)?;
        let side_target = side_target(scale, side, &path)?;
        for aesthetic in scale.aesthetic.as_slice() {
            let target = match (aesthetic, side, &side_target) {
                (Aesthetic::X, None, _) => &mut merged[0],
                (Aesthetic::Y, None, _) => &mut merged[1],
                (Aesthetic::Z, None, _) => &mut merged[4],
                (Aesthetic::Y, Some(true), Some(wanted))
                | (Aesthetic::X, Some(false), Some(wanted)) => {
                    let at = match targeted.iter().position(|scale| scale.target == *wanted) {
                        Some(at) => at,
                        None => {
                            targeted.push(TargetedSideScale {
                                target: wanted.clone(),
                                spec: AxisScaleSpec::default(),
                            });
                            targeted.len() - 1
                        }
                    };
                    &mut targeted[at].spec
                }
                (Aesthetic::Y, Some(true), None) => &mut merged[2],
                (Aesthetic::X, Some(false), None) => &mut merged[3],
                // Style scales: see `style::spec`.
                (other, _, _) if super::style::default_mapper(*other).is_some() => continue,
                (Aesthetic::X | Aesthetic::Y, Some(shares_x), _) => {
                    let (panel, other) = if shares_x { ("x", "y") } else { ("y", "x") };
                    return Err(Error::spec(
                        format!("{path}.aesthetic"),
                        format!(
                            "the {panel} side panel shares the main {panel} axis and has its \
                             own {other} axis only; use scale_{panel}side{other}_*()"
                        ),
                    ));
                }
                (other, _, _) => {
                    warnings.push(format!(
                        "at '{path}': scales for '{other}' are not supported yet; ignored"
                    ));
                    continue;
                }
            };
            merge(target, scale, &path);
            if let Some(position) = &scale.position {
                let at = format!("{path}.position");
                let sides = AxisSides::parse(position, *aesthetic == Aesthetic::X, &at)?;
                target.sides = Some((sides, at));
            }
            if let Some(value) = scale
                .parameters
                .get("sec_axis")
                .filter(|value| !value.is_null())
            {
                let at = format!("{path}.sec_axis");
                let axis = SecondaryAxis::parse(value, &spec.functions, &at)?;
                target.secondary = Some((axis, at));
            }
            report_unbuilt(scale, &path, warnings);
        }
    }
    // `guides(x=guide_axis_mark())`, unless the scale sets its own mark guide.
    for (aesthetic, target) in ["x", "y"].into_iter().zip(merged.iter_mut()) {
        if target.mark_guide.is_none()
            && let Some(options) = spec.guides.get(aesthetic).and_then(mark_options)
        {
            target.mark_guide = Some((options, format!("guides.{aesthetic}")));
        }
        if target.nested_guide.is_none()
            && let Some(options) = spec
                .guides
                .get(aesthetic)
                .and_then(|guide| named_options(guide, "axis_nested"))
        {
            target.nested_guide = Some((options, format!("guides.{aesthetic}")));
        }
    }
    let [x, y, xside_y, yside_x, z] = merged;
    Ok(PositionScales {
        x,
        y,
        z,
        xside_y,
        yside_x,
        targeted,
    })
}

/// The bands a side scale names with `side_pos=` / `side_band=` (`None`: all
/// bands on its axis without a targeted scale, or not a side scale).
fn side_target(scale: &ScaleSpec, side: Option<bool>, path: &str) -> Result<Option<SideTarget>> {
    let given = |value: &Option<Value>| value.clone().filter(|value| !value.is_null());
    let (position, band) = (given(&scale.side_pos), given(&scale.side_band));
    let Some(shares_x) = side else {
        if let Some(key) = [("side_pos", &position), ("side_band", &band)]
            .into_iter()
            .find_map(|(key, value)| value.is_some().then_some(key))
        {
            return Err(Error::spec(
                format!("{path}.{key}"),
                format!("only side band scales (scale_xsidey_*(), scale_ysidex_*()) take {key}"),
            ));
        }
        return Ok(None);
    };
    if position.is_none() && band.is_none() {
        return Ok(None);
    }
    let accepted: [&str; 2] = if shares_x {
        ["top", "bottom"]
    } else {
        ["left", "right"]
    };
    let far = match &position {
        None => None,
        Some(value) => {
            let text = value.as_str().unwrap_or_default();
            if !accepted.contains(&text) {
                let panel = if shares_x { "xsidey" } else { "ysidex" };
                return Err(Error::spec(
                    format!("{path}.side_pos"),
                    format!(
                        "a scale_{panel}_*() band goes '{}' or '{}', got {value}{}",
                        accepted[0],
                        accepted[1],
                        crate::suggest::did_you_mean(text, accepted.iter().copied())
                    ),
                ));
            }
            Some(text == "top" || text == "right")
        }
    };
    let name = match &band {
        None => None,
        Some(Value::String(text)) if !text.is_empty() => Some(text.clone()),
        Some(value) => {
            return Err(Error::spec(
                format!("{path}.side_band"),
                format!("expected the band's name as a non-empty string, got {value}"),
            ));
        }
    };
    Ok(Some(SideTarget {
        shares_x,
        far,
        name,
    }))
}

/// The side panel a scale belongs to (`"ggside"`: `Some(true)` the x side panel,
/// `Some(false)` the y one), `None` for the main panel.
fn side_of_scale(scale: &ScaleSpec, path: &str) -> Result<Option<bool>> {
    match &scale.ggside {
        None | Some(Value::Null) => Ok(None),
        Some(value) => match value.as_str() {
            Some("x") => Ok(Some(true)),
            Some("y") => Ok(Some(false)),
            _ => Err(Error::spec(
                format!("{path}.ggside"),
                format!("expected 'x' (scale_xsidey_*()) or 'y' (scale_ysidex_*()), got {value}"),
            )),
        },
    }
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
    if let Some(breaks) = &scale.breaks {
        target.breaks = Some((breaks.clone(), at("breaks")));
    }
    if let Some(labels) = &scale.labels {
        target.labels = Some((labels.clone(), at("labels")));
    }
    if let Some(format) = &scale.format {
        target.format = Some((format.clone(), at("format")));
    }
    if let Some(na_value) = scale.na_value.as_ref().filter(|value| !value.is_null()) {
        target.na_value = Some((na_value.clone(), at("na_value")));
    }
    for (key, kind) in [
        ("datetime", TemporalKind::DateTime),
        ("time", TemporalKind::Duration),
    ] {
        if scale.parameters.get(key) == Some(&Value::Bool(true)) {
            target.temporal = Some((kind, at(key)));
        }
    }
    if let Some(limit) = scale
        .parameters
        .get("lablim")
        .filter(|value| !value.is_null())
    {
        target.label_limit = Some((limit.clone(), at("lablim")));
    }
    if let Some(width) = scale.parameters.get("break_width")
        && !width.is_null()
    {
        target.break_width = Some((width.clone(), at("break_width")));
    }
    if let Some(tree) = scale.parameters.get("dendrogram") {
        // The tree and its runs are one unit: a later tree drops earlier runs.
        target.dendrogram = Some((tree.clone(), at("dendrogram")));
        target.leaf_levels = scale
            .parameters
            .get("leaf_levels")
            .map(|runs| (runs.clone(), at("leaf_levels")));
    }
    if let Some(options) = dendrogram_guide(scale) {
        target.dendrogram_guide = Some((options, at("guide")));
    }
    if let Some(options) = mark_guide(scale) {
        target.mark_guide = Some((options, at("guide")));
    }
    if let Some(options) = scale
        .guide
        .as_ref()
        .and_then(|guide| named_options(guide, "axis_nested"))
    {
        target.nested_guide = Some((options, at("guide")));
    }
}

/// The options of a `guide_axis_mark()` (or `guide="axis_mark"`) on `scale`.
fn mark_guide(scale: &ScaleSpec) -> Option<Options> {
    mark_options(scale.guide.as_ref()?)
}

/// The options of `guide` when it is a `guide_axis_mark()` (or `"axis_mark"`).
fn mark_options(guide: &GuideSpec) -> Option<Options> {
    match guide {
        GuideSpec::Name(name) if name == "axis_mark" => Some(Options::new()),
        GuideSpec::Full(options) if options.name.as_deref() == Some("axis_mark") => {
            Some(options.unsupported.clone())
        }
        _ => None,
    }
}

/// The options of `guide` when it is the guide `name` (`guide_axis_<name>()`).
fn named_options(guide: &GuideSpec, name: &str) -> Option<Options> {
    match guide {
        GuideSpec::Name(given) if given == name => Some(Options::new()),
        GuideSpec::Full(options) if options.name.as_deref() == Some(name) => {
            Some(options.unsupported.clone())
        }
        _ => None,
    }
}

/// The options of a `guide_axis_dendro()` (or `guide="axis_dendro"`) on `scale`.
fn dendrogram_guide(scale: &ScaleSpec) -> Option<Options> {
    match scale.guide.as_ref()? {
        GuideSpec::Name(name) if name == "axis_dendro" => Some(Options::new()),
        GuideSpec::Full(options) if options.name.as_deref() == Some("axis_dendro") => {
            Some(options.unsupported.clone())
        }
        _ => None,
    }
}

/// The runs of `scale_*_dendro(leaf_levels={leaf: [levels]})`, indexed like
/// `tree.labels`: every leaf has a non-empty run, and no level is in two runs
/// (or twice in one).
fn leaf_levels_from_value(value: &Value, path: &str, tree: &Tree) -> Result<Vec<Vec<String>>> {
    let Some(entries) = value.as_object() else {
        return Err(Error::spec(
            path,
            format!("leaf_levels is a mapping {{leaf: [levels]}}, got {value}"),
        ));
    };
    let leaf_index: std::collections::HashMap<&str, usize> = tree
        .labels
        .iter()
        .enumerate()
        .map(|(index, label)| (label.as_str(), index))
        .collect();
    let mut runs: Vec<Option<Vec<String>>> = vec![None; tree.labels.len()];
    let mut owner: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (leaf, levels) in entries {
        let at = format!("{path}.{leaf}");
        let Some(&index) = leaf_index.get(leaf.as_str()) else {
            return Err(Error::spec(
                &at,
                format!(
                    "'{leaf}' is not a leaf of the tree; leaf_levels keys are the tree's leaf \
                     names{}",
                    crate::suggest::did_you_mean(leaf, tree.labels.iter().map(String::as_str))
                ),
            ));
        };
        let run = levels
            .as_array()
            .and_then(|items| {
                items
                    .iter()
                    .map(|item| item.as_str().map(str::to_owned))
                    .collect::<Option<Vec<_>>>()
            })
            .ok_or_else(|| {
                Error::spec(
                    &at,
                    format!("the levels of leaf '{leaf}' are a list of strings, got {levels}"),
                )
            })?;
        if run.is_empty() {
            return Err(Error::spec(
                &at,
                format!("leaf '{leaf}' has no levels; give it at least one, or drop the leaf"),
            ));
        }
        for level in &run {
            if let Some(previous) = owner.insert(level.clone(), leaf.clone()) {
                let place = if previous == *leaf {
                    format!("twice in the run of leaf '{leaf}'")
                } else {
                    format!("in the runs of leaves '{previous}' and '{leaf}'")
                };
                return Err(Error::spec(
                    &at,
                    format!("level '{level}' is {place}; each level belongs to one leaf, once"),
                ));
            }
        }
        runs[index] = Some(run);
    }
    let missing: Vec<String> = tree
        .labels
        .iter()
        .zip(&runs)
        .filter(|(_, run)| run.is_none())
        .map(|(label, _)| format!("'{label}'"))
        .collect();
    if !missing.is_empty() {
        return Err(Error::spec(
            path,
            format!(
                "leaf_levels has no run for the leaves {}; give every leaf of the tree its levels",
                missing.join(", ")
            ),
        ));
    }
    Ok(runs.into_iter().flatten().collect())
}

/// A tree from `{"labels": [...], "merge": [[a, b], ...], "height": [...],
/// "order": [...]}` (what `ggrust.hclust()` writes).
fn tree_from_value(value: &Value, path: &str) -> Result<Tree> {
    let field = |key: &str| {
        value.get(key).ok_or_else(|| {
            Error::spec(
                path,
                format!(
                    "a dendrogram needs 'labels', 'merge', 'height' and 'order'; '{key}' is \
                     missing; build it with ggrust.hclust()"
                ),
            )
        })
    };
    let wrong = |key: &str, expected: &str| {
        Error::spec(
            format!("{path}.{key}"),
            format!("expected {expected}; build the tree with ggrust.hclust()"),
        )
    };
    let labels = field("labels")?
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|item| item.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
        })
        .ok_or_else(|| wrong("labels", "a list of leaf names (strings)"))?;
    let merge = field("merge")?
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .map(|row| match row.as_array().map(Vec::as_slice) {
                    Some([first, second]) => Some([first.as_i64()?, second.as_i64()?]),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
        })
        .ok_or_else(|| wrong("merge", "a list of [a, b] integer pairs"))?;
    let height = field("height")?
        .as_array()
        .and_then(|items| items.iter().map(Value::as_f64).collect::<Option<Vec<_>>>())
        .ok_or_else(|| {
            wrong(
                "height",
                "a list of finite numbers (NaN and infinity arrive as null)",
            )
        })?;
    let order = field("order")?
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|item| item.as_u64().map(|leaf| leaf as usize))
                .collect::<Option<Vec<_>>>()
        })
        .ok_or_else(|| wrong("order", "a list of 0-based leaf indices"))?;
    let mut seen = std::collections::HashSet::new();
    if let Some(label) = labels.iter().find(|label| !seen.insert(label.as_str())) {
        return Err(Error::spec(
            format!("{path}.labels"),
            format!("leaf '{label}' appears twice; leaves name distinct axis levels"),
        ));
    }
    Tree::from_spec(path, labels, merge, height, order)
}

/// A category name in `breaks` of a discrete axis; numbers are written as given.
fn category(value: &Value, path: &str, key: &str) -> Result<String> {
    match value {
        Value::String(text) => Ok(text.clone()),
        Value::Number(number) => Ok(number.to_string()),
        Value::Bool(flag) => Ok(flag.to_string()),
        other => Err(Error::spec(
            path,
            format!("{key} of a discrete axis are category names, got {other}"),
        )),
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
                Error::spec(path, format!("limits are numbers or null, got {value}"))
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
    Ok(if transform.is_decreasing() {
        [high, low]
    } else {
        [low, high]
    })
}

/// Warn about scale options that are parsed but not built yet.
fn report_unbuilt(scale: &ScaleSpec, path: &str, warnings: &mut Vec<String>) {
    let present = [
        (
            "guide",
            scale.guide.is_some()
                && dendrogram_guide(scale).is_none()
                && mark_guide(scale).is_none()
                && scale
                    .guide
                    .as_ref()
                    .and_then(|guide| named_options(guide, "axis_nested"))
                    .is_none(),
        ),
        ("scale_mapper_kind", scale.scale_mapper_kind.is_some()),
    ];
    for (key, _) in present.iter().filter(|(_, present)| *present) {
        warnings.push(format!("at '{path}.{key}': not supported yet; ignored"));
    }
    let built = [
        "datetime",
        "time",
        "break_width",
        "lablim",
        "dendrogram",
        "leaf_levels",
        "sec_axis",
    ];
    for key in scale
        .parameters
        .keys()
        .filter(|key| !built.contains(&key.as_str()))
    {
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

    const TREE: &str = r#"{"labels": ["a", "b", "c"], "merge": [[-1, -2], [-3, 1]],
        "height": [1, 2], "order": [2, 0, 1]}"#;

    fn dendro_scale(leaf_levels: &str) -> Result<PositionScale> {
        let json = plot_with(&format!(
            r#"{{"aesthetic": "y", "discrete": true, "dendrogram": {TREE},
                 "leaf_levels": {leaf_levels}}}"#
        ));
        let (scales, warnings) = scales(&json)?;
        assert!(warnings.is_empty(), "{warnings:?}");
        built(&scales.y, true)
    }

    #[test]
    fn leaf_levels_put_runs_in_tree_order_and_keep_their_order() {
        let PositionScale::Discrete(scale) =
            dendro_scale(r#"{"a": ["a2", "a1"], "b": ["b1"], "c": ["c1", "c2", "c3"]}"#).unwrap()
        else {
            panic!("expected a discrete scale");
        };
        assert_eq!(
            scale.effective_levels(),
            ["c1", "c2", "c3", "a2", "a1", "b1"]
        );
        let dendrogram = scale.dendrogram.as_deref().unwrap();
        assert_eq!(dendrogram.level_set().len(), 6);
    }

    #[test]
    fn leaf_levels_are_checked() {
        let error = |runs: &str| dendro_scale(runs).err().unwrap().to_string();
        let message = error(r#"{"a": ["x"], "b": ["y"], "cc": ["z"]}"#);
        assert!(message.contains("'cc' is not a leaf"), "{message}");
        let message = error(r#"{"a": ["x"]}"#);
        assert!(
            message.contains("no run for the leaves 'b', 'c'"),
            "{message}"
        );
        let message = error(r#"{"a": ["x"], "b": [], "c": ["z"]}"#);
        assert!(message.contains("leaf 'b' has no levels"), "{message}");
        let message = error(r#"{"a": ["x"], "b": ["x"], "c": ["z"]}"#);
        assert!(
            message.contains("in the runs of leaves 'a' and 'b'"),
            "{message}"
        );
        let message = error(r#"{"a": ["x", "x"], "b": ["y"], "c": ["z"]}"#);
        assert!(
            message.contains("twice in the run of leaf 'a'"),
            "{message}"
        );
        let message = error(r#"{"a": [1], "b": ["y"], "c": ["z"]}"#);
        assert!(message.contains("a list of strings"), "{message}");
        assert!(message.contains("leaf_levels.a"), "{message}");
        let message = error(r#"["a"]"#);
        assert!(message.contains("a mapping"), "{message}");
    }

    #[test]
    fn side_scales_target_bands_by_side_and_name() {
        let json = plot_with(
            r#"{"aesthetic": "x", "ggside": "y", "expand": [0, 1]},
               {"aesthetic": "x", "ggside": "y", "side_pos": "left", "expand": [0, 2]},
               {"aesthetic": "x", "ggside": "y", "side_band": "score", "expand": [0, 3]},
               {"aesthetic": "x", "ggside": "y", "side_pos": "left", "side_band": "score",
                "expand": [0, 4]},
               {"aesthetic": "x", "ggside": "y", "side_pos": "left", "name": "later"}"#,
        );
        let (scales, warnings) = scales(&json).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(scales.targeted.len(), 3);
        let expand = |spec: &AxisScaleSpec| spec.expand.as_ref().map(|(value, _)| value[1]);
        assert_eq!(expand(&scales.yside_x), Some(1.0));
        // Same target merges key by key.
        let left = scales.side_spec(false, false, None).unwrap();
        assert_eq!(
            (expand(left), left.name.as_deref()),
            (Some(2.0), Some("later"))
        );
        assert_eq!(
            expand(scales.side_spec(false, false, Some("score")).unwrap()),
            Some(4.0)
        );
        assert_eq!(
            expand(scales.side_spec(false, true, Some("score")).unwrap()),
            Some(3.0)
        );
        // A right band without a name, or an x side band: no targeted scale.
        assert!(scales.side_spec(false, true, None).is_none());
        assert!(scales.side_spec(true, true, None).is_none());
    }

    #[test]
    fn side_scale_targets_are_checked() {
        let error = |scale: &str| scales(&plot_with(scale)).err().unwrap().to_string();
        let message = error(r#"{"aesthetic": "y", "ggside": "x", "side_pos": "left"}"#);
        assert!(
            message.contains("scales[0].side_pos") && message.contains("'top'"),
            "{message}"
        );
        let message = error(r#"{"aesthetic": "x", "side_band": "a"}"#);
        assert!(
            message.contains("scales[0].side_band") && message.contains("scale_xsidey_*()"),
            "{message}"
        );
        let message = error(r#"{"aesthetic": "x", "ggside": "y", "side_band": 3}"#);
        assert!(message.contains("non-empty string"), "{message}");
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
                r#"{"aesthetic": "x", "trans": "lg"}"#,
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
                {"aesthetic": "x", "scale_mapper_kind": "identity", "discrete": true,
                 "reverse": false},
                {"aesthetic": "weight"}]}"#,
        )
        .unwrap();
        assert_eq!(
            warnings,
            [
                "at 'scales[0].scale_mapper_kind': not supported yet; ignored",
                "at 'scales[1]': scales for 'weight' are not supported yet; ignored",
            ]
        );
    }

    #[test]
    fn position_picks_the_axis_sides_per_aesthetic() {
        let sides = |json: &str| {
            let (scales, warnings) = scales(json).unwrap();
            assert!(warnings.is_empty(), "{warnings:?}");
            (scales.x.sides(), scales.y.sides())
        };
        let (x, y) = sides(
            r#"{"kind": "plot", "scales": [
                {"aesthetic": "x", "position": "top"},
                {"aesthetic": "y", "position": "both"}]}"#,
        );
        assert_eq!((x.primary, x.secondary), (false, true));
        assert_eq!((y.primary, y.secondary), (true, true));
        let (x, y) =
            sides(r#"{"kind": "plot", "scales": [{"aesthetic": "y", "position": "right"}]}"#);
        assert_eq!(x, AxisSides::default());
        assert_eq!((y.primary, y.secondary), (false, true));
        // Built scales carry the sides, continuous or discrete.
        let (scales, _) =
            scales(r#"{"kind": "plot", "scales": [{"aesthetic": "x", "position": "top"}]}"#)
                .unwrap();
        for discrete in [false, true] {
            assert!(built(&scales.x, discrete).unwrap().sides().secondary);
        }
    }

    #[test]
    fn a_position_of_the_other_axis_is_an_error() {
        let error =
            scales(r#"{"kind": "plot", "scales": [{"aesthetic": "x", "position": "rigth"}]}"#)
                .unwrap_err()
                .to_string();
        assert!(error.contains("at 'scales[0].position'"), "{error}");
        assert!(error.contains("'bottom', 'top', 'both'"), "{error}");
        let error =
            scales(r#"{"kind": "plot", "scales": [{"aesthetic": "y", "position": "rigth"}]}"#)
                .unwrap_err()
                .to_string();
        assert!(error.contains("did you mean 'right'?"), "{error}");
    }
}
