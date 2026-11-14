//! Style scales: map data to colors, sizes and alphas (lets-plot mapper providers).
//!
//! A [`StyleMapper`] turns a column into typed [`StyleValues`]; a [`StyleScale`] is
//! one mapper plus the domain trained over every layer (categories: explicit
//! `breaks` first, then data levels in order of first appearance, or `limits` when
//! given; continuous: the numeric range). Mappers are built from the merged
//! `scale_*()` options in [`spec`]. Add a mapper = one file + one branch in
//! [`spec::StyleScaleSpec::mapper`].
//!
//! Not here yet: line types.

mod brewer;
mod brewer_data;
mod color;
pub mod color_space;
mod colormap;
mod colormap_data;
mod ggsci;
mod ggsci_data;
mod gradient;
mod hue;
mod identity;
mod linetype;
mod manual;
mod numeric;
mod shape;
pub mod spec;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub use brewer::{
    BrewerMapper, Overflow, PaletteChoice, PaletteType, scheme_by_name, scheme_names,
};
pub use color::{DefaultColorMapper, GRADIENT_HIGH, GRADIENT_LOW, NA_COLOR, discrete_palette};
pub use colormap::{COLORMAP_NAMES, ColormapMapper, colormap};
pub use ggsci::{FAMILIES as GGSCI_FAMILIES, GgsciMapper, family_by_kind as ggsci_family};
pub use gradient::{
    GRADIENT2_HIGH, GRADIENT2_LOW, GRADIENT2_MID, Gradient2Mapper, GradientMapper, GreyMapper,
};
pub use hue::HueMapper;
pub use identity::IdentityMapper;
pub use linetype::{ALL_LINETYPES, LinetypeMapper};
pub use manual::ManualColorMapper;
pub use manual::ManualNumberMapper;
pub use numeric::{
    ALPHA, LINEWIDTH, LinearMapper, ProportionalMapper, SIZE, SIZE_AREA_MAX, STROKE,
};
pub use shape::{ALL_SHAPES, HIDDEN_SHAPE, HOLLOW_SHAPES, NA_SHAPE, ShapeMapper};

use super::breaks::{is_degenerate_range, linear_breaks};
use super::format::{Label, cut_label, format_breaks};
use super::label_pattern::{LabelPattern, PatternValue};
use super::number_format::ExponentFormat;
use super::{ContinuousPositionScale, Transform};
use crate::data::{Column, ContinuousColumn};
use crate::spec::{Aesthetic, Value};
use crate::style::{StyleValues, ValueKind};

/// Maps data to visual values of one kind.
pub trait StyleMapper: Send + Sync {
    /// Name for debugging and messages, e.g. `color_hue`.
    fn name(&self) -> &'static str;

    /// The kind of values produced.
    fn value_kind(&self) -> ValueKind;

    /// Values for continuous data; `domain` is the trained range (never zero-width).
    fn map_continuous(&self, column: &ContinuousColumn, domain: (f64, f64)) -> StyleValues;

    /// Values for discrete data: each row's index into the trained categories
    /// (`None`: missing or unknown).
    fn map_discrete(&self, indices: &[Option<usize>], level_count: usize) -> StyleValues;

    /// Values for discrete data given the categories themselves; mappers that need
    /// the values (identity scales) override this, others map by index. Errors
    /// name a category that cannot be used.
    fn map_levels(
        &self,
        indices: &[Option<usize>],
        levels: &[Level],
    ) -> std::result::Result<StyleValues, String> {
        Ok(self.map_discrete(indices, levels.len()))
    }

    /// Whether only continuous data can be mapped (`scale_*_gradient2()`).
    fn continuous_only(&self) -> bool {
        false
    }

    /// How many colors a fixed palette has (ggsci); more categories warn
    /// ([`StyleScale::palette_overflow`]). `None`: no fixed size.
    fn palette_size(&self) -> Option<usize> {
        None
    }
}

/// Which style scale a mapping, a `scale_*()` call or a guide belongs to: the
/// aesthetic plus its ggnewscale scale group (0 unless `new_scale_*()` came
/// before). Group 0 behaves exactly as a plot without `new_scale_*()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScaleKey {
    /// The mapped style aesthetic.
    pub aesthetic: Aesthetic,
    /// The ggnewscale group: how many `new_scale_*()` of the aesthetic came before.
    pub group: u32,
}

impl ScaleKey {
    /// The scale of `aesthetic` in scale group `group`.
    pub fn new(aesthetic: Aesthetic, group: u32) -> Self {
        ScaleKey { aesthetic, group }
    }

    /// The key of the scale's guide in `guides`: `color` for group 0, else
    /// `color:1` (what Python writes for `guides()` and `labs()` after
    /// `new_scale_color()`).
    pub fn guide_key(self) -> String {
        match self.group {
            0 => self.aesthetic.as_str().to_owned(),
            group => format!("{}:{group}", self.aesthetic),
        }
    }

    /// The scale key of a `guides` key (`color`, `color:1`); `None` for anything
    /// else (manual legend groups).
    pub fn parse_guide_key(key: &str) -> Option<Self> {
        let (aesthetic, group) = match key.split_once(':') {
            Some((aesthetic, group)) => (aesthetic, group.parse().ok()?),
            None => (key, 0),
        };
        Some(ScaleKey::new(aesthetic.parse().ok()?, group))
    }
}

impl std::fmt::Display for ScaleKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.guide_key())
    }
}

/// The mapper lets-plot uses for an aesthetic without a `scale_*()` call.
pub fn default_mapper(aesthetic: Aesthetic) -> Option<Arc<dyn StyleMapper>> {
    match aesthetic {
        Aesthetic::Color
        | Aesthetic::Fill
        | Aesthetic::PaintA
        | Aesthetic::PaintB
        | Aesthetic::PaintC => Some(Arc::new(DefaultColorMapper { na: NA_COLOR })),
        Aesthetic::Size => Some(Arc::new(SIZE.clone())),
        Aesthetic::Alpha => Some(Arc::new(ALPHA.clone())),
        Aesthetic::Linewidth => Some(Arc::new(LINEWIDTH.clone())),
        Aesthetic::Stroke => Some(Arc::new(STROKE.clone())),
        Aesthetic::Shape => Some(Arc::new(ShapeMapper::all())),
        Aesthetic::Linetype => Some(Arc::new(LinetypeMapper::all())),
        _ => None,
    }
}

/// lets-plot `Mappers.linear`: `domain` onto `range`; a zero-width domain maps
/// everything to the middle of the range.
pub fn linear(domain: (f64, f64), range: (f64, f64)) -> impl Fn(f64) -> f64 {
    let slope = (range.1 - range.0) / (domain.1 - domain.0);
    let finite = slope.is_finite();
    let intercept = range.0 - domain.0 * slope;
    let middle = (range.1 - range.0) / 2.0 + range.0;
    move |value| {
        if finite {
            value * slope + intercept
        } else {
            middle
        }
    }
}

/// A category of a discrete style scale. Numbers mapped next to discrete columns
/// become categories too; the number 2 and the text "2" are different categories
/// (as in lets-plot).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Level {
    /// A text category.
    Text(String),
    /// `f64` bits: -0.0 and 0.0 differ, as in lets-plot.
    Number(u64),
}

impl Level {
    /// A numeric category.
    pub fn number(value: f64) -> Self {
        Level::Number(value.to_bits())
    }

    /// A category from a spec value (`breaks`, `limits`): text, number or boolean
    /// (booleans are read as the texts `true` / `false`, like boolean columns).
    /// Null and other values give `None`.
    pub fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::String(text) => Some(Level::Text(text.clone())),
            Value::Number(number) => number.as_f64().map(Level::number),
            Value::Bool(flag) => Some(Level::Text(flag.to_string())),
            _ => None,
        }
    }

    /// The legend label: the text, or the number without a trailing `.0` (-0 reads
    /// "0", so it shares a key with 0, as in lets-plot).
    pub fn label(&self) -> String {
        match self {
            Level::Text(text) => text.clone(),
            Level::Number(bits) => {
                let value = f64::from_bits(*bits) + 0.0;
                if value.fract() == 0.0 && value.abs() < 1e15 {
                    format!("{value:.0}")
                } else {
                    value.to_string()
                }
            }
        }
    }
}

