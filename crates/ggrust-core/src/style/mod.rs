//! Style values: what non-position aesthetics resolve to before a geom draws.
//!
//! Every aesthetic has one value kind ([`crate::spec::Aesthetic::kind`]); geom
//! defaults, constants (`color="red"`) and style scales all produce [`StyleValues`]
//! of that kind, and asking for the wrong kind is an error, never a silent fallback.

use std::collections::BTreeMap;

use crate::scene::{Color, PerItem};
use crate::spec::{Aesthetic, Value};
use crate::{Error, Result};

/// The value type of a style aesthetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueKind {
    /// Colors: `color`, `fill`.
    Color,
    /// Numbers: `size`, `alpha`, `stroke`, `linewidth`, `angle`, ...
    Number,
    /// Point shapes: lets-plot shape codes 0..25.
    Shape,
    /// Line types: dash patterns.
    Linetype,
    /// Text: `label`, `family`, `fontface`.
    Text,
}

/// A line type as a dash pattern in units of line width (empty = solid).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LineType(pub Vec<f64>);

impl LineType {
    /// Parse a lets-plot line type: a code 0..6, a name (`solid`, `dashed`,
    /// `dotted`, `dotdash`, `longdash`, `twodash`, `blank`) or a dash list.
    pub fn parse(value: &Value) -> std::result::Result<Self, String> {
        let named = |code: i64| match code {
            0 => Some(vec![0.0, 1.0]),
            1 => Some(vec![]),
            2 => Some(vec![4.0, 4.0]),
            3 => Some(vec![1.0, 3.0]),
            4 => Some(vec![1.0, 3.0, 4.0, 3.0]),
            5 => Some(vec![7.0, 3.0]),
            6 => Some(vec![2.0, 2.0, 6.0, 2.0]),
            _ => None,
        };
        let code = match value {
            Value::Number(number) => number.as_i64(),
            Value::String(name) => [
                "blank", "solid", "dashed", "dotted", "dotdash", "longdash", "twodash",
            ]
            .iter()
            .position(|candidate| candidate == name)
            .map(|index| index as i64),
            Value::Array(items) => {
                let dash: Option<Vec<f64>> = items
                    .iter()
                    .map(|item| {
                        item.as_f64()
                            .filter(|length| length.is_finite() && *length >= 0.0)
                    })
                    .collect();
                return dash.map(LineType).ok_or_else(|| {
                    format!("invalid dash list {value}; expected non-negative numbers")
                });
            }
            _ => None,
        };
        code.and_then(named).map(LineType).ok_or_else(|| {
            format!(
                "invalid line type {value}; expected 0..6, 'solid', 'dashed', 'dotted', \
                 'dotdash', 'longdash', 'twodash', 'blank' or a dash list"
            )
        })
    }
}

/// Parse a constant option (e.g. `color="red"`, `size=2`) as values of `kind`.
pub fn parse_constant(kind: ValueKind, value: &Value) -> std::result::Result<StyleValues, String> {
    match kind {
        ValueKind::Color => match value {
            Value::String(text) => {
                Color::parse(text).map(|color| StyleValues::Colors(PerItem::Uniform(color)))
            }
            other => Err(format!("expected a color string, got {other}")),
        },
        ValueKind::Number => value
            .as_f64()
            .filter(|number| number.is_finite())
            .map(|number| StyleValues::Numbers(PerItem::Uniform(number)))
            .ok_or_else(|| format!("expected a finite number, got {value}")),
        ValueKind::Shape => value
            .as_u64()
            .filter(|code| *code <= 25)
            .map(|code| StyleValues::Shapes(PerItem::Uniform(code as u8)))
            .ok_or_else(|| format!("expected a shape code 0..25, got {value}")),
        ValueKind::Linetype => LineType::parse(value)
            .map(|linetype| StyleValues::Linetypes(PerItem::Uniform(linetype))),
        ValueKind::Text => match value {
            Value::String(text) => Ok(StyleValues::Texts(PerItem::Uniform(text.clone()))),
            Value::Number(number) => Ok(StyleValues::Texts(PerItem::Uniform(number.to_string()))),
            other => Err(format!("expected a text, got {other}")),
        },
    }
}

