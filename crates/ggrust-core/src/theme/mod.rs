//! Themes: resolved style values for every non-data element of a plot.
//!
//! What it does: [`Theme::from_spec`] builds lets-plot's theme from the spec: a named
//! theme (`minimal2` by default, `classic`, `bw`, `grey`, `light`, `minimal`, `none`),
//! `theme(...)` options on top, a flavor for symbolic colors ([`values`]), then each
//! element resolved through lets-plot's inheritance chains (`axis_text_x` <-
//! `axis_text` <- `text` ...). Layout and drawing read only this struct.
//!
//! What it does not do (yet): text margins other than on strips and plot and axis
//! titles, text angles and vertical justification other than on axis labels and
//! titles; options that set them are reported as warnings.

mod values;

pub use values::{FLAVOR_NAMES, THEME_NAMES};

use values::{Paint, Props, Symbol, Values, theme_values};

use crate::geom::Arrow;
use crate::scale::number_format::{ExponentFormat, ExponentStyle};
use crate::scene::{Color, Stroke};
use crate::spec::{ThemeSpec, ThemeValue, Value};
use crate::style::{GeomPalette, LineType};
use crate::text::{Font, FontFace, FontFamily};
use crate::{Error, Result};

/// lets-plot `SymbolicColor.BLACK` in the default flavor: the "ink" of text and lines.
pub const INK: Color = Color::rgb(0x47, 0x47, 0x47);

/// lets-plot `SymbolicColor.GREY_1` (`#E9E9E9`): panel grid lines.
pub const GRID: Color = Color::rgb(0xE9, 0xE9, 0xE9);

/// ggside's default `ggside.panel.spacing`, 2 pt, in px (96 / 72 px per pt).
pub const GGSIDE_PANEL_SPACING: f64 = 2.0 * 96.0 / 72.0;
/// legendry's `legendry.bracket.size` (2 mm) in px at 96 dpi.
pub const LEGENDRY_BRACKET_SIZE: f64 = 2.0 * 96.0 / 25.4;
/// `legendry.guide.spacing` inside `guide_axis_nested()`: legendry sets 0 unless
/// the guide's own theme gives one.
pub const LEGENDRY_GUIDE_SPACING: f64 = 0.0;

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
    /// Vertical justification 0..1 (axis labels and titles); NaN: the element's default.
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
    /// Dash pattern (`linetype=`).
    pub dash: Dash,
}

impl LineStyle {
    /// As a stroke: the dash pattern scales with the width, as geom line types.
    pub fn stroke(self) -> Stroke {
        Stroke {
            dash: self
                .dash
                .lengths()
                .iter()
                .map(|length| length * self.width)
                .collect(),
            ..Stroke::solid(self.color, self.width)
        }
    }
}

/// Dash pattern of a theme line in units of its width (lets-plot line types:
/// codes, names or a dash list of at most [`Dash::MAX`] numbers). Empty is solid.
/// Fixed size so styles stay `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Dash {
    lengths: [f64; Dash::MAX],
    count: usize,
}

impl Dash {
    /// Longest dash list a theme line takes.
    pub const MAX: usize = 8;

    /// A pattern from a line type (see [`LineType`]); `None` if longer than [`Dash::MAX`].
    pub fn new(linetype: &LineType) -> Option<Dash> {
        let mut dash = Dash::default();
        if linetype.0.len() > Dash::MAX {
            return None;
        }
        dash.lengths[..linetype.0.len()].copy_from_slice(&linetype.0);
        dash.count = linetype.0.len();
        Some(dash)
    }

    /// Dash and gap lengths, in units of the line width.
    pub fn lengths(&self) -> &[f64] {
        &self.lengths[..self.count]
    }
}

/// An axis line: its style, optional arrow heads (`element_line(arrow=)`, as
/// ggplot2) and the fraction of the panel it covers from the origin corner
/// (`element_line(extent=)`; the left end for x, the bottom for y). Heads and the
/// extent are in pixels and panel space, not data space, so zoom does not move them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisLine {
    /// Line style.
    pub style: LineStyle,
    /// Arrow heads, if any.
    pub arrow: Option<Arrow>,
    /// Covered fraction of the panel length, `0 < extent <= 1`.
    pub extent: f64,
}

/// The elements of one side band's own axis (R ggside `ggside.axis.*`), for the
/// axis drawn on one screen side. Each falls back to the main axis element.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandAxisTheme {
    /// Tick labels (`ggside_axis_text*`).
    pub text: Option<TextStyle>,
    /// Ticks (`ggside_axis_ticks*`).
    pub ticks: Option<LineStyle>,
    /// Tick length (`ggside_axis_ticks_length*`).
    pub ticks_length: f64,
    /// Minor ticks (`ggside_axis_minor_ticks*`).
    pub minor_ticks: Option<LineStyle>,
    /// Minor tick length (`ggside_axis_minor_ticks_length*`).
    pub minor_ticks_length: f64,
    /// Axis line (`ggside_axis_line*`).
    pub line: Option<AxisLine>,
}

/// The side band axis elements by the screen side the band's axis is drawn on.
/// As in ggside, `_x` elements reach only the horizontal axes of y-side bands
/// (`top`, `bottom`), `_y` elements only the vertical axes of x-side bands
/// (`left`, `right`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandAxisThemes {
    /// Axis on top of a y-side band (`ggside_axis_*_x_top`).
    pub top: BandAxisTheme,
    /// Axis below a y-side band (`ggside_axis_*_x_bottom`).
    pub bottom: BandAxisTheme,
    /// Axis left of an x-side band (`ggside_axis_*_y_left`).
    pub left: BandAxisTheme,
    /// Axis right of an x-side band (`ggside_axis_*_y_right`).
    pub right: BandAxisTheme,
}

/// A side band's own panel background and border (R ggside `ggside.panel.*`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandPanelTheme {
    /// Background (`ggside_panel_background`).
    pub background: Option<RectStyle>,
    /// Border, drawn above the band's layers (`ggside_panel_border`).
    pub border: Option<LineStyle>,
}

/// The side band panel elements by the axis the band shares, resolved through
/// R ggside's chain (`ggside_xside_panel_border` <- `ggside_panel_border` <-
/// `panel_border`). Under `coord_flip` they swap with the other axis elements, so
/// `xside` always styles the bands above or below the panel on screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandPanelThemes {
    /// Bands that share the horizontal axis (`ggside_xside_panel_*`).
    pub xside: BandPanelTheme,
    /// Bands that share the vertical axis (`ggside_yside_panel_*`).
    pub yside: BandPanelTheme,
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

/// What a plot title or caption is justified along (lets-plot `TitlePosition`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitlePosition {
    /// The panels (lets-plot's default).
    Panel,
    /// The whole plot, inside its margin.
    Plot,
}

/// Where a plot tag goes (lets-plot `plot_tag_position`): fractions of its area,
/// 0 = left or bottom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TagPosition {
    /// A side or corner: the tag's box touches those edges of the area.
    Named(f64, f64),
    /// `[x, y]`: the box is centred there, kept inside the area.
    Point(f64, f64),
}

