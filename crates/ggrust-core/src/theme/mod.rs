//! Themes: resolved style values for every non-data element of a plot.
//!
//! What it does: [`Theme::from_spec`] builds lets-plot's theme from the spec: a named
//! theme (`minimal2` by default, `classic`, `bw`, `grey`, `light`, `minimal`, `none`),
//! `theme(...)` options on top, a flavor for symbolic colors ([`values`]), then each
//! element resolved through lets-plot's inheritance chains (`axis_text_x` <-
//! `axis_text` <- `text` ...). Layout and drawing read only this struct.
//!
//! What it does not do (yet): line types, text margins, text angles and vertical
//! justification other than on axis labels, and facet strips; options that set them
//! are reported as warnings.

mod values;

pub use values::{FLAVOR_NAMES, THEME_NAMES};

use values::{Paint, Props, Values, theme_values};

use crate::scene::{Color, Stroke};
use crate::spec::{ThemeSpec, ThemeValue, Value};
use crate::text::{Font, FontFace, FontFamily};
use crate::{Error, Result};

/// lets-plot `SymbolicColor.BLACK` in the default flavor: the "ink" of text and lines.
pub const INK: Color = Color::rgb(0x47, 0x47, 0x47);

/// lets-plot `SymbolicColor.GREY_1` (`#E9E9E9`): panel grid lines.
pub const GRID: Color = Color::rgb(0xE9, 0xE9, 0xE9);

/// Style of a text element.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    /// Font size in pixels.
    pub size: f64,
    /// Text color.
    pub color: Color,
    /// Family, weight and slant.
    pub font: Font,
    /// Horizontal justification 0..1 (titles, axis labels); NaN: the element's default.
    pub hjust: f64,
    /// Vertical justification 0..1 (axis labels); NaN: the element's default.
    pub vjust: f64,
    /// Counter-clockwise angle in degrees (axis labels); NaN: unset.
    pub angle: f64,
}

/// Style of a line element.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineStyle {
    /// Line color.
    pub color: Color,
    /// Line width in pixels.
    pub width: f64,
}

impl LineStyle {
    /// As a solid stroke.
    pub fn stroke(self) -> Stroke {
        Stroke::solid(self.color, self.width)
    }
}

/// Style of a rectangle element: fill and an optional border.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectStyle {
    /// Fill.
    pub fill: Color,
    /// Border; `None` when its width is 0.
    pub border: Option<LineStyle>,
}

/// Space around a box, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Margins {
    /// Top.
    pub top: f64,
    /// Right.
    pub right: f64,
    /// Bottom.
    pub bottom: f64,
    /// Left.
    pub left: f64,
}

impl Margins {
    /// The same margin on every side.
    pub const fn uniform(value: f64) -> Self {
        Margins {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// Side by side sum.
    pub fn plus(self, other: Margins) -> Margins {
        Margins {
            top: self.top + other.top,
            right: self.right + other.right,
            bottom: self.bottom + other.bottom,
            left: self.left + other.left,
        }
    }
}

/// Where legends go (`legend_position`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LegendPosition {
    /// Right of the panel.
    Right,
    /// Left of the panel.
    Left,
    /// Above the panel.
    Top,
    /// Below the panel.
    Bottom,
    /// No legends.
    None,
    /// Inside the panel area, at (x, y) in 0..1 from the bottom-left.
    Inside(f64, f64),
}

/// Direction of legend keys (`legend_direction`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegendDirection {
    /// Vertical at the left/right, horizontal at the top/bottom.
    Auto,
    /// Keys in a row.
    Horizontal,
    /// Keys in a column.
    Vertical,
}

/// `legend_box_just` as given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegendBoxJust {
    /// Left for a stack, top for a row.
    Auto,
    /// Left.
    Left,
    /// Right.
    Right,
    /// Top.
    Top,
    /// Bottom.
    Bottom,
    /// Centre.
    Center,
}