/// A geom's default for one style aesthetic (`const`-friendly).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StyleDefault {
    /// A color.
    Color(Color),
    /// A number. NaN may mean "unset" where a geom documents it (point alpha).
    Number(f64),
    /// A lets-plot shape code.
    Shape(u8),
    /// A dash pattern (empty = solid).
    Linetype(&'static [f64]),
    /// A text.
    Text(&'static str),
}

impl StyleDefault {
    /// The value kind of this default.
    pub fn kind(self) -> ValueKind {
        match self {
            StyleDefault::Color(_) => ValueKind::Color,
            StyleDefault::Number(_) => ValueKind::Number,
            StyleDefault::Shape(_) => ValueKind::Shape,
            StyleDefault::Linetype(_) => ValueKind::Linetype,
            StyleDefault::Text(_) => ValueKind::Text,
        }
    }

    /// The default as values for every row.
    pub fn uniform(self) -> StyleValues {
        match self {
            StyleDefault::Color(color) => StyleValues::Colors(PerItem::Uniform(color)),
            StyleDefault::Number(number) => StyleValues::Numbers(PerItem::Uniform(number)),
            StyleDefault::Shape(shape) => StyleValues::Shapes(PerItem::Uniform(shape)),
            StyleDefault::Linetype(dash) => {
                StyleValues::Linetypes(PerItem::Uniform(LineType(dash.to_vec())))
            }
            StyleDefault::Text(text) => StyleValues::Texts(PerItem::Uniform(text.to_owned())),
        }
    }
}

/// Resolved values of one style aesthetic, uniform or one per row.
#[derive(Debug, Clone, PartialEq)]
pub enum StyleValues {
    /// Colors.
    Colors(PerItem<Color>),
    /// Numbers.
    Numbers(PerItem<f64>),
    /// Shape codes.
    Shapes(PerItem<u8>),
    /// Line types.
    Linetypes(PerItem<LineType>),
    /// Texts.
    Texts(PerItem<String>),
}

impl StyleValues {
    /// The value kind held.
    pub fn kind(&self) -> ValueKind {
        match self {
            StyleValues::Colors(_) => ValueKind::Color,
            StyleValues::Numbers(_) => ValueKind::Number,
            StyleValues::Shapes(_) => ValueKind::Shape,
            StyleValues::Linetypes(_) => ValueKind::Linetype,
            StyleValues::Texts(_) => ValueKind::Text,
        }
    }
}

/// Resolved style values of a layer, by aesthetic.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Styles {
    values: BTreeMap<Aesthetic, StyleValues>,
}

impl Styles {
    /// Set the values of an aesthetic. Their kind must match the aesthetic's.
    pub fn insert(&mut self, aesthetic: Aesthetic, values: StyleValues) -> Result<()> {
        if aesthetic.kind().style() != Some(values.kind()) {
            return Err(Error::InvalidData(format!(
                "internal: {:?} values given for aesthetic '{aesthetic}', which takes {:?}",
                values.kind(),
                aesthetic.kind()
            )));
        }
        self.values.insert(aesthetic, values);
        Ok(())
    }

    /// Whether an aesthetic has one value for every item (absent counts as uniform).
    pub fn is_uniform(&self, aesthetic: Aesthetic) -> bool {
        match self.values.get(&aesthetic) {
            None => true,
            Some(StyleValues::Colors(values)) => matches!(values, PerItem::Uniform(_)),
            Some(StyleValues::Numbers(values)) => matches!(values, PerItem::Uniform(_)),
            Some(StyleValues::Shapes(values)) => matches!(values, PerItem::Uniform(_)),
            Some(StyleValues::Linetypes(values)) => matches!(values, PerItem::Uniform(_)),
            Some(StyleValues::Texts(values)) => matches!(values, PerItem::Uniform(_)),
        }
    }

