//! Theme values: lets-plot's option maps (`ThemeValuesBase`, `ThemeValuesRClassic`,
//! ...), flavors (`ThemeFlavor`) and the merge of `theme(...)` options on top
//! (`ThemeUtil.getThemeValues`).
//!
//! An option is an element (a map of properties: color, size, blank, ...) or a plain
//! value (`legend_position`). Merging two elements merges their properties; a more
//! specific option wins over a general one when resolving (see [`Values::element`]).
//! Colors in named themes are symbolic and resolved by the flavor.

use std::collections::BTreeMap;

use crate::scene::Color;
use crate::spec::{ElementSpec, ThemeSpec, ThemeValue, Value};

/// lets-plot `ThemeFlavor.SymbolicColor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symbol {
    /// Paper.
    White,
    /// Ink.
    Black,
    /// `panel_grid`.
    Grey1,
    /// Facet strip background.
    Grey2,
    /// `panel_background` (grey theme).
    Grey3,
    /// Borders (`light`, `bw`).
    Grey4,
}

/// A color in a theme: symbolic (flavor) or concrete.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Paint {
    /// Resolved by the flavor.
    Symbol(Symbol),
    /// A color.
    Color(Color),
}

/// Properties of a theme element; `None` inherits from more general options.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Props {
    /// `element_blank()`.
    pub blank: Option<bool>,
    /// Line or text color.
    pub color: Option<Paint>,
    /// Fill color.
    pub fill: Option<Paint>,
    /// Line width or font size.
    pub size: Option<f64>,
    /// Font face name.
    pub face: Option<String>,
    /// Font family name.
    pub family: Option<String>,
    /// Horizontal justification.
    pub hjust: Option<f64>,
    /// Vertical justification.
    pub vjust: Option<f64>,
    /// Text angle.
    pub angle: Option<f64>,
    /// Margins or insets: top, right, bottom, left.
    pub margin: [Option<f64>; 4],
    /// Line type (not drawn yet).
    pub linetype: Option<Value>,
}

impl Props {
    /// `self` with `other`'s given properties on top.
    fn merged(&self, other: &Props) -> Props {
        let side = |index: usize| other.margin[index].or(self.margin[index]);
        Props {
            blank: other.blank.or(self.blank),
            color: other.color.or(self.color),
            fill: other.fill.or(self.fill),
            size: other.size.or(self.size),
            face: other.face.clone().or_else(|| self.face.clone()),
            family: other.family.clone().or_else(|| self.family.clone()),
            hjust: other.hjust.or(self.hjust),
            vjust: other.vjust.or(self.vjust),
            angle: other.angle.or(self.angle),
            margin: [side(0), side(1), side(2), side(3)],
            linetype: other.linetype.clone().or_else(|| self.linetype.clone()),
        }
    }
}

/// One theme option.
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    /// An element (or margins).
    Element(Props),
    /// A plain value: a number, a name, a pair.
    Value(Value),
}

/// The options of a theme, by name.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Values {
    entries: BTreeMap<String, Entry>,
}

impl Values {
    fn set(&mut self, key: &str, entry: Entry) {
        match (self.entries.get(key), &entry) {
            (Some(Entry::Element(old)), Entry::Element(new)) => {
                let merged = old.merged(new);
                self.entries.insert(key.to_owned(), Entry::Element(merged));
            }
            _ => {
                self.entries.insert(key.to_owned(), entry);
            }
        }
    }

    fn element(&mut self, key: &str, props: Props) {
        self.set(key, Entry::Element(props));
    }

    fn value(&mut self, key: &str, value: Value) {
        self.set(key, Entry::Value(value));
    }

    /// lets-plot `getElemValue`: the properties of `keys` (most specific first),
    /// merged from the most general up.
    pub fn element_props(&self, keys: &[&str]) -> Props {
        keys.iter().rev().fold(Props::default(), |props, key| {
            match self.entries.get(*key) {
                Some(Entry::Element(element)) => props.merged(element),
                _ => props,
            }
        })
    }

