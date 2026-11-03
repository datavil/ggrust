//! Style scales from `scale_color_*()`, `scale_fill_*()`, ... calls.
//!
//! Several calls for one aesthetic merge key by key, later ones winning, as in
//! lets-plot (`PlotConfigUtil.mergeScaleOptions`), except that a call picking a
//! mapper (palette or `values`) replaces the earlier mapper. The merged options choose the
//! mapper (lets-plot `ScaleConfig.createMapperProvider`) and the discrete domain.

use crate::scale::style::LinetypeMapper;
use crate::style::LineType;
use std::collections::BTreeMap;
use std::sync::Arc;

use super::{
    ALPHA, BrewerMapper, COLORMAP_NAMES, ColormapMapper, ContinuousDomain, DiscreteDomain,
    GRADIENT_HIGH, GRADIENT_LOW, GRADIENT2_HIGH, GRADIENT2_LOW, GRADIENT2_MID, Gradient2Mapper,
    GradientMapper, GreyMapper, HOLLOW_SHAPES, HueMapper, IdentityMapper, Level, ManualColorMapper,
    ManualNumberMapper, NA_COLOR, NA_SHAPE, Oob, Overflow, PaletteChoice, PaletteType,
    ProportionalMapper, SIZE, SIZE_AREA_MAX, ShapeMapper, StyleMapper, colormap, default_mapper,
    scheme_by_name, scheme_names,
};
use crate::scale::Transform;
use crate::scale::label_pattern::LabelPattern;
use crate::scene::{Color, PerItem};
use crate::spec::{Aesthetic, GuideSpec, PlotSpec, ScaleSpec, Value};
use crate::style::{StyleValues, ValueKind, parse_constant};
use crate::{Error, Result};

/// One merged option with the path of the call that set it.
type Set<T> = Option<(T, String)>;

/// Mapper options read here; any other `parameters` key is reported.
const MAPPER_OPTIONS: &[&str] = &[
    "values",
    "h",
    "c",
    "l",
    "h_start",
    "direction",
    "low",
    "mid",
    "high",
    "midpoint",
    "colors",
    "alpha",
    "begin",
    "start",
    "end",
    "option",
    "type",
    "palette",
    "overflow",
    "solid",
    "range",
    "max_size",
];

/// The `parameters` key of `lablim=...`.
const LABEL_LIMIT: &str = "lablim";

/// The `parameters` key of `oob=...` (ggplot2; lets-plot has no such option).
const OOB: &str = "oob";

/// Merged `scale_*()` options of one style aesthetic.
#[derive(Debug, Clone, Default)]
pub struct StyleScaleSpec {
    /// `name=...`: the legend title.
    pub name: Option<String>,
    breaks: Set<Vec<Value>>,
    labels: Set<Value>,
    limits: Set<Vec<Value>>,
    na_value: Set<Value>,
    kind: Set<String>,
    /// `discrete=True` (`scale_color_discrete()`).
    discrete: bool,
    trans: Set<String>,
    guide: Set<GuideSpec>,
    /// `format=...`, parsed (an unsupported pattern is reported and dropped).
    format: Option<LabelPattern>,
    /// `lablim=...`: discrete labels longer than this are cut (0: no limit).
    label_limit: Option<usize>,
    /// `oob=...`, checked by [`StyleScaleSpec::oob`]. Merges like `limits`.
    oob: Set<Value>,
    options: BTreeMap<String, (Value, String)>,
}

/// Merge the `scale_*()` calls of every style aesthetic (those with a default
/// mapper: color, fill, size, alpha). Options not built yet are reported.
pub fn style_scale_specs(
    spec: &PlotSpec,
    warnings: &mut Vec<String>,
) -> BTreeMap<Aesthetic, StyleScaleSpec> {
    let mut merged: BTreeMap<Aesthetic, StyleScaleSpec> = BTreeMap::new();
    for (index, scale) in spec.scales.iter().enumerate() {
        let path = format!("scales[{index}]");
        for &aesthetic in scale.aesthetic.as_slice() {
            if default_mapper(aesthetic).is_none() {
                continue; // position scales, or reported by `position_scales`
            }
            merge(merged.entry(aesthetic).or_default(), scale, &path, warnings);
            report_unbuilt(scale, &path, warnings);
        }
    }
    merged
}

