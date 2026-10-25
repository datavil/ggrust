//! Theme: a named base theme plus per-element overrides.
//!
//! Element names (`axis_text_x`, `legend_position`, ...) are validated by the
//! theme module, not here.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::{Options, Value, from_value_tracked, join_path, nest_error_path, report_unsupported};

/// `theme_*()` + `theme(...)`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(try_from = "Value")]
pub struct ThemeSpec {
    /// Base theme: `classic`, `bw`, `grey`, `minimal`, `none`, ...
    pub name: Option<String>,
    /// Per-element overrides keyed by element name.
    pub elements: BTreeMap<String, ThemeValue>,
}

impl TryFrom<Value> for ThemeSpec {
    type Error = String;

    /// Parsed by hand (not `flatten`) so errors keep the element path,
    /// e.g. `theme.axis_text.size`.
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let Value::Object(entries) = value else {
            return Err(format!("expected theme options, got {value}"));
        };
        let mut theme = ThemeSpec::default();
        for (key, value) in entries {
            if key == "name" {
                match value {
                    Value::String(name) => theme.name = Some(name),
                    other => return Err(format!("at 'name': expected a string, got {other}")),
                }
            } else {
                let element = ThemeValue::try_from(value)
                    .map_err(|message| nest_error_path(&key, &message))?;
                theme.elements.insert(key, element);
            }
        }
        Ok(theme)
    }
}

impl ThemeSpec {
    pub(super) fn collect_unsupported(&self, parent: &str, found: &mut Vec<String>) {
        for (name, value) in &self.elements {
            if let ThemeValue::Element(element) = value {
                report_unsupported(&join_path(parent, name), &element.unsupported, found);
            }
        }
    }
}

/// The value of one theme entry.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "Value")]
pub enum ThemeValue {
    /// `element_blank()`, or the string `"blank"` that `theme_void()` emits.
    Blank,
    /// `element_text(...)`, `element_line(...)`, `element_rect(...)`.
    Element(Box<ElementSpec>),
    /// A plain value, e.g. `legend_position="bottom"` or `[1, 1]`.
    Scalar(Value),
}

impl TryFrom<Value> for ThemeValue {
    type Error = String;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::String(text) if text == "blank" => Ok(ThemeValue::Blank),
            Value::Object(_) => {
                let element: ElementSpec = from_value_tracked(value)?;
                if element.blank {
                    Ok(ThemeValue::Blank)
                } else {
                    Ok(ThemeValue::Element(Box::new(element)))
                }
            }
            other => Ok(ThemeValue::Scalar(other)),
        }
    }
}

/// Options of `element_text`, `element_line` or `element_rect`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct ElementSpec {
    /// `true` for `element_blank()`.
    pub blank: bool,
    /// Line or text color.
    pub color: Option<String>,
    /// Fill color (rect).
    pub fill: Option<String>,
    /// Line width or font size.
    pub size: Option<f64>,
    /// Line type, e.g. `dashed`, or a dash pattern.
    pub linetype: Option<Value>,
    /// Text rotation in degrees.
    pub angle: Option<f64>,
    /// Horizontal justification, 0..1.
    pub hjust: Option<f64>,
    /// Vertical justification, 0..1.
    pub vjust: Option<f64>,
    /// Font face: `plain`, `bold`, `italic`, `bold_italic`.
    pub face: Option<String>,
    /// Font family.
    pub family: Option<String>,
    /// Text margin: one number or `[top, right, bottom, left]` (2 or 3 values allowed).
    pub margin: Option<Value>,
    /// Line height multiplier.
    pub lineheight: Option<f64>,
    /// Keys no component consumes.
    #[serde(flatten)]
    pub unsupported: Options,
}
