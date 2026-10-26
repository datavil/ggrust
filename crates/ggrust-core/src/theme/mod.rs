//! Themes: resolved style values for every non-data element of a plot.
//!
//! What it does: [`Theme::default`] is lets-plot's default theme (`minimal2`) with
//! lets-plot's constants. Layout and drawing read only this struct.
//!
//! What it does not do (yet): named themes (`theme_classic`, ...) and `theme(...)`
//! overrides from the spec. Those resolve into this same struct in Phase 1.4.

use crate::scene::{Color, Stroke};
use crate::text::FontFace;

/// lets-plot `SymbolicColor.BLACK` in the default flavor: the "ink" of text and lines.
pub const INK: Color = Color::rgb(0x47, 0x47, 0x47);

/// lets-plot `SymbolicColor.GREY_1` (`#E9E9E9`): panel grid lines.
pub const GRID: Color = Color::rgb(0xE9, 0xE9, 0xE9);

/// Style of a text element. A text element's layout block is `size` pixels tall
/// (lets-plot convention).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    /// Font size in pixels.
    pub size: f64,
    /// Text color.
    pub color: Color,
    /// Weight and slant.
    pub face: FontFace,
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
    /// As a solid scene stroke.
    pub fn stroke(self) -> Stroke {
        Stroke::solid(self.color, self.width)
    }
}

/// Spacing on the four sides of a box, in pixels.
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
    /// The same spacing on every side.
    pub const fn uniform(value: f64) -> Self {
        Margins {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// Component-wise sum.
    pub fn plus(self, other: Margins) -> Margins {
        Margins {
            top: self.top + other.top,
            right: self.right + other.right,
            bottom: self.bottom + other.bottom,
            left: self.left + other.left,
        }
    }
}

/// Resolved theme. `None` means the element is blank (not drawn, takes no space).
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// Space outside the plot background.
    pub plot_margin: Margins,
    /// Space between the plot background edge and its content.
    pub plot_inset: Margins,
    /// Plot background fill.
    pub plot_background: Option<Color>,
    /// Plot title.
    pub plot_title: Option<TextStyle>,
    /// Plot subtitle.
    pub plot_subtitle: Option<TextStyle>,
    /// Gap above the subtitle.
    pub plot_subtitle_margin_top: f64,
    /// x axis title.
    pub axis_title_x: Option<TextStyle>,
    /// y axis title.
    pub axis_title_y: Option<TextStyle>,
    /// x axis tick labels.
    pub axis_text_x: Option<TextStyle>,
    /// y axis tick labels.
    pub axis_text_y: Option<TextStyle>,
    /// Gap between ticks (or the axis) and tick labels.
    pub axis_text_spacing: f64,
    /// Gap between tick labels and the axis title. lets-plot has no such option; its
    /// baseline placement leaves about 3 px, which this reproduces with real metrics.
    pub axis_title_spacing: f64,
    /// x axis ticks.
    pub axis_ticks_x: Option<LineStyle>,
    /// y axis ticks.
    pub axis_ticks_y: Option<LineStyle>,
    /// Tick length.
    pub axis_ticks_length: f64,
    /// x axis line.
    pub axis_line_x: Option<LineStyle>,
    /// y axis line.
    pub axis_line_y: Option<LineStyle>,
    /// Vertical major grid lines (at x breaks).
    pub panel_grid_major_x: Option<LineStyle>,
    /// Horizontal major grid lines (at y breaks).
    pub panel_grid_major_y: Option<LineStyle>,
    /// Panel background fill.
    pub panel_background: Option<Color>,
    /// Legend title (`legend_title`); `None` hides it.
    pub legend_title: Option<TextStyle>,
    /// Legend labels (`legend_text`); `None` hides them.
    pub legend_text: Option<TextStyle>,
    /// Legend box fill (`legend_background`; lets-plot draws no border).
    pub legend_background: Option<Color>,
    /// Inner padding of a legend box (`legend_margin`).
    pub legend_margin: Margins,
    /// Smallest key cell, square (`legend_key_size`).
    pub legend_key_size: f64,
    /// Space between key cells (`legend_key_spacing`), horizontal and vertical.
    pub legend_key_spacing: (f64, f64),
    /// Space between stacked legend boxes (`legend_spacing`).
    pub legend_spacing: f64,
    /// Space between the plot area and the legends (`legend_box_spacing`).
    pub legend_box_spacing: f64,
}

impl Default for Theme {
    /// lets-plot's default theme (`minimal2`): grid lines, x axis line and ticks,
    /// no y axis line or ticks, no panel background.
    fn default() -> Self {
        let small = TextStyle {
            size: 13.0,
            color: INK,
            face: FontFace::Plain,
        };
        let medium = TextStyle {
            size: 15.0,
            ..small
        };
        let large = TextStyle {
            size: 16.0,
            ..small
        };
        let ink_line = LineStyle {
            color: INK,
            width: 1.0,
        };
        let grid_line = LineStyle {
            color: GRID,
            width: 1.0,
        };
        Theme {
            plot_margin: Margins::uniform(3.0),
            plot_inset: Margins::uniform(3.0),
            plot_background: Some(Color::WHITE),
            plot_title: Some(large),
            plot_subtitle: Some(medium),
            plot_subtitle_margin_top: 4.0,
            axis_title_x: Some(medium),
            axis_title_y: Some(medium),
            axis_text_x: Some(small),
            axis_text_y: Some(small),
            axis_text_spacing: 2.0,
            axis_title_spacing: 3.0,
            axis_ticks_x: Some(ink_line),
            axis_ticks_y: None,
            axis_ticks_length: 4.0,
            axis_line_x: Some(ink_line),
            axis_line_y: None,
            panel_grid_major_x: Some(grid_line),
            panel_grid_major_y: Some(grid_line),
            panel_background: None,
            // lets-plot `ThemeValuesBase` legend defaults.
            legend_title: Some(medium),
            legend_text: Some(small),
            legend_background: Some(Color::WHITE),
            legend_margin: Margins::uniform(5.0),
            legend_key_size: 23.0,
            legend_key_spacing: (0.0, 0.0),
            legend_spacing: 10.0,
            legend_box_spacing: 0.0,
        }
    }
}
