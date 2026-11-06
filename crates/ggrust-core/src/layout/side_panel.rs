//! Side panels: bands beside the main panel that share one of its axes (ggside
//! side layers, lets-plot `ggmarginal()` margins).
//!
//! A band sits on one screen side, between the panel and that side's axis: going
//! out from the panel come the gap, the band, then the axis (ticks, labels, an
//! axis dendrogram). Its length along the shared axis is the panel's; its
//! thickness is a share of the panel (ggside, `ggside_panel_scale`) or of panel
//! plus bands (lets-plot `margin_size`). A ggside band also draws the panel's
//! background and grid and its own axis across it, on the side where the main
//! perpendicular axis is drawn; a margin draws none of these.
//!
//! What it does not do: draw the band's layers (`build` draws them with the
//! panel machinery, through the band's [`Panel`]) or decide which layers go in a
//! band (`spec::side`).

use super::side::{AxisEdge, Side};
use super::{
    AxisBreaks, LabelArrangement, Orientation, PlotLayout, axis_side_nodes, choose_breaks,
    fit_x_labels, fit_y_labels, label_sizes, minor_breaks, rect_node, segment, text_band,
    x_axis_band, x_labels_height, y_axis_width,
};
use crate::coord::Panel;
use crate::scale::{AxisSides, PositionScale};
use crate::scene::{Group, Node, Rect};
use crate::theme::Theme;

/// How thick a band is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BandSize {
    /// A share of the main panel (ggside `ggside_panel_scale`).
    OfPanel(f64),
    /// A share of the panel plus its bands along the axis (lets-plot `margin_size`).
    OfTotal(f64),
}

/// A band the plot asks for: what [`layout_bands`] needs to size it.
#[derive(Debug, Clone, Copy)]
pub struct BandRequest<'a> {
    /// The screen side.
    pub side: Side,
    /// Thickness rule.
    pub size: BandSize,
    /// Gap between the panel and the band (px).
    pub spacing: f64,
    /// The band's own scale, across it (screen terms).
    pub cross: &'a PositionScale,
    /// ggside look: background, grid and an axis across the band.
    pub decorated: bool,
}

/// A laid out band (see the module docs). Its rectangle follows the panel
/// ([`PlotLayout::band_rect`]), so an aspect ratio fit or an aligned grid panel
/// moves it along.
#[derive(Debug, Clone, PartialEq)]
pub struct BandLayout {
    /// The screen side.
    pub side: Side,
    /// Gap between the panel and the band (px).
    pub gap: f64,
    /// Thickness across the shared axis (px).
    pub thickness: f64,
    /// ggside look (see [`BandRequest::decorated`]).
    pub decorated: bool,
    /// Breaks of the band's own axis (decorated bands with an axis).
    pub breaks: Option<AxisBreaks>,
    /// The side of the band its own axis is drawn on.
    pub axis_side: Option<Side>,
}

impl BandLayout {
    /// Gap plus thickness: how far the band pushes that side's axis out.
    pub fn extent(&self) -> f64 {
        self.gap + self.thickness
    }
}

/// The panel's length along one screen direction and the thickness of each band
/// on that direction's sides (`requests` in order), out of `available` px for the
/// panel, its bands and their gaps. The panel keeps at least `panel_min_size`.
pub(super) fn split(
    available: f64,
    requests: &[&BandRequest],
    panel_min_size: f64,
) -> (f64, Vec<f64>) {
    if requests.is_empty() {
        return (available.max(panel_min_size), Vec::new());
    }
    let total = (available - requests.iter().map(|band| band.spacing).sum::<f64>()).max(0.0);
    let margins: f64 = requests
        .iter()
        .map(|band| match band.size {
            BandSize::OfTotal(share) => share * total,
            BandSize::OfPanel(_) => 0.0,
        })
        .sum();
    let panel_shares: f64 = requests
        .iter()
        .map(|band| match band.size {
            BandSize::OfPanel(share) => share,
            BandSize::OfTotal(_) => 0.0,
        })
        .sum();
    let panel = ((total - margins) / (1.0 + panel_shares)).max(panel_min_size);
    let mut thicknesses: Vec<f64> = requests
        .iter()
        .map(|band| match band.size {
            BandSize::OfTotal(share) => share * total,
            BandSize::OfPanel(share) => share * panel,
        })
        .collect();
    // Bands share what the panel leaves (a panel at its minimum size).
    let bands: f64 = thicknesses.iter().sum();
    if bands > 0.0 && panel + bands > total {
        let factor = (total - panel).max(0.0) / bands;
        for thickness in &mut thicknesses {
            *thickness *= factor;
        }
    }
    (panel, thicknesses)
}