/// Resolved theme. `None` means the element is blank (not drawn, takes no space).
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// Space outside everything (`plot_margin`).
    pub plot_margin: Margins,
    /// Space between the plot background edge and the content (`plot_inset`).
    pub plot_inset: Margins,
    /// Plot background (`plot_background`).
    pub plot_background: Option<RectStyle>,
    /// Plot title.
    pub plot_title: Option<TextStyle>,
    /// Plot subtitle.
    pub plot_subtitle: Option<TextStyle>,
    /// Space above the subtitle (its top margin).
    pub plot_subtitle_margin_top: f64,
    /// x axis title.
    pub axis_title_x: Option<TextStyle>,
    /// y axis title.
    pub axis_title_y: Option<TextStyle>,
    /// x tick labels.
    pub axis_text_x: Option<TextStyle>,
    /// y tick labels.
    pub axis_text_y: Option<TextStyle>,
    /// Space between ticks and their labels (`axis_text_spacing`).
    pub axis_text_spacing: f64,
    /// Space between the y title and the y labels (ggrust: lets-plot gets it from
    /// its baseline placement).
    pub axis_title_spacing: f64,
    /// x ticks.
    pub axis_ticks_x: Option<LineStyle>,
    /// y ticks.
    pub axis_ticks_y: Option<LineStyle>,
    /// Tick length (`axis_ticks_length`).
    pub axis_ticks_length: f64,
    /// x axis line.
    pub axis_line_x: Option<LineStyle>,
    /// y axis line.
    pub axis_line_y: Option<LineStyle>,
    /// Vertical major grid lines (at x breaks).
    pub panel_grid_major_x: Option<LineStyle>,
    /// Horizontal major grid lines (at y breaks).
    pub panel_grid_major_y: Option<LineStyle>,
    /// Vertical minor grid lines (between x breaks).
    pub panel_grid_minor_x: Option<LineStyle>,
    /// Horizontal minor grid lines (between y breaks).
    pub panel_grid_minor_y: Option<LineStyle>,
    /// Panel background.
    pub panel_background: Option<RectStyle>,
    /// Panel border, drawn above the data.
    pub panel_border: Option<LineStyle>,
    /// Legend title (`legend_title`).
    pub legend_title: Option<TextStyle>,
    /// Legend labels (`legend_text`).
    pub legend_text: Option<TextStyle>,
    /// Legend box (`legend_background`).
    pub legend_background: Option<RectStyle>,
    /// Background of each legend key: `legend_key` when shown, else the panel
    /// background's fill (lets-plot `LegendComponent.createKeyElement`).
    pub legend_key_background: Option<RectStyle>,
    /// Inner padding of a legend box (`legend_margin`).
    pub legend_margin: Margins,
    /// Smallest key cell, square (`legend_key_size`).
    pub legend_key_size: f64,
    /// Space between key cells (`legend_key_spacing`), horizontal and vertical.
    pub legend_key_spacing: (f64, f64),
    /// Space between legend boxes (`legend_spacing`), horizontal and vertical.
    pub legend_spacing: (f64, f64),
    /// Space between the plot area and the legends (`legend_box_spacing`).
    pub legend_box_spacing: f64,
    /// Colorbar ticks (`legend_ticks`; colored like the legend background unless set).
    pub legend_ticks: Option<LineStyle>,
    /// Colorbar tick length (`legend_ticks_length`).
    pub legend_ticks_length: f64,
    /// `legend_position`.
    pub legend_position: LegendPosition,
    /// `legend_justification`: (x, y) in 0..1.
    pub legend_justification: (f64, f64),
    /// `legend_direction`.
    pub legend_direction: LegendDirection,
    /// `legend_box`: several legends side by side (`true`) or stacked.
    pub legend_box_horizontal: bool,
    /// `legend_box_just`: alignment of stacked legends (see
    /// [`Theme::legend_box_justification`]).
    pub legend_box_just: LegendBoxJust,
}

impl Default for Theme {
    /// lets-plot's default theme (`minimal2`).
    fn default() -> Self {
        // The built-in tables always resolve (unit-tested); no user input is involved.
        Theme::from_spec(None, &mut Vec::new()).expect("the built-in default theme resolves")
    }
}