/// The area a plot tag is placed in (lets-plot `plot_tag_location`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagLocation {
    /// The plot inside its margin, over everything (the default).
    Plot,
    /// The panels.
    Panel,
    /// A band of the plot margin, reserved by the layout (named positions only;
    /// with `[x, y]` it acts as `Plot`, as lets-plot).
    Margin,
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
    /// Space around the title (`plot_title` margins).
    pub plot_title_margin: Margins,
    /// Space around the subtitle (`plot_subtitle` margins; 4 px on top by default).
    pub plot_subtitle_margin: Margins,
    /// Plot caption, below everything (`plot_caption`).
    pub plot_caption: Option<TextStyle>,
    /// Space around the caption (`plot_caption` margins).
    pub plot_caption_margin: Margins,
    /// What the title and subtitle are justified along (`plot_title_position`).
    pub plot_title_position: TitlePosition,
    /// What the caption is justified along (`plot_caption_position`).
    pub plot_caption_position: TitlePosition,
    /// Plot tag (`plot_tag`, `labs(tag=...)`); its margins only widen its box, as
    /// lets-plot.
    pub plot_tag: Option<TextStyle>,
    /// Space around the tag (`plot_tag` margins).
    pub plot_tag_margin: Margins,
    /// Where the tag goes in its area (`plot_tag_position`).
    pub plot_tag_position: TagPosition,
    /// The area the tag is placed in (`plot_tag_location`).
    pub plot_tag_location: TagLocation,
    /// Text before and after every tag (`plot_tag_prefix`, `plot_tag_suffix`).
    pub plot_tag_affixes: (String, String),
    /// x axis title.
    pub axis_title_x: Option<TextStyle>,
    /// y axis title.
    pub axis_title_y: Option<TextStyle>,
    /// Space around the x axis title; only top and bottom are used, as lets-plot.
    pub axis_title_x_margin: Margins,
    /// Space around the y axis title; only left and right are used.
    pub axis_title_y_margin: Margins,
    /// x tick labels.
    pub axis_text_x: Option<TextStyle>,
    /// y tick labels.
    pub axis_text_y: Option<TextStyle>,
    /// Space between ticks and their labels, x and y axis (`axis_text_spacing_x`,
    /// `axis_text_spacing_y`, `axis_text_spacing`).
    pub axis_text_spacing: (f64, f64),
    /// Space between the y title and the y labels (ggrust: lets-plot gets it from
    /// its baseline placement).
    pub axis_title_spacing: f64,
    /// x ticks.
    pub axis_ticks_x: Option<LineStyle>,
    /// y ticks.
    pub axis_ticks_y: Option<LineStyle>,
    /// Tick length, x and y axis (`axis_ticks_length_x`, `axis_ticks_length_y`,
    /// `axis_ticks_length`).
    pub axis_ticks_length: (f64, f64),
    /// x minor ticks, between the breaks (`axis_minor_ticks_x`; blank by default).
    pub axis_minor_ticks_x: Option<LineStyle>,
    /// y minor ticks (`axis_minor_ticks_y`).
    pub axis_minor_ticks_y: Option<LineStyle>,
    /// Minor tick length, x and y axis (`axis_minor_ticks_length_x`,
    /// `axis_minor_ticks_length_y`, `axis_minor_ticks_length`; 2 px by default).
    pub axis_minor_ticks_length: (f64, f64),
    /// x axis line.
    pub axis_line_x: Option<AxisLine>,
    /// y axis line.
    pub axis_line_y: Option<AxisLine>,
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
    /// Space around the legend title (`legend_title` margins, ggplot2).
    pub legend_title_margin: Margins,
    /// Smallest key cell, width and height (`legend_key_width`,
    /// `legend_key_height`, `legend_key_size`).
    pub legend_key_size: (f64, f64),
    /// Space between key cells (`legend_key_spacing`), horizontal and vertical.
    pub legend_key_spacing: (f64, f64),
    /// ggside side panel size relative to the main panel (`ggside_panel_scale`):
    /// the x side panel's height, the y side panel's width (data axes).
    pub ggside_panel_scale: (f64, f64),
    /// Gap between the main panel and a ggside side panel (`ggside_panel_spacing`,
    /// px): horizontal (left/right panels), vertical (top/bottom panels).
    pub ggside_panel_spacing: (f64, f64),
    /// Side band axis elements (`ggside_axis_text*`, `ggside_axis_ticks*`,
    /// `ggside_axis_line*`, ...), each resolved down to the main axis element.
    pub ggside_axis: BandAxisThemes,
    /// Side band panel background and border (`ggside_panel_background`,
    /// `ggside_panel_border` and their `ggside_xside_` / `ggside_yside_` forms),
    /// each resolved down to the main panel element.
    pub ggside_panel: BandPanelThemes,
    /// Bracket line of `guide_axis_nested()` (`legendry_bracket`, from `line`).
    pub legendry_bracket: Option<LineStyle>,
    /// Bracket depth across the axis, px (`legendry_bracket_size`).
    pub legendry_bracket_size: f64,
    /// Box of `guide_axis_nested(type="box")` (`legendry_box`, from `rect`; legendry
    /// inherits `strip.background`, blank in lets-plot themes).
    pub legendry_box: Option<RectStyle>,
    /// Gap between an axis's labels and its nested guide, px (`legendry_guide_spacing`).
    pub legendry_guide_spacing: f64,
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
    /// `legend_position_inside`: where inside legends go, (x, y) in 0..1 from the
    /// panel's bottom-left; `None` uses `legend_justification` (ggplot2 3.5).
    pub legend_position_inside: Option<(f64, f64)>,
    /// `legend_interactive` (ggrust extension, HTML only): clickable discrete
    /// legend keys. `false` drops the key data from the output.
    pub legend_interactive: bool,
    /// `legend_justification`: (x, y) in 0..1.
    pub legend_justification: (f64, f64),
    /// `legend_direction`.
    pub legend_direction: LegendDirection,
    /// `legend_box`: several legends side by side (`true`) or stacked.
    pub legend_box_horizontal: bool,
    /// `legend_box_just`: alignment of stacked legends (see
    /// [`Theme::legend_box_justification`]).
    pub legend_box_just: LegendBoxJust,
    /// Facet strip labels above the panels (`strip_text_x`); `None` hides the strips.
    pub strip_text: Option<TextStyle>,
    /// Space around a strip label (`strip_text_x` margins).
    pub strip_margin: Margins,
    /// Strip background (`strip_background_x`), drawn only with the strip text.
    pub strip_background: Option<RectStyle>,
    /// Space between strips and their panel (`strip_spacing_y`).
    pub strip_spacing: f64,
    /// `facet_grid` row strip labels right of the panels (`strip_text_y`); `None`
    /// hides those strips.
    pub strip_text_y: Option<TextStyle>,
    /// Space around a row strip label (`strip_text_y` margins).
    pub strip_margin_y: Margins,
    /// Row strip background (`strip_background_y`), drawn only with the text.
    pub strip_background_y: Option<RectStyle>,
    /// Space between row strips and their panel (`strip_spacing_x`).
    pub strip_spacing_x: f64,
    /// Space between facet panels (`panel_spacing_x`, `panel_spacing_y`).
    pub panel_spacing: (f64, f64),
    /// How numbers write exponents (`exponent_format`; see [`exponent_format`]).
    pub exponent_format: ExponentFormat,
    /// Axes and axis titles a containing `gggrid(axes=, axis_titles=)` collected
    /// from this plot, per screen side. Not a theme option: the grid sets it on
    /// the plot it prepared ([`crate::grid_axes`]).
    pub grid_collected: crate::layout::side::CollectedSides,
}

impl Default for Theme {
    /// lets-plot's default theme (`minimal2`).
    fn default() -> Self {
        // The built-in tables always resolve (unit-tested); no user input is involved.
        Theme::from_spec(None, &mut Vec::new()).expect("the built-in default theme resolves")
    }
}

/// The ggside side band axis elements (`ggside_axis_text`, `..._x`, `..._x_top`, ...).
const GGSIDE_AXIS_ELEMENTS: &[&str] = &[
    "text",
    "ticks",
    "ticks_length",
    "minor_ticks",
    "minor_ticks_length",
    "line",
];

/// The suffixes of every ggside side band axis element, most general first.
const GGSIDE_AXIS_SUFFIXES: &[&str] =
    &["", "_x", "_x_top", "_x_bottom", "_y", "_y_left", "_y_right"];

