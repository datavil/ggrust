//! `scale_*_identity()` (lets-plot `IdentityMapperProvider`): the data are the
//! values. Categories are converted as option values are (`"red"` a color, `"2"` a
//! number, `21` a shape); numbers pass through for sizes, alphas and shapes.
//! A category that does not convert fails the plot, as in lets-plot; missing
//! values and numbers that do not fit map to the scale's NA value.

use super::{Level, StyleMapper};
use crate::data::ContinuousColumn;
use crate::scene::{Color, PerItem};
use crate::spec::Value;
use crate::style::{StyleValues, ValueKind, parse_constant};

/// Identity mapping onto one value kind.
#[derive(Debug, Clone, PartialEq)]
pub struct IdentityMapper {
    /// The kind produced.
    pub kind: ValueKind,
    /// Value of missing or unconvertible data (a one-item batch of `kind`).
    pub na: StyleValues,
}

impl IdentityMapper {
    /// One value converted from a category or a number; `None` if it does not fit.
    fn convert(&self, value: &Value) -> Option<StyleValues> {
        let value = match (self.kind, value) {
            // Categories holding numbers: "2" is 2 for numeric kinds.
            (ValueKind::Number | ValueKind::Shape, Value::String(text)) => {
                serde_json::Number::from_f64(text.trim().parse().ok()?).map(Value::Number)?
            }
            (ValueKind::Shape, Value::Number(number)) => {
                let code = number.as_f64()?;
                (code.fract() == 0.0 && (0.0..=25.0).contains(&code))
                    .then(|| Value::from(code as u64))?
            }
            (_, value) => value.clone(),
        };
        parse_constant(self.kind, &value).ok()
    }

    /// Gather one-item values into a batch, NA where `None`.
    fn batch(&self, values: impl Iterator<Item = Option<StyleValues>>) -> StyleValues {
        fn item<T: Copy>(values: &PerItem<T>) -> T {
            values.get(0)
        }
        let values: Vec<StyleValues> = values
            .map(|value| value.unwrap_or_else(|| self.na.clone()))
            .collect();
        match self.kind {
            ValueKind::Color => StyleValues::Colors(PerItem::Varying(
                values
                    .iter()
                    .map(|value| match value {
                        StyleValues::Colors(colors) => item(colors),
                        _ => Color::TRANSPARENT,
                    })
                    .collect(),
            )),
            ValueKind::Number => StyleValues::Numbers(PerItem::Varying(
                values
                    .iter()
                    .map(|value| match value {
                        StyleValues::Numbers(numbers) => item(numbers),
                        _ => f64::NAN,
                    })
                    .collect(),
            )),
            ValueKind::Shape => StyleValues::Shapes(PerItem::Varying(
                values
                    .iter()
                    .map(|value| match value {
                        StyleValues::Shapes(shapes) => item(shapes),
                        _ => super::NA_SHAPE,
                    })
                    .collect(),
            )),
            ValueKind::Linetype | ValueKind::Text => {
                // Not style-scaled yet (no default mapper reaches here).
                self.na.clone()
            }
        }
    }
}

impl StyleMapper for IdentityMapper {
    fn name(&self) -> &'static str {
        "identity"
    }

    fn value_kind(&self) -> ValueKind {
        self.kind
    }

    fn map_continuous(&self, column: &ContinuousColumn, _domain: (f64, f64)) -> StyleValues {
        self.batch((0..column.len()).map(|row| {
            let value = column.get(row).filter(|value| value.is_finite())?;
            self.convert(&Value::Number(serde_json::Number::from_f64(value)?))
        }))
    }

    fn map_discrete(&self, indices: &[Option<usize>], _level_count: usize) -> StyleValues {
        self.batch(indices.iter().map(|_| None))
    }

    fn map_levels(
        &self,
        indices: &[Option<usize>],
        levels: &[Level],
    ) -> Result<StyleValues, String> {
        let converted = levels
            .iter()
            .map(|level| match level {
                Level::Text(text) => self.convert(&Value::String(text.clone())).ok_or_else(|| {
                    format!(
                        "identity scale: can't use '{text}' as a {}",
                        kind_name(self.kind)
                    )
                }),
                Level::Number(bits) => {
                    let value = f64::from_bits(*bits);
                    serde_json::Number::from_f64(value)
                        .and_then(|number| self.convert(&Value::Number(number)))
                        .ok_or_else(|| {
                            format!(
                                "identity scale: can't use {value} as a {}",
                                kind_name(self.kind)
                            )
                        })
                }
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(self.batch(
            indices
                .iter()
                .map(|index| index.and_then(|index| converted.get(index).cloned())),
        ))
    }
}

fn kind_name(kind: ValueKind) -> &'static str {
    match kind {
        ValueKind::Color => "color",
        ValueKind::Number => "number",
        ValueKind::Shape => "shape code (0..25)",
        ValueKind::Linetype => "line type",
        ValueKind::Text => "text",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_and_numbers_pass_through() {
        let colors = IdentityMapper {
            kind: ValueKind::Color,
            na: StyleValues::Colors(PerItem::Uniform(Color::rgb(128, 128, 128))),
        };
        let levels = [Level::Text("red".into())];
        let StyleValues::Colors(PerItem::Varying(mapped)) =
            colors.map_levels(&[Some(0), None], &levels).unwrap()
        else {
            panic!("expected colors");
        };
        assert_eq!(mapped[0], Color::rgb(255, 0, 0));
        assert_eq!(mapped[1], Color::rgb(128, 128, 128));
        let error = colors
            .map_levels(&[Some(0)], &[Level::Text("nope".into())])
            .unwrap_err();
        assert!(error.contains("'nope' as a color"), "{error}");
        let sizes = IdentityMapper {
            kind: ValueKind::Number,
            na: StyleValues::Numbers(PerItem::Uniform(0.0)),
        };
        let column = ContinuousColumn::from_options(vec![Some(4.0), None]);
        assert_eq!(
            sizes.map_continuous(&column, (0.0, 1.0)),
            StyleValues::Numbers(PerItem::Varying(vec![4.0, 0.0]))
        );
        let shapes = IdentityMapper {
            kind: ValueKind::Shape,
            na: StyleValues::Shapes(PerItem::Uniform(super::super::NA_SHAPE)),
        };
        let column = ContinuousColumn::from_options(vec![Some(21.0), Some(2.5)]);
        assert_eq!(
            shapes.map_continuous(&column, (0.0, 1.0)),
            StyleValues::Shapes(PerItem::Varying(vec![21, super::super::NA_SHAPE]))
        );
    }
}