/// The side of the band of `request` its own axis is drawn on: the side its own
/// scale's `position=` asks for (`scale_xsidey_*(position=...)`), else where the main
/// perpendicular axis is drawn (`cross_sides`); none for an undecorated band.
pub(super) fn band_axis_side(request: &BandRequest, cross_sides: AxisSides) -> Option<Side> {
    if !request.decorated {
        return None;
    }
    let own = request.cross.sides();
    let sides = if own == AxisSides::default() {
        cross_sides
    } else {
        own
    };
    let vertical_axis = request.side.is_horizontal();
    (sides.primary || sides.secondary).then(|| Side::of(!vertical_axis, !sides.primary))
}

/// The theme a band's own axis on screen `side` is laid out and drawn with: the
/// main theme with that side's ggside axis elements (`Theme::ggside_axis`, already
/// resolved down to the main ones) in place of the main axis elements, so blank
/// band labels take no room and the main axes keep their own look.
pub(super) fn band_axis_theme(theme: &Theme, side: Side) -> Theme {
    let axes = &theme.ggside_axis;
    let band = match side {
        Side::Top => axes.top,
        Side::Bottom => axes.bottom,
        Side::Left => axes.left,
        Side::Right => axes.right,
    };
    let mut theme = theme.clone();
    if side.is_horizontal() {
        theme.axis_text_x = band.text;
        theme.axis_ticks_x = band.ticks;
        theme.axis_ticks_length.0 = band.ticks_length;
        theme.axis_minor_ticks_x = band.minor_ticks;
        theme.axis_minor_ticks_length.0 = band.minor_ticks_length;
        theme.axis_line_x = band.line;
    } else {
        theme.axis_text_y = band.text;
        theme.axis_ticks_y = band.ticks;
        theme.axis_ticks_length.1 = band.ticks_length;
        theme.axis_minor_ticks_y = band.minor_ticks;
        theme.axis_minor_ticks_length.1 = band.minor_ticks_length;
        theme.axis_line_y = band.line;
    }
    theme
}

/// Blank the labels of `breaks` whose box would leave the band's span `length`
/// (px along its own axis, from its start): a label at the band's edge would run
/// into the main axis's end label. Ticks stay. Category labels all stay: they
/// name the band's tracks, and the scale's expansion keeps them near the band.
fn keep_labels_inside(
    mut breaks: AxisBreaks,
    cross: &PositionScale,
    length: f64,
    extents: &[f64],
) -> AxisBreaks {
    if cross.is_discrete() {
        return breaks;
    }
    for (index, value) in breaks.values.iter().enumerate() {
        let at = cross.map(*value) * length;
        let half = extents.get(index).copied().unwrap_or(0.0) / 2.0;
        if at - half < -0.5 || at + half > length + 0.5 {
            breaks.labels[index] = crate::scale::format::Label::plain("");
        }
    }
    breaks
}

/// The largest break count asked of a band's scale when lets-plot's count rule
/// leaves its axis without a label (see [`layout_bands`]).
const BAND_FALLBACK_MAX_COUNT: usize = 3;

/// Whether any label of `breaks` has text.
fn has_visible_label(breaks: &AxisBreaks) -> bool {
    breaks.labels.iter().any(|label| !label.text.is_empty())
}

/// Whether two labels with text of `breaks` overlap along a band axis of
/// `length` px; `extents` are the labels' sizes along it (empty: turned labels,
/// which never count as overlapping here).
fn visible_labels_overlap(
    breaks: &AxisBreaks,
    cross: &PositionScale,
    length: f64,
    extents: &[f64],
) -> bool {
    let mut spans: Vec<(f64, f64)> = breaks
        .values
        .iter()
        .zip(&breaks.labels)
        .zip(extents)
        .filter(|((_, label), _)| !label.text.is_empty())
        .map(|((value, _), extent)| {
            let at = cross.map(*value) * length;
            (at - extent / 2.0, at + extent / 2.0)
        })
        .collect();
    spans.sort_by(|a, b| a.0.total_cmp(&b.0));
    spans.windows(2).any(|pair| pair[1].0 < pair[0].1)
}