/// Every theme option the resolver reads; other `theme(...)` options are reported.
const BUILT_OPTIONS: &[&str] = &[
    "line",
    "rect",
    "text",
    "title",
    "axis",
    "axis_x",
    "axis_y",
    "axis_title",
    "axis_title_x",
    "axis_title_y",
    "axis_text",
    "axis_text_x",
    "axis_text_y",
    "axis_text_spacing",
    "axis_ticks",
    "axis_ticks_x",
    "axis_ticks_y",
    "axis_ticks_length",
    "axis_line",
    "axis_line_x",
    "axis_line_y",
    "panel_background",
    "panel_border",
    "panel_grid",
    "panel_grid_x",
    "panel_grid_y",
    "panel_grid_major",
    "panel_grid_major_x",
    "panel_grid_major_y",
    "panel_grid_minor",
    "panel_grid_minor_x",
    "panel_grid_minor_y",
    "plot_background",
    "plot_title",
    "plot_subtitle",
    "plot_margin",
    "plot_inset",
    "legend_background",
    "legend_text",
    "legend_title",
    "legend_key",
    "legend_position",
    "legend_justification",
    "legend_direction",
    "legend_margin",
    "legend_spacing",
    "legend_spacing_x",
    "legend_spacing_y",
    "legend_key_size",
    "legend_key_spacing",
    "legend_key_spacing_x",
    "legend_key_spacing_y",
    "legend_box",
    "legend_box_just",
    "legend_box_spacing",
    "legend_ticks",
    "legend_ticks_length",
    "flavor",
];

impl Theme {
    /// The theme with every x and y axis element swapped (`coord_flip`: lets-plot
    /// keeps elements with their aesthetic, so x's go to the vertical axis).
    pub fn flipped(&self) -> Theme {
        let mut theme = self.clone();
        std::mem::swap(&mut theme.axis_title_x, &mut theme.axis_title_y);
        std::mem::swap(&mut theme.axis_text_x, &mut theme.axis_text_y);
        std::mem::swap(&mut theme.axis_ticks_x, &mut theme.axis_ticks_y);
        std::mem::swap(&mut theme.axis_line_x, &mut theme.axis_line_y);
        std::mem::swap(&mut theme.panel_grid_major_x, &mut theme.panel_grid_major_y);
        std::mem::swap(&mut theme.panel_grid_minor_x, &mut theme.panel_grid_minor_y);
        theme
    }

    /// The theme of a spec (named theme + `theme(...)` options + flavor). Options and
    /// element properties ggrust does not draw yet are reported in `warnings`.
    pub fn from_spec(spec: Option<&ThemeSpec>, warnings: &mut Vec<String>) -> Result<Theme> {
        let (values, flavor) = theme_values(spec).map_err(Error::InvalidSpec)?;
        if let Some(spec) = spec {
            report_unbuilt(spec, warnings);
        }
        let mut resolver = Resolver {
            values: &values,
            flavor,
            warnings: Vec::new(),
        };
        let theme = resolver.theme()?;
        for warning in resolver.warnings {
            if !warnings.contains(&warning) {
                warnings.push(warning);
            }
        }
        Ok(theme)
    }
}

fn report_unbuilt(spec: &ThemeSpec, warnings: &mut Vec<String>) {
    for (key, value) in &spec.elements {
        if !BUILT_OPTIONS.contains(&key.as_str()) {
            warnings.push(format!("at 'theme.{key}': not supported yet; ignored"));
            continue;
        }
        let ThemeValue::Element(element) = value else {
            continue;
        };
        // Axis labels read angle and vjust (layout/rotated.rs); other text does not yet.
        let axis_text = matches!(key.as_str(), "axis_text" | "axis_text_x" | "axis_text_y");
        let unbuilt = [
            ("linetype", element.linetype.is_some()),
            ("margin", element.margin.is_some()),
            ("vjust", element.vjust.is_some() && !axis_text),
            ("angle", element.angle.is_some() && !axis_text),
            ("lineheight", element.lineheight.is_some()),
        ];
        for (property, present) in unbuilt {
            if present {
                warnings.push(format!(
                    "at 'theme.{key}.{property}': not supported yet; ignored"
                ));
            }
        }
    }
}

/// Resolves elements through lets-plot's inheritance chains.
struct Resolver<'a> {
    values: &'a Values,
    flavor: values::Flavor,
    warnings: Vec<String>,
}

