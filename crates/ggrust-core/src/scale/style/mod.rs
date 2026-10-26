//! Style scales: map data to colors, sizes and alphas (lets-plot mapper providers).
//!
//! A [`StyleMapper`] turns a column into typed [`StyleValues`]; a [`StyleScale`] is
//! one mapper plus the domain trained over every layer (categories joined in order
//! of first appearance, or the numeric range). Add a mapper = one file + one line in
//! [`STYLE_MAPPERS`] (and in [`default_mapper`] when it is an aesthetic's default).
//!
//! Not here yet: `scale_color_*()` options, legends, shapes and line types.

mod color;
pub mod color_space;
mod numeric;

use std::collections::HashMap;

pub use color::{DefaultColorMapper, NA_COLOR, discrete_palette};
pub use numeric::{ALPHA, LinearMapper, SIZE};

use crate::data::{Column, ContinuousColumn};
use crate::spec::Aesthetic;
use crate::style::{StyleValues, ValueKind};

/// Maps data to visual values of one kind.
pub trait StyleMapper: Sync {
    /// Registry name, e.g. `color_default`.
    fn name(&self) -> &'static str;

    /// The kind of values produced.
    fn value_kind(&self) -> ValueKind;

    /// Values for continuous data; `domain` is the trained range (never zero-width).
    fn map_continuous(&self, column: &ContinuousColumn, domain: (f64, f64)) -> StyleValues;

    /// Values for discrete data: each row's index into the trained categories
    /// (`None`: missing or unknown).
    fn map_discrete(&self, indices: &[Option<usize>], level_count: usize) -> StyleValues;
}

/// Every style mapper, in one place: lookup and names derive from it.
pub static STYLE_MAPPERS: &[&dyn StyleMapper] = &[&DefaultColorMapper, &SIZE, &ALPHA];

/// Look up a mapper by name.
pub fn mapper(name: &str) -> Option<&'static dyn StyleMapper> {
    STYLE_MAPPERS
        .iter()
        .copied()
        .find(|item| item.name() == name)
}

/// The mapper lets-plot uses for an aesthetic without a `scale_*()` call.
pub fn default_mapper(aesthetic: Aesthetic) -> Option<&'static dyn StyleMapper> {
    let name = match aesthetic {
        Aesthetic::Color | Aesthetic::Fill => "color_default",
        Aesthetic::Size => "size",
        Aesthetic::Alpha => "alpha",
        _ => return None,
    };
    mapper(name)
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
enum Level {
    Text(String),
    /// `f64` bits: -0.0 and 0.0 differ, as in lets-plot.
    Number(u64),
}

impl Level {
    fn number(value: f64) -> Self {
        Level::Number(value.to_bits())
    }
}

/// What a style scale has seen.
#[derive(Debug, Clone, PartialEq, Default)]
enum Trained {
    #[default]
    Nothing,
    Continuous(Option<(f64, f64)>),
    /// Categories in order of first appearance.
    Discrete {
        levels: Vec<Level>,
        index_by_level: HashMap<Level, usize>,
    },
}

impl Trained {
    fn discrete() -> Self {
        Trained::Discrete {
            levels: Vec::new(),
            index_by_level: HashMap::new(),
        }
    }
}

/// A style mapper with its domain trained over all layers.
#[derive(Clone)]
pub struct StyleScale {
    mapper: &'static dyn StyleMapper,
    trained: Trained,
}

impl std::fmt::Debug for StyleScale {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StyleScale")
            .field("mapper", &self.mapper.name())
            .field("trained", &self.trained)
            .finish()
    }
}

impl StyleScale {
    /// An untrained scale; its kind follows the first column trained.
    pub fn new(mapper: &'static dyn StyleMapper) -> Self {
        StyleScale {
            mapper,
            trained: Trained::Nothing,
        }
    }

    /// An untrained discrete scale: continuous columns trained on it become
    /// categories (lets-plot makes a scale discrete when any layer maps a discrete
    /// column to it).
    pub fn discrete(mapper: &'static dyn StyleMapper) -> Self {
        StyleScale {
            mapper,
            trained: Trained::discrete(),
        }
    }