/// Lay out the bands on the sides of one screen direction: `vertical` (top and
/// bottom bands, whose own axis is vertical) or not. `thicknesses` come from
/// [`split`] in the order of the matching `requests`; `cross_sides` are the sides
/// of the main axis perpendicular to the shared one (where a band's axis goes).
/// A band's vertical axis may be `width_limit` px wide, its horizontal axis
/// labels `height_limit` px tall (see [`super::fit_x_labels`]).
pub(super) fn layout_bands(
    requests: &[&BandRequest],
    thicknesses: &[f64],
    cross_sides: AxisSides,
    theme: &Theme,
    (width_limit, height_limit): (f64, f64),
) -> Vec<BandLayout> {
    requests
        .iter()
        .zip(thicknesses)
        .map(|(request, thickness)| {
            let vertical_axis = request.side.is_horizontal();
            let axis_side = band_axis_side(request, cross_sides);
            let breaks = axis_side.map(|side| {
                let theme = &band_axis_theme(theme, side);
                // Fit labels to the band, then blank the end labels that leave it.
                // Returns the breaks and each label's extent along the axis.
                let fit = |breaks: AxisBreaks| -> (AxisBreaks, Vec<f64>) {
                    if vertical_axis {
                        let breaks =
                            fit_y_labels(breaks, request.cross, *thickness, width_limit, theme);
                        let heights: Vec<f64> = theme.axis_text_y.map_or_else(Vec::new, |text| {
                            label_sizes(&breaks.labels, text)
                                .iter()
                                .map(|(_, height)| *height)
                                .collect()
                        });
                        let breaks =
                            keep_labels_inside(breaks, request.cross, *thickness, &heights);
                        (breaks, heights)
                    } else {
                        let breaks =
                            fit_x_labels(breaks, request.cross, *thickness, height_limit, theme);
                        // Only flat labels run along the axis into the main axis's
                        // end label; turned ones (tilted, vertical, a theme angle)
                        // reach at most half a line past their tick, so all stay.
                        let flat = matches!(
                            breaks.arrangement,
                            LabelArrangement::Simple | LabelArrangement::TwoRows(_)
                        );
                        let widths: Vec<f64> =
                            theme
                                .axis_text_x
                                .filter(|_| flat)
                                .map_or_else(Vec::new, |text| {
                                    label_sizes(&breaks.labels, text)
                                        .iter()
                                        .map(|(width, _)| *width)
                                        .collect()
                                });
                        let breaks = keep_labels_inside(breaks, request.cross, *thickness, &widths);
                        (breaks, widths)
                    }
                };
                let (orientation, text) = if vertical_axis {
                    (Orientation::Vertical, theme.axis_text_y)
                } else {
                    (Orientation::Horizontal, theme.axis_text_x)
                };
                let (breaks, _) = fit(choose_breaks(
                    request.cross,
                    *thickness,
                    orientation,
                    text,
                    (0.0, 0.0),
                ));
                if text.is_none() || has_visible_label(&breaks) {
                    return breaks;
                }
                // A thin band allows one break, the first, which sits at the band's
                // edge and loses its label (a density band read 0 to 0.03 with no
                // number). Ask the scale for a few more and keep the first set
                // with a label inside the band.
                (2..=BAND_FALLBACK_MAX_COUNT)
                    .map(|count| {
                        let scale_breaks = request.cross.breaks(count);
                        let mut candidate =
                            AxisBreaks::new(scale_breaks.positions, scale_breaks.labels);
                        candidate.target_count = count;
                        fit(candidate)
                    })
                    .find(|(candidate, extents)| {
                        has_visible_label(candidate)
                            && !visible_labels_overlap(
                                candidate,
                                request.cross,
                                *thickness,
                                extents,
                            )
                    })
                    .map_or(breaks, |(candidate, _)| candidate)
            });
            BandLayout {
                side: request.side,
                gap: request.spacing,
                thickness: *thickness,
                decorated: request.decorated,
                breaks,
                axis_side,
            }
        })
        .collect()
}