impl Resolver<'_> {
    fn color(&self, paint: Option<Paint>) -> Option<Color> {
        paint.map(|paint| match paint {
            Paint::Symbol(symbol) => self.flavor.color(symbol),
            Paint::Color(color) => color,
        })
    }

    fn props(&self, keys: &[&str]) -> Option<Props> {
        let props = self.values.element_props(keys);
        (props.blank != Some(true)).then_some(props)
    }

    fn text(&mut self, keys: &[&str]) -> Option<TextStyle> {
        let props = self.props(keys)?;
        let face = match props.face.as_deref() {
            None => FontFace::Plain,
            Some(name) => {
                let face = FontFace::from_name(name);
                let unknown = FontFace::unknown_words(name);
                if !unknown.is_empty() {
                    self.warnings.push(format!(
                        "at 'theme.{}.face': unknown word(s) {unknown:?} in font face '{name}' \
                         were ignored (case-sensitive); expected plain, bold, italic or \
                         bold_italic; {face:?} was used",
                        keys[0]
                    ));
                }
                face
            }
        };
        let family = match props.family.as_deref() {
            None => FontFamily::Sans,
            Some(name) => {
                let (family, embedded) = FontFamily::from_name(name);
                if !embedded {
                    self.warnings.push(format!(
                        "font family '{name}' is not embedded; Liberation Sans was used (ggrust \
                         embeds Liberation Sans and Liberation Mono)"
                    ));
                }
                family
            }
        };
        Some(TextStyle {
            size: props.size.unwrap_or(13.0),
            color: self.color(props.color).unwrap_or(INK),
            font: Font { family, face },
            hjust: props.hjust.unwrap_or(f64::NAN),
            vjust: props.vjust.unwrap_or(f64::NAN),
            angle: props.angle.unwrap_or(f64::NAN),
        })
    }

    fn line(&self, keys: &[&str]) -> Option<LineStyle> {
        let props = self.props(keys)?;
        Some(LineStyle {
            color: self.color(props.color).unwrap_or(INK),
            width: props.size.unwrap_or(1.0),
        })
    }

    fn rect(&self, keys: &[&str]) -> Option<RectStyle> {
        let props = self.props(keys)?;
        let width = props.size.unwrap_or(1.0);
        Some(RectStyle {
            fill: self.color(props.fill).unwrap_or(Color::WHITE),
            border: (width > 0.0).then(|| LineStyle {
                color: self.color(props.color).unwrap_or(INK),
                width,
            }),
        })
    }

    fn margins(&self, key: &str, default: f64) -> Margins {
        let margin = self.values.element_props(&[key]).margin;
        let side = |index: usize| margin[index].unwrap_or(default);
        Margins {
            top: side(0),
            right: side(1),
            bottom: side(2),
            left: side(3),
        }
    }

    fn number(&self, keys: &[&str], default: f64) -> Result<f64> {
        match self.values.value_of(keys) {
            None => Ok(default),
            Some(value) => value
                .as_f64()
                .filter(|number| number.is_finite())
                .ok_or_else(|| {
                    Error::InvalidSpec(format!(
                        "at 'theme.{}': expected a number, got {value}",
                        keys[0]
                    ))
                }),
        }
    }

    fn theme(&mut self) -> Result<Theme> {
        let axis = |suffix: &str| {
            let x = |name: &str| format!("{name}_{suffix}");
            (
                x("axis_line"),
                x("axis_text"),
                x("axis_title"),
                x("axis_ticks"),
                x("axis"),
            )
        };
        let (line_x, text_x, title_x, ticks_x, axis_x) = axis("x");
        let (line_y, text_y, title_y, ticks_y, axis_y) = axis("y");
        let grid = |kind: &str, suffix: &str| {
            [
                format!("panel_grid_{kind}_{suffix}"),
                format!("panel_grid_{kind}"),
                format!("panel_grid_{suffix}"),
                "panel_grid".to_owned(),
                "line".to_owned(),
            ]
        };

        let legend_background = self.rect(&["legend_background", "rect"]);
        let tick_default = legend_background.map_or(Color::WHITE, |background| background.fill);
        let legend_ticks = self.props(&["legend_ticks"]).map(|props| LineStyle {
            color: self.color(props.color).unwrap_or(tick_default),
            width: props.size.unwrap_or(1.0),
        });
        let subtitle_margin = self
            .values
            .element_props(&["plot_subtitle", "title", "text"]);
        Ok(Theme {
            plot_margin: self.margins("plot_margin", 3.0),
            plot_inset: self.margins("plot_inset", 3.0),
            plot_background: self.rect(&["plot_background", "rect"]),
            plot_title: self.text(&["plot_title", "title", "text"]),
            plot_subtitle: self.text(&["plot_subtitle", "title", "text"]),
            plot_subtitle_margin_top: subtitle_margin.margin[0].unwrap_or(0.0),
            axis_title_x: self.text(&[&title_x, "axis_title", "title", "text", &axis_x, "axis"]),
            axis_title_y: self.text(&[&title_y, "axis_title", "title", "text", &axis_y, "axis"]),
            axis_text_x: self.text(&[&text_x, "axis_text", "text", &axis_x, "axis"]),
            axis_text_y: self.text(&[&text_y, "axis_text", "text", &axis_y, "axis"]),
            axis_text_spacing: self.number(&["axis_text_spacing"], 2.0)?,
            axis_title_spacing: 3.0,
            axis_ticks_x: self.line(&[&ticks_x, "axis_ticks", &axis_x, "axis", "line"]),
            axis_ticks_y: self.line(&[&ticks_y, "axis_ticks", &axis_y, "axis", "line"]),
            axis_ticks_length: self.number(&["axis_ticks_length"], 4.0)?,
            axis_line_x: self.line(&[&line_x, "axis_line", &axis_x, "axis", "line"]),
            axis_line_y: self.line(&[&line_y, "axis_line", &axis_y, "axis", "line"]),
            panel_grid_major_x: self.line(&as_refs(&grid("major", "x"))),
            panel_grid_major_y: self.line(&as_refs(&grid("major", "y"))),
            panel_grid_minor_x: self.line(&as_refs(&grid("minor", "x"))),
            panel_grid_minor_y: self.line(&as_refs(&grid("minor", "y"))),
            panel_background: self.rect(&["panel_background", "rect"]),
            panel_border: self
                .rect(&["panel_border", "rect"])
                .and_then(|rect| rect.border),
            legend_title: self.text(&["legend_title", "title", "text"]),
            legend_text: self.text(&["legend_text", "text"]),
            legend_background,
            legend_key_background: self.rect(&["legend_key", "rect"]).or_else(|| {
                self.rect(&["panel_background", "rect"])
                    .map(|panel| RectStyle {
                        fill: panel.fill,
                        border: None,
                    })
            }),
            legend_margin: self.margins("legend_margin", 5.0),
            legend_key_size: self.number(&["legend_key_size"], 23.0)?,
            legend_key_spacing: (
                self.number(&["legend_key_spacing_x", "legend_key_spacing"], 0.0)?,
                self.number(&["legend_key_spacing_y", "legend_key_spacing"], 0.0)?,
            ),
            legend_spacing: (
                self.number(&["legend_spacing_x", "legend_spacing"], 10.0)?,
                self.number(&["legend_spacing_y", "legend_spacing"], 10.0)?,
            ),
            legend_box_spacing: self.number(&["legend_box_spacing"], 0.0)?,
            legend_ticks,
            legend_ticks_length: self.number(&["legend_ticks_length"], 4.0)?,
            legend_position: legend_position(self.values.value_of(&["legend_position"]))?,
            legend_justification: legend_justification(
                self.values.value_of(&["legend_justification"]),
            )?,
            legend_direction: legend_direction(self.values.value_of(&["legend_direction"]))?,
            legend_box_horizontal: match self.values.value_of(&["legend_box"]) {
                None => false,
                Some(Value::String(name)) if name == "vertical" => false,
                Some(Value::String(name)) if name == "horizontal" => true,
                Some(other) => {
                    return Err(Error::InvalidSpec(format!(
                        "at 'theme.legend_box': expected 'horizontal' or 'vertical', got {other}"
                    )));
                }
            },
            legend_box_just: legend_box_just(self.values.value_of(&["legend_box_just"]))?,
        })
    }
}