    /// The mapper.
    pub fn mapper(&self) -> &'static dyn StyleMapper {
        self.mapper
    }

    /// Learn the domain from a column. A discrete column cannot be trained on a scale
    /// that is already continuous: create it with [`StyleScale::discrete`] instead.
    pub fn train(&mut self, column: &Column) -> Result<(), String> {
        match (&self.trained, column) {
            (Trained::Nothing, Column::Continuous(_)) => {
                self.trained = Trained::Continuous(None);
            }
            (Trained::Nothing, Column::Discrete(_)) => self.trained = Trained::discrete(),
            (Trained::Continuous(_), Column::Discrete(_)) => {
                return Err(
                    "a discrete column was trained on a continuous style scale; \
                            create the scale as discrete"
                        .to_owned(),
                );
            }
            _ => {}
        }
        match (&mut self.trained, column) {
            (Trained::Continuous(range), Column::Continuous(column)) => {
                for row in 0..column.len() {
                    if let Some(value) = column.get(row).filter(|value| value.is_finite()) {
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

    /// Map a column with the trained domain.
    pub fn map(&self, column: &Column) -> Result<StyleValues, String> {
        match (&self.trained, column) {
            (Trained::Continuous(range), Column::Continuous(column)) => Ok(self
                .mapper
                .map_continuous(column, applicable_domain(*range))),
            (
                Trained::Discrete {
                    levels,
                    index_by_level,
                },
                column,
            ) => {
                let indices: Vec<Option<usize>> = match column {
                    Column::Discrete(column) => {
                        let index_by_code: Vec<Option<usize>> = column
                            .levels()
                            .iter()
                            .map(|level| index_by_level.get(&Level::Text(level.clone())).copied())
                            .collect();
                        (0..column.len())
                            .map(|row| {
                                column
                                    .code(row)
                                    .and_then(|code| index_by_code[code as usize])
                            })
                            .collect()
                    }
                    Column::Continuous(column) => {
                        (0..column.len())
                            .map(|row| {
                                column.get(row).filter(|value| value.is_finite()).and_then(
                                    |value| index_by_level.get(&Level::number(value)).copied(),
                                )
                            })
                            .collect()
                    }
                };
                Ok(self.mapper.map_discrete(&indices, levels.len()))
            }
            _ => Err("the scale was not trained on a column of this kind".to_owned()),
        }
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

    #[test]
    fn registry_names_round_trip_and_defaults_have_the_right_kind() {
        for item in STYLE_MAPPERS {
            assert_eq!(
                mapper(item.name()).map(|found| found.name()),
                Some(item.name())
            );
        }
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
    fn categories_join_across_layers_in_first_appearance() {
        let mut scale = StyleScale::new(&DefaultColorMapper);
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
        let mut scale = StyleScale::discrete(&DefaultColorMapper);
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
        let mut scale = StyleScale::new(&SIZE);
        scale
            .train(&Column::Continuous(ContinuousColumn::from_values(vec![
                1.0,
            ])))
            .unwrap();
        assert!(scale.train(&strings(vec!["a"])).is_err());
    }

    #[test]
    fn near_constant_values_map_to_the_middle() {
        let mut scale = StyleScale::new(&SIZE);
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
        let mut scale = StyleScale::new(&SIZE);
        let column = Column::Continuous(ContinuousColumn::from_values(vec![-1.0, -1.0 - 1e-14]));
        scale.train(&column).unwrap();
        let StyleValues::Numbers(PerItem::Varying(sizes)) = scale.map(&column).unwrap() else {
            panic!("expected sizes");
        };
        assert!((sizes[0] - sizes[1]).abs() > 1.0, "{sizes:?}");
    }

    #[test]
    fn single_values_map_to_the_middle() {
        let mut scale = StyleScale::new(&SIZE);
        let column = Column::Continuous(ContinuousColumn::from_values(vec![5.0, 5.0]));
        scale.train(&column).unwrap();
        let StyleValues::Numbers(PerItem::Varying(sizes)) = scale.map(&column).unwrap() else {
            panic!("expected sizes");
        };
        assert!((sizes[0] * 2.2 - 12.0).abs() < 1e-12, "{sizes:?}");
    }
}