/// Width of the widest axis of the top and bottom `bands` drawn on `side` (left
/// or right), 0 without one: the main y axis band grows to hold it.
pub(super) fn vertical_axis_width(bands: &[BandLayout], side: Side, theme: &Theme) -> f64 {
    bands
        .iter()
        .filter(|band| band.axis_side == Some(side))
        .filter_map(|band| band.breaks.as_ref())
        .map(|breaks| y_axis_width(breaks, &band_axis_theme(theme, side)))
        .fold(0.0, f64::max)
}

/// Height of the tallest own axis of the left and right `bands` drawn on `side`
/// (top or bottom): ticks, spacing and labels (a simple text band at least), from
/// the band's axis theme; none without one. The main x axis band grows to hold it.
pub(super) fn horizontal_axis_height(
    bands: &[BandLayout],
    side: Side,
    theme: &Theme,
) -> Option<f64> {
    let theme = band_axis_theme(theme, side);
    let simple = theme.axis_text_x.map_or(0.0, text_band);
    bands
        .iter()
        .filter(|band| band.axis_side == Some(side))
        .filter_map(|band| band.breaks.as_ref())
        .map(|breaks| x_axis_band(x_labels_height(breaks, &theme).max(simple), &theme))
        .reduce(f64::max)
}

impl PlotLayout {
    /// How far the band on `side` pushes that side's axis out (0 without one).
    pub fn band_extent(&self, side: Side) -> f64 {
        self.bands
            .iter()
            .find(|band| band.side == side)
            .map_or(0.0, BandLayout::extent)
    }

    /// The rectangle of `band`, beside the panel.
    pub fn band_rect(&self, band: &BandLayout) -> Rect {
        let panel = self.panel;
        match band.side {
            Side::Top => Rect {
                y: panel.y - band.gap - band.thickness,
                height: band.thickness,
                ..panel
            },
            Side::Bottom => Rect {
                y: panel.y + panel.height + band.gap,
                height: band.thickness,
                ..panel
            },
            Side::Left => Rect {
                x: panel.x - band.gap - band.thickness,
                width: band.thickness,
                ..panel
            },
            Side::Right => Rect {
                x: panel.x + panel.width + band.gap,
                width: band.thickness,
                ..panel
            },
        }
    }

    /// The layout of `band` seen as a panel of its own: its rectangle as the
    /// panel, the shared axis's breaks and its own axis's breaks, no bands.
    fn band_as_panel(&self, band: &BandLayout) -> PlotLayout {
        let own = band
            .breaks
            .clone()
            .unwrap_or_else(|| AxisBreaks::new(Vec::new(), Vec::new()));
        let (x_breaks, y_breaks) = if band.side.is_horizontal() {
            (self.x_breaks.clone(), own)
        } else {
            (own, self.y_breaks.clone())
        };
        PlotLayout {
            panel: self.band_rect(band),
            x_breaks,
            y_breaks,
            dendrograms: [None, None],
            secondary: [None, None],
            bands: Vec::new(),
            ..self.clone()
        }
    }
}

/// The static parts of a decorated band under its layers: background and the grid
/// lines across the shared axis (at its own axis's breaks). `panel` maps the band.
pub fn draw_band_background(
    layout: &PlotLayout,
    band: &BandLayout,
    theme: &Theme,
    panel: &Panel,
) -> Vec<Node> {
    if !band.decorated {
        return Vec::new();
    }
    let rect = layout.band_rect(band);
    let mut nodes = Vec::new();
    if let Some(style) = theme.panel_background {
        nodes.push(rect_node(rect, style));
    }
    let Some(breaks) = &band.breaks else {
        return nodes;
    };
    let mut grid = Vec::new();
    if band.side.is_horizontal() {
        // Horizontal lines at the band's own (vertical) breaks.
        let lines = [
            (theme.panel_grid_minor_y, minor_breaks(&breaks.values)),
            (theme.panel_grid_major_y, breaks.values.clone()),
        ];
        for (line, values) in lines {
            let Some(line) = line else { continue };
            for value in values {
                let y = panel.y_pixel(value);
                if y >= rect.y && y <= rect.y + rect.height {
                    grid.push(segment((rect.x, y), (rect.x + rect.width, y), line));
                }
            }
        }
    } else {
        let lines = [
            (theme.panel_grid_minor_x, minor_breaks(&breaks.values)),
            (theme.panel_grid_major_x, breaks.values.clone()),
        ];
        for (line, values) in lines {
            let Some(line) = line else { continue };
            for value in values {
                let x = panel.x_pixel(value);
                if x >= rect.x && x <= rect.x + rect.width {
                    grid.push(segment((x, rect.y), (x, rect.y + rect.height), line));
                }
            }
        }
    }
    if !grid.is_empty() {
        nodes.push(group(Some("grid"), None, grid));
    }
    nodes
}

