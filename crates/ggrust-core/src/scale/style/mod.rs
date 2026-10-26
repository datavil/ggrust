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
mod gradient;
mod hue;
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
pub use gradient::{GRADIENT2_HIGH, GRADIENT2_LOW, GRADIENT2_MID, Gradient2Mapper, GradientMapper};
pub use hue::HueMapper;
pub use manual::ManualColorMapper;
pub use numeric::{ALPHA, LinearMapper, SIZE};
pub use shape::{ALL_SHAPES, HOLLOW_SHAPES, NA_SHAPE, ShapeMapper};

use super::breaks::linear_breaks;
use super::format::{Label, format_breaks};
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

    /// Whether only continuous data can be mapped (`scale_*_gradient2()`).
    fn continuous_only(&self) -> bool {
        false
    }
}

/// The mapper lets-plot uses for an aesthetic without a `scale_*()` call.
pub fn default_mapper(aesthetic: Aesthetic) -> Option<Arc<dyn StyleMapper>> {
    match aesthetic {
        Aesthetic::Color | Aesthetic::Fill => Some(Arc::new(DefaultColorMapper { na: NA_COLOR })),
        Aesthetic::Size => Some(Arc::new(SIZE.clone())),
        Aesthetic::Alpha => Some(Arc::new(ALPHA.clone())),
        Aesthetic::Shape => Some(Arc::new(ShapeMapper::all())),
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
}

/// Continuous domain options from `scale_*()`: numeric `limits`, `breaks`, `labels`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ContinuousDomain {
    /// `[low, high]`, either side open; values outside map to the NA value.
    pub limits: [Option<f64>; 2],
    /// Breaks shown in the legend (default: about 5 nice breaks).
    pub breaks: Option<Vec<f64>>,
    /// Labels aligned with the breaks.
    pub labels: Option<Vec<String>>,
}

/// Breaks of a continuous style scale, for a colorbar or a legend.
#[derive(Debug, Clone, PartialEq)]
pub struct ContinuousGuide {
    /// The domain the mapper uses.
    pub domain: (f64, f64),
    /// Break values with their labels, ascending.
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
        match (&mut self.trained, column) {
            (Trained::Continuous(range), Column::Continuous(column)) => {
                for row in 0..column.len() {
                    if let Some(value) = column
                        .get(row)
                        .filter(|value| value.is_finite() && within(*value))
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

    /// The categories values are mapped by: `limits` when given, else the trained
    /// levels (lets-plot `DiscreteTransform.effectiveDomain`). Empty if continuous.
    fn effective_domain(&self) -> &[Level] {
        match (&self.trained, &self.domain.limits) {
            (Trained::Discrete { .. }, Some(limits)) => limits,
            (Trained::Discrete { levels, .. }, None) => levels,
            _ => &[],
        }
    }

    /// Map a column with the trained domain.
    pub fn map(&self, column: &Column) -> Result<StyleValues, String> {
        match (&self.trained, column) {
            (Trained::Continuous(_), Column::Continuous(column)) => {
                let domain = self.continuous_domain();
                let limits = self
                    .continuous
                    .as_ref()
                    .map_or([None, None], |domain| domain.limits);
                if limits == [None, None] {
                    return Ok(self.mapper.map_continuous(column, domain));
                }
                // lets-plot: values outside the limits are missing.
                let limited = ContinuousColumn::from_options(
                    (0..column.len())
                        .map(|row| {
                            column.get(row).filter(|value| {
                                limits[0].is_none_or(|low| *value >= low)
                                    && limits[1].is_none_or(|high| *value <= high)
                            })
                        })
                        .collect(),
                );
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
                Ok(self.mapper.map_discrete(&indices, domain.len()))
            }
            _ => Err("the scale was not trained on a column of this kind".to_owned()),
        }
    }

    /// The domain continuous data is mapped over: the data range within the
    /// limits, widened to the limits (lets-plot `rangeWithLimitsAfterTransform`),
    /// then made applicable (never zero-width).
    fn continuous_domain(&self) -> (f64, f64) {
        let Trained::Continuous(range) = self.trained else {
            return (-0.5, 0.5);
        };
        let limits = self
            .continuous
            .as_ref()
            .map_or([None, None], |domain| domain.limits);
        let mut ends: Vec<f64> = limits.iter().flatten().copied().collect();
        if let Some((low, high)) = range {
            ends.extend([low, high]);
        }
        let low = ends.iter().copied().fold(f64::INFINITY, f64::min);
        let high = ends.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        applicable_domain((low <= high).then_some((low, high)))
    }

    /// Breaks of a continuous scale for its guide (`None` if discrete): the given
    /// `breaks` or about 5 nice ones, labelled by `labels` or formatted.
    pub fn continuous_guide(&self) -> Option<ContinuousGuide> {
        if !matches!(self.trained, Trained::Continuous(_)) {
            return None;
        }
        let domain = self.continuous_domain();
        let options = self.continuous.as_ref().ok();
        let values: Vec<f64> = match options.and_then(|options| options.breaks.clone()) {
            Some(breaks) => breaks,
            None => linear_breaks(domain.0, domain.1, 5),
        };
        let labels: Vec<Label> = match options.and_then(|options| options.labels.as_ref()) {
            Some(labels) => align_labels(values.len(), labels)
                .map(|label| Label::plain(label.clone()))
                .collect(),
            None => format_breaks(&values),
        };
        Some(ContinuousGuide {
            domain,
            breaks: values.into_iter().zip(labels).collect(),
        })
    }

    /// Mapped values of numbers on a continuous scale (for guides).
    pub fn map_numbers(&self, values: &[f64]) -> StyleValues {
        let column = ContinuousColumn::from_values(values.to_vec());
        self.mapper
            .map_continuous(&column, self.continuous_domain())
    }

    /// Legend entries of a style scale: a discrete scale's categories, or a
    /// continuous scale's breaks (lets-plot shows those in a legend, not a colorbar,
    /// for sizes and alphas, and for colors when a legend is asked for).
    pub fn legend_entries(&self) -> Option<Vec<LegendEntry>> {
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
        let labels: Vec<String> = match (&self.domain.labels, &self.domain.breaks) {
            (None, _) => breaks.iter().map(|(_, level)| level.label()).collect(),
            (Some(labels), Some(provided)) if self.domain.limits.is_some() => {
                // Labels belong to the provided breaks, reordered by the limits.
                let by_break: HashMap<&Level, &String> = provided
                    .iter()
                    .zip(align_labels(provided.len(), labels))
                    .collect();
                breaks
                    .iter()
                    .map(|(_, level)| {
                        by_break
                            .get(level)
                            .map_or(String::new(), |label| (*label).clone())
                    })
                    .collect()
            }
            (Some(labels), _) => align_labels(breaks.len(), labels).cloned().collect(),
        };
        let indices: Vec<Option<usize>> = breaks.iter().map(|(index, _)| Some(*index)).collect();
        let values = self.mapper.map_discrete(&indices, domain.len());
        Some(
            labels
                .into_iter()
                .enumerate()
                .map(|(item, label)| LegendEntry {
                    label: Label::plain(label),
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
    use crate::scene::PerItem;

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
    fn defaults_have_the_right_kind() {
        for aesthetic in [
            Aesthetic::Color,
            Aesthetic::Fill,
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
}