/// What a style scale has seen.
#[derive(Debug, Clone, PartialEq, Default)]
enum Trained {
    #[default]
    Nothing,
    Continuous(Option<(f64, f64)>),
    /// Categories: `breaks` first, then data levels in order of first appearance.
    Discrete {
        levels: Vec<Level>,
        index_by_level: HashMap<Level, usize>,
    },
}

/// Discrete domain options from `scale_*()`: `breaks`, `limits`, `labels`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DiscreteDomain {
    /// Categories shown in the legend, and placed first in the domain.
    pub breaks: Option<Vec<Level>>,
    /// The whole domain, in order; other values map to the NA value.
    pub limits: Option<Vec<Level>>,
    /// Legend labels, aligned with `breaks` (or the domain).
    pub labels: Option<Vec<String>>,
    /// `format=...` for categories without a given label.
    pub format: Option<LabelPattern>,
    /// `lablim=...`: labels longer than this many characters are cut to it plus
    /// `...` (lets-plot applies it to discrete scales only).
    pub label_limit: Option<usize>,
}

/// Continuous domain options from `scale_*()`: numeric `limits`, `breaks`, `labels`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ContinuousDomain {
    /// `[low, high]`, either side open; values outside follow `oob`.
    pub limits: [Option<f64>; 2],
    /// `oob=...`: what values outside `limits` map to.
    pub oob: Oob,
    /// Breaks shown in the legend (default: about 5 nice breaks).
    pub breaks: Option<Vec<f64>>,
    /// Labels aligned with the breaks.
    pub labels: Option<Vec<String>>,
    /// `trans=...`: values are mapped (and breaks generated) in transformed units.
    pub transform: Transform,
    /// `format=...` for break labels not given in `labels`.
    pub format: Option<LabelPattern>,
    /// `scale_*_steps()` / `scale_*_binned()`: values map to the color of their bin.
    pub binning: Option<Binning>,
    /// `rescaler=...` (ggplot2): maps values to 0..1 before the palette.
    pub rescaler: Option<Rescaler>,
}

/// A continuous scale's `rescaler` (ggplot2 `continuous_scale(rescaler=)`):
/// called once per mapping as `rescaler(x, to = (0, 1), from = domain)` with the
/// finite values in transformed units; results outside 0..1 map to the NA value.
#[derive(Clone)]
pub struct Rescaler(pub Arc<dyn crate::scale::secondary::AxisFunction>);

impl std::fmt::Debug for Rescaler {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Rescaler")
    }
}

impl PartialEq for Rescaler {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Rescaler {
    /// `column` with every finite value rescaled into `domain` (as the palette
    /// sees it: `domain.0 + r * (domain.1 - domain.0)` for the rescaled `r`).
    /// Missing and non-finite values stay; results outside 0..1 become missing.
    fn apply(
        &self,
        column: &ContinuousColumn,
        domain: (f64, f64),
    ) -> std::result::Result<ContinuousColumn, String> {
        let mut values: Vec<Option<f64>> = (0..column.len()).map(|row| column.get(row)).collect();
        let finite: Vec<f64> = values
            .iter()
            .flatten()
            .copied()
            .filter(|value| value.is_finite())
            .collect();
        if finite.is_empty() {
            return Ok(ContinuousColumn::from_options(values));
        }
        let rescaled = self
            .0
            .rescale(&finite, (0.0, 1.0), domain)
            .map_err(|reason| format!("the scale's rescaler failed: {reason}"))?;
        if rescaled.len() != finite.len() {
            return Err(format!(
                "the scale's rescaler returned {} values for {} inputs; return one value per \
                 value of x",
                rescaled.len(),
                finite.len()
            ));
        }
        let (low, high) = domain;
        let mut results = rescaled.into_iter();
        for value in values.iter_mut().flatten() {
            if !value.is_finite() {
                continue;
            }
            let rescaled = results.next().unwrap_or(f64::NAN);
            *value = if (0.0..=1.0).contains(&rescaled) {
                (low + rescaled * (high - low)).clamp(low.min(high), low.max(high))
            } else {
                f64::NAN
            };
        }
        Ok(ContinuousColumn::from_options(
            values
                .into_iter()
                .map(|value| value.filter(|value| !value.is_nan()))
                .collect(),
        ))
    }
}

/// Options of a binned style scale (ggplot2 `binned_scale`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Binning {
    /// `n_breaks=...`: how many breaks to aim for (default 5).
    pub n_breaks: Option<usize>,
    /// `nice_breaks=...`: round break values (else evenly spaced over the range).
    pub nice: bool,
    /// `right=...`: bins are closed on the right, so a value on a break joins the
    /// bin below it.
    pub right: bool,
    /// `show_limits=...`: the guide also labels the scale's ends.
    pub show_limits: bool,
}

/// The bins of a binned scale: the outer ends and the inner breaks, ascending
/// (transformed units).
#[derive(Debug, Clone, PartialEq)]
struct Bins {
    domain: (f64, f64),
    inner: Vec<f64>,
    right: bool,
}

impl Bins {
    /// All edges, low end first.
    fn edges(&self) -> Vec<f64> {
        let mut edges = Vec::with_capacity(self.inner.len() + 2);
        edges.push(self.domain.0);
        edges.extend(&self.inner);
        edges.push(self.domain.1);
        edges
    }

    /// The middle of the bin holding `value` (ggplot2 `cut(include.lowest = TRUE)`);
    /// `None` outside the ends.
    fn midpoint(&self, value: f64) -> Option<f64> {
        let (low, high) = self.domain;
        if !value.is_finite() || value < low || value > high {
            return None;
        }
        let edge = |index: usize| match index {
            0 => low,
            index if index > self.inner.len() => high,
            index => self.inner[index - 1],
        };
        // Bin k runs from edge k to edge k + 1; inner edge k + 1 is inner[k].
        let index = if self.right {
            self.inner.partition_point(|edge| *edge < value)
        } else {
            self.inner.partition_point(|edge| *edge <= value)
        };
        Some((edge(index) + edge(index + 1)) / 2.0)
    }
}

/// What a continuous style scale does with values outside its `limits`
/// (ggplot2 `oob`; lets-plot always censors).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Oob {
    /// Map to the NA value (lets-plot, ggplot2 `scales::censor`).
    #[default]
    Censor,
    /// Clamp to the nearest limit, so they take the end colors (`scales::squish`).
    Squish,
}

impl Oob {
    /// The accepted names, for messages.
    pub const NAMES: &'static str = "'censor', 'squish'";

    /// `censor` or `squish`.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "censor" => Some(Oob::Censor),
            "squish" => Some(Oob::Squish),
            _ => None,
        }
    }
}

/// Breaks of a continuous style scale, for a colorbar or a legend.
#[derive(Debug, Clone, PartialEq)]
pub struct ContinuousGuide {
    /// The domain the mapper uses (transformed units).
    pub domain: (f64, f64),
    /// Break values (transformed units) with their labels, in data order.
    pub breaks: Vec<(f64, Label)>,
}

/// One legend entry of a style scale.
#[derive(Debug, Clone, PartialEq)]
pub struct LegendEntry {
    /// Label shown next to the key.
    pub label: Label,
    /// The mapped value, as a single item.
    pub value: StyleValues,
}

/// A style mapper with its domain trained over all layers.
#[derive(Clone)]
pub struct StyleScale {
    mapper: Arc<dyn StyleMapper>,
    domain: DiscreteDomain,
    /// Read only once the scale turns out continuous (string limits are fine on a
    /// discrete scale): an error is reported then.
    continuous: std::result::Result<ContinuousDomain, String>,
    trained: Trained,
    /// The theme's `exponent_format` for generated labels.
    exponent_format: ExponentFormat,
}

impl std::fmt::Debug for StyleScale {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StyleScale")
            .field("mapper", &self.mapper.name())
            .field("domain", &self.domain)
            .field("trained", &self.trained)
            .finish()
    }
}