    /// Colors of a color aesthetic.
    pub fn colors(&self, aesthetic: Aesthetic) -> Result<&PerItem<Color>> {
        match self.values.get(&aesthetic) {
            Some(StyleValues::Colors(colors)) => Ok(colors),
            other => Err(missing(aesthetic, ValueKind::Color, other)),
        }
    }

    /// Numbers of a numeric aesthetic.
    pub fn numbers(&self, aesthetic: Aesthetic) -> Result<&PerItem<f64>> {
        match self.values.get(&aesthetic) {
            Some(StyleValues::Numbers(numbers)) => Ok(numbers),
            other => Err(missing(aesthetic, ValueKind::Number, other)),
        }
    }

    /// Shape codes of the shape aesthetic.
    pub fn shapes(&self, aesthetic: Aesthetic) -> Result<&PerItem<u8>> {
        match self.values.get(&aesthetic) {
            Some(StyleValues::Shapes(shapes)) => Ok(shapes),
            other => Err(missing(aesthetic, ValueKind::Shape, other)),
        }
    }

    /// Line types of a linetype aesthetic.
    pub fn linetypes(&self, aesthetic: Aesthetic) -> Result<&PerItem<LineType>> {
        match self.values.get(&aesthetic) {
            Some(StyleValues::Linetypes(linetypes)) => Ok(linetypes),
            other => Err(missing(aesthetic, ValueKind::Linetype, other)),
        }
    }

    /// Texts of a text aesthetic.
    pub fn texts(&self, aesthetic: Aesthetic) -> Result<&PerItem<String>> {
        match self.values.get(&aesthetic) {
            Some(StyleValues::Texts(texts)) => Ok(texts),
            other => Err(missing(aesthetic, ValueKind::Text, other)),
        }
    }
}

fn missing(aesthetic: Aesthetic, wanted: ValueKind, found: Option<&StyleValues>) -> Error {
    Error::InvalidData(match found {
        None => format!("internal: no {wanted:?} values resolved for aesthetic '{aesthetic}'"),
        Some(values) => format!(
            "internal: aesthetic '{aesthetic}' holds {:?} values, not {wanted:?}",
            values.kind()
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn line_types_follow_lets_plot() {
        assert_eq!(
            LineType::parse(&json!("dashed")),
            Ok(LineType(vec![4.0, 4.0]))
        );
        assert_eq!(LineType::parse(&json!(1)), Ok(LineType(vec![])));
        assert_eq!(
            LineType::parse(&json!([2, 1])),
            Ok(LineType(vec![2.0, 1.0]))
        );
        assert!(LineType::parse(&json!("wavy")).is_err());
        assert!(LineType::parse(&json!(9)).is_err());
    }

    #[test]
    fn constants_parse_by_kind() {
        assert_eq!(
            parse_constant(ValueKind::Shape, &json!(21)),
            Ok(StyleValues::Shapes(PerItem::Uniform(21)))
        );
        assert!(parse_constant(ValueKind::Shape, &json!(30)).is_err());
        assert!(parse_constant(ValueKind::Number, &json!("big")).is_err());
        assert_eq!(
            parse_constant(ValueKind::Text, &json!(3)),
            Ok(StyleValues::Texts(PerItem::Uniform("3".into())))
        );
    }

    #[test]
    fn styles_reject_wrong_kinds() {
        let mut styles = Styles::default();
        assert!(
            styles
                .insert(
                    Aesthetic::Size,
                    StyleValues::Colors(PerItem::Uniform(Color::BLACK))
                )
                .is_err()
        );
        assert!(styles.colors(Aesthetic::Color).is_err());
    }
}