/// The grid lines of a decorated band along the shared axis (at the main
/// panel's breaks of that axis): drawn with the band's layers, so they move
/// with them when the shared axis zooms.
pub fn draw_band_shared_grid(
    layout: &PlotLayout,
    band: &BandLayout,
    theme: &Theme,
    panel: &Panel,
) -> Vec<Node> {
    if !band.decorated {
        return Vec::new();
    }
    let rect = layout.band_rect(band);
    let mut grid = Vec::new();
    if band.side.is_horizontal() {
        let values = &layout.x_breaks.values;
        for (line, values) in [
            (theme.panel_grid_minor_x, minor_breaks(values)),
            (theme.panel_grid_major_x, values.clone()),
        ] {
            let Some(line) = line else { continue };
            for value in values {
                let x = panel.x_pixel(value);
                if x >= rect.x && x <= rect.x + rect.width {
                    grid.push(segment((x, rect.y), (x, rect.y + rect.height), line));
                }
            }
        }
    } else {
        let values = &layout.y_breaks.values;
        for (line, values) in [
            (theme.panel_grid_minor_y, minor_breaks(values)),
            (theme.panel_grid_major_y, values.clone()),
        ] {
            let Some(line) = line else { continue };
            for value in values {
                let y = panel.y_pixel(value);
                if y >= rect.y && y <= rect.y + rect.height {
                    grid.push(segment((rect.x, y), (rect.x + rect.width, y), line));
                }
            }
        }
    }
    grid
}

/// The band's border (`panel_border`) and its own axis, drawn above its layers.
pub fn draw_band_axis(
    layout: &PlotLayout,
    band: &BandLayout,
    theme: &Theme,
    panel: &Panel,
) -> Vec<Node> {
    if !band.decorated {
        return Vec::new();
    }
    let rect = layout.band_rect(band);
    let mut nodes = Vec::new();
    if let Some(line) = theme.panel_border {
        nodes.push(Node::Rects(crate::scene::Rects {
            rects: vec![rect],
            fill: crate::scene::PerItem::Uniform(None),
            stroke: Some(line.stroke()),
        }));
    }
    let (Some(side), Some(breaks)) = (band.axis_side, &band.breaks) else {
        return nodes;
    };
    // The axis runs at the main axis's line on that side, beside the band.
    let theme = &band_axis_theme(theme, side);
    let base = AxisEdge::of(layout, side).base;
    let edge = AxisEdge { side, base };
    let own = layout.band_as_panel(band);
    let axis = if side.is_horizontal() {
        axis_side_nodes(
            edge,
            &own,
            breaks,
            theme,
            &|value| panel.x_pixel(value),
            (rect.x, rect.x + rect.width),
        )
    } else {
        axis_side_nodes(
            edge,
            &own,
            breaks,
            theme,
            &|value| panel.y_pixel(value),
            (rect.y, rect.y + rect.height),
        )
    };
    nodes.push(group(Some("side-axis"), None, axis));
    nodes
}

/// The group a band's layers draw in: clipped to the band, with the role the HTML
/// runtime moves along the shared axis (`side-x`: the band shares the horizontal
/// axis, `side-y`: the vertical one).
pub fn band_layers_group(layout: &PlotLayout, band: &BandLayout, nodes: Vec<Node>) -> Node {
    let role = if band.side.is_horizontal() {
        "side-x"
    } else {
        "side-y"
    };
    group(Some(role), Some(layout.band_rect(band)), nodes)
}