impl StyleScale {
    /// An untrained scale; its kind follows the first column trained.
    pub fn new(mapper: Arc<dyn StyleMapper>, domain: DiscreteDomain) -> Self {
        StyleScale {
            mapper,
            domain,
            continuous: Ok(ContinuousDomain::default()),
            trained: Trained::Nothing,
            exponent_format: ExponentFormat::default(),
        }
    }

    /// Write exponents as the theme's `exponent_format` says (generated labels
    /// and `format=` patterns).
    pub fn set_exponent_format(&mut self, exponent: ExponentFormat) {
        self.exponent_format = exponent;
        if let Some(format) = &self.domain.format {
            self.domain.format = Some(format.with_exponent(exponent));
        }
        if let Ok(continuous) = &mut self.continuous
            && let Some(format) = &continuous.format
        {
            continuous.format = Some(format.with_exponent(exponent));
        }
    }

    /// The same scale with continuous domain options (or the error reading them).
    pub fn with_continuous(
        mut self,
        continuous: std::result::Result<ContinuousDomain, String>,
    ) -> Self {
        self.continuous = continuous;
        self
    }

    /// An untrained discrete scale: continuous columns trained on it become
    /// categories (lets-plot makes a scale discrete when any layer maps a discrete
    /// column to it).
    pub fn discrete(mapper: Arc<dyn StyleMapper>, domain: DiscreteDomain) -> Self {
        let mut scale = StyleScale::new(mapper, domain);
        scale.start_discrete();
        scale
    }

    /// The mapper.
    pub fn mapper(&self) -> &dyn StyleMapper {
        self.mapper.as_ref()
    }

    /// Whether the scale holds categories.
    pub fn is_discrete(&self) -> bool {
        matches!(self.trained, Trained::Discrete { .. })
    }

    /// The domain starts with the `breaks` (lets-plot `PlotConfigTransforms`).
    fn start_discrete(&mut self) {
        let mut levels = Vec::new();
        let mut index_by_level = HashMap::new();
        for level in self.domain.breaks.iter().flatten() {
            if !index_by_level.contains_key(level) {
                index_by_level.insert(level.clone(), levels.len());
                levels.push(level.clone());
            }
        }
        self.trained = Trained::Discrete {
            levels,
            index_by_level,
        };
    }

    /// Put trained text categories in the order of `reference` (facet panels can
    /// train them in another order), after the `breaks`, which lead the domain;
    /// categories in neither keep their order after the rest. Continuous scales are
    /// unchanged.
    pub fn order_levels_by(&mut self, reference: &[String]) {
        let breaks = self.domain.breaks.as_deref().unwrap_or_default();
        let Trained::Discrete {
            levels,
            index_by_level,
        } = &mut self.trained
        else {
            return;
        };
        let mut rank: HashMap<Level, usize> = HashMap::new();
        for level in breaks
            .iter()
            .cloned()
            .chain(reference.iter().map(|level| Level::Text(level.clone())))
        {
            let next = rank.len();
            rank.entry(level).or_insert(next);
        }
        let key = |level: &Level| rank.get(level).copied().unwrap_or(usize::MAX);
        levels.sort_by_key(key);
        *index_by_level = levels
            .iter()
            .enumerate()
            .map(|(index, level)| (level.clone(), index))
            .collect();
    }