fn merge(target: &mut StyleScaleSpec, scale: &ScaleSpec, path: &str, warnings: &mut Vec<String>) {
    let at = |key: &str| format!("{path}.{key}");
    // A call that picks a mapper (a palette scale, manual `values`, or a size or
    // alpha `range`) replaces the mapper of earlier calls, its `na_value` included,
    // instead of merging into it, as in ggplot2. lets-plot merges, so
    // `scale_color_brewer() + scale_color_manual(values)` keeps brewer there
    // (deviations.md). Names, breaks, labels, limits and transforms still merge.
    let picks_mapper = scale.scale_mapper_kind.is_some()
        || ["values", "range"].iter().any(|key| {
            scale
                .parameters
                .get(*key)
                .is_some_and(|value| !value.is_null())
        });
    if picks_mapper {
        target.kind = None;
        target.options.clear();
        target.na_value = None;
    }
    if let Some(format) = &scale.format {
        // A later call wins even when its pattern is not supported (lets-plot).
        target.format = LabelPattern::parse(format)
            .map_err(|message| warnings.push(format!("at '{}': {message}; ignored", at("format"))))
            .ok();
    }
    match scale.parameters.get(LABEL_LIMIT) {
        None | Some(Value::Null) => {}
        // lets-plot truncates fractions (2.5 -> 2); 0 and below mean no limit.
        Some(Value::Number(number)) if number.as_f64().is_some_and(f64::is_finite) => {
            let limit = number.as_f64().unwrap_or_default().trunc();
            target.label_limit = (limit >= 1.0).then_some(limit as usize);
        }
        Some(other) => warnings.push(format!(
            "at '{}': expected a number of characters (e.g. lablim=10), got {other}; ignored",
            at(LABEL_LIMIT)
        )),
    }
    if let Some(name) = &scale.name {
        target.name = Some(name.clone());
    }
    if let Some(breaks) = &scale.breaks {
        target.breaks = Some((breaks.clone(), at("breaks")));
    }
    if let Some(labels) = &scale.labels {
        target.labels = Some((labels.clone(), at("labels")));
    }
    if let Some(limits) = &scale.limits {
        target.limits = Some((limits.clone(), at("limits")));
    }
    if let Some(oob) = scale.parameters.get(OOB).filter(|value| !value.is_null()) {
        target.oob = Some((oob.clone(), at(OOB)));
    }
    if let Some(na_value) = &scale.na_value {
        target.na_value = Some((na_value.clone(), at("na_value")));
    }
    if let Some(guide) = &scale.guide {
        target.guide = Some((guide.clone(), at("guide")));
    }
    if let Some(kind) = &scale.scale_mapper_kind {
        target.kind = Some((kind.clone(), at("scale_mapper_kind")));
    }
    if let Some(discrete) = scale.discrete {
        target.discrete = discrete;
    }
    if let Some(trans) = &scale.trans {
        target.trans = Some((trans.clone(), at("trans")));
    }
    for (key, value) in &scale.parameters {
        if MAPPER_OPTIONS.contains(&key.as_str()) && !value.is_null() {
            target.options.insert(key.clone(), (value.clone(), at(key)));
        }
    }
}

/// Warn about options of a style scale that are parsed but not built yet.
fn report_unbuilt(scale: &ScaleSpec, path: &str, warnings: &mut Vec<String>) {
    let present = [
        (
            "guide",
            scale
                .guide
                .as_ref()
                .is_some_and(|guide| !guide.is_built_style_guide()),
        ),
        ("expand", scale.expand.is_some()),
        ("position", scale.position.is_some()),
        ("reverse", scale.reverse.is_some()),
    ];
    for (key, _) in present.iter().filter(|(_, present)| *present) {
        warnings.push(format!("at '{path}.{key}': not supported yet; ignored"));
    }
    for (key, value) in &scale.parameters {
        if !MAPPER_OPTIONS.contains(&key.as_str())
            && key != LABEL_LIMIT
            && key != OOB
            && !value.is_null()
        {
            warnings.push(format!("at '{path}.{key}': not supported yet; ignored"));
        }
    }
}