fn group(role: Option<&'static str>, clip: Option<Rect>, nodes: Vec<Node>) -> Node {
    Node::Group(Group {
        key: None,
        clip,
        role,
        anchor: None,
        nodes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::ContinuousPositionScale;

    #[test]
    fn panel_shares_follow_ggside_and_total_shares_follow_lets_plot() {
        let scale = PositionScale::Continuous(ContinuousPositionScale::default());
        let ggside = BandRequest {
            side: Side::Top,
            size: BandSize::OfPanel(0.1),
            spacing: 2.0,
            cross: &scale,
            decorated: true,
        };
        // 0.1 of the main panel: 222 = panel + 2 + 0.1 * panel.
        let (panel, thicknesses) = split(222.0, &[&ggside], 10.0);
        assert!((panel - 200.0).abs() < 1e-9);
        assert!((thicknesses[0] - 20.0).abs() < 1e-9);
        // lets-plot: 0.2 and 0.1 of panel plus margins (measured 70.8 / 35.4 of 354).
        let top = BandRequest {
            size: BandSize::OfTotal(0.2),
            spacing: 0.0,
            ..ggside
        };
        let bottom = BandRequest {
            side: Side::Bottom,
            size: BandSize::OfTotal(0.1),
            ..top
        };
        let (panel, thicknesses) = split(354.0, &[&top, &bottom], 10.0);
        assert!((panel - 247.8).abs() < 1e-9);
        assert!((thicknesses[0] - 70.8).abs() < 1e-9);
        assert!((thicknesses[1] - 35.4).abs() < 1e-9);
    }

    /// A trained discrete scale with these levels.
    fn discrete(levels: &[&str]) -> PositionScale {
        let mut scale = crate::scale::DiscretePositionScale::default();
        let levels: Vec<String> = levels.iter().map(|level| level.to_string()).collect();
        scale.train_levels(&levels);
        PositionScale::Discrete(scale)
    }

    #[test]
    fn discrete_band_axes_keep_every_label_and_get_room_for_it() {
        let mut main = ContinuousPositionScale::default();
        main.train_range(0.0, 10.0);
        let main = PositionScale::Continuous(main);
        let species = discrete(&["Adelie", "Gentoo", "Chinstrap"]);
        let band = |side| BandRequest {
            side,
            size: BandSize::OfPanel(0.1),
            spacing: 2.0,
            cross: &species,
            decorated: true,
        };
        let theme = Theme::default();
        let area = Rect {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        };
        let layout = super::super::layout_plot(
            area,
            &theme,
            &super::super::Titles::default(),
            &main,
            &main,
            crate::layout::Margins::uniform(0.0),
            None,
            super::super::PANEL_MIN_SIZE,
            &[band(Side::Right), band(Side::Top)],
        );
        let simple = theme.axis_text_x.map_or(0.0, super::super::text_band);
        for side in [Side::Right, Side::Top] {
            let band = layout.bands.iter().find(|band| band.side == side);
            let breaks = band
                .and_then(|band| band.breaks.as_ref())
                .expect("band axis");
            let labels: Vec<&str> = breaks
                .labels
                .iter()
                .map(|label| label.text.as_str())
                .collect();
            assert_eq!(labels, ["Adelie", "Gentoo", "Chinstrap"], "{side:?}");
        }
        // The narrow right band turns its labels; the x axis band holds them.
        let needed = horizontal_axis_height(&layout.bands, Side::Bottom, &theme).unwrap();
        assert!(needed > 2.0 * simple, "axis {needed} px tall");
        assert!(layout.axis_bands.bottom >= needed - 1e-9);
        // The top band's own axis widens the y axis band to its widest label.
        assert!(layout.axis_bands.left >= vertical_axis_width(&layout.bands, Side::Left, &theme));
    }

    #[test]
    fn band_axis_theme_takes_the_band_elements_and_blank_labels_take_no_room() {
        let mut main = ContinuousPositionScale::default();
        main.train_range(0.0, 10.0);
        let main = PositionScale::Continuous(main);
        let species = discrete(&["Adelie", "Gentoo", "Chinstrap"]);
        let band = |side| BandRequest {
            side,
            size: BandSize::OfPanel(0.1),
            spacing: 2.0,
            cross: &species,
            decorated: true,
        };
        let spec: crate::spec::ThemeSpec = serde_json::from_str(
            r#"{"ggside_axis_text_y": {"blank": true}, "ggside_axis_ticks_y": {"blank": true}}"#,
        )
        .unwrap();
        let theme = Theme::from_spec(Some(&spec), &mut Vec::new()).unwrap();
        let area = Rect {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        };
        let layout_of = |theme: &Theme| {
            super::super::layout_plot(
                area,
                theme,
                &super::super::Titles::default(),
                &main,
                &main,
                crate::layout::Margins::uniform(0.0),
                None,
                super::super::PANEL_MIN_SIZE,
                &[band(Side::Right), band(Side::Top)],
            )
        };
        let layout = layout_of(&theme);
        let plain = layout_of(&Theme::default());
        // The top band's (y) axis has no labels and no ticks: it adds no width.
        let left = band_axis_theme(&theme, Side::Left);
        assert_eq!((left.axis_text_y, left.axis_ticks_y), (None, None));
        assert_eq!(
            format!("{:?}", left.axis_text_x),
            format!("{:?}", theme.axis_text_x)
        );
        assert_eq!(vertical_axis_width(&layout.bands, Side::Left, &theme), 0.0);
        assert!(vertical_axis_width(&plain.bands, Side::Left, &plain_theme()) > 0.0);
        assert!(layout.axis_bands.left < plain.axis_bands.left);
        // The right band's (x) axis keeps its labels and room.
        assert_eq!(
            horizontal_axis_height(&layout.bands, Side::Bottom, &theme),
            horizontal_axis_height(&plain.bands, Side::Bottom, &plain_theme())
        );
    }

    fn plain_theme() -> Theme {
        Theme::default()
    }

    #[test]
    fn thin_continuous_band_axes_keep_a_label_inside_the_band() {
        // A density band (0 to about 0.034): lets-plot's count rule allows one
        // break on a ~50 px band, the first (0), at the edge where it is blanked.
        let mut main = ContinuousPositionScale::default();
        main.train_range(0.0, 10.0);
        let main = PositionScale::Continuous(main);
        let mut density = ContinuousPositionScale::default();
        density.train_range(0.0, 0.0335);
        let density = PositionScale::Continuous(density);
        let band = |side| BandRequest {
            side,
            size: BandSize::OfPanel(0.1),
            spacing: 2.0,
            cross: &density,
            decorated: true,
        };
        let theme = Theme::default();
        let area = Rect {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        };
        let layout = super::super::layout_plot(
            area,
            &theme,
            &super::super::Titles::default(),
            &main,
            &main,
            crate::layout::Margins::uniform(0.0),
            None,
            super::super::PANEL_MIN_SIZE,
            &[band(Side::Right), band(Side::Top)],
        );
        for side in [Side::Right, Side::Top] {
            let band = layout
                .bands
                .iter()
                .find(|band| band.side == side)
                .expect("band");
            let breaks = band.breaks.as_ref().expect("band axis");
            let visible: Vec<(f64, &str)> = breaks
                .values
                .iter()
                .zip(&breaks.labels)
                .filter(|(_, label)| !label.text.is_empty())
                .map(|(value, label)| (*value, label.text.as_str()))
                .collect();
            assert!(
                !visible.is_empty(),
                "{side:?}: no label on {:?}",
                breaks.values
            );
            for (value, text) in visible {
                let at = density.map(value);
                assert!(
                    at > 0.0 && at < 1.0,
                    "{side:?}: '{text}' at the band's edge"
                );
            }
        }
    }

    #[test]
    fn bands_shrink_rather_than_push_the_panel_below_its_minimum() {
        let scale = PositionScale::Continuous(ContinuousPositionScale::default());
        let wide = BandRequest {
            side: Side::Top,
            size: BandSize::OfTotal(0.95),
            spacing: 0.0,
            cross: &scale,
            decorated: false,
        };
        let (panel, thicknesses) = split(100.0, &[&wide], 40.0);
        assert!((panel - 40.0).abs() < 1e-9);
        assert!((panel + thicknesses[0] - 100.0).abs() < 1e-9);
    }
}