    /// Learn the domain from a column. A discrete column cannot be trained on a scale
    /// that is already continuous: create it with [`StyleScale::discrete`] instead.
    pub fn train(&mut self, column: &Column) -> Result<(), String> {
        match (&self.trained, column) {
            (Trained::Nothing, Column::Continuous(_)) => {
                self.trained = Trained::Continuous(None);
            }
            (Trained::Nothing, Column::Discrete(_)) => self.start_discrete(),
            (Trained::Continuous(_), Column::Discrete(_)) => {
                return Err(
                    "a discrete column was trained on a continuous style scale; \
                            create the scale as discrete"
                        .to_owned(),
                );
            }
            _ => {}
        }
        if self.is_discrete() && self.mapper.continuous_only() {
            return Err(format!(
                "'{}' scales map continuous data only, but discrete data is mapped",
                self.mapper.name()
            ));
        }
        let limits = match (&self.trained, &self.continuous) {
            (Trained::Continuous(_), Ok(domain)) => domain.limits,
            (Trained::Continuous(_), Err(message)) => return Err(message.clone()),
            _ => [None, None],
        };
        let within = |value: f64| {
            limits[0].is_none_or(|low| value >= low) && limits[1].is_none_or(|high| value <= high)
        };
        let transform = self.transform();
        match (&mut self.trained, column) {
            (Trained::Continuous(range), Column::Continuous(column)) => {
                for row in 0..column.len() {
                    if let Some(value) = column
                        .get(row)
                        .filter(|value| value.is_finite() && within(*value))
                        .and_then(|value| transform.apply(value))
                    {
                        *range = Some(match *range {
                            Some((low, high)) => (low.min(value), high.max(value)),
                            None => (value, value),
                        });
                    }
                }
            }
            (
                Trained::Discrete {
                    levels,
                    index_by_level,
                },
                column,
            ) => {
                let mut add = |level: Level| {
                    if !index_by_level.contains_key(&level) {
                        index_by_level.insert(level.clone(), levels.len());
                        levels.push(level);
                    }
                };
                match column {
                    Column::Discrete(column) => {
                        for level in column.levels() {
                            add(Level::Text(level.clone()));
                        }
                    }
                    Column::Continuous(column) => {
                        for row in 0..column.len() {
                            if let Some(value) = column.get(row).filter(|value| value.is_finite()) {
                                add(Level::number(value));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// `(palette size, category count)` when a discrete scale has more categories
    /// than its mapper's fixed palette ([`StyleMapper::palette_size`]).
    pub fn palette_overflow(&self) -> Option<(usize, usize)> {
        let size = self.mapper.palette_size()?;
        let count = self.effective_domain().len();
        (self.is_discrete() && count > size).then_some((size, count))
    }

    /// The categories values are mapped by: `limits` when given, else the trained
    /// levels (lets-plot `DiscreteTransform.effectiveDomain`). Empty if continuous.
    fn effective_domain(&self) -> &[Level] {
        match (&self.trained, &self.domain.limits) {
            (Trained::Discrete { .. }, Some(limits)) => limits,
            (Trained::Discrete { levels, .. }, None) => levels,
            _ => &[],
        }
    }

    /// The first row of `column` this scale maps to its NA value: a missing value
    /// (null or NaN), a category outside the effective domain (`limits`), or a
    /// number outside the continuous limits. `None` when every row has a value.
    pub fn first_missing_row(&self, column: &Column) -> Option<usize> {
        match column {
            Column::Continuous(values) => {
                if let Some(row) =
                    (0..values.len()).find(|row| values.get(*row).is_none_or(f64::is_nan))
                {
                    return Some(row);
                }
                let limited = self
                    .continuous
                    .as_ref()
                    .is_ok_and(|domain| domain.limits != [None, None]);
                if !limited {
                    return None;
                }
                // Limits (in data or transformed space) are applied by `map`: a row
                // mapped like a null one fell outside them.
                let mapped = self.map(column).ok()?;
                let na = self
                    .map(&Column::Continuous(ContinuousColumn::from_options(vec![
                        None,
                    ])))
                    .ok()?;
                let na = single(&na, 0);
                (0..values.len()).find(|row| single(&mapped, *row) == na)
            }
            Column::Discrete(values) => {
                let domain: HashSet<String> =
                    self.effective_domain().iter().map(Level::label).collect();
                (0..values.len()).find(|row| {
                    values
                        .label(*row)
                        .is_none_or(|label| self.is_discrete() && !domain.contains(label))
                })
            }
        }
    }

    /// Map a column with the trained domain.
    pub fn map(&self, column: &Column) -> Result<StyleValues, String> {
        match (&self.trained, column) {
            (Trained::Continuous(_), Column::Continuous(column)) => {
                let domain = self.continuous_domain();
                let (limits, oob) = self
                    .continuous
                    .as_ref()
                    .map_or(([None, None], Oob::Censor), |domain| {
                        (domain.limits, domain.oob)
                    });
                let transform = self.transform();
                let bins = self.bins();
                let rescaler = self.rescaler();
                if limits == [None, None] && transform == Transform::Identity && bins.is_none() {
                    return Ok(match rescaler {
                        Some(rescaler) => self
                            .mapper
                            .map_continuous(&rescaler.apply(column, domain)?, domain),
                        None => self.mapper.map_continuous(column, domain),
                    });
                }
                // lets-plot: values outside the limits (or the transform's domain)
                // are missing. Squished values are clamped to the limits first;
                // non-finite values stay as they are (ggplot2 `only.finite`).
                let limited = ContinuousColumn::from_options(
                    (0..column.len())
                        .map(|row| {
                            let value = column.get(row)?;
                            let value = match oob {
                                Oob::Squish if value.is_finite() => {
                                    let value = limits[0].map_or(value, |low| value.max(low));
                                    limits[1].map_or(value, |high| value.min(high))
                                }
                                _ => value,
                            };
                            (limits[0].is_none_or(|low| value >= low)
                                && limits[1].is_none_or(|high| value <= high))
                            .then_some(value)
                            .and_then(|value| transform.apply(value))
                            .and_then(|value| match &bins {
                                Some(bins) => bins.midpoint(value),
                                None => Some(value),
                            })
                        })
                        .collect(),
                );
                let limited = match rescaler {
                    Some(rescaler) => rescaler.apply(&limited, domain)?,
                    None => limited,
                };
                Ok(self.mapper.map_continuous(&limited, domain))
            }
            (Trained::Discrete { .. }, column) => {
                let domain = self.effective_domain();
                let index_by_level: HashMap<&Level, usize> = domain
                    .iter()
                    .enumerate()
                    .rev()
                    .map(|(index, level)| (level, index))
                    .collect();
                let index_of = |level: Level| index_by_level.get(&level).copied();
                let indices: Vec<Option<usize>> = match column {
                    Column::Discrete(column) => {
                        let index_by_code: Vec<Option<usize>> = column
                            .levels()
                            .iter()
                            .map(|level| index_of(Level::Text(level.clone())))
                            .collect();
                        (0..column.len())
                            .map(|row| {
                                column
                                    .code(row)
                                    .and_then(|code| index_by_code[code as usize])
                            })
                            .collect()
                    }
                    Column::Continuous(column) => (0..column.len())
                        .map(|row| {
                            column
                                .get(row)
                                .filter(|value| value.is_finite())
                                .and_then(|value| index_of(Level::number(value)))
                        })
                        .collect(),
                };
                self.mapper.map_levels(&indices, domain)
            }
            _ => Err("the scale was not trained on a column of this kind".to_owned()),
        }
    }

    /// The domain continuous data is mapped over: [`Self::trained_domain`], or
    /// the ends of a binned scale's bins.
    fn continuous_domain(&self) -> (f64, f64) {
        self.bins()
            .map_or_else(|| self.trained_domain(), |bins| bins.domain)
    }

    /// The bins of a binned scale (ggplot2 `ScaleBinned$get_breaks`): the given
    /// breaks, or about `n_breaks` generated ones; without given breaks or
    /// limits, the ends move out so the end bins are as wide as their
    /// neighbours.
    fn bins(&self) -> Option<Bins> {
        let options = self.continuous.as_ref().ok()?;
        let binning = options.binning?;
        let (mut low, mut high) = self.trained_domain();
        let transform = self.transform();
        let generated = options.breaks.is_none();
        let mut breaks: Vec<f64> = match &options.breaks {
            Some(given) => given
                .iter()
                .filter_map(|value| transform.apply(*value))
                .collect(),
            None if binning.nice => linear_breaks(low, high, binning.n_breaks.unwrap_or(5)),
            None => {
                let count = binning.n_breaks.unwrap_or(5);
                (1..=count)
                    .map(|index| low + (high - low) * index as f64 / (count + 1) as f64)
                    .collect()
            }
        };
        breaks.retain(|value| value.is_finite() && *value >= low && *value <= high);
        breaks.sort_by(f64::total_cmp);
        breaks.dedup();
        if generated && options.limits == [None, None] {
            breaks.retain(|value| *value != low && *value != high);
            let count = breaks.len();
            if count >= 2 {
                let (first, last) = (breaks[0], breaks[count - 1]);
                low = first - (breaks[1] - first);
                high = last + (last - breaks[count - 2]);
            } else if let [single] = breaks.as_slice() {
                let half = (single - low).max(high - single);
                (low, high) = (single - half, single + half);
            }
            // An end the transform cannot invert keeps the data end.
            let (data_low, data_high) = self.trained_domain();
            if !transform.inverse(low).is_finite() {
                low = data_low;
            }
            if !transform.inverse(high).is_finite() {
                high = data_high;
            }
        }
        let inner = breaks
            .into_iter()
            .filter(|value| *value > low && *value < high)
            .collect();
        Some(Bins {
            domain: (low, high),
            inner,
            right: binning.right,
        })
    }

    /// The colorbar of a binned scale (ggplot2 `guide_coloursteps(even.steps =
    /// TRUE)`): every bin the same length, so the guide's domain is bin index
    /// space (0 to the bin count) with the breaks on their edge indices; plus the
    /// bin midpoints (top bin first) whose colors fill the steps. `None` when the
    /// scale is not binned.
    pub fn steps_guide(&self) -> Option<(ContinuousGuide, Vec<f64>)> {
        let bins = self.bins()?;
        let guide = self.continuous_guide()?;
        let edges = bins.edges();
        let count = edges.len() - 1;
        let tolerance = 1e-9 * (bins.domain.1 - bins.domain.0).abs().max(f64::MIN_POSITIVE);
        let breaks = guide
            .breaks
            .into_iter()
            .filter_map(|(value, label)| {
                let next = edges.partition_point(|edge| *edge < value - tolerance);
                let index = (next < edges.len() && (edges[next] - value).abs() <= tolerance)
                    .then_some(next)?;
                Some((index as f64, label))
            })
            .collect();
        let midpoints = edges
            .windows(2)
            .rev()
            .map(|pair| (pair[0] + pair[1]) / 2.0)
            .collect();
        Some((
            ContinuousGuide {
                domain: (0.0, count as f64),
                breaks,
            },
            midpoints,
        ))
    }

    /// The domain of continuous data before binning: the data range within the
    /// limits, widened to the limits (lets-plot `rangeWithLimitsAfterTransform`),
    /// then made applicable (never zero-width).
    fn trained_domain(&self) -> (f64, f64) {
        applicable_domain(self.trained_range())
    }

    /// The data range within the limits, widened to the limits, in transformed
    /// units; may be zero-width. `None` without data or limits.
    fn trained_range(&self) -> Option<(f64, f64)> {
        let Trained::Continuous(range) = self.trained else {
            return None;
        };
        let limits = self
            .continuous
            .as_ref()
            .map_or([None, None], |domain| domain.limits);
        let transform = self.transform();
        let mut ends: Vec<f64> = limits
            .iter()
            .flatten()
            .filter_map(|limit| transform.apply(*limit))
            .collect();
        if let Some((low, high)) = range {
            ends.extend([low, high]);
        }
        let low = ends.iter().copied().fold(f64::INFINITY, f64::min);
        let high = ends.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        (low <= high).then_some((low, high))
    }

    /// Breaks of a continuous scale for its guide (`None` if discrete): the given
    /// `breaks` or about 5 nice ones, labelled by `labels` or formatted.
    pub fn continuous_guide(&self) -> Option<ContinuousGuide> {
        if !matches!(self.trained, Trained::Continuous(_)) {
            return None;
        }
        let domain = self.continuous_domain();
        let options = self.continuous.as_ref().ok();
        let transform = self.transform();
        // Given breaks (data units; those outside the transform's domain dropped),
        // else about 5 generated as on an axis (lets-plot
        // `TransformedDomainBreaksGenerator`): linear in data units for linear
        // transforms, else linear in transformed units, labelled one by one.
        let format = options.and_then(|options| options.format.as_ref());
        let labelled = |values: &[f64], default: Vec<Label>| match format {
            Some(format) => values
                .iter()
                .map(|&value| format.format(PatternValue::Number(value)))
                .collect(),
            None => default,
        };
        let bins = self.bins();
        let (positions, mut labels): (Vec<f64>, Vec<Label>) = if let Some(bins) = &bins {
            // ggplot2 `guide_coloursteps`: the inner breaks, plus the ends with
            // `show_limits`.
            let show_limits = options
                .and_then(|options| options.binning)
                .is_some_and(|binning| binning.show_limits);
            let positions = if show_limits {
                bins.edges()
            } else {
                bins.inner.clone()
            };
            let values: Vec<f64> = positions
                .iter()
                .map(|value| transform.inverse(*value))
                .collect();
            let mut labels = labelled(&values, format_breaks(&values, self.exponent_format));
            // Given labels belong to the breaks; the ends keep their own.
            if let Some(given) = options.and_then(|options| options.labels.as_ref()) {
                let offset = usize::from(show_limits);
                for (label, given) in labels[offset..offset + bins.inner.len()]
                    .iter_mut()
                    .zip(align_labels(bins.inner.len(), given))
                {
                    *label = Label::plain(given.clone());
                }
            }
            return Some(ContinuousGuide {
                domain,
                breaks: positions.into_iter().zip(labels).collect(),
            });
        } else {
            match options.and_then(|options| options.breaks.as_ref()) {
                Some(breaks) => {
                    let kept: Vec<f64> = breaks
                        .iter()
                        .copied()
                        .filter(|value| transform.apply(*value).is_some())
                        .collect();
                    let positions = kept
                        .iter()
                        .filter_map(|value| transform.apply(*value))
                        .collect();
                    let labels = format_breaks(&kept, self.exponent_format);
                    (positions, labelled(&kept, labels))
                }
                // One value (no limits widening it): one key at that value, as
                // ggplot2 (lets-plot lists 5 near-identical breaks, #1267).
                None if self
                    .trained_range()
                    .is_some_and(|(low, high)| is_degenerate_range(low, high)) =>
                {
                    let position = self.trained_range().map_or(domain.0, |(_, high)| high);
                    let value = transform.inverse(position);
                    let labels = labelled(&[value], format_breaks(&[value], self.exponent_format));
                    (vec![position], labels)
                }
                None if transform == Transform::Identity => {
                    let values = linear_breaks(domain.0, domain.1, 5);
                    let labels = labelled(&values, format_breaks(&values, self.exponent_format));
                    (values, labels)
                }
                None => {
                    let mut axis = ContinuousPositionScale::default();
                    axis.transform = transform;
                    axis.multiplicative_expand = 0.0;
                    axis.format = format.cloned();
                    axis.exponent_format = self.exponent_format;
                    axis.train_range(domain.0, domain.1);
                    let breaks = axis.breaks(5);
                    (breaks.positions, breaks.labels)
                }
            }
        };
        if let Some(given) = options.and_then(|options| options.labels.as_ref()) {
            labels = align_labels(positions.len(), given)
                .map(|label| Label::plain(label.clone()))
                .collect();
        }
        Some(ContinuousGuide {
            domain,
            breaks: positions.into_iter().zip(labels).collect(),
        })
    }

    /// `trans=...` of a continuous scale (identity otherwise).
    pub(crate) fn transform(&self) -> Transform {
        self.continuous
            .as_ref()
            .map_or(Transform::Identity, |domain| domain.transform)
    }

    /// `rescaler=...` of a continuous scale.
    fn rescaler(&self) -> Option<&Rescaler> {
        self.continuous.as_ref().ok()?.rescaler.as_ref()
    }

    /// Mapped values of transformed numbers on a continuous scale (for guides).
    /// A failing `rescaler` (already reported when the layers were mapped) maps
    /// them to the NA value.
    pub fn map_numbers(&self, values: &[f64]) -> StyleValues {
        let column = match self.bins() {
            Some(bins) => ContinuousColumn::from_options(
                values.iter().map(|value| bins.midpoint(*value)).collect(),
            ),
            None => ContinuousColumn::from_values(values.to_vec()),
        };
        self.map_guide_column(&column)
    }

    /// Map a guide's column over the scale's domain, through the `rescaler`.
    fn map_guide_column(&self, column: &ContinuousColumn) -> StyleValues {
        let domain = self.continuous_domain();
        match self.rescaler() {
            Some(rescaler) => {
                let rescaled = rescaler
                    .apply(column, domain)
                    .unwrap_or_else(|_| ContinuousColumn::from_options(vec![None; column.len()]));
                self.mapper.map_continuous(&rescaled, domain)
            }
            None => self.mapper.map_continuous(column, domain),
        }
    }

    /// Legend entries of a style scale: a discrete scale's categories, or a
    /// continuous scale's breaks (lets-plot shows those in a legend, not a colorbar,
    /// for sizes and alphas, and for colors when a legend is asked for).
    pub fn legend_entries(&self) -> Option<Vec<LegendEntry>> {
        if let Some(bins) = self.bins() {
            return Some(self.bin_legend_entries(&bins));
        }
        if let Some(guide) = self.continuous_guide() {
            let values: Vec<f64> = guide.breaks.iter().map(|(value, _)| *value).collect();
            let mapped = self.map_numbers(&values);
            return Some(
                guide
                    .breaks
                    .into_iter()
                    .enumerate()
                    .map(|(index, (_, label))| LegendEntry {
                        label,
                        value: single(&mapped, index),
                    })
                    .collect(),
            );
        }
        self.discrete_legend_entries()
    }

    /// The labels of a binned scale's edges, low end first: formatted, the
    /// inner edges taking given `labels` when `given` is set.
    fn edge_labels(&self, bins: &Bins, given: bool) -> Vec<Label> {
        let transform = self.transform();
        let edges = bins.edges();
        let values: Vec<f64> = edges.iter().map(|edge| transform.inverse(*edge)).collect();
        let options = self.continuous.as_ref().ok();
        let format = options.and_then(|options| options.format.as_ref());
        let mut labels: Vec<Label> = match format {
            Some(format) => values
                .iter()
                .map(|&value| format.format(PatternValue::Number(value)))
                .collect(),
            None => format_breaks(&values, self.exponent_format),
        };
        if let Some(given) = options
            .and_then(|options| options.labels.as_ref())
            .filter(|_| given)
        {
            let inner = bins.inner.len();
            for (label, given) in labels[1..=inner].iter_mut().zip(align_labels(inner, given)) {
                *label = Label::plain(given.clone());
            }
        }
        labels
    }

    /// The edge labels of a binned scale for `guide_bins()` (ggplot2): one per
    /// bin edge, low end first; the two ends are blank unless `show_limits`
    /// (the guide's, else the scale's). `None` when the scale is not binned.
    pub fn bin_edge_labels(&self, show_limits: Option<bool>) -> Option<Vec<Label>> {
        let bins = self.bins()?;
        let show_limits = show_limits.unwrap_or_else(|| {
            self.continuous
                .as_ref()
                .ok()
                .and_then(|options| options.binning)
                .is_some_and(|binning| binning.show_limits)
        });
        let mut labels = self.edge_labels(&bins, true);
        if !show_limits {
            let last = labels.len() - 1;
            labels[0] = Label::plain(String::new());
            labels[last] = Label::plain(String::new());
        }
        Some(labels)
    }

    /// Whether the scale is binned (`scale_*_steps()`, `scale_*_binned()`).
    pub fn is_binned(&self) -> bool {
        self.bins().is_some()
    }

    /// One legend entry per bin of a binned scale, low bin first, labelled with
    /// its ends ("20 - 40"); `guide_bins()` labels the edges instead
    /// ([`Self::bin_edge_labels`]).
    fn bin_legend_entries(&self, bins: &Bins) -> Vec<LegendEntry> {
        let edges = bins.edges();
        let labels = self.edge_labels(bins, false);
        let midpoints: Vec<f64> = edges
            .windows(2)
            .map(|pair| (pair[0] + pair[1]) / 2.0)
            .collect();
        let column = ContinuousColumn::from_values(midpoints);
        let mapped = self.map_guide_column(&column);
        labels
            .windows(2)
            .enumerate()
            .map(|(index, pair)| LegendEntry {
                label: Label::plain(format!("{} - {}", pair[0].text, pair[1].text)),
                value: single(&mapped, index),
            })
            .collect()
    }

    /// A category's label without a given one: `format` (numbers formatted as
    /// numbers), else the category itself.
    fn category_label(&self, level: &Level) -> Label {
        match (&self.domain.format, level) {
            (None, _) => Label::plain(level.label()),
            (Some(format), Level::Number(bits)) => {
                format.format(PatternValue::Number(f64::from_bits(*bits)))
            }
            (Some(format), Level::Text(text)) => format.format(PatternValue::Text(text)),
        }
    }

    /// Legend entries of a discrete scale (lets-plot `DiscreteScale.createScaleBreaks`):
    /// the effective domain filtered by `breaks`, labelled by `labels`. `None` for a
    /// continuous scale.
    fn discrete_legend_entries(&self) -> Option<Vec<LegendEntry>> {
        if !self.is_discrete() {
            return None;
        }
        let domain = self.effective_domain();
        let breaks: Vec<(usize, &Level)> = match &self.domain.breaks {
            Some(breaks) => {
                let wanted: HashSet<&Level> = breaks.iter().collect();
                domain
                    .iter()
                    .enumerate()
                    .filter(|(_, level)| wanted.contains(level))
                    .collect()
            }
            None => domain.iter().enumerate().collect(),
        };
        let labels: Vec<Label> = match (&self.domain.labels, &self.domain.breaks) {
            (None, _) => breaks
                .iter()
                .map(|(_, level)| self.category_label(level))
                .collect(),
            (Some(labels), Some(provided)) if self.domain.limits.is_some() => {
                // Labels belong to the provided breaks, reordered by the limits.
                let by_break: HashMap<&Level, &String> = provided
                    .iter()
                    .zip(align_labels(provided.len(), labels))
                    .collect();
                breaks
                    .iter()
                    .map(|(_, level)| {
                        Label::plain(
                            by_break
                                .get(level)
                                .map_or(String::new(), |label| (*label).clone()),
                        )
                    })
                    .collect()
            }
            (Some(labels), _) => align_labels(breaks.len(), labels)
                .map(|label| Label::plain(label.clone()))
                .collect(),
        };
        let indices: Vec<Option<usize>> = breaks.iter().map(|(index, _)| Some(*index)).collect();
        let values = self.mapper.map_levels(&indices, domain).ok()?;
        Some(
            labels
                .into_iter()
                .enumerate()
                .map(|(item, label)| LegendEntry {
                    label: cut_label(label, self.domain.label_limit),
                    value: single(&values, item),
                })
                .collect(),
        )
    }
}

/// lets-plot `alignLablesAndBreaks`: as many labels as breaks, missing ones empty.
fn align_labels(count: usize, labels: &[String]) -> impl Iterator<Item = &String> {
    static EMPTY: String = String::new();
    (0..count).map(move |index| labels.get(index).unwrap_or(&EMPTY))
}

/// Item `index` of per-item values, as a one-item batch.
fn single(values: &StyleValues, index: usize) -> StyleValues {
    use crate::scene::PerItem;
    fn pick<T: Clone>(values: &PerItem<T>, index: usize) -> PerItem<T> {
        match values {
            PerItem::Uniform(value) => PerItem::Uniform(value.clone()),
            PerItem::Varying(items) => PerItem::Uniform(items[index].clone()),
        }
    }
    match values {
        StyleValues::Colors(colors) => StyleValues::Colors(pick(colors, index)),
        StyleValues::Numbers(numbers) => StyleValues::Numbers(pick(numbers, index)),
        StyleValues::Shapes(shapes) => StyleValues::Shapes(pick(shapes, index)),
        StyleValues::Linetypes(linetypes) => StyleValues::Linetypes(pick(linetypes, index)),
        StyleValues::Texts(texts) => StyleValues::Texts(pick(texts, index)),
    }
}

/// lets-plot `Transforms.ensureApplicableDomain` (identity): no data -> -0.5..0.5,
/// a zero-width or beyond-precision range -> its upper end +/- 0.5.
fn applicable_domain(range: Option<(f64, f64)>) -> (f64, f64) {
    match range {
        None => (-0.5, 0.5),
        Some((low, high)) if is_beyond_precision(low, high) => (high - 0.5, high + 0.5),
        Some(range) => range,
    }
}

/// lets-plot `SeriesUtil.isBeyondPrecision(range)` as written: `log10` of a negative
/// end is NaN, so only zero-width ranges count there (unlike axis breaks, which
/// use magnitudes; see `scale::breaks`).
fn is_beyond_precision(low: f64, high: f64) -> bool {
    const MAX_DECIMAL_PLACES: f64 = 12.0;
    let delta = high - low;
    let beyond = |base: f64| base != 0.0 && base.log10() - delta.log10() > MAX_DECIMAL_PLACES;
    delta < 1e-50 || beyond(low) || beyond(high)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow_array::{ArrayRef, RecordBatch, StringArray};

    use super::*;
    use crate::data::DataFrame;
    use crate::scene::{Color, PerItem};

    fn strings(values: Vec<&str>) -> Column {
        let array: ArrayRef = Arc::new(StringArray::from(values));
        let batch = RecordBatch::try_from_iter([("value", array)]).unwrap();
        DataFrame::new(batch.schema(), vec![batch])
            .unwrap()
            .column("value")
            .unwrap()
    }

    fn scale(aesthetic: Aesthetic) -> StyleScale {
        StyleScale::new(
            default_mapper(aesthetic).unwrap(),
            DiscreteDomain::default(),
        )
    }

    fn texts(values: &[&str]) -> Vec<Level> {
        values
            .iter()
            .map(|value| Level::Text((*value).to_owned()))
            .collect()
    }

    #[test]
    fn a_fixed_palette_reports_overflow_by_category_count() {
        let family = ggsci_family("color_jama").unwrap();
        let mapper = GgsciMapper::new(
            family.kind,
            family.palette(None).unwrap(),
            1.0,
            Overflow::Auto,
            NA_COLOR,
        );
        let mut jama = StyleScale::new(Arc::new(mapper.clone()), DiscreteDomain::default());
        jama.train(&strings(vec!["a", "b", "c", "d", "e", "f", "g"]))
            .unwrap();
        assert_eq!(jama.palette_overflow(), None);
        jama.train(&strings(vec!["h", "i"])).unwrap();
        assert_eq!(jama.palette_overflow(), Some((7, 9)));
        // Limits are the categories mapped.
        let limits = DiscreteDomain {
            limits: Some(texts(&["a", "b"])),
            ..DiscreteDomain::default()
        };
        let mut limited = StyleScale::new(Arc::new(mapper), limits);
        limited
            .train(&strings(vec!["a", "b", "c", "d", "e", "f", "g", "h"]))
            .unwrap();
        assert_eq!(limited.palette_overflow(), None);
        // Mappers without a fixed palette never report.
        let mut hue = scale(Aesthetic::Color);
        hue.train(&strings(vec!["a"; 1])).unwrap();
        assert_eq!(hue.palette_overflow(), None);
    }

    #[test]
    fn defaults_have_the_right_kind() {
        for aesthetic in [
            Aesthetic::Color,
            Aesthetic::Fill,
            Aesthetic::PaintC,
            Aesthetic::Size,
            Aesthetic::Alpha,
        ] {
            let found = default_mapper(aesthetic).unwrap();
            assert_eq!(
                aesthetic.kind().style(),
                Some(found.value_kind()),
                "{aesthetic}"
            );
        }
    }

    #[test]
    fn breaks_lead_the_domain_and_limits_replace_it() {
        let data = strings(vec!["b", "a", "c"]);
        let labels = |scale: &StyleScale| -> Vec<String> {
            scale
                .legend_entries()
                .unwrap()
                .into_iter()
                .map(|entry| entry.label.text)
                .collect()
        };
        let mut with_breaks = StyleScale::new(
            default_mapper(Aesthetic::Color).unwrap(),
            DiscreteDomain {
                breaks: Some(texts(&["c", "a"])),
                ..DiscreteDomain::default()
            },
        );
        with_breaks.train(&data).unwrap();
        assert_eq!(with_breaks.effective_domain(), texts(&["c", "a", "b"]));
        assert_eq!(labels(&with_breaks), ["c", "a"]);

        let mut limited = StyleScale::new(
            default_mapper(Aesthetic::Color).unwrap(),
            DiscreteDomain {
                limits: Some(texts(&["c", "a", "b"])),
                breaks: Some(texts(&["b", "c"])),
                labels: Some(vec!["BB".into(), "CC".into()]),
                ..DiscreteDomain::default()
            },
        );
        limited.train(&data).unwrap();
        // Ordered by the limits, labels follow their breaks.
        assert_eq!(labels(&limited), ["CC", "BB"]);
    }

    #[test]
    fn categories_join_across_layers_in_first_appearance() {
        let mut scale = scale(Aesthetic::Color);
        scale.train(&strings(vec!["b", "a"])).unwrap();
        scale.train(&strings(vec!["c", "a"])).unwrap();
        let StyleValues::Colors(PerItem::Varying(colors)) =
            scale.map(&strings(vec!["c", "b"])).unwrap()
        else {
            panic!("expected colors");
        };
        let palette = discrete_palette(3);
        assert_eq!(colors, [palette[2], palette[0]]);
    }

    #[test]
    fn numbers_on_a_discrete_scale_become_categories() {
        let numbers =
            Column::Continuous(ContinuousColumn::from_values(vec![3.0, 1.0, 3.0, f64::NAN]));
        let mut scale = StyleScale::discrete(
            default_mapper(Aesthetic::Color).unwrap(),
            DiscreteDomain::default(),
        );
        scale.train(&strings(vec!["a", "3"])).unwrap();
        scale.train(&numbers).unwrap();
        let StyleValues::Colors(PerItem::Varying(colors)) = scale.map(&numbers).unwrap() else {
            panic!("expected colors");
        };
        let palette = discrete_palette(4);
        assert_eq!(colors, [palette[2], palette[3], palette[2], NA_COLOR]);
    }

    #[test]
    fn discrete_labels_are_formatted_then_cut() {
        let numbers = Column::Continuous(ContinuousColumn::from_values(vec![1.5, 30.25]));
        let labels = |format: &str, label_limit: Option<usize>| -> Vec<String> {
            let mut scale = StyleScale::discrete(
                default_mapper(Aesthetic::Color).unwrap(),
                DiscreteDomain {
                    format: Some(LabelPattern::parse(format).unwrap()),
                    label_limit,
                    ..DiscreteDomain::default()
                },
            );
            scale.train(&strings(vec!["alphabet"])).unwrap();
            scale.train(&numbers).unwrap();
            let entries = scale.legend_entries().unwrap();
            entries.into_iter().map(|entry| entry.label.text).collect()
        };
        // Numeric categories format as numbers, texts as texts (lets-plot).
        assert_eq!(labels(".2f", None), ["alphabet", "1.50", "30.25"]);
        assert_eq!(labels("<{}>", Some(3)), ["<al...", "<1....", "<30..."]);
        assert_eq!(cut_label(Label::plain("abc"), Some(3)).text, "abc");
    }

    #[test]
    fn continuous_labels_follow_format_unless_given() {
        let mut scale = StyleScale::new(
            default_mapper(Aesthetic::Color).unwrap(),
            DiscreteDomain::default(),
        );
        scale.continuous = Ok(ContinuousDomain {
            breaks: Some(vec![1200.0, 2800.0]),
            format: Some(LabelPattern::parse("{d}$").unwrap()),
            ..ContinuousDomain::default()
        });
        scale
            .train(&Column::Continuous(ContinuousColumn::from_values(vec![
                1000.0, 3000.0,
            ])))
            .unwrap();
        let texts = |scale: &StyleScale| -> Vec<String> {
            let guide = scale.continuous_guide().unwrap();
            guide
                .breaks
                .into_iter()
                .map(|(_, label)| label.text)
                .collect()
        };
        assert_eq!(texts(&scale), ["1200$", "2800$"]);
        if let Ok(domain) = &mut scale.continuous {
            domain.labels = Some(vec!["low".into()]);
        }
        assert_eq!(texts(&scale), ["low", ""]);
    }

    #[test]
    fn a_discrete_column_on_a_continuous_scale_is_an_error() {
        let mut scale = scale(Aesthetic::Size);
        scale
            .train(&Column::Continuous(ContinuousColumn::from_values(vec![
                1.0,
            ])))
            .unwrap();
        assert!(scale.train(&strings(vec!["a"])).is_err());
    }

    #[test]
    fn near_constant_values_map_to_the_middle() {
        let mut scale = scale(Aesthetic::Size);
        let column = Column::Continuous(ContinuousColumn::from_values(vec![1.0, 1.0 + 1e-14]));
        scale.train(&column).unwrap();
        let StyleValues::Numbers(PerItem::Varying(sizes)) = scale.map(&column).unwrap() else {
            panic!("expected sizes");
        };
        for size in sizes {
            assert!((size * 2.2 - 12.0).abs() < 1e-9, "{size}");
        }
    }

    #[test]
    fn near_constant_negative_values_spread_like_lets_plot() {
        let mut scale = scale(Aesthetic::Size);
        let column = Column::Continuous(ContinuousColumn::from_values(vec![-1.0, -1.0 - 1e-14]));
        scale.train(&column).unwrap();
        let StyleValues::Numbers(PerItem::Varying(sizes)) = scale.map(&column).unwrap() else {
            panic!("expected sizes");
        };
        assert!((sizes[0] - sizes[1]).abs() > 1.0, "{sizes:?}");
    }

    #[test]
    fn single_values_map_to_the_middle() {
        let mut scale = scale(Aesthetic::Size);
        let column = Column::Continuous(ContinuousColumn::from_values(vec![5.0, 5.0]));
        scale.train(&column).unwrap();
        let StyleValues::Numbers(PerItem::Varying(sizes)) = scale.map(&column).unwrap() else {
            panic!("expected sizes");
        };
        assert!((sizes[0] * 2.2 - 12.0).abs() < 1e-12, "{sizes:?}");
    }

    /// A binned default color scale trained on `values`.
    fn binned(values: Vec<f64>, binning: Binning, breaks: Option<Vec<f64>>) -> StyleScale {
        let mut scale = StyleScale::new(
            default_mapper(Aesthetic::Color).unwrap(),
            DiscreteDomain::default(),
        );
        scale.continuous = Ok(ContinuousDomain {
            binning: Some(binning),
            breaks,
            ..ContinuousDomain::default()
        });
        scale
            .train(&Column::Continuous(ContinuousColumn::from_values(values)))
            .unwrap();
        scale
    }

    const BINNING: Binning = Binning {
        n_breaks: None,
        nice: true,
        right: true,
        show_limits: false,
    };

    #[test]
    fn binned_ends_move_out_to_full_bins() {
        // ggplot2 `ScaleBinned$get_breaks`: 3..99 with breaks 20..80 bins 0..100.
        let scale = binned(vec![3.0, 50.0, 99.0], BINNING, None);
        let bins = scale.bins().unwrap();
        assert_eq!(bins.domain, (0.0, 100.0));
        assert_eq!(bins.inner, [20.0, 40.0, 60.0, 80.0]);
    }

    #[test]
    fn bin_edge_labels_blank_the_ends_unless_limits_are_shown() {
        let texts = |labels: Vec<Label>| -> Vec<String> {
            labels.into_iter().map(|label| label.text).collect()
        };
        let steps = binned(vec![3.0, 50.0, 99.0], BINNING, None);
        assert_eq!(
            texts(steps.bin_edge_labels(None).unwrap()),
            ["", "20", "40", "60", "80", ""]
        );
        assert_eq!(
            texts(steps.bin_edge_labels(Some(true)).unwrap()),
            ["0", "20", "40", "60", "80", "100"]
        );
        // The scale's show_limits, unless the guide says otherwise.
        let shown = binned(
            vec![3.0, 50.0, 99.0],
            Binning {
                show_limits: true,
                ..BINNING
            },
            None,
        );
        assert_eq!(texts(shown.bin_edge_labels(None).unwrap())[0], "0");
        assert_eq!(texts(shown.bin_edge_labels(Some(false)).unwrap())[0], "");
        assert!(steps.is_binned());
        assert!(scale(Aesthetic::Fill).bin_edge_labels(None).is_none());
    }

    #[test]
    fn binned_values_take_their_bin_midpoint_closed_on_the_right() {
        let bins = Bins {
            domain: (0.0, 30.0),
            inner: vec![10.0, 20.0],
            right: true,
        };
        assert_eq!(bins.midpoint(0.0), Some(5.0));
        assert_eq!(bins.midpoint(10.0), Some(5.0));
        assert_eq!(bins.midpoint(10.5), Some(15.0));
        assert_eq!(bins.midpoint(30.0), Some(25.0));
        assert_eq!(bins.midpoint(31.0), None);
        let left = Bins {
            right: false,
            ..bins
        };
        assert_eq!(left.midpoint(10.0), Some(15.0));
        assert_eq!(left.midpoint(30.0), Some(25.0));
    }

    #[test]
    fn binned_values_share_their_bin_color() {
        let scale = binned(vec![3.0, 17.0, 22.0, 99.0], BINNING, None);
        let column = Column::Continuous(ContinuousColumn::from_values(vec![3.0, 17.0, 22.0]));
        let StyleValues::Colors(colors) = scale.map(&column).unwrap() else {
            panic!("colors expected");
        };
        assert_eq!(colors.get(0), colors.get(1));
        assert_ne!(colors.get(1), colors.get(2));
    }

    #[test]
    fn steps_guide_is_in_bin_index_space() {
        let scale = binned(vec![0.0, 100.0], BINNING, Some(vec![20.0, 30.0, 70.0]));
        let (guide, midpoints) = scale.steps_guide().unwrap();
        assert_eq!(guide.domain, (0.0, 4.0));
        let positions: Vec<f64> = guide.breaks.iter().map(|(value, _)| *value).collect();
        assert_eq!(positions, [1.0, 2.0, 3.0]);
        assert_eq!(midpoints, [85.0, 50.0, 25.0, 10.0]);
        let limits = binned(
            vec![0.0, 100.0],
            Binning {
                show_limits: true,
                ..BINNING
            },
            Some(vec![50.0]),
        );
        let labels: Vec<String> = limits
            .steps_guide()
            .unwrap()
            .0
            .breaks
            .iter()
            .map(|(_, label)| label.text.clone())
            .collect();
        assert_eq!(labels, ["0", "50", "100"]);
    }

    /// Colors of `values` on a gradient scale with `limits` and `oob`, trained on
    /// the same values.
    fn limited_colors(
        values: Vec<f64>,
        limits: [Option<f64>; 2],
        oob: Oob,
        transform: Transform,
    ) -> Vec<Color> {
        let mut scale = scale(Aesthetic::Fill);
        scale.continuous = Ok(ContinuousDomain {
            limits,
            oob,
            transform,
            ..ContinuousDomain::default()
        });
        let column = Column::Continuous(ContinuousColumn::from_values(values));
        scale.train(&column).unwrap();
        let StyleValues::Colors(PerItem::Varying(colors)) = scale.map(&column).unwrap() else {
            panic!("expected colors");
        };
        colors
    }

    /// One rescaler call: values, `to`, `from`.
    type RescaleCall = (Vec<f64>, (f64, f64), (f64, f64));

    /// A rescaler that reverses the range and remembers what it was called with.
    struct Reverse(std::sync::Mutex<Vec<RescaleCall>>);

    impl crate::scale::secondary::AxisFunction for Reverse {
        fn call(&self, _: &[f64]) -> std::result::Result<Vec<f64>, String> {
            Err("not a transform".into())
        }

        fn rescale(
            &self,
            values: &[f64],
            to: (f64, f64),
            from: (f64, f64),
        ) -> std::result::Result<Vec<f64>, String> {
            self.0.lock().unwrap().push((values.to_vec(), to, from));
            // 1 - linear, and 2 (outside 0..1) for the value 3.
            Ok(values
                .iter()
                .map(|value| match *value {
                    3.0 => 2.0,
                    value => 1.0 - (value - from.0) / (from.1 - from.0),
                })
                .collect())
        }
    }

    fn rescaled_scale(values: &[f64]) -> (StyleScale, Column, Arc<Reverse>) {
        let reverse = Arc::new(Reverse(std::sync::Mutex::new(Vec::new())));
        let mut scale = scale(Aesthetic::Fill);
        scale.continuous = Ok(ContinuousDomain {
            rescaler: Some(Rescaler(reverse.clone())),
            ..ContinuousDomain::default()
        });
        let column = Column::Continuous(ContinuousColumn::from_values(values.to_vec()));
        scale.train(&column).unwrap();
        (scale, column, reverse)
    }

    #[test]
    fn a_rescaler_maps_values_before_the_palette() {
        let (scale, column, reverse) = rescaled_scale(&[0.0, 1.0, 3.0, 4.0, f64::NAN]);
        let StyleValues::Colors(PerItem::Varying(colors)) = scale.map(&column).unwrap() else {
            panic!("expected colors");
        };
        let plain = limited_colors(
            vec![4.0, 3.0, 1.0, 0.0],
            [None, None],
            Oob::Censor,
            Transform::Identity,
        );
        // Reversed: 0 takes the color of 4, 4 the color of 0.
        assert_eq!(colors[0], plain[0]);
        assert_eq!(colors[1], plain[1]);
        assert_eq!(colors[3], plain[3]);
        // Outside 0..1, and missing: the NA color.
        assert_eq!(colors[2], NA_COLOR);
        assert_eq!(colors[4], NA_COLOR);
        // One call with the finite values, to = 0..1, from = the domain.
        let calls = reverse.0.lock().unwrap();
        assert_eq!(
            calls.as_slice(),
            [(vec![0.0, 1.0, 3.0, 4.0], (0.0, 1.0), (0.0, 4.0))]
        );
    }

    #[test]
    fn a_rescaler_also_colors_the_guide() {
        let (scale, _, _) = rescaled_scale(&[0.0, 4.0]);
        let StyleValues::Colors(guide) = scale.map_numbers(&[0.0, 4.0]) else {
            panic!("expected colors");
        };
        let plain = limited_colors(
            vec![0.0, 4.0],
            [None, None],
            Oob::Censor,
            Transform::Identity,
        );
        assert_eq!(guide.get(0), plain[1]);
        assert_eq!(guide.get(1), plain[0]);
    }

    #[test]
    fn squish_clamps_to_the_limits_and_censor_drops() {
        let values = vec![-5.0, -2.5, 0.0, 2.5, 5.0, f64::NAN];
        let limits = [Some(-2.5), Some(2.5)];
        let squished = limited_colors(values.clone(), limits, Oob::Squish, Transform::Identity);
        assert_eq!(squished[0], squished[1]);
        assert_eq!(squished[4], squished[3]);
        assert_ne!(squished[0], squished[4]);
        assert_eq!(squished[5], NA_COLOR);
        let censored = limited_colors(values, limits, Oob::Censor, Transform::Identity);
        assert_eq!(censored[0], NA_COLOR);
        assert_eq!(censored[4], NA_COLOR);
        assert_eq!(censored[1..4], squished[1..4]);
    }

    #[test]
    fn squish_keeps_infinite_values_missing_and_open_ends_open() {
        let squished = limited_colors(
            vec![f64::INFINITY, 10.0, 2.0, 0.0, -10.0],
            [None, Some(2.0)],
            Oob::Squish,
            Transform::Identity,
        );
        assert_eq!(squished[0], NA_COLOR);
        assert_eq!(squished[1], squished[2]);
        // No low limit: -10 is the data minimum, not clamped.
        assert_ne!(squished[4], squished[3]);
    }

    #[test]
    fn squish_clamps_in_data_units_before_the_transform() {
        let squished = limited_colors(
            vec![1.0, 10.0, 100.0, 1000.0],
            [Some(10.0), Some(100.0)],
            Oob::Squish,
            Transform::Log10,
        );
        assert_eq!(squished[0], squished[1]);
        assert_eq!(squished[3], squished[2]);
        assert_ne!(squished[1], squished[2]);
    }
}