impl Theme {
    /// Where legends of different sizes align across the box, 0..1 (lets-plot
    /// `LegendBoxesLayoutUtil`): a stack honours left/right, a row top/bottom; other
    /// values centre.
    pub fn legend_box_justification(&self) -> f64 {
        use LegendBoxJust::*;
        match (self.legend_box_horizontal, self.legend_box_just) {
            (false, Auto | Left) | (true, Auto | Top) => 0.0,
            (false, Right) | (true, Bottom) => 1.0,
            // lets-plot centres for values of the other axis.
            _ => 0.5,
        }
    }

    /// Whether legend keys run in rows (lets-plot `LegendAssemblerUtil.legendDirection`):
    /// `legend_direction`, else horizontal at the top or bottom.
    pub fn legends_horizontal(&self) -> bool {
        match self.legend_direction {
            LegendDirection::Horizontal => true,
            LegendDirection::Vertical => false,
            LegendDirection::Auto => matches!(
                self.legend_position,
                LegendPosition::Top | LegendPosition::Bottom
            ),
        }
    }
}

fn pair(value: &Value, key: &str) -> Result<(f64, f64)> {
    let numbers = value
        .as_array()
        .filter(|items| items.len() == 2)
        .and_then(|items| Some((items[0].as_f64()?, items[1].as_f64()?)));
    numbers.ok_or_else(|| {
        Error::InvalidSpec(format!(
            "at 'theme.{key}': expected a name or two numbers, got {value}"
        ))
    })
}