    /// The first plain value among `keys` (most specific first).
    pub fn value_of(&self, keys: &[&str]) -> Option<&Value> {
        keys.iter().find_map(|key| match self.entries.get(*key) {
            Some(Entry::Value(value)) => Some(value),
            _ => None,
        })
    }
}

/// A flavor: the colors of the symbols.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flavor {
    white: Color,
    black: Color,
    grey: [Color; 4],
}

impl Flavor {
    /// The color of a symbol.
    pub fn color(&self, symbol: Symbol) -> Color {
        match symbol {
            Symbol::White => self.white,
            Symbol::Black => self.black,
            Symbol::Grey1 => self.grey[0],
            Symbol::Grey2 => self.grey[1],
            Symbol::Grey3 => self.grey[2],
            Symbol::Grey4 => self.grey[3],
        }
    }

    /// lets-plot `ThemeFlavor.baseStylePalette`.
    fn base(grey2: Color, grey3: Color, grey4: Color) -> Self {
        Flavor {
            white: Color::WHITE,
            black: DARK_GREY,
            grey: [LIGHT_GREY, grey2, grey3, grey4],
        }
    }

    fn of(white: &str, black: &str, grey: [&str; 4]) -> Self {
        let hex = |text: &str| Color::from_hex(text).unwrap_or(Color::WHITE);
        Flavor {
            white: hex(white),
            black: hex(black),
            grey: grey.map(hex),
        }
    }

    /// `flavor_*()` by name; `standard` is `None` (the theme's own).
    pub fn named(name: &str) -> Option<Option<Flavor>> {
        Some(match name {
            "standard" => None,
            "darcula" => Some(Flavor::of(
                "#303030",
                "#BBBBBB",
                ["#474747", "#363636", "#3B3B3B", "#BBBBBB"],
            )),
            "solarized_light" => Some(Flavor::of(
                "#FDF6E3",
                "#2E4E58",
                ["#D7D4CB", "#E6DFCA", "#EEE8D5", "#2E4E58"],
            )),
            "solarized_dark" => Some(Flavor::of(
                "#0E3C4A",
                "#A7B6BA",
                ["#455458", "#1F4650", "#1B4854", "#A7B6BA"],
            )),
            "high_contrast_light" => Some(Flavor::of(
                "#FFFFFF",
                "#000000",
                ["#E9E9E9", "#D9D9D9", "#EBEBEB", "#000000"],
            )),
            "high_contrast_dark" => Some(Flavor::of(
                "#000000",
                "#FFFFFF",
                ["#474747", "#363636", "#3B3B3B", "#FFFFFF"],
            )),
            _ => return None,
        })
    }
}

const DARK_GREY: Color = Color::rgb(0x47, 0x47, 0x47);
const LIGHT_GREY: Color = Color::rgb(0xE9, 0xE9, 0xE9);
const GREY85: Color = Color::rgb(0xD9, 0xD9, 0xD9);

/// Named theme names, for error messages.
pub const THEME_NAMES: &str =
    "'minimal2', 'classic', 'bw', 'grey' ('gray'), 'light', 'minimal', 'none'";

/// Flavor names, for error messages.
pub const FLAVOR_NAMES: &str = "'standard', 'darcula', 'solarized_light', 'solarized_dark', \
     'high_contrast_light', 'high_contrast_dark'";

fn blank() -> Props {
    Props {
        blank: Some(true),
        ..Props::default()
    }
}

fn symbol(color: Symbol) -> Option<Paint> {
    Some(Paint::Symbol(color))
}

fn margins(value: f64) -> [Option<f64>; 4] {
    [Some(value); 4]
}