/// Whether `key` is a ggside side band axis option of `element` (`"text"`, ...).
fn is_ggside_axis(key: &str, element: &str) -> bool {
    key.strip_prefix("ggside_axis_")
        .and_then(|rest| rest.strip_prefix(element))
        .is_some_and(|suffix| GGSIDE_AXIS_SUFFIXES.contains(&suffix))
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
    "axis_text_spacing_x",
    "axis_text_spacing_y",
    "axis_ticks",
    "axis_ticks_x",
    "axis_ticks_y",
    "axis_ticks_length",
    "axis_ticks_length_x",
    "axis_ticks_length_y",
    "axis_minor_ticks",
    "axis_minor_ticks_x",
    "axis_minor_ticks_y",
    "axis_minor_ticks_length",
    "axis_minor_ticks_length_x",
    "axis_minor_ticks_length_y",
    "exponent_format",
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
    "plot_caption",
    "plot_title_position",
    "plot_caption_position",
    "plot_tag",
    "plot_tag_position",
    "plot_tag_location",
    "plot_tag_prefix",
    "plot_tag_suffix",
    "plot_margin",
    "plot_inset",
    "legend_background",
    "legend_text",
    "legend_title",
    "legend_key",
    "legend_position",
    "legend_position_inside",
    "legend_justification",
    "legend_direction",
    "legend_margin",
    "legend_spacing",
    "legend_spacing_x",
    "legend_spacing_y",
    "legend_key_size",
    "legend_key_width",
    "legend_key_height",
    "legend_key_spacing",
    "legend_key_spacing_x",
    "legend_key_spacing_y",
    "ggside_panel_scale",
    "ggside_panel_scale_x",
    "ggside_panel_scale_y",
    "ggside_panel_spacing",
    "ggside_panel_spacing_x",
    "ggside_panel_spacing_y",
    "ggside_panel_background",
    "ggside_xside_panel_background",
    "ggside_yside_panel_background",
    "ggside_panel_border",
    "ggside_xside_panel_border",
    "ggside_yside_panel_border",
    "legendry_bracket",
    "legendry_bracket_size",
    "legendry_box",
    "legendry_guide_spacing",
    "legend_box",
    "legend_box_just",
    "legend_box_spacing",
    "legend_ticks",
    "legend_ticks_length",
    "legend_interactive",
    "strip_text",
    "strip_text_x",
    "strip_text_y",
    "strip_background",
    "strip_background_x",
    "strip_background_y",
    "strip_spacing",
    "strip_spacing_x",
    "strip_spacing_y",
    "panel_spacing",
    "panel_spacing_x",
    "panel_spacing_y",
    "flavor",
    "geom",
];

impl Theme {
    /// The theme with every x and y axis element swapped (`coord_flip`: lets-plot
    /// keeps elements with their aesthetic, so x's go to the vertical axis).
    pub fn flipped(&self) -> Theme {
        let mut theme = self.clone();
        std::mem::swap(&mut theme.axis_title_x, &mut theme.axis_title_y);
        std::mem::swap(
            &mut theme.axis_title_x_margin,
            &mut theme.axis_title_y_margin,
        );
        std::mem::swap(&mut theme.axis_text_x, &mut theme.axis_text_y);
        std::mem::swap(&mut theme.axis_ticks_x, &mut theme.axis_ticks_y);
        std::mem::swap(&mut theme.axis_minor_ticks_x, &mut theme.axis_minor_ticks_y);
        std::mem::swap(&mut theme.axis_line_x, &mut theme.axis_line_y);
        std::mem::swap(&mut theme.panel_grid_major_x, &mut theme.panel_grid_major_y);
        std::mem::swap(&mut theme.panel_grid_minor_x, &mut theme.panel_grid_minor_y);
        let swap = |(x, y): (f64, f64)| (y, x);
        theme.axis_text_spacing = swap(theme.axis_text_spacing);
        theme.axis_ticks_length = swap(theme.axis_ticks_length);
        theme.axis_minor_ticks_length = swap(theme.axis_minor_ticks_length);
        // Band axes follow their aesthetic too: x bottom <-> y left, top <-> right.
        let axes = &mut theme.ggside_axis;
        std::mem::swap(&mut axes.bottom, &mut axes.left);
        std::mem::swap(&mut axes.top, &mut axes.right);
        let panels = &mut theme.ggside_panel;
        std::mem::swap(&mut panels.xside, &mut panels.yside);
        theme
    }

    /// The theme of a spec (named theme + `theme(...)` options + flavor). Options and
    /// element properties ggrust does not draw yet are reported in `warnings`.
    pub fn from_spec(spec: Option<&ThemeSpec>, warnings: &mut Vec<String>) -> Result<Theme> {
        let (values, flavor) = theme_values(spec).map_err(Error::spec_located)?;
        if let Some(spec) = spec {
            report_unbuilt(spec, warnings);
        }
        let mut resolver = Resolver {
            values: &values,
            flavor,
            warnings: Vec::new(),
        };
        let mut theme = resolver.theme()?;
        theme.exponent_format = exponent_format(spec)?;
        for warning in resolver.warnings {
            if !warnings.contains(&warning) {
                warnings.push(warning);
            }
        }
        Ok(theme)
    }
}

/// The geom colors of a spec's theme (lets-plot `ThemeFlavor` and `element_geom`):
/// pen is the flavor's ink, paper its background, brush lets-plot's blue; each one
/// `theme(geom=element_geom(...))` sets wins.
pub fn geom_palette(spec: Option<&ThemeSpec>) -> Result<GeomPalette> {
    let (_, flavor) = theme_values(spec).map_err(Error::spec_located)?;
    let mut palette = GeomPalette {
        pen: flavor.color(Symbol::Black),
        brush: GeomPalette::STANDARD.brush,
        paper: flavor.color(Symbol::White),
    };
    let element = spec.and_then(|spec| match spec.elements.get("geom") {
        Some(ThemeValue::Element(element)) => Some(element),
        _ => None,
    });
    if let Some(element) = element {
        let flavored = palette;
        for (name, given, slot) in [
            ("pen", &element.pen, &mut palette.pen),
            ("brush", &element.brush, &mut palette.brush),
            ("paper", &element.paper, &mut palette.paper),
        ] {
            if let Some(text) = given {
                *slot = flavored
                    .parse_color(text)
                    .map_err(|message| Error::spec(format!("theme.geom.{name}"), message))?;
            }
        }
    }
    Ok(palette)
}