impl StyleScaleSpec {
    /// The mapper the options ask for: `scale_mapper_kind` (lets-plot lets it win
    /// over `values`), then manual `values`, then the aesthetic's default. `na_value`
    /// applies to the first two only, as in lets-plot.
    pub fn mapper(
        &self,
        aesthetic: Aesthetic,
        warnings: &mut Vec<String>,
    ) -> Result<Arc<dyn StyleMapper>> {
        let Some(default) = default_mapper(aesthetic) else {
            return Err(Error::spec("", format!("'{aesthetic}' has no style scale")));
        };
        let kind = default.value_kind();
        let is_color = kind == ValueKind::Color;
        let is_shape = kind == ValueKind::Shape;
        let is_number = kind == ValueKind::Number;
        // lets-plot: a discrete color scale without values defaults to brewer
        // (Set1), which reads `direction`.
        let default_kind = (self.discrete && is_color && !self.options.contains_key("values"))
            .then(|| ("color_brewer".to_owned(), String::new()));
        match self.kind.as_ref().or(default_kind.as_ref()) {
            Some((name, _)) if name == "identity" => {
                return Ok(Arc::new(IdentityMapper {
                    kind,
                    na: self.na_value(aesthetic, &default)?,
                }));
            }
            Some((name, path)) if name == "size_area" => {
                let max = match self.options.get("max_size") {
                    None => SIZE_AREA_MAX,
                    Some((value, path)) => positive_number(value, path)?,
                };
                if aesthetic != Aesthetic::Size {
                    return Err(Error::spec(
                        path,
                        format!("size_area applies to 'size', not '{aesthetic}'"),
                    ));
                }
                return Ok(Arc::new(ProportionalMapper {
                    max,
                    missing: self.na_number(SIZE.missing())?,
                }));
            }
            Some((kind, _)) if kind == "color_hue" && is_color => {
                return self
                    .hue()
                    .map(|mapper| Arc::new(mapper) as Arc<dyn StyleMapper>);
            }
            Some((kind, path)) if is_color && kind.starts_with("color_") => {
                if let Some(mapper) = self.continuous_color(kind, path)? {
                    return Ok(mapper);
                }
                warnings.push(format!(
                    "at '{path}': '{kind}' scales for '{aesthetic}' are not supported yet; the \
                     default scale was used"
                ));
            }
            Some((kind, path)) => warnings.push(format!(
                "at '{path}': '{kind}' scales for '{aesthetic}' are not supported yet; the \
                 default scale was used"
            )),
            None => {}
        }
        if let Some((values, path)) = self.options.get("values") {
            if is_color {
                return self
                    .manual_colors(values, path)
                    .map(|mapper| Arc::new(mapper) as Arc<dyn StyleMapper>);
            }
            if is_shape {
                return self
                    .manual_shapes(values, path)
                    .map(|mapper| Arc::new(mapper) as Arc<dyn StyleMapper>);
            }
            if kind == ValueKind::Linetype {
                return self
                    .manual_linetypes(values, path)
                    .map(|mapper| Arc::new(mapper) as Arc<dyn StyleMapper>);
            }
            if is_number {
                return self
                    .manual_numbers(aesthetic, values, path)
                    .map(|mapper| Arc::new(mapper) as Arc<dyn StyleMapper>);
            }
            warnings.push(format!(
                "at '{path}': manual values for '{aesthetic}' are not supported yet; the \
                 default scale was used"
            ));
        }
        if let Some((range, path)) = self.options.get("range") {
            let linear = match aesthetic {
                Aesthetic::Size => &SIZE,
                Aesthetic::Alpha => &ALPHA,
                _ => {
                    return Err(Error::spec(
                        path,
                        format!("range applies to size and alpha scales, not '{aesthetic}'"),
                    ));
                }
            };
            let ends = range
                .as_array()
                .filter(|items| items.len() == 2)
                .and_then(|items| Some((items[0].as_f64()?, items[1].as_f64()?)))
                .filter(|(low, high)| low.is_finite() && high.is_finite())
                .ok_or_else(|| {
                    Error::spec(path, format!("expected [low, high] numbers, got {range}"))
                })?;
            // lets-plot `DoubleSpan` sorts the ends: [5, 1] is 1..5.
            let ends = (ends.0.min(ends.1), ends.0.max(ends.1));
            return Ok(Arc::new(
                linear.with_range(ends, self.na_number(linear.missing())?),
            ));
        }
        if is_shape && let Some((solid, path)) = self.options.get("solid") {
            match solid {
                Value::Bool(false) => {
                    return Ok(Arc::new(ShapeMapper {
                        shapes: HOLLOW_SHAPES.to_vec(),
                        na: NA_SHAPE,
                    }));
                }
                Value::Bool(true) => {}
                other => {
                    return Err(Error::spec(
                        path,
                        format!("expected true or false, got {other}"),
                    ));
                }
            }
        }
        if let Some((_, path)) = &self.na_value {
            // lets-plot passes `na_value` only to scales built from options (values,
            // a mapper kind); the default mappers keep their own.
            warnings.push(format!(
                "at '{path}': na_value applies to hue and manual scales; ignored for the \
                 default '{aesthetic}' scale (as in lets-plot)"
            ));
        }
        Ok(default)
    }