/// lets-plot `ThemeValuesBase` (the parts ggrust reads).
fn base() -> Values {
    let mut values = Values::default();
    values.element(
        "line",
        Props {
            size: Some(1.0),
            color: symbol(Symbol::Black),
            ..Props::default()
        },
    );
    values.element(
        "rect",
        Props {
            size: Some(1.0),
            color: symbol(Symbol::Black),
            fill: symbol(Symbol::White),
            ..Props::default()
        },
    );
    values.element(
        "text",
        Props {
            size: Some(13.0),
            face: Some("plain".into()),
            family: Some("sans".into()),
            hjust: Some(0.0),
            vjust: Some(0.5),
            color: symbol(Symbol::Black),
            ..Props::default()
        },
    );
    values.element(
        "title",
        Props {
            size: Some(15.0),
            margin: margins(0.0),
            ..Props::default()
        },
    );
    values.element(
        "plot_title",
        Props {
            size: Some(16.0),
            ..Props::default()
        },
    );
    values.element(
        "plot_subtitle",
        Props {
            size: Some(15.0),
            margin: [Some(4.0), None, None, None],
            ..Props::default()
        },
    );
    values.element(
        "plot_caption",
        Props {
            hjust: Some(1.0),
            size: Some(13.0),
            ..Props::default()
        },
    );
    values.element("panel_border", blank());
    values.element(
        "plot_background",
        Props {
            size: Some(0.0),
            ..Props::default()
        },
    );
    values.element(
        "legend_background",
        Props {
            size: Some(0.0),
            ..Props::default()
        },
    );
    values.value("axis_ticks_length", Value::from(4.0));
    values.element("axis_minor_ticks", blank());
    values.element(
        "strip_text",
        Props {
            hjust: Some(0.5),
            margin: margins(3.0),
            ..Props::default()
        },
    );
    values.value("panel_spacing", Value::from(10.0));
    values.value("strip_spacing", Value::from(0.0));
    values.element(
        "axis_text",
        Props {
            margin: margins(0.0),
            angle: Some(f64::NAN),
            ..Props::default()
        },
    );
    values.value("axis_text_spacing", Value::from(2.0));
    for key in ["axis_text_x", "axis_text_y"] {
        values.element(
            key,
            Props {
                hjust: Some(f64::NAN),
                vjust: Some(f64::NAN),
                ..Props::default()
            },
        );
    }
    values.element(
        "axis_title",
        Props {
            size: Some(15.0),
            hjust: Some(0.5),
            ..Props::default()
        },
    );
    values.element(
        "panel_grid_minor",
        Props {
            size: Some(0.5),
            ..Props::default()
        },
    );
    values.element(
        "legend_title",
        Props {
            size: Some(15.0),
            ..Props::default()
        },
    );
    values.value("legend_position", Value::from("right"));
    values.value("legend_justification", Value::from("center"));
    values.value("legend_spacing", Value::from(10.0));
    values.element("legend_key", blank());
    values.value("legend_key_size", Value::from(23.0));
    values.value("legend_key_spacing", Value::from(0.0));
    values.element(
        "legend_margin",
        Props {
            margin: margins(5.0),
            ..Props::default()
        },
    );
    values.value("legend_box", Value::from("vertical"));
    values.value("legend_box_spacing", Value::from(0.0));
    values.value("legend_ticks_length", Value::from(4.0));
    values.element(
        "legend_ticks",
        Props {
            size: Some(1.0),
            ..Props::default()
        },
    );
    values.element(
        "plot_margin",
        Props {
            margin: margins(3.0),
            ..Props::default()
        },
    );
    values.element(
        "plot_inset",
        Props {
            margin: margins(3.0),
            ..Props::default()
        },
    );
    values
}