fn report_unbuilt(spec: &ThemeSpec, warnings: &mut Vec<String>) {
    for (key, value) in &spec.elements {
        let ggside_axis = GGSIDE_AXIS_ELEMENTS
            .iter()
            .any(|element| is_ggside_axis(key, element));
        if !BUILT_OPTIONS.contains(&key.as_str()) && !ggside_axis {
            warnings.push(format!("at 'theme.{key}': not supported yet; ignored"));
            continue;
        }
        let ThemeValue::Element(element) = value else {
            continue;
        };
        // Axis labels read angle and vjust (layout/rotated.rs); other text does not yet.
        let axis_text = matches!(key.as_str(), "axis_text" | "axis_text_x" | "axis_text_y")
            || is_ggside_axis(key, "text");
        // Axis titles read angle and vjust (layout `axis_title_justification`):
        // turned by the angle, vjust moves a title along its axis.
        let axis_title = matches!(key.as_str(), "axis_title" | "axis_title_x" | "axis_title_y");
        // Tags read vjust: it places the text in the tag's box.
        let plot_tag = key == "plot_tag";
        // Strip labels (layout/facet.rs), plot, axis and legend titles and captions
        // read their margins.
        let strip_text = matches!(
            key.as_str(),
            "strip_text"
                | "strip_text_x"
                | "strip_text_y"
                | "title"
                | "plot_title"
                | "plot_subtitle"
                | "plot_caption"
                | "plot_tag"
                | "axis_title"
                | "axis_title_x"
                | "axis_title_y"
                | "legend_title"
        );
        let unbuilt = [
            ("margin", element.margin.is_some() && !strip_text),
            (
                "vjust",
                element.vjust.is_some() && !axis_text && !axis_title && !plot_tag,
            ),
            (
                "angle",
                element.angle.is_some() && !axis_text && !axis_title,
            ),
            ("lineheight", element.lineheight.is_some()),
        ];
        for (property, present) in unbuilt {
            if present {
                warnings.push(format!(
                    "at 'theme.{key}.{property}': not supported yet; ignored"
                ));
            }
        }
        let axis_line = matches!(key.as_str(), "axis_line" | "axis_line_x" | "axis_line_y")
            || is_ggside_axis(key, "line");
        for (property, present) in [
            ("arrow", element.arrow.is_some()),
            ("extent", element.extent.is_some()),
        ] {
            if present && !axis_line {
                warnings.push(format!(
                    "at 'theme.{key}.{property}': only axis lines take {property} \
                     (axis_line, axis_line_x, axis_line_y); ignored"
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
        // One path for every element, so a family inherited by many warns once.
        let family_path = "theme";
        let family = match props.family.as_deref() {
            None => FontFamily::Sans,
            Some(name) => {
                self.warnings
                    .extend(crate::text::family_warning(name, family_path));
                FontFamily::from_name(name).0
            }
        };
        self.warnings.extend(crate::text::face_warning(
            Font { family, face },
            family_path,
        ));
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
            dash: dash(&props),
        })
    }

    /// An axis line from its style chain; arrow and extent come only from the
    /// first `line_keys` keys, the axis line options (`axis_line_x`, `axis_line`),
    /// not from `line` or `axis`.
    fn axis_line(&self, keys: &[&str], line_keys: usize) -> Option<AxisLine> {
        let style = self.line(keys)?;
        let props = self.values.element_props(&keys[..line_keys]);
        Some(AxisLine {
            style,
            arrow: props.arrow,
            extent: props.extent.unwrap_or(1.0),
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
                dash: dash(&props),
            }),
        })
    }

    fn margins(&self, keys: &[&str], default: f64) -> Margins {
        let margin = self.values.element_props(keys).margin;
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
                    Error::spec(
                        format!("theme.{}", keys[0]),
                        format!("expected a number, got {value}"),
                    )
                }),
        }
    }

    /// The side band axis elements for the `axis` ("x" or "y") drawn on screen
    /// `side`: R ggside's chain (`ggside_axis_text_y_left` <- `ggside_axis_text_y`
    /// <- `ggside_axis_text`), then the main axis element's chain.
    fn band_axis(&mut self, axis: &str, side: &str) -> Result<BandAxisTheme> {
        let chain = |element: &str, main: &[&str]| -> Vec<String> {
            [
                format!("ggside_axis_{element}_{axis}_{side}"),
                format!("ggside_axis_{element}_{axis}"),
                format!("ggside_axis_{element}"),
            ]
            .into_iter()
            .chain(main.iter().map(|key| (*key).to_owned()))
            .collect()
        };
        let axis_key = format!("axis_{axis}");
        let text = chain(
            "text",
            &[
                &format!("axis_text_{axis}"),
                "axis_text",
                "text",
                &axis_key,
                "axis",
            ],
        );
        let ticks = chain(
            "ticks",
            &[
                &format!("axis_ticks_{axis}"),
                "axis_ticks",
                &axis_key,
                "axis",
                "line",
            ],
        );
        let ticks_length = chain(
            "ticks_length",
            &[&format!("axis_ticks_length_{axis}"), "axis_ticks_length"],
        );
        let minor_ticks = chain(
            "minor_ticks",
            &[
                &format!("axis_minor_ticks_{axis}"),
                "axis_minor_ticks",
                &axis_key,
                "axis",
                "line",
            ],
        );
        let minor_ticks_length = chain(
            "minor_ticks_length",
            &[
                &format!("axis_minor_ticks_length_{axis}"),
                "axis_minor_ticks_length",
            ],
        );
        let line = chain(
            "line",
            &[
                &format!("axis_line_{axis}"),
                "axis_line",
                &axis_key,
                "axis",
                "line",
            ],
        );
        Ok(BandAxisTheme {
            text: self.band_text(&as_refs(&text)),
            ticks: self.line(&as_refs(&ticks)),
            ticks_length: self.number_of_set(&as_refs(&ticks_length), 4.0)?,
            minor_ticks: self.line(&as_refs(&minor_ticks)),
            minor_ticks_length: self.number_of_set(&as_refs(&minor_ticks_length), 2.0)?,
            line: self.axis_line(&as_refs(&line), 5),
        })
    }

    /// The panel elements of the bands sharing one axis (`"xside"` or `"yside"`):
    /// R ggside's chain (`ggside_xside_panel_border` <- `ggside_panel_border`),
    /// then the main panel element's chain.
    fn band_panel(&self, axis: &str) -> BandPanelTheme {
        let background = format!("ggside_{axis}_panel_background");
        let border = format!("ggside_{axis}_panel_border");
        BandPanelTheme {
            background: self.rect(&[
                &background,
                "ggside_panel_background",
                "panel_background",
                "rect",
            ]),
            border: self
                .rect(&[&border, "ggside_panel_border", "panel_border", "rect"])
                .and_then(|rect| rect.border),
        }
    }

    /// [`Self::text`] for a band axis: resolved from the first key that is set, so
    /// warnings name it; warnings from a main axis key are dropped, since the main
    /// element reports them already.
    fn band_text(&mut self, keys: &[&str]) -> Option<TextStyle> {
        let first = keys
            .iter()
            .position(|key| self.values.has_element(key))
            .unwrap_or(0);
        let reported = self.warnings.len();
        let text = self.text(&keys[first..]);
        if !keys[first].starts_with("ggside_") {
            self.warnings.truncate(reported);
        }
        text
    }

    /// [`Self::number`], with an error naming the key that holds the bad value
    /// (a band axis chain starts at keys the user may not have written).
    fn number_of_set(&self, keys: &[&str], default: f64) -> Result<f64> {
        match keys
            .iter()
            .position(|key| self.values.value_of(&[key]).is_some())
        {
            None => Ok(default),
            Some(index) => self.number(&keys[index..], default),
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
            dash: dash(&props),
        });
        let strip_text = self.text(&["strip_text_x", "strip_text", "text"]);
        let strip_text_y = self.text(&["strip_text_y", "strip_text", "text"]);
        Ok(Theme {
            plot_margin: self.margins(&["plot_margin"], 3.0),
            plot_inset: self.margins(&["plot_inset"], 3.0),
            plot_background: self.rect(&["plot_background", "rect"]),
            plot_title: self.text(&["plot_title", "title", "text"]),
            plot_subtitle: self.text(&["plot_subtitle", "title", "text"]),
            plot_title_margin: self.margins(&["plot_title", "title", "text"], 0.0),
            plot_subtitle_margin: self.margins(&["plot_subtitle", "title", "text"], 0.0),
            plot_caption: self.text(&["plot_caption", "title", "text"]),
            plot_caption_margin: self.margins(&["plot_caption", "title", "text"], 0.0),
            plot_title_position: title_position(
                "plot_title_position",
                self.values.value_of(&["plot_title_position"]),
            )?,
            plot_caption_position: title_position(
                "plot_caption_position",
                self.values.value_of(&["plot_caption_position"]),
            )?,
            plot_tag: self.text(&["plot_tag", "title", "text"]),
            plot_tag_margin: self.margins(&["plot_tag", "title", "text"], 0.0),
            plot_tag_position: tag_position(self.values.value_of(&["plot_tag_position"]))?,
            plot_tag_location: tag_location(self.values.value_of(&["plot_tag_location"]))?,
            plot_tag_affixes: (
                affix(self.values, "plot_tag_prefix")?,
                affix(self.values, "plot_tag_suffix")?,
            ),
            axis_title_x: self.text(&[&title_x, "axis_title", "title", "text", &axis_x, "axis"]),
            axis_title_y: self.text(&[&title_y, "axis_title", "title", "text", &axis_y, "axis"]),
            axis_title_x_margin: self.margins(
                &[&title_x, "axis_title", "title", "text", &axis_x, "axis"],
                0.0,
            ),
            axis_title_y_margin: self.margins(
                &[&title_y, "axis_title", "title", "text", &axis_y, "axis"],
                0.0,
            ),
            axis_text_x: self.text(&[&text_x, "axis_text", "text", &axis_x, "axis"]),
            axis_text_y: self.text(&[&text_y, "axis_text", "text", &axis_y, "axis"]),
            axis_text_spacing: (
                self.number(&["axis_text_spacing_x", "axis_text_spacing"], 2.0)?,
                self.number(&["axis_text_spacing_y", "axis_text_spacing"], 2.0)?,
            ),
            axis_title_spacing: 3.0,
            axis_ticks_x: self.line(&[&ticks_x, "axis_ticks", &axis_x, "axis", "line"]),
            axis_ticks_y: self.line(&[&ticks_y, "axis_ticks", &axis_y, "axis", "line"]),
            axis_ticks_length: (
                self.number(&["axis_ticks_length_x", "axis_ticks_length"], 4.0)?,
                self.number(&["axis_ticks_length_y", "axis_ticks_length"], 4.0)?,
            ),
            // lets-plot: minor ticks inherit from the axis and `line`, not from
            // `axis_ticks` (measured).
            axis_minor_ticks_x: self.line(&[
                "axis_minor_ticks_x",
                "axis_minor_ticks",
                &axis_x,
                "axis",
                "line",
            ]),
            axis_minor_ticks_y: self.line(&[
                "axis_minor_ticks_y",
                "axis_minor_ticks",
                &axis_y,
                "axis",
                "line",
            ]),
            axis_minor_ticks_length: (
                self.number(
                    &["axis_minor_ticks_length_x", "axis_minor_ticks_length"],
                    2.0,
                )?,
                self.number(
                    &["axis_minor_ticks_length_y", "axis_minor_ticks_length"],
                    2.0,
                )?,
            ),
            axis_line_x: self.axis_line(&[&line_x, "axis_line", &axis_x, "axis", "line"], 2),
            axis_line_y: self.axis_line(&[&line_y, "axis_line", &axis_y, "axis", "line"], 2),
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
            legend_margin: self.margins(&["legend_margin"], 5.0),
            legend_title_margin: self.margins(&["legend_title", "title", "text"], 0.0),
            legend_key_size: (
                self.number(&["legend_key_width", "legend_key_size"], 23.0)?,
                self.number(&["legend_key_height", "legend_key_size"], 23.0)?,
            ),
            legend_key_spacing: (
                self.number(&["legend_key_spacing_x", "legend_key_spacing"], 0.0)?,
                self.number(&["legend_key_spacing_y", "legend_key_spacing"], 0.0)?,
            ),
            // ggside defaults: 0.1 of the main panel, 2 pt apart.
            ggside_panel_scale: (
                self.number(&["ggside_panel_scale_x", "ggside_panel_scale"], 0.1)?,
                self.number(&["ggside_panel_scale_y", "ggside_panel_scale"], 0.1)?,
            ),
            ggside_panel_spacing: (
                self.number(
                    &["ggside_panel_spacing_x", "ggside_panel_spacing"],
                    GGSIDE_PANEL_SPACING,
                )?,
                self.number(
                    &["ggside_panel_spacing_y", "ggside_panel_spacing"],
                    GGSIDE_PANEL_SPACING,
                )?,
            ),
            ggside_axis: BandAxisThemes {
                top: self.band_axis("x", "top")?,
                bottom: self.band_axis("x", "bottom")?,
                left: self.band_axis("y", "left")?,
                right: self.band_axis("y", "right")?,
            },
            ggside_panel: BandPanelThemes {
                xside: self.band_panel("xside"),
                yside: self.band_panel("yside"),
            },
            legendry_bracket: self.line(&["legendry_bracket", "line"]),
            legendry_bracket_size: self
                .number(&["legendry_bracket_size"], LEGENDRY_BRACKET_SIZE)?,
            legendry_box: self.rect(&["legendry_box", "rect"]),
            legendry_guide_spacing: self
                .number(&["legendry_guide_spacing"], LEGENDRY_GUIDE_SPACING)?,
            legend_spacing: (
                self.number(&["legend_spacing_x", "legend_spacing"], 10.0)?,
                self.number(&["legend_spacing_y", "legend_spacing"], 10.0)?,
            ),
            legend_box_spacing: self.number(&["legend_box_spacing"], 0.0)?,
            legend_ticks,
            legend_ticks_length: self.number(&["legend_ticks_length"], 4.0)?,
            legend_position: legend_position(
                self.values.value_of(&["legend_position"]),
                self.values
                    .value_of(&["legend_position_inside"])
                    .map(inside_point)
                    .transpose()?
                    .unwrap_or(legend_justification(
                        self.values.value_of(&["legend_justification"]),
                    )?),
            )?,
            legend_position_inside: self
                .values
                .value_of(&["legend_position_inside"])
                .map(inside_point)
                .transpose()?,
            legend_interactive: legend_interactive(self.values.value_of(&["legend_interactive"]))?,
            legend_justification: legend_justification(
                self.values.value_of(&["legend_justification"]),
            )?,
            legend_direction: legend_direction(self.values.value_of(&["legend_direction"]))?,
            legend_box_horizontal: match self.values.value_of(&["legend_box"]) {
                None => false,
                Some(Value::String(name)) if name == "vertical" => false,
                Some(Value::String(name)) if name == "horizontal" => true,
                Some(other) => {
                    return Err(Error::spec(
                        "theme.legend_box",
                        format!("expected 'horizontal' or 'vertical', got {other}"),
                    ));
                }
            },
            legend_box_just: legend_box_just(self.values.value_of(&["legend_box_just"]))?,
            strip_text,
            strip_margin: self.margins(&["strip_text_x", "strip_text"], 3.0),
            // lets-plot `showStripBackground`: only with the strip text.
            strip_background: strip_text
                .and_then(|_| self.rect(&["strip_background_x", "strip_background", "rect"])),
            strip_spacing: self.number(&["strip_spacing_y", "strip_spacing"], 0.0)?,
            strip_text_y,
            strip_margin_y: self.margins(&["strip_text_y", "strip_text"], 3.0),
            strip_background_y: strip_text_y
                .and_then(|_| self.rect(&["strip_background_y", "strip_background", "rect"])),
            strip_spacing_x: self.number(&["strip_spacing_x", "strip_spacing"], 0.0)?,
            panel_spacing: (
                self.number(&["panel_spacing_x", "panel_spacing"], 10.0)?,
                self.number(&["panel_spacing_y", "panel_spacing"], 10.0)?,
            ),
            exponent_format: ExponentFormat::default(),
            grid_collected: Default::default(),
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

    /// Where inside legends go (ggplot2 `legend.position.inside`, else a numeric
    /// `legend_position`, else the justification): (x, y) in 0..1 from the panel's bottom-left.
    pub fn inside_point(&self) -> (f64, f64) {
        match (self.legend_position_inside, self.legend_position) {
            (Some(point), _) => point,
            // A numeric `legend_position` is the inside point (ggplot2 3.5).
            (None, LegendPosition::Inside(x, y)) => (x, y),
            (None, _) => self.legend_justification,
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
        Error::spec(
            format!("theme.{key}"),
            format!("expected a name or two numbers, got {value}"),
        )
    })
}

/// `legend_position_inside`: two numbers.
fn inside_point(value: &Value) -> Result<(f64, f64)> {
    pair(value, "legend_position_inside").map_err(|_| {
        Error::spec(
            "theme.legend_position_inside",
            format!("expected two numbers [x, y] (0 to 1 over the panel), got {value}"),
        )
    })
}

fn as_refs(keys: &[String]) -> Vec<&str> {
    keys.iter().map(String::as_str).collect()
}

/// lets-plot `LegendThemeConfig.toPosition`.
fn legend_interactive(value: Option<&Value>) -> Result<bool> {
    match value {
        None => Ok(true),
        Some(Value::Bool(flag)) => Ok(*flag),
        Some(other) => Err(Error::spec(
            "theme.legend_interactive",
            format!("expected True or False, got {other}"),
        )),
    }
}

/// `legend_position`; `"inside"` puts legends at `inside` (ggplot2 3.5).
fn legend_position(value: Option<&Value>, inside: (f64, f64)) -> Result<LegendPosition> {
    Ok(match value {
        None => LegendPosition::Right,
        Some(Value::String(name)) => match name.as_str() {
            "inside" => LegendPosition::Inside(inside.0, inside.1),
            "right" => LegendPosition::Right,
            "left" => LegendPosition::Left,
            "top" => LegendPosition::Top,
            "bottom" => LegendPosition::Bottom,
            "none" => LegendPosition::None,
            _ => {
                return Err(Error::spec(
                    "theme.legend_position",
                    format!(
                        "expected left, right, top, bottom, inside, none or two numbers, \
                         got '{name}'{}",
                        crate::suggest::did_you_mean(
                            name,
                            ["left", "right", "top", "bottom", "inside", "none"]
                        )
                    ),
                ));
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
                return Err(Error::spec(
                    "theme.legend_justification",
                    format!(
                        "expected center, left, right, top, bottom \
                     or two numbers, got '{name}'"
                    ),
                ));
            }
        },
        Some(value) => pair(value, "legend_justification")?,
    })
}

/// The dash pattern of resolved element properties (validated when the theme
/// options were read, see `values::user_props`).
fn dash(props: &Props) -> Dash {
    props
        .linetype
        .as_ref()
        .and_then(|value| LineType::parse(value).ok())
        .and_then(|linetype| Dash::new(&linetype))
        .unwrap_or_default()
}

/// The theme's `exponent_format` (lets-plot's default `pow` without one): a style
/// name, or `[style, lower, upper]` with optional integer bounds. Number labels
/// read it while scales are built, before the rest of the theme is resolved.
pub fn exponent_format(spec: Option<&ThemeSpec>) -> Result<ExponentFormat> {
    let Some(ThemeValue::Scalar(value)) =
        spec.and_then(|spec| spec.elements.get("exponent_format"))
    else {
        return Ok(ExponentFormat::default());
    };
    let invalid = |problem: String| {
        Error::spec(
            "theme.exponent_format",
            format!(
                "{problem}; expected 'e', 'pow' or 'pow_full', or a tuple (style, lower, upper) \
                 with integer or None bounds, e.g. ('e', -3, 3)"
            ),
        )
    };
    let style = |value: &Value| match value {
        Value::String(name) => ExponentStyle::from_name(name).ok_or_else(|| {
            let suggestion = crate::suggest::did_you_mean(name, ["e", "pow", "pow_full"]);
            invalid(format!("unknown style '{name}'{suggestion}"))
        }),
        other => Err(invalid(format!("got style {other}"))),
    };
    let bound = |value: Option<&Value>| match value {
        None | Some(Value::Null) => Ok(None),
        // Whole floats (`-3.0`) count, as lets-plot.
        Some(value) => value
            .as_f64()
            .filter(|number| number.fract() == 0.0 && number.abs() <= f64::from(i32::MAX))
            .map(|number| number as i32)
            .map(Some)
            .ok_or_else(|| invalid(format!("got bound {value}"))),
    };
    match value {
        Value::Array(items) if (1..=3).contains(&items.len()) => Ok(ExponentFormat {
            style: style(&items[0])?,
            min: bound(items.get(1))?,
            max: bound(items.get(2))?,
        }),
        Value::Array(items) => Err(invalid(format!("got {} items", items.len()))),
        other => Ok(ExponentFormat {
            style: style(other)?,
            min: None,
            max: None,
        }),
    }
}

fn title_position(key: &str, value: Option<&Value>) -> Result<TitlePosition> {
    match value {
        None => Ok(TitlePosition::Panel),
        Some(Value::String(name)) if name == "panel" => Ok(TitlePosition::Panel),
        Some(Value::String(name)) if name == "plot" => Ok(TitlePosition::Plot),
        Some(other) => Err(Error::spec(
            format!("theme.{key}"),
            format!("expected 'panel' or 'plot', got {other}"),
        )),
    }
}

/// lets-plot `plot_tag_position`: a side or corner name (`top-left`, `topleft` or
/// `top_left`), or `[x, y]` (kept inside the area, as lets-plot clamps it).
fn tag_position(value: Option<&Value>) -> Result<TagPosition> {
    const NAMES: [&str; 8] = [
        "left",
        "top-left",
        "top",
        "top-right",
        "right",
        "bottom-right",
        "bottom",
        "bottom-left",
    ];
    Ok(match value {
        None => TagPosition::Named(0.0, 1.0),
        Some(Value::String(name)) => {
            let normal = name.replace('_', "-");
            let normal = match normal.as_str() {
                "topleft" => "top-left",
                "topright" => "top-right",
                "bottomleft" => "bottom-left",
                "bottomright" => "bottom-right",
                other => other,
            };
            let (x, y) = match normal {
                "left" => (0.0, 0.5),
                "top-left" => (0.0, 1.0),
                "top" => (0.5, 1.0),
                "top-right" => (1.0, 1.0),
                "right" => (1.0, 0.5),
                "bottom-right" => (1.0, 0.0),
                "bottom" => (0.5, 0.0),
                "bottom-left" => (0.0, 0.0),
                _ => {
                    return Err(Error::spec(
                        "theme.plot_tag_position",
                        format!(
                            "expected one of {} or [x, y], got '{name}'{}",
                            NAMES.join(", "),
                            crate::suggest::did_you_mean(name, NAMES)
                        ),
                    ));
                }
            };
            TagPosition::Named(x, y)
        }
        Some(value) => {
            let (x, y) = pair(value, "plot_tag_position")?;
            TagPosition::Point(x, y)
        }
    })
}

/// lets-plot `plot_tag_location`.
fn tag_location(value: Option<&Value>) -> Result<TagLocation> {
    match value {
        None => Ok(TagLocation::Plot),
        Some(Value::String(name)) if name == "plot" => Ok(TagLocation::Plot),
        Some(Value::String(name)) if name == "panel" => Ok(TagLocation::Panel),
        Some(Value::String(name)) if name == "margin" => Ok(TagLocation::Margin),
        Some(other) => Err(Error::spec(
            "theme.plot_tag_location",
            format!("expected 'plot', 'panel' or 'margin', got {other}"),
        )),
    }
}

/// `plot_tag_prefix` / `plot_tag_suffix`: a string, empty by default.
fn affix(values: &Values, key: &str) -> Result<String> {
    match values.value_of(&[key]) {
        None => Ok(String::new()),
        Some(Value::String(text)) => Ok(text.clone()),
        Some(other) => Err(Error::spec(
            format!("theme.{key}"),
            format!("expected a string, got {other}"),
        )),
    }
}

fn legend_direction(value: Option<&Value>) -> Result<LegendDirection> {
    match value {
        None => Ok(LegendDirection::Auto),
        Some(Value::String(name)) if name == "horizontal" => Ok(LegendDirection::Horizontal),
        Some(Value::String(name)) if name == "vertical" => Ok(LegendDirection::Vertical),
        Some(other) => Err(Error::spec(
            "theme.legend_direction",
            format!("expected 'horizontal' or 'vertical', got {other}"),
        )),
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
            _ => Err(Error::spec(
                "theme.legend_box_just",
                format!(
                    "expected left, right, top, bottom or center, got \
                 '{name}'"
                ),
            )),
        },
        Some(other) => Err(Error::spec(
            "theme.legend_box_just",
            format!("expected a name, got {other}"),
        )),
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
            dash: Dash::default(),
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
        assert_eq!(theme.plot_subtitle_margin.top, 4.0);
        assert_eq!(theme.plot_title_position, TitlePosition::Panel);
        let text_x = theme.axis_text_x.unwrap();
        assert_eq!(
            (text_x.size, text_x.color, text_x.font),
            (small.size, small.color, small.font)
        );
        let title_x = theme.axis_title_x.unwrap();
        assert_eq!((title_x.size, title_x.hjust), (15.0, 0.5));
        assert_eq!(theme.axis_ticks_x, Some(ink));
        assert_eq!(theme.axis_ticks_y, None);
        assert_eq!(theme.axis_line_x.map(|line| line.style), Some(ink));
        assert_eq!(theme.axis_line_y, None);
        let grid = LineStyle {
            color: GRID,
            width: 1.0,
            dash: Dash::default(),
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
                width: 1.0,
                dash: Dash::default(),
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
            custom.axis_line_y.map(|line| line.style),
            Some(LineStyle {
                color: Color::rgb(255, 0, 0),
                width: 1.0,
                dash: Dash::default(),
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
    fn dark_and_linedraw_follow_ggplot2() {
        let theme = |name: &str| {
            Theme::from_spec(
                Some(&spec(&format!(r#"{{"name": "{name}"}}"#))),
                &mut Vec::new(),
            )
            .unwrap()
        };
        // ggplot2 4.0.1 theme_dark: col_mix(ink, paper, 0.499 / 0.42 / 0.15 / 0.899).
        let dark = theme("dark");
        let panel = dark.panel_background.unwrap();
        assert_eq!(panel.fill, Color::rgb(0x7F, 0x7F, 0x7F));
        assert_eq!(panel.border, None);
        let grid = dark.panel_grid_major_x.unwrap();
        assert_eq!(
            (grid.color, grid.width),
            (Color::rgb(0x6B, 0x6B, 0x6B), 0.7)
        );
        assert_eq!(dark.panel_grid_minor_x.unwrap().width, 0.25);
        assert_eq!(dark.axis_ticks_x.unwrap().width, 0.7);
        assert_eq!(
            dark.strip_background.unwrap().fill,
            Color::rgb(0x26, 0x26, 0x26)
        );
        assert_eq!(dark.strip_text.unwrap().color, Color::rgb(0xE5, 0xE5, 0xE5));

        // theme_linedraw: theme_bw in pure ink.
        let linedraw = theme("linedraw");
        let border = linedraw.panel_background.unwrap().border.unwrap();
        assert_eq!(border.color, Color::BLACK);
        let grid = linedraw.panel_grid_major_x.unwrap();
        assert_eq!((grid.color, grid.width), (Color::BLACK, 0.14));
        assert_eq!(linedraw.panel_grid_minor_x.unwrap().width, 0.07);
        assert_eq!(linedraw.strip_background.unwrap().fill, Color::BLACK);
        assert_eq!(linedraw.strip_text.unwrap().color, Color::WHITE);
        assert_eq!(linedraw.axis_text_x.unwrap().color, Color::BLACK);
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
            Some(&spec(r#"{"strip_text_y": {"size": 3, "angle": 0, "blank": false}, "plot_title": {"angle": 45, "blank": false}}"#)),
            &mut warnings,
        )
        .unwrap();
        assert_eq!(
            warnings,
            [
                "at 'theme.plot_title.angle': not supported yet; ignored",
                "at 'theme.strip_text_y.angle': not supported yet; ignored"
            ]
        );
    }

    #[test]
    fn minor_ticks_inherit_from_the_axis_not_the_ticks() {
        // Measured on lets-plot 4.11: blank by default, 2 px long, colored by
        // `axis`/`line` but not by `axis_ticks`.
        let theme = Theme::default();
        assert_eq!(theme.axis_minor_ticks_x, None);
        assert_eq!(theme.axis_minor_ticks_length, (2.0, 2.0));
        let theme = Theme::from_spec(
            Some(&spec(
                r#"{"axis_ticks": {"color": "red"}, "axis": {"color": "green"},
                    "axis_minor_ticks": {}, "axis_minor_ticks_length_x": 7}"#,
            )),
            &mut Vec::new(),
        )
        .unwrap();
        let green = Color::parse("green").unwrap();
        assert_eq!(theme.axis_minor_ticks_x.map(|tick| tick.color), Some(green));
        assert_eq!(theme.axis_minor_ticks_y.map(|tick| tick.color), Some(green));
        assert_eq!(theme.axis_minor_ticks_length, (7.0, 2.0));
        // coord_flip: x's minor ticks go to the vertical axis.
        assert_eq!(theme.flipped().axis_minor_ticks_length, (2.0, 7.0));
        // One axis only.
        let theme = Theme::from_spec(
            Some(&spec(r#"{"axis_minor_ticks_y": {"color": "red"}}"#)),
            &mut Vec::new(),
        )
        .unwrap();
        assert!(theme.axis_minor_ticks_x.is_none() && theme.axis_minor_ticks_y.is_some());
    }

    #[test]
    fn exponent_format_parses_names_and_tuples() {
        let parse = |json: &str| exponent_format(Some(&spec(json)));
        assert_eq!(exponent_format(None).unwrap(), ExponentFormat::default());
        assert_eq!(
            parse(r#"{"exponent_format": "e"}"#).unwrap(),
            ExponentFormat {
                style: ExponentStyle::E,
                min: None,
                max: None
            }
        );
        assert_eq!(
            parse(r#"{"exponent_format": ["pow_full", null, 3]}"#).unwrap(),
            ExponentFormat {
                style: ExponentStyle::PowerFull,
                min: None,
                max: Some(3)
            }
        );
        let error = parse(r#"{"exponent_format": "pwo"}"#)
            .unwrap_err()
            .to_string();
        assert!(error.contains("theme.exponent_format"), "{error}");
        assert!(error.contains("did you mean 'pow'"), "{error}");
        let error = parse(r#"{"exponent_format": ["e", 1.5, 3]}"#)
            .unwrap_err()
            .to_string();
        assert!(error.contains("bound 1.5"), "{error}");
        // The resolved theme carries it, and it is not reported as unsupported.
        let mut warnings = Vec::new();
        let theme =
            Theme::from_spec(Some(&spec(r#"{"exponent_format": "e"}"#)), &mut warnings).unwrap();
        assert_eq!(theme.exponent_format.style, ExponentStyle::E);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn ggside_axis_elements_fall_back_to_the_main_axis_elements() {
        let theme = Theme::default();
        let axes = theme.ggside_axis;
        for band in [axes.top, axes.bottom] {
            assert_eq!(
                format!("{:?}", band.text),
                format!("{:?}", theme.axis_text_x)
            );
            assert_eq!(band.ticks, theme.axis_ticks_x);
            assert_eq!(band.ticks_length, theme.axis_ticks_length.0);
            assert_eq!(band.minor_ticks, theme.axis_minor_ticks_x);
            assert_eq!(band.minor_ticks_length, theme.axis_minor_ticks_length.0);
            assert_eq!(band.line, theme.axis_line_x);
        }
        for band in [axes.left, axes.right] {
            assert_eq!(
                format!("{:?}", band.text),
                format!("{:?}", theme.axis_text_y)
            );
            assert_eq!(band.ticks, theme.axis_ticks_y);
            assert_eq!(band.ticks_length, theme.axis_ticks_length.1);
            assert_eq!(band.line, theme.axis_line_y);
        }
        // Main options reach the band axes.
        let theme = Theme::from_spec(
            Some(&spec(
                r#"{"axis_text_y": {"color": "red", "blank": false}, "axis_ticks_length": 9}"#,
            )),
            &mut Vec::new(),
        )
        .unwrap();
        let left = theme.ggside_axis.left;
        assert_eq!(left.text.unwrap().color, Color::rgb(255, 0, 0));
        assert_eq!(left.ticks_length, 9.0);
        assert_eq!(theme.ggside_axis.top.ticks_length, 9.0);
    }

    #[test]
    fn ggside_panel_elements_fall_back_through_ggside_to_the_main_panel() {
        // Default and bw: the band panels look like the main panel.
        for named in [None, Some(r#"{"name": "bw"}"#)] {
            let spec = named.map(spec);
            let theme = Theme::from_spec(spec.as_ref(), &mut Vec::new()).unwrap();
            for band in [theme.ggside_panel.xside, theme.ggside_panel.yside] {
                assert_eq!(band.background, theme.panel_background);
                assert_eq!(band.border, theme.panel_border);
            }
        }
        let mut warnings = Vec::new();
        let theme = Theme::from_spec(
            Some(&spec(
                r#"{"name": "bw",
                    "panel_border": {"color": "black", "blank": false},
                    "ggside_panel_border": {"blank": true},
                    "ggside_yside_panel_border": {"color": "red", "size": 2, "blank": false},
                    "ggside_panel_background": {"fill": "blue", "blank": false},
                    "ggside_xside_panel_background": {"blank": true}}"#,
            )),
            &mut warnings,
        )
        .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(theme.panel_border.is_some() && theme.panel_background.is_some());
        let panels = theme.ggside_panel;
        assert_eq!(panels.xside.border, None);
        let yside = panels.yside.border.unwrap();
        assert_eq!((yside.color, yside.width), (Color::rgb(255, 0, 0), 2.0));
        assert_eq!(panels.xside.background, None);
        assert_eq!(panels.yside.background.unwrap().fill, Color::rgb(0, 0, 255));
        // coord_flip: the elements follow the axis the band shares on screen.
        let flipped = theme.flipped().ggside_panel;
        assert_eq!((flipped.xside, flipped.yside), (panels.yside, panels.xside));
    }

    #[test]
    fn ggside_axis_y_styles_only_the_vertical_band_axes_most_specific_first() {
        let mut warnings = Vec::new();
        let theme = Theme::from_spec(
            Some(&spec(
                r#"{"ggside_axis_text_y": {"blank": true},
                    "ggside_axis_text_y_right": {"color": "blue", "blank": false},
                    "ggside_axis_text": {"size": 20, "blank": false},
                    "ggside_axis_ticks_x": {"blank": true},
                    "ggside_axis_ticks_length_x_top": 11,
                    "ggside_axis_ticks_length": 7,
                    "ggside_axis_line_y_left": {"color": "red", "extent": 0.5, "blank": false},
                    "ggside_axis_minor_ticks_x": {"color": "green", "blank": false}}"#,
            )),
            &mut warnings,
        )
        .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        let axes = theme.ggside_axis;
        // `_y` blanks the left band axis label; `_y_right` wins on the right.
        assert_eq!(axes.left.text, None);
        let right = axes.right.text.unwrap();
        assert_eq!(right.color, Color::rgb(0, 0, 255));
        assert_eq!(right.size, 20.0);
        // `ggside_axis_text` reaches the x band axes, the main axes stay.
        assert_eq!(axes.top.text.unwrap().size, 20.0);
        assert_eq!(
            theme.axis_text_x.unwrap().size,
            Theme::default().axis_text_x.unwrap().size
        );
        assert!(theme.axis_text_y.is_some());
        // Ticks and lengths.
        assert_eq!((axes.top.ticks, axes.bottom.ticks), (None, None));
        assert_eq!(axes.left.ticks, theme.axis_ticks_y);
        assert!(theme.axis_ticks_x.is_some());
        assert_eq!(axes.top.ticks_length, 11.0);
        assert_eq!(axes.bottom.ticks_length, 7.0);
        assert_eq!(theme.axis_ticks_length, Theme::default().axis_ticks_length);
        // Lines: style, extent, and only on the left.
        let line = axes.left.line.unwrap();
        assert_eq!(
            (line.style.color, line.extent),
            (Color::rgb(255, 0, 0), 0.5)
        );
        assert_eq!(axes.right.line, theme.axis_line_y);
        assert_eq!(axes.top.minor_ticks.unwrap().color, Color::rgb(0, 128, 0));
        assert_eq!(theme.axis_minor_ticks_x, None);
    }

    #[test]
    fn ggside_axis_elements_follow_their_aesthetic_under_coord_flip() {
        let theme = Theme::from_spec(
            Some(&spec(r#"{"ggside_axis_text_y": {"blank": true}}"#)),
            &mut Vec::new(),
        )
        .unwrap()
        .flipped();
        // The x-side band's (y) axis is horizontal after the flip.
        assert_eq!(theme.ggside_axis.bottom.text, None);
        assert!(theme.ggside_axis.left.text.is_some());
    }

    #[test]
    fn bad_ggside_tick_lengths_name_the_key_set() {
        let error = Theme::from_spec(
            Some(&spec(r#"{"ggside_axis_ticks_length": "long"}"#)),
            &mut Vec::new(),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("theme.ggside_axis_ticks_length'"), "{error}");
    }

    #[test]
    fn ggside_text_warnings_name_the_key_set_once() {
        let mut warnings = Vec::new();
        Theme::from_spec(
            Some(&spec(
                r#"{"ggside_axis_text_y": {"face": "bogus", "blank": false}}"#,
            )),
            &mut warnings,
        )
        .unwrap();
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(
            warnings[0].contains("theme.ggside_axis_text_y.face"),
            "{warnings:?}"
        );
        let mut main = Vec::new();
        Theme::from_spec(
            Some(&spec(
                r#"{"axis_text_y": {"face": "bogus", "blank": false}}"#,
            )),
            &mut main,
        )
        .unwrap();
        assert_eq!(main.len(), 1, "{main:?}");
    }

    #[test]
    fn unknown_ggside_axis_options_are_still_reported() {
        let mut warnings = Vec::new();
        Theme::from_spec(
            Some(&spec(
                r#"{"ggside_axis_text_z": {"blank": true}, "ggside_axis_txt": {"blank": true}}"#,
            )),
            &mut warnings,
        )
        .unwrap();
        assert_eq!(warnings.len(), 2, "{warnings:?}");
    }

    #[test]
    fn axis_line_arrows_and_extent_inherit_from_axis_line_only() {
        let mut warnings = Vec::new();
        let theme = Theme::from_spec(
            Some(&spec(
                r#"{"name": "classic",
                    "axis_line": {"arrow": {"name": "arrow", "length": 8}, "extent": 0.25, "blank": false},
                    "axis_line_y": {"extent": 0.5, "color": "red", "blank": false}}"#,
            )),
            &mut warnings,
        )
        .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        let expected = Arrow::from_value(&serde_json::json!({"length": 8})).unwrap();
        let (x, y) = (theme.axis_line_x.unwrap(), theme.axis_line_y.unwrap());
        assert_eq!((x.arrow, x.extent), (Some(expected), 0.25));
        assert_eq!((y.arrow, y.extent), (Some(expected), 0.5));
        assert_eq!(y.style.color, Color::rgb(255, 0, 0));
        // Plain axis lines cover the whole axis and have no heads.
        let plain = Theme::from_spec(Some(&spec(r#"{"name": "classic"}"#)), &mut warnings)
            .unwrap()
            .axis_line_x
            .unwrap();
        assert_eq!((plain.arrow, plain.extent), (None, 1.0));
        // `line` and other elements do not hand their arrow to the axis lines.
        let theme = Theme::from_spec(
            Some(&spec(
                r#"{"name": "classic", "line": {"arrow": {}, "extent": 0.5, "blank": false},
                    "panel_grid": {"arrow": {}, "blank": false}}"#,
            )),
            &mut warnings,
        )
        .unwrap();
        assert_eq!(theme.axis_line_x.unwrap().arrow, None);
        assert_eq!(theme.axis_line_x.unwrap().extent, 1.0);
        assert_eq!(
            warnings,
            [
                "at 'theme.line.arrow': only axis lines take arrow (axis_line, axis_line_x, \
                 axis_line_y); ignored",
                "at 'theme.line.extent': only axis lines take extent (axis_line, axis_line_x, \
                 axis_line_y); ignored",
                "at 'theme.panel_grid.arrow': only axis lines take arrow (axis_line, \
                 axis_line_x, axis_line_y); ignored",
            ]
        );
    }

    #[test]
    fn bad_arrows_and_extents_name_the_option() {
        let error = |json: &str| {
            Theme::from_spec(Some(&spec(json)), &mut Vec::new())
                .unwrap_err()
                .to_string()
        };
        for extent in ["0", "1.5", "-0.2", "\"half\""] {
            let message = error(&format!(
                r#"{{"axis_line_x": {{"extent": {extent}, "blank": false}}}}"#
            ));
            assert!(
                message.contains("theme.axis_line_x")
                    && message.contains("0 < extent <= 1")
                    && message.contains(extent),
                "{message}"
            );
        }
        let message = error(r#"{"axis_line": {"arrow": "yes", "blank": false}}"#);
        assert!(
            message.contains("theme.axis_line") && message.contains("expected arrow()"),
            "{message}"
        );
        let message = error(r#"{"axis_line": {"arrow": {"ends": "middle"}, "blank": false}}"#);
        assert!(
            message.contains("arrow: ends: expected one of last, first, both"),
            "{message}"
        );
        let message = error(r#"{"axis_line": {"arrow": {"length": -3}, "blank": false}}"#);
        assert!(
            message.contains("length: expected a non-negative number"),
            "{message}"
        );
    }

    #[test]
    fn geom_palette_follows_the_flavor_and_element_geom() {
        let standard = geom_palette(None).unwrap();
        assert_eq!(standard, GeomPalette::STANDARD);
        let darcula = geom_palette(Some(&spec(r#"{"flavor": "darcula"}"#))).unwrap();
        assert_eq!(darcula.pen, Color::from_hex("#BBBBBB").unwrap());
        assert_eq!(darcula.paper, Color::from_hex("#303030").unwrap());
        assert_eq!(darcula.brush, GeomPalette::STANDARD.brush);
        let custom = geom_palette(Some(&spec(
            r#"{"flavor": "darcula", "geom": {"brush": "orange", "paper": "pen"}}"#,
        )))
        .unwrap();
        assert_eq!(custom.pen, darcula.pen);
        assert_eq!(custom.brush, Color::parse("orange").unwrap());
        // A name inside element_geom reads the flavor's colors.
        assert_eq!(custom.paper, darcula.pen);
        let error = geom_palette(Some(&spec(r#"{"geom": {"pen": "reddish"}}"#))).unwrap_err();
        assert!(error.to_string().contains("theme.geom.pen"), "{error}");
    }
}