fn as_refs(keys: &[String]) -> Vec<&str> {
    keys.iter().map(String::as_str).collect()
}

/// lets-plot `LegendThemeConfig.toPosition`.
fn legend_position(value: Option<&Value>) -> Result<LegendPosition> {
    Ok(match value {
        None => LegendPosition::Right,
        Some(Value::String(name)) => match name.as_str() {
            "right" => LegendPosition::Right,
            "left" => LegendPosition::Left,
            "top" => LegendPosition::Top,
            "bottom" => LegendPosition::Bottom,
            "none" => LegendPosition::None,
            _ => {
                return Err(Error::InvalidSpec(format!(
                    "at 'theme.legend_position': expected left, right, top, bottom, none or two \
                     numbers, got '{name}'"
                )));
            }
        },
        Some(value) => {
            let (x, y) = pair(value, "legend_position")?;
            LegendPosition::Inside(x, y)
        }
    })
}

/// lets-plot `LegendThemeConfig.toJustification`.
fn legend_justification(value: Option<&Value>) -> Result<(f64, f64)> {
    Ok(match value {
        None => (0.5, 0.5),
        Some(Value::String(name)) => match name.as_str() {
            "center" => (0.5, 0.5),
            "left" => (0.0, 0.5),
            "right" => (1.0, 0.5),
            "top" => (0.5, 1.0),
            "bottom" => (0.5, 0.0),
            _ => {
                return Err(Error::InvalidSpec(format!(
                    "at 'theme.legend_justification': expected center, left, right, top, bottom \
                     or two numbers, got '{name}'"
                )));
            }
        },
        Some(value) => pair(value, "legend_justification")?,
    })
}

fn legend_direction(value: Option<&Value>) -> Result<LegendDirection> {
    match value {
        None => Ok(LegendDirection::Auto),
        Some(Value::String(name)) if name == "horizontal" => Ok(LegendDirection::Horizontal),
        Some(Value::String(name)) if name == "vertical" => Ok(LegendDirection::Vertical),
        Some(other) => Err(Error::InvalidSpec(format!(
            "at 'theme.legend_direction': expected 'horizontal' or 'vertical', got {other}"
        ))),
    }
}