/// A named theme and its default flavor (lets-plot `ThemeValues.forName`).
fn named(name: &str) -> Option<(Values, Option<Flavor>)> {
    let mut values = base();
    let common = Some(Flavor::base(LIGHT_GREY, LIGHT_GREY, LIGHT_GREY));
    let grid = |values: &mut Values, color: Symbol| {
        values.element(
            "panel_grid",
            Props {
                color: symbol(color),
                ..Props::default()
            },
        );
    };
    let flavor = match name {
        "minimal2" => {
            values.element("panel_background", blank());
            values.element("panel_grid_minor", blank());
            grid(&mut values, Symbol::Grey1);
            values.element("axis_line_y", blank());
            values.element("axis_ticks_y", blank());
            values.element(
                "strip_background",
                Props {
                    blank: Some(true),
                    fill: symbol(Symbol::Grey2),
                    ..Props::default()
                },
            );
            common
        }
        "classic" => {
            values.element("panel_background", blank());
            values.element(
                "panel_grid",
                Props {
                    blank: Some(true),
                    color: symbol(Symbol::Grey1),
                    ..Props::default()
                },
            );
            common
        }
        "bw" => {
            values.element(
                "panel_background",
                Props {
                    color: symbol(Symbol::Grey4),
                    ..Props::default()
                },
            );
            grid(&mut values, Symbol::Grey1);
            values.element(
                "panel_grid_major",
                Props {
                    size: Some(1.4),
                    ..Props::default()
                },
            );
            values.element(
                "panel_grid_minor",
                Props {
                    size: Some(0.5),
                    ..Props::default()
                },
            );
            values.element("axis_line", blank());
            values.element(
                "axis",
                Props {
                    color: symbol(Symbol::Grey4),
                    ..Props::default()
                },
            );
            values.element(
                "strip_background",
                Props {
                    color: symbol(Symbol::Grey4),
                    fill: symbol(Symbol::Grey2),
                    ..Props::default()
                },
            );
            Some(Flavor::base(
                GREY85,
                Color::WHITE,
                Color::rgb(0x33, 0x33, 0x33),
            ))
        }
        "grey" | "gray" => {
            values.element(
                "panel_background",
                Props {
                    size: Some(0.0),
                    fill: symbol(Symbol::Grey3),
                    ..Props::default()
                },
            );
            grid(&mut values, Symbol::White);
            values.element(
                "panel_grid_major",
                Props {
                    size: Some(1.4),
                    ..Props::default()
                },
            );
            values.element(
                "panel_grid_minor",
                Props {
                    size: Some(0.5),
                    ..Props::default()
                },
            );
            values.element("axis_line", blank());
            values.element(
                "axis_ticks",
                Props {
                    size: Some(1.4),
                    ..Props::default()
                },
            );
            values.element(
                "strip_background",
                Props {
                    size: Some(0.0),
                    fill: symbol(Symbol::Grey2),
                    ..Props::default()
                },
            );
            Some(Flavor::base(
                GREY85,
                Color::rgb(0xEB, 0xEB, 0xEB),
                DARK_GREY,
            ))
        }
        "light" => {
            values.element(
                "panel_background",
                Props {
                    color: symbol(Symbol::Grey4),
                    ..Props::default()
                },
            );
            grid(&mut values, Symbol::Grey1);
            values.element("axis_line", blank());
            values.element(
                "axis",
                Props {
                    color: symbol(Symbol::Grey4),
                    ..Props::default()
                },
            );
            values.element(
                "strip_background",
                Props {
                    size: Some(0.0),
                    fill: symbol(Symbol::Grey2),
                    ..Props::default()
                },
            );
            Some(Flavor::base(
                GREY85,
                Color::WHITE,
                Color::rgb(0xC9, 0xC9, 0xC9),
            ))
        }
        "minimal" => {
            values.element("panel_background", blank());
            grid(&mut values, Symbol::Grey1);
            values.element("axis_line", blank());
            values.element("axis_ticks", blank());
            values.element(
                "strip_background",
                Props {
                    blank: Some(true),
                    fill: symbol(Symbol::Grey2),
                    ..Props::default()
                },
            );
            common
        }
        "none" => {
            let blue = Some(Paint::Color(Color::rgb(0, 0, 255)));
            values.element(
                "line",
                Props {
                    color: blue,
                    ..Props::default()
                },
            );
            values.element(
                "rect",
                Props {
                    color: blue,
                    fill: Some(Paint::Color(Color::rgb(0xAD, 0xD8, 0xE6))),
                    ..Props::default()
                },
            );
            values.element(
                "text",
                Props {
                    color: blue,
                    ..Props::default()
                },
            );
            // lets-plot applies no flavor to 'none'; its remaining symbols are
            // resolved with the common palette here.
            common
        }
        _ => return None,
    };
    Some((values, flavor))
}

