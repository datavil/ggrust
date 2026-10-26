//! Style scales from `scale_color_*()`, `scale_fill_*()`, ... calls.
//!
//! Several calls for one aesthetic merge key by key, later ones winning, as in
//! lets-plot (`PlotConfigUtil.mergeScaleOptions`). The merged options choose the
//! mapper (lets-plot `ScaleConfig.createMapperProvider`) and the discrete domain.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::{
    DefaultColorMapper, DiscreteDomain, HueMapper, Level, ManualColorMapper, NA_COLOR, StyleMapper,
    default_mapper,
};
use crate::scene::{Color, PerItem};
use crate::spec::{Aesthetic, PlotSpec, ScaleSpec, Value};
use crate::style::{StyleValues, ValueKind, parse_constant};
use crate::{Error, Result};

/// One merged option with the path of the call that set it.
type Set<T> = Option<(T, String)>;

/// Mapper options read here; any other `parameters` key is reported.
const MAPPER_OPTIONS: &[&str] = &["values", "h", "c", "l", "h_start", "direction"];

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
            merge(merged.entry(aesthetic).or_default(), scale, &path);
            report_unbuilt(scale, &path, warnings);
        }
    }
    merged
}

fn merge(target: &mut StyleScaleSpec, scale: &ScaleSpec, path: &str) {
    let at = |key: &str| format!("{path}.{key}");
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
    if let Some(na_value) = &scale.na_value {
        target.na_value = Some((na_value.clone(), at("na_value")));
    }
    if let Some(kind) = &scale.scale_mapper_kind {
        target.kind = Some((kind.clone(), at("scale_mapper_kind")));
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
        ("format", scale.format.is_some()),
        ("guide", scale.guide.is_some()),
        ("trans", scale.trans.is_some()),
        ("expand", scale.expand.is_some()),
        ("position", scale.position.is_some()),
        ("discrete", scale.discrete.is_some()),
        ("reverse", scale.reverse.is_some()),
    ];
    for (key, _) in present.iter().filter(|(_, present)| *present) {
        warnings.push(format!("at '{path}.{key}': not supported yet; ignored"));
    }
    for (key, value) in &scale.parameters {
        if !MAPPER_OPTIONS.contains(&key.as_str()) && !value.is_null() {
            warnings.push(format!("at '{path}.{key}': not supported yet; ignored"));
        }
    }
}

impl StyleScaleSpec {
    /// The mapper the options ask for: `scale_mapper_kind` (lets-plot lets it win
    /// over `values`), then manual `values`, then the aesthetic's default; each with
    /// `na_value` when given.
    pub fn mapper(
        &self,
        aesthetic: Aesthetic,
        warnings: &mut Vec<String>,
    ) -> Result<Arc<dyn StyleMapper>> {
        let Some(default) = default_mapper(aesthetic) else {
            return Err(Error::InvalidSpec(format!(
                "'{aesthetic}' has no style scale"
            )));
        };
        let is_color = default.value_kind() == ValueKind::Color;
        match &self.kind {
            Some((kind, _)) if kind == "color_hue" && is_color => {
                return self
                    .hue()
                    .map(|mapper| Arc::new(mapper) as Arc<dyn StyleMapper>);
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
            warnings.push(format!(
                "at '{path}': manual values for '{aesthetic}' are not supported yet; the \
                 default scale was used"
            ));
        }
        match (&self.na_value, aesthetic) {
            (None, _) => Ok(default),
            (Some(_), Aesthetic::Color | Aesthetic::Fill) => Ok(Arc::new(DefaultColorMapper {
                na: self.na_color()?,
            })),
            (Some(_), Aesthetic::Size | Aesthetic::Alpha) => {
                let mut mapper = if aesthetic == Aesthetic::Size {
                    super::SIZE.clone()
                } else {
                    super::ALPHA.clone()
                };
                mapper.missing = self.na_number()?;
                Ok(Arc::new(mapper))
            }
            (Some(_), _) => Ok(default),
        }
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
                        other => Err(Error::InvalidSpec(format!(
                            "at '{path}': labels are texts, got {other}"
                        ))),
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
            Some((other, path)) => {
                return Err(Error::InvalidSpec(format!(
                    "at '{path}': expected a list of labels, got {other}"
                )));
            }
        };
        Ok(DiscreteDomain {
            breaks: levels(&self.breaks),
            limits: levels(&self.limits).filter(|limits| !limits.is_empty()),
            labels,
        })
    }

    /// Whether `breaks`, `limits` or `labels` were given (they apply to discrete
    /// scales only for now).
    pub fn has_domain_options(&self) -> Option<&str> {
        [&self.breaks, &self.limits]
            .into_iter()
            .find_map(|set| set.as_ref().map(|(_, path)| path.as_str()))
            .or_else(|| self.labels.as_ref().map(|(_, path)| path.as_str()))
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
                Error::InvalidSpec(format!(
                    "at '{path}': expected a hue range [from, to] in degrees, got {value}"
                ))
            })?;
        }
        let number = |key: &str| -> Result<Option<f64>> {
            match self.options.get(key) {
                None => Ok(None),
                Some((value, path)) => value
                    .as_f64()
                    .filter(|number| number.is_finite())
                    .map(Some)
                    .ok_or_else(|| {
                        Error::InvalidSpec(format!("at '{path}': expected a number, got {value}"))
                    }),
            }
        };
        mapper.chroma = number("c")?.unwrap_or(mapper.chroma);
        mapper.luminance = number("l")?.unwrap_or(mapper.luminance);
        mapper.start_hue = number("h_start")?.unwrap_or(mapper.start_hue);
        mapper.reversed = number("direction")?.is_some_and(|direction| direction < 0.0);
        Ok(mapper)
    }

    fn manual_colors(&self, values: &Value, path: &str) -> Result<ManualColorMapper> {
        let items = values.as_array().ok_or_else(|| {
            Error::InvalidSpec(format!(
                "at '{path}': expected a list of colors, got {values}"
            ))
        })?;
        // lets-plot skips null values (`filterNotNull`).
        let colors = items
            .iter()
            .enumerate()
            .filter(|(_, item)| !item.is_null())
            .map(|(index, item)| color(item, &format!("{path}[{index}]")))
            .collect::<Result<Vec<_>>>()?;
        if colors.is_empty() {
            return Err(Error::InvalidSpec(format!(
                "at '{path}': manual scales need at least one color"
            )));
        }
        Ok(ManualColorMapper {
            colors,
            na: self.na_color()?,
        })
    }

    fn na_color(&self) -> Result<Color> {
        match &self.na_value {
            None => Ok(NA_COLOR),
            Some((value, path)) => color(value, path),
        }
    }

    fn na_number(&self) -> Result<f64> {
        let Some((value, path)) = &self.na_value else {
            return Ok(f64::NAN);
        };
        match parse_constant(ValueKind::Number, value) {
            Ok(StyleValues::Numbers(PerItem::Uniform(number))) => Ok(number),
            Ok(_) => Err(Error::InvalidSpec(format!(
                "at '{path}': expected a number"
            ))),
            Err(message) => Err(Error::InvalidSpec(format!("at '{path}': {message}"))),
        }
    }
}

fn color(value: &Value, path: &str) -> Result<Color> {
    match parse_constant(ValueKind::Color, value) {
        Ok(StyleValues::Colors(PerItem::Uniform(color))) => Ok(color),
        Ok(_) => Err(Error::InvalidSpec(format!("at '{path}': expected a color"))),
        Err(message) => Err(Error::InvalidSpec(format!("at '{path}': {message}"))),
    }
}