fn legend_box_just(value: Option<&Value>) -> Result<LegendBoxJust> {
    match value {
        None => Ok(LegendBoxJust::Auto),
        Some(Value::String(name)) => match name.as_str() {
            "left" => Ok(LegendBoxJust::Left),
            "right" => Ok(LegendBoxJust::Right),
            "top" => Ok(LegendBoxJust::Top),
            "bottom" => Ok(LegendBoxJust::Bottom),
            "center" => Ok(LegendBoxJust::Center),
            _ => Err(Error::InvalidSpec(format!(
                "at 'theme.legend_box_just': expected left, right, top, bottom or center, got \
                 '{name}'"
            ))),
        },
        Some(other) => Err(Error::InvalidSpec(format!(
            "at 'theme.legend_box_just': expected a name, got {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(json: &str) -> ThemeSpec {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn default_is_lets_plot_minimal2() {
        let theme = Theme::default();
        let small = TextStyle {
            size: 13.0,
            color: INK,
            font: Font::default(),
            hjust: f64::NAN,
            vjust: f64::NAN,
            angle: f64::NAN,
        };
        let ink = LineStyle {
            color: INK,
            width: 1.0,
        };
        assert_eq!(theme.plot_margin, Margins::uniform(3.0));
        assert_eq!(theme.plot_inset, Margins::uniform(3.0));
        assert_eq!(
            theme.plot_background,
            Some(RectStyle {
                fill: Color::WHITE,
                border: None
            })
        );
        assert_eq!(theme.plot_title.map(|text| text.size), Some(16.0));
        assert_eq!(theme.plot_subtitle_margin_top, 4.0);
        let text_x = theme.axis_text_x.unwrap();
        assert_eq!(
            (text_x.size, text_x.color, text_x.font),
            (small.size, small.color, small.font)
        );
        let title_x = theme.axis_title_x.unwrap();
        assert_eq!((title_x.size, title_x.hjust), (15.0, 0.5));
        assert_eq!(theme.axis_ticks_x, Some(ink));
        assert_eq!(theme.axis_ticks_y, None);
        assert_eq!(theme.axis_line_x, Some(ink));
        assert_eq!(theme.axis_line_y, None);
        let grid = LineStyle {
            color: GRID,
            width: 1.0,
        };
        assert_eq!(theme.panel_grid_major_x, Some(grid));
        assert_eq!(theme.panel_grid_major_y, Some(grid));
        assert_eq!(theme.panel_grid_minor_x, None);
        assert_eq!(theme.panel_background, None);
        assert_eq!(theme.panel_border, None);
        assert_eq!(theme.legend_title.map(|text| text.size), Some(15.0));
        assert_eq!(
            theme.legend_ticks,
            Some(LineStyle {
                color: Color::WHITE,
                width: 1.0
            })
        );
        assert_eq!(theme.legend_position, LegendPosition::Right);
    }

    #[test]
    fn named_themes_and_user_options_layer_up() {
        let mut warnings = Vec::new();
        let classic =
            Theme::from_spec(Some(&spec(r#"{"name": "classic"}"#)), &mut warnings).unwrap();
        assert_eq!(classic.panel_grid_major_x, None);
        assert!(classic.axis_line_y.is_some() && classic.axis_ticks_y.is_some());

        let void = Theme::from_spec(
            Some(&spec(
                r#"{"name": "classic", "line": "blank", "axis": "blank"}"#,
            )),
            &mut warnings,
        )
        .unwrap();
        assert_eq!(
            (void.axis_text_x, void.axis_title_x, void.axis_line_x),
            (None, None, None)
        );

        let custom = Theme::from_spec(
            Some(&spec(
                r#"{"axis_line_y": {"color": "red", "blank": false},
                    "text": {"family": "mono", "blank": false},
                    "legend_position": "bottom", "plot_margin": [1, 2]}"#,
            )),
            &mut warnings,
        )
        .unwrap();
        // The element is no longer blank and inherits the line width.
        assert_eq!(
            custom.axis_line_y,
            Some(LineStyle {
                color: Color::rgb(255, 0, 0),
                width: 1.0
            })
        );
        assert_eq!(custom.axis_text_x.unwrap().font.family, FontFamily::Mono);
        assert_eq!(custom.legend_position, LegendPosition::Bottom);
        assert_eq!(custom.plot_margin.right, 2.0);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn bw_colors_come_from_its_flavor() {
        let bw = Theme::from_spec(Some(&spec(r#"{"name": "bw"}"#)), &mut Vec::new()).unwrap();
        let border = bw.panel_background.unwrap().border.unwrap();
        assert_eq!(border.color, Color::rgb(0x33, 0x33, 0x33));
        assert_eq!(bw.panel_grid_major_x.unwrap().width, 1.4);
        assert_eq!(bw.panel_grid_minor_x.unwrap().width, 0.5);
    }

    #[test]
    fn errors_and_warnings_name_the_option() {
        let error = Theme::from_spec(
            Some(&spec(r#"{"legend_position": "middle"}"#)),
            &mut Vec::new(),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("theme.legend_position"), "{error}");
        let mut warnings = Vec::new();
        Theme::from_spec(
            Some(&spec(r#"{"strip_text": {"size": 3, "blank": false}, "axis_title_x": {"angle": 45, "blank": false}}"#)),
            &mut warnings,
        )
        .unwrap();
        assert_eq!(
            warnings,
            [
                "at 'theme.axis_title_x.angle': not supported yet; ignored",
                "at 'theme.strip_text': not supported yet; ignored"
            ]
        );
    }
}