    /// The scale's own `guide` and its path, if given.
    pub fn guide_with_path(&self) -> Option<(&GuideSpec, &str)> {
        self.guide
            .as_ref()
            .map(|(guide, path)| (guide, path.as_str()))
    }

    /// `breaks`, `limits` and `labels` for a discrete scale.
    pub fn discrete_domain(&self) -> Result<DiscreteDomain> {
        let levels = |set: &Set<Vec<Value>>| -> Option<Vec<Level>> {
            set.as_ref().map(|(values, _)| {
                let mut levels: Vec<Level> = Vec::new();
                for level in values.iter().filter_map(Level::from_value) {
                    if !levels.contains(&level) {
                        levels.push(level);
                    }
                }
                levels
            })
        };
        let labels = match &self.labels {
            None => None,
            Some((Value::Array(items), path)) => Some(
                items
                    .iter()
                    .map(|item| match item {
                        Value::String(text) => Ok(text.clone()),
                        Value::Number(number) => Ok(number.to_string()),
                        Value::Null => Ok(String::new()),
                        other => Err(Error::spec(path, format!("labels are texts, got {other}"))),
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
            Some((other, path)) => {
                return Err(Error::spec(
                    path,
                    format!("expected a list of labels, got {other}"),
                ));
            }
        };
        Ok(DiscreteDomain {
            breaks: levels(&self.breaks),
            limits: levels(&self.limits).filter(|limits| !limits.is_empty()),
            labels,
            format: self.format.clone(),
            label_limit: self.label_limit.filter(|&limit| limit > 0),
        })
    }

    /// `limits`, `breaks` and `labels` for a continuous scale. An error (as a
    /// message with its path) when they are not numbers; it is reported only if the
    /// scale turns out continuous.
    pub fn continuous_domain(&self) -> std::result::Result<ContinuousDomain, String> {
        let number = |value: &Value, path: &str| -> std::result::Result<Option<f64>, String> {
            match value {
                Value::Null => Ok(None),
                Value::Number(number) => Ok(number.as_f64().filter(|value| value.is_finite())),
                other => Err(format!(
                    "at '{path}': limits and breaks of a continuous scale are numbers, got {other}"
                )),
            }
        };
        let mut domain = ContinuousDomain::default();
        if let Some((limits, path)) = &self.limits {
            let mut ends = [
                limits
                    .first()
                    .map_or(Ok(None), |value| number(value, path))?,
                limits
                    .get(1)
                    .map_or(Ok(None), |value| number(value, path))?,
            ];
            if let [Some(low), Some(high)] = ends
                && low > high
            {
                ends = [Some(high), Some(low)];
            }
            domain.limits = ends;
        }
        if let Some((breaks, path)) = &self.breaks {
            let mut values = Vec::new();
            for value in breaks {
                if let Some(value) = number(value, path)? {
                    values.push(value);
                }
            }
            domain.breaks = Some(values);
        }
        domain.format = self.format.clone();
        domain.oob = self.oob().map_err(|error| error.to_string())?;
        domain.labels = self
            .discrete_domain()
            .map_err(|error| error.to_string())?
            .labels;
        if let Some((name, path)) = &self.trans {
            domain.transform = Transform::parse(name, path).map_err(|error| error.to_string())?;
        }
        Ok(domain)
    }

    /// `oob=...`: `censor` (the default) or `squish`.
    pub fn oob(&self) -> Result<Oob> {
        let Some((value, path)) = &self.oob else {
            return Ok(Oob::Censor);
        };
        let accepted = format!("use {}", Oob::NAMES);
        match value {
            Value::String(name) => Oob::parse(name).ok_or_else(|| {
                Error::spec(
                    path,
                    format!(
                        "unknown oob \"{name}\"; {accepted}{}",
                        crate::suggest::did_you_mean(name, crate::suggest::quoted(Oob::NAMES))
                    ),
                )
            }),
            other => Err(Error::spec(
                path,
                format!("expected an oob name, got {other}; {accepted}"),
            )),
        }
    }

    /// Gradient, gradient2, gradientn and color map scales; `None` for other kinds.
    fn continuous_color(&self, kind: &str, path: &str) -> Result<Option<Arc<dyn StyleMapper>>> {
        let na = self.na_color()?;
        let color_option = |key: &str, default: Color| -> Result<Color> {
            match self.options.get(key) {
                None => Ok(default),
                Some((value, path)) => color(value, path),
            }
        };
        let mapper: Arc<dyn StyleMapper> = match kind {
            "color_gradient" => Arc::new(GradientMapper {
                name: "color_gradient",
                colors: vec![
                    color_option("low", GRADIENT_LOW)?,
                    color_option("high", GRADIENT_HIGH)?,
                ],
                na,
                opacity: 1.0,
            }),
            "color_gradient2" => Arc::new(Gradient2Mapper {
                low: color_option("low", GRADIENT2_LOW)?,
                mid: color_option("mid", GRADIENT2_MID)?,
                high: color_option("high", GRADIENT2_HIGH)?,
                midpoint: self.number("midpoint")?.unwrap_or(0.0),
                na,
            }),
            "color_gradientn" => {
                let Some((value, colors_path)) = self.options.get("colors") else {
                    return Err(Error::spec(
                        path,
                        "scale_*_gradientn() needs 'colors' (two or more)",
                    ));
                };
                let items = value.as_array().ok_or_else(|| {
                    Error::spec(
                        colors_path,
                        format!("expected a list of colors, got {value}"),
                    )
                })?;
                let colors = items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| color(item, &format!("{colors_path}[{index}]")))
                    .collect::<Result<Vec<_>>>()?;
                if colors.len() < 2 {
                    return Err(Error::spec(
                        colors_path,
                        format!("a gradient needs two or more colors, got {}", colors.len()),
                    ));
                }
                Arc::new(GradientMapper {
                    name: "color_gradientn",
                    colors,
                    na,
                    opacity: 1.0,
                })
            }
            "color_cmap" => {
                let table = match self.options.get("option") {
                    None => colormap("viridis"),
                    Some((Value::String(name), option_path)) => {
                        Some(colormap(name).ok_or_else(|| {
                            Error::spec(
                                option_path,
                                format!(
                                    "unknown color map \"{name}\"; use {COLORMAP_NAMES}{}",
                                    crate::suggest::did_you_mean(
                                        name,
                                        crate::suggest::quoted(COLORMAP_NAMES)
                                    )
                                ),
                            )
                        })?)
                    }
                    Some((other, option_path)) => {
                        return Err(Error::spec(
                            option_path,
                            format!("expected a color map name, got {other}"),
                        ));
                    }
                };
                let fraction = |key: &str, default: f64| -> Result<f64> {
                    let value = self.number(key)?.unwrap_or(default);
                    if (0.0..=1.0).contains(&value) {
                        Ok(value)
                    } else {
                        Err(Error::spec(
                            self.options
                                .get(key)
                                .map_or(path, |(_, path)| path.as_str()),
                            format!("'{key}' should be in range [0..1], got {value}"),
                        ))
                    }
                };
                // lets-plot `DoubleSpan(begin, end)`: the ends are sorted.
                let (begin, end) = (fraction("begin", 0.0)?, fraction("end", 1.0)?);
                let mapper = ColormapMapper {
                    table: table.unwrap_or_default(),
                    opacity: fraction("alpha", 1.0)?,
                    range: (begin.min(end), begin.max(end)),
                    reversed: self
                        .number("direction")?
                        .is_some_and(|direction| direction < 0.0),
                    na,
                };
                if mapper.stop_count() < 2 {
                    return Err(Error::spec(
                        path,
                        format!(
                            "begin ({begin}) and end ({end}) select a single color of \
                         the map; widen the range"
                        ),
                    ));
                }
                Arc::new(mapper)
            }
            "color_brewer" => Arc::new(self.brewer(na)?),
            "color_grey" => {
                let lightness = |key: &str, default: f64| -> Result<f64> {
                    let value = self.number(key)?.unwrap_or(default);
                    if !(0.0..=1.0).contains(&value) {
                        return Err(Error::spec(
                            self.options
                                .get(key)
                                .map_or(path, |(_, path)| path.as_str()),
                            format!("'{key}' should be in range [0..1], got {value}"),
                        ));
                    }
                    Ok(value)
                };
                Arc::new(GreyMapper {
                    start: lightness("start", 0.2)?,
                    end: lightness("end", 0.8)?,
                    na,
                })
            }
            _ => return Ok(None),
        };
        Ok(Some(mapper))
    }

    /// lets-plot `ColorBrewerMapperProvider` options, checked as it does.
    fn brewer(&self, na: Color) -> Result<BrewerMapper> {
        let text = |key: &str| -> Result<Option<(&str, &str)>> {
            match self.options.get(key) {
                None => Ok(None),
                Some((Value::String(text), path)) => Ok(Some((text.as_str(), path.as_str()))),
                Some((other, path)) => {
                    Err(Error::spec(path, format!("expected a text, got {other}")))
                }
            }
        };
        // lets-plot: a palette name picks the family and `type` is never read.
        let named = matches!(self.options.get("palette"), Some((Value::String(_), _)));
        let kind = match if named { None } else { text("type")? } {
            None => None,
            Some((name, path)) => Some(PaletteType::parse(name).ok_or_else(|| {
                Error::spec(
                    path,
                    format!(
                        "palette type expected one of 'seq' (sequential), 'div' \
                     (diverging) or 'qual' (qualitative), got '{name}'"
                    ),
                )
            })?),
        };
        let palette = match self.options.get("palette") {
            None => PaletteChoice::Default,
            Some((Value::String(name), path)) => {
                PaletteChoice::Named(scheme_by_name(name).ok_or_else(|| {
                    Error::spec(
                        path,
                        format!(
                            "brewer palette '{name}' was not found. Sequential: {}. \
                         Diverging: {}. Qualitative: {}{}",
                            scheme_names(PaletteType::Sequential),
                            scheme_names(PaletteType::Diverging),
                            scheme_names(PaletteType::Qualitative),
                            crate::suggest::did_you_mean(name, super::brewer::all_scheme_names())
                        ),
                    )
                })?)
            }
            Some((Value::Number(number), path)) => {
                if kind.is_none() {
                    return Err(Error::spec(
                        path,
                        "a palette index needs a palette type: 'seq', 'div' or \
                         'qual'",
                    ));
                }
                // Kotlin `toInt()`: truncates and saturates at the 32-bit range.
                let index = number
                    .as_f64()
                    .unwrap_or(0.0)
                    .clamp(0.0, f64::from(i32::MAX)) as i64;
                if number.as_f64().is_some_and(|value| value < 0.0) {
                    return Err(Error::spec(
                        path,
                        format!("a palette index is 0 or more, got {number}"),
                    ));
                }
                PaletteChoice::Index(index)
            }
            Some((other, path)) => {
                return Err(Error::spec(
                    path,
                    format!("expected a palette name or index, got {other}"),
                ));
            }
        };
        let overflow = match text("overflow")? {
            None => Overflow::Auto,
            Some((name, path)) => Overflow::parse(name).ok_or_else(|| {
                Error::spec(
                    path,
                    format!(
                        "overflow expected one of 'interpolate' ('i'), 'cycle' ('c'), \
                     'generate' ('g'), got '{name}'"
                    ),
                )
            })?,
        };
        Ok(BrewerMapper {
            kind,
            palette,
            reversed: self
                .number("direction")?
                .is_some_and(|direction| direction < 0.0),
            overflow,
            na,
        })
    }

    /// A finite number option.
    fn number(&self, key: &str) -> Result<Option<f64>> {
        match self.options.get(key) {
            None => Ok(None),
            Some((value, path)) => value
                .as_f64()
                .filter(|number| number.is_finite())
                .map(Some)
                .ok_or_else(|| Error::spec(path, format!("expected a number, got {value}"))),
        }
    }

    fn hue(&self) -> Result<HueMapper> {
        let mut mapper = HueMapper::default_with(self.na_color()?);
        if let Some((value, path)) = self.options.get("h") {
            let range = value
                .as_array()
                .filter(|items| items.len() == 2)
                .and_then(|items| Some((items[0].as_f64()?, items[1].as_f64()?)))
                .filter(|(low, high)| low.is_finite() && high.is_finite());
            mapper.hue_range = range.ok_or_else(|| {
                Error::spec(
                    path,
                    format!("expected a hue range [from, to] in degrees, got {value}"),
                )
            })?;
        }
        let number = |key: &str| -> Result<Option<f64>> {
            match self.options.get(key) {
                None => Ok(None),
                Some((value, path)) => value
                    .as_f64()
                    .filter(|number| number.is_finite())
                    .map(Some)
                    .ok_or_else(|| Error::spec(path, format!("expected a number, got {value}"))),
            }
        };
        mapper.chroma = number("c")?.unwrap_or(mapper.chroma);
        mapper.luminance = number("l")?.unwrap_or(mapper.luminance);
        mapper.start_hue = number("h_start")?.unwrap_or(mapper.start_hue);
        mapper.reversed = number("direction")?.is_some_and(|direction| direction < 0.0);
        Ok(mapper)
    }

    fn manual_colors(&self, values: &Value, path: &str) -> Result<ManualColorMapper> {
        let items = values
            .as_array()
            .ok_or_else(|| Error::spec(path, format!("expected a list of colors, got {values}")))?;
        // lets-plot skips null values (`filterNotNull`).
        let colors = items
            .iter()
            .enumerate()
            .filter(|(_, item)| !item.is_null())
            .map(|(index, item)| color(item, &format!("{path}[{index}]")))
            .collect::<Result<Vec<_>>>()?;
        if colors.is_empty() {
            return Err(Error::spec(path, "manual scales need at least one color"));
        }
        Ok(ManualColorMapper {
            colors,
            na: self.na_color()?,
        })
    }

    fn manual_linetypes(&self, values: &Value, path: &str) -> Result<LinetypeMapper> {
        let items = values.as_array().ok_or_else(|| {
            Error::spec(path, format!("expected a list of line types, got {values}"))
        })?;
        let linetype = |item: &Value, at: &str| {
            LineType::parse(item).map_err(|message| Error::spec(at, message))
        };
        // lets-plot skips null values (`filterNotNull`).
        let linetypes = items
            .iter()
            .enumerate()
            .filter(|(_, item)| !item.is_null())
            .map(|(index, item)| linetype(item, &format!("{path}[{index}]")))
            .collect::<Result<Vec<_>>>()?;
        if linetypes.is_empty() {
            return Err(Error::spec(
                path,
                "manual scales need at least one line type",
            ));
        }
        let na = match &self.na_value {
            None => LinetypeMapper::all().na,
            Some((value, path)) => linetype(value, path)?,
        };
        Ok(LinetypeMapper { linetypes, na })
    }

    fn manual_shapes(&self, values: &Value, path: &str) -> Result<ShapeMapper> {
        let items = values.as_array().ok_or_else(|| {
            Error::spec(
                path,
                format!("expected a list of shape codes, got {values}"),
            )
        })?;
        let shape = |item: &Value, at: &str| match parse_constant(ValueKind::Shape, item) {
            Ok(StyleValues::Shapes(PerItem::Uniform(code))) => Ok(code),
            Ok(_) | Err(_) => Err(Error::spec(
                at,
                format!("expected a shape code 0..25, got {item}"),
            )),
        };
        // lets-plot skips null values (`filterNotNull`).
        let shapes = items
            .iter()
            .enumerate()
            .filter(|(_, item)| !item.is_null())
            .map(|(index, item)| shape(item, &format!("{path}[{index}]")))
            .collect::<Result<Vec<_>>>()?;
        if shapes.is_empty() {
            return Err(Error::spec(path, "manual scales need at least one shape"));
        }
        let na = match &self.na_value {
            None => NA_SHAPE,
            Some((value, path)) => shape(value, path)?,
        };
        Ok(ShapeMapper { shapes, na })
    }

    fn manual_numbers(
        &self,
        aesthetic: Aesthetic,
        values: &Value,
        path: &str,
    ) -> Result<ManualNumberMapper> {
        let items = values.as_array().ok_or_else(|| {
            Error::spec(path, format!("expected a list of numbers, got {values}"))
        })?;
        // lets-plot skips null values (`filterNotNull`).
        let numbers = items
            .iter()
            .enumerate()
            .filter(|(_, item)| !item.is_null())
            .map(|(index, item)| {
                item.as_f64()
                    .filter(|value| value.is_finite())
                    .ok_or_else(|| {
                        Error::spec(
                            format!("{path}[{index}]"),
                            format!("expected a number, got {item}"),
                        )
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        if numbers.is_empty() {
            return Err(Error::spec(path, "manual scales need at least one value"));
        }
        let default_na = match aesthetic {
            Aesthetic::Alpha => ALPHA.missing(),
            _ => SIZE.missing(),
        };
        Ok(ManualNumberMapper {
            numbers,
            na: self.na_number(default_na)?,
        })
    }

    /// `na_value` as a number, else `default`.
    fn na_number(&self, default: f64) -> Result<f64> {
        match &self.na_value {
            None => Ok(default),
            Some((value, path)) => value
                .as_f64()
                .filter(|value| value.is_finite())
                .ok_or_else(|| Error::spec(path, format!("expected a number, got {value}"))),
        }
    }

    /// `na_value` as a one-item value of the scale's kind, else the default
    /// mapper's NA (its value for a missing category).
    fn na_value(
        &self,
        aesthetic: Aesthetic,
        default: &Arc<dyn StyleMapper>,
    ) -> Result<StyleValues> {
        match &self.na_value {
            Some((value, path)) => parse_constant(default.value_kind(), value).map_err(|message| {
                Error::spec(path, format!("{message} ('{aesthetic}' na_value)"))
            }),
            None => Ok(match default.map_discrete(&[None], 1) {
                StyleValues::Colors(colors) => StyleValues::Colors(PerItem::Uniform(colors.get(0))),
                StyleValues::Numbers(numbers) => {
                    StyleValues::Numbers(PerItem::Uniform(numbers.get(0)))
                }
                StyleValues::Shapes(shapes) => StyleValues::Shapes(PerItem::Uniform(shapes.get(0))),
                other => other,
            }),
        }
    }

    fn na_color(&self) -> Result<Color> {
        match &self.na_value {
            None => Ok(NA_COLOR),
            Some((value, path)) => color(value, path),
        }
    }
}

fn color(value: &Value, path: &str) -> Result<Color> {
    match parse_constant(ValueKind::Color, value) {
        Ok(StyleValues::Colors(PerItem::Uniform(color))) => Ok(color),
        Ok(_) => Err(Error::spec(path, "expected a color")),
        Err(message) => Err(Error::spec(path, message)),
    }
}

/// A positive finite number option.
fn positive_number(value: &Value, path: &str) -> Result<f64> {
    value
        .as_f64()
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or_else(|| Error::spec(path, format!("expected a positive number, got {value}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Figure;

    /// The merged fill scale of `scales` (JSON scale objects), with warnings.
    fn fill(scales: &str) -> (StyleScaleSpec, Vec<String>) {
        let json = format!(r#"{{"kind": "plot", "scales": [{scales}]}}"#);
        let Figure::Plot(plot) = Figure::from_json(&json).unwrap() else {
            panic!("expected a plot");
        };
        let mut warnings = Vec::new();
        let mut merged = style_scale_specs(&plot, &mut warnings);
        (merged.remove(&Aesthetic::Fill).unwrap(), warnings)
    }

    #[test]
    fn oob_defaults_to_censor_and_is_not_reported() {
        let (spec, warnings) = fill(r#"{"aesthetic": "fill", "limits": [0, 1]}"#);
        assert_eq!(spec.oob().unwrap(), Oob::Censor);
        let (spec, warnings_squish) = fill(r#"{"aesthetic": "fill", "oob": "squish"}"#);
        assert_eq!(spec.oob().unwrap(), Oob::Squish);
        assert_eq!(spec.continuous_domain().unwrap().oob, Oob::Squish);
        assert!(warnings.is_empty() && warnings_squish.is_empty());
    }

    #[test]
    fn oob_merges_like_limits_across_a_new_mapper() {
        // A later palette replaces the mapper but keeps limits and oob.
        let (spec, _) = fill(
            r#"{"aesthetic": "fill", "limits": [-2.5, 2.5], "oob": "squish"},
               {"aesthetic": "fill", "scale_mapper_kind": "color_gradientn",
                "colors": ["black", "yellow"]}"#,
        );
        assert_eq!(spec.oob().unwrap(), Oob::Squish);
        assert_eq!(
            spec.continuous_domain().unwrap().limits,
            [Some(-2.5), Some(2.5)]
        );
        // A later explicit oob wins.
        let (spec, _) = fill(
            r#"{"aesthetic": "fill", "oob": "squish"},
               {"aesthetic": "fill", "scale_mapper_kind": "color_gradient", "oob": "censor"}"#,
        );
        assert_eq!(spec.oob().unwrap(), Oob::Censor);
    }

    #[test]
    fn unknown_oob_names_the_path_and_a_close_match() {
        let (spec, _) = fill(r#"{"aesthetic": "fill"}, {"aesthetic": "fill", "oob": "sqush"}"#);
        let message = spec.oob().unwrap_err().to_string();
        assert!(message.contains("scales[1].oob"), "{message}");
        assert!(message.contains("'censor', 'squish'"), "{message}");
        assert!(message.contains("did you mean 'squish'?"), "{message}");
        let (spec, _) = fill(r#"{"aesthetic": "fill", "oob": 1}"#);
        let message = spec.oob().unwrap_err().to_string();
        assert!(message.contains("expected an oob name, got 1"), "{message}");
    }
}