/// The theme options of a spec: the named theme (default `minimal2`), the user's
/// options on top, and the flavor for symbolic colors. Errors name the option.
pub fn theme_values(spec: Option<&ThemeSpec>) -> Result<(Values, Flavor), String> {
    let name = spec
        .and_then(|spec| spec.name.as_deref())
        .unwrap_or("minimal2");
    let (mut values, default_flavor) = named(name).ok_or_else(|| {
        format!("at 'theme.name': unknown theme '{name}'; expected one of {THEME_NAMES}")
    })?;
    let mut flavor = default_flavor;
    if let Some(spec) = spec {
        for (key, value) in &spec.elements {
            if key == "flavor" {
                let Value::String(flavor_name) = scalar(value) else {
                    return Err(format!("at 'theme.flavor': expected one of {FLAVOR_NAMES}"));
                };
                flavor = Flavor::named(flavor_name)
                    .ok_or_else(|| {
                        format!(
                            "at 'theme.flavor': unknown flavor '{flavor_name}'; expected one \
                             of {FLAVOR_NAMES}"
                        )
                    })?
                    .or(default_flavor);
                continue;
            }
            values.set(key, user_entry(key, value)?);
        }
    }
    let flavor = flavor.unwrap_or(Flavor::base(LIGHT_GREY, LIGHT_GREY, LIGHT_GREY));
    Ok((values, flavor))
}

fn scalar(value: &ThemeValue) -> &Value {
    static NULL: Value = Value::Null;
    match value {
        ThemeValue::Scalar(value) => value,
        _ => &NULL,
    }
}

/// Keys whose plain value is a margin list (lets-plot `convertMargins`/`convertInset`).
const MARGIN_KEYS: &[&str] = &["plot_margin", "legend_margin", "plot_inset", "panel_inset"];

/// A user option as a theme entry (lets-plot `standardizeThemeOptionValue`).
fn user_entry(key: &str, value: &ThemeValue) -> Result<Entry, String> {
    let at = |message: String| format!("at 'theme.{key}': {message}");
    Ok(match value {
        ThemeValue::Blank => Entry::Element(blank()),
        ThemeValue::Element(element) => Entry::Element(user_props(element).map_err(at)?),
        ThemeValue::Scalar(value) if MARGIN_KEYS.contains(&key) => Entry::Element(Props {
            margin: thickness(value).map_err(at)?,
            ..Props::default()
        }),
        ThemeValue::Scalar(value) => Entry::Value(value.clone()),
    })
}

fn user_props(element: &ElementSpec) -> Result<Props, String> {
    let color = |text: &Option<String>| -> Result<Option<Paint>, String> {
        text.as_deref()
            .map(|text| Color::parse(text).map(Paint::Color))
            .transpose()
    };
    Ok(Props {
        blank: Some(false),
        color: color(&element.color)?,
        fill: color(&element.fill)?,
        size: element.size,
        face: element.face.clone(),
        family: element.family.clone(),
        hjust: element.hjust,
        vjust: element.vjust,
        angle: element.angle,
        margin: match &element.margin {
            Some(value) => thickness(value)?,
            None => [None; 4],
        },
        linetype: element.linetype.clone(),
    })
}

/// lets-plot `toThickness`: a number, or 1-4 numbers (CSS order); nulls keep a side.
fn thickness(value: &Value) -> Result<[Option<f64>; 4], String> {
    let numbers: Vec<Option<f64>> = match value {
        Value::Number(number) => vec![number.as_f64()],
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::Null => Ok(None),
                Value::Number(number) => Ok(number.as_f64()),
                other => Err(format!("margins are numbers, got {other}")),
            })
            .collect::<Result<_, _>>()?,
        other => {
            return Err(format!(
                "expected a number or a list of numbers, got {other}"
            ));
        }
    };
    Ok(match numbers.as_slice() {
        [all] => [*all; 4],
        [vertical, horizontal] => [*vertical, *horizontal, *vertical, *horizontal],
        [top, horizontal, bottom] => [*top, *horizontal, *bottom, *horizontal],
        [top, right, bottom, left] => [*top, *right, *bottom, *left],
        _ => return Err(format!("expected 1 to 4 margins, got {}", numbers.len())),
    })
}
