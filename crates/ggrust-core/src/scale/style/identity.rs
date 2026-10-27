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
            // lets-plot rounds data on a shape identity scale (2.7 is 3), unlike
            // constants, which truncate.
            (ValueKind::Shape, Value::Number(number)) => {
                let code = number.as_f64()?.round();
                (0.0..=25.0)
                    .contains(&code)
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
            if self.kind == ValueKind::Color {
                return rgb_integer(value);
            }
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
        // lets-plot maps missing categories to null and drops those rows (its
        // `na_value` is not used here): they get a value that is not drawn.
        let hidden = self.hidden();
        Ok(self.batch(indices.iter().map(|index| {
            Some(
                index
                    .and_then(|index| converted.get(index).cloned())
                    .unwrap_or_else(|| hidden.clone()),
            )
        })))
    }
}

impl IdentityMapper {
    /// A value that hides the row: transparent, size 0 (NaN), no shape.
    fn hidden(&self) -> StyleValues {
        match self.kind {
            ValueKind::Color => StyleValues::Colors(PerItem::Uniform(Color::TRANSPARENT)),
            ValueKind::Number => StyleValues::Numbers(PerItem::Uniform(f64::NAN)),
            ValueKind::Shape => StyleValues::Shapes(PerItem::Uniform(super::HIDDEN_SHAPE)),
            ValueKind::Linetype | ValueKind::Text => self.na.clone(),
        }
    }
}

/// lets-plot `TypedContinuousIdentityMappers.COLOR`: a number is an RGB integer
/// (`0xff0000` is red).
fn rgb_integer(value: f64) -> Option<StyleValues> {
    let value = value.trunc();
    if !(0.0..=f64::from(0xff_ffff)).contains(&value) {
        return None;
    }
    let rgb = value as u32;
    Some(StyleValues::Colors(PerItem::Uniform(Color::rgb(
        (rgb >> 16) as u8,
        (rgb >> 8) as u8,
        rgb as u8,
    ))))
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
        // A missing category hides the row (lets-plot drops it).
        assert_eq!(mapped[1], Color::TRANSPARENT);
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
        // Float codes round; out of range is NA.
        let column = ContinuousColumn::from_options(vec![Some(21.0), Some(2.5), Some(30.0)]);
        assert_eq!(
            shapes.map_continuous(&column, (0.0, 1.0)),
            StyleValues::Shapes(PerItem::Varying(vec![21, 3, super::super::NA_SHAPE]))
        );
        // Numbers on colors are RGB integers.
        assert_eq!(
            colors.map_continuous(&ContinuousColumn::from_values(vec![255.0]), (0.0, 1.0)),
            StyleValues::Colors(PerItem::Varying(vec![Color::rgb(0, 0, 255)]))
        );
    }
}
