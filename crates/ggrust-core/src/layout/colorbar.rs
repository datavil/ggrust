//! Colorbars for continuous color scales (lets-plot `ColorBarAssembler`,
//! `ColorBarComponentLayout`, `ColorBarComponent`): a bar of `bins` color segments
//! with ticks at both edges. Vertical: high values at the top, labels at the right.
//! Horizontal (legends at the top or bottom): high values at the right, labels below.

use super::axis::estimated_label_width;
use super::{label_node, text_band};
use crate::scale::format::Label;
use crate::scene::{Color, HorizontalAlign, Node, Path, PerItem, Rect, Rects};
use crate::text::{measure, measure_label};
use crate::theme::Theme;

/// Lines of a colorbar label: split at newlines (lets-plot does not wrap these).
fn label_lines(label: &Label) -> Vec<Label> {
    if label.superscript.is_some() || !label.text.contains('\n') {
        return vec![label.clone()];
    }
    label
        .text
        .split('\n')
        .map(|line| Label::plain(line.trim()))
        .collect()
}

/// lets-plot `ColorBarComponentSpec.DEF_NUM_BIN`.
pub const DEFAULT_BINS: usize = 20;
/// lets-plot `ColorBarComponentLayout.barLengthExpand`: pixels at each bar end kept
/// free of ticks.
const BAR_END_PADDING: f64 = 2.0;
/// lets-plot `DEF_BAR_LENGTH`, in key sizes.
const BAR_LENGTH_IN_KEYS: f64 = 5.0;

/// A colorbar.
pub struct Colorbar {
    /// Title; empty takes no space.
    pub title: String,
    /// The mapped domain (bottom to top of the bar).
    pub domain: (f64, f64),
    /// Break values and labels.
    pub breaks: Vec<(f64, Label)>,
    /// Segment colors, top first (`bins` of them, at least 2).
    pub colors: Vec<Color>,
    /// `guide_colorbar(barwidth=...)`.
    pub bar_width: Option<f64>,
    /// `guide_colorbar(barheight=...)`.
    pub bar_height: Option<f64>,
    /// `guide_colorbar(label_position="ends")`: only the lowest and highest break
    /// labels, beside the bar's ends (left and right, or below and above).
    pub label_ends: bool,
    /// The `na_value` color, when the scale sets one and the data has missing
    /// values: drawn as an "NA" swatch after the bar (ggplot2 issue 4567; neither
    /// lets-plot nor ggplot2 shows one).
    pub na: Option<Color>,
    /// `guide_colorbar(reverse=True)`: high values at the bottom (left); `colors`
    /// are then low values first.
    pub reverse: bool,
    /// `guide_colorbar(draw_llim, draw_ulim)`: whether a break at the lower (upper)
    /// end of the domain gets its ticks.
    pub limit_ticks: (bool, bool),
    /// `guide_colorbar(direction, theme)`.
    pub style: super::legend::GuideStyle,
}

impl Colorbar {
    /// The values whose colors fill the bins, top first: bin centres over the domain
    /// (lets-plot `addColorBar`).
    pub fn bin_values(domain: (f64, f64), bins: usize) -> Vec<f64> {
        let count = bins.max(2);
        let step = (domain.1 - domain.0) / count as f64;
        (0..count)
            .rev()
            .map(|index| domain.0 + step / 2.0 + step * index as f64)
            .collect()
    }
}

/// A colorbar measured relative to its graph origin.
pub(super) struct ColorbarLayout {
    /// The bar.
    pub bar: Rect,
    /// Whether the bar runs left to right.
    pub horizontal: bool,
    /// Per break: tick position along the bar (y, or x when horizontal) and label
    /// anchor (vertical: left end, vertical middle; horizontal: centre, top). With
    /// `ends`: the end labels' anchors (horizontal: the bar-side edge, vertical
    /// middle; vertical: centre, top), and NaN for breaks without a label.
    pub ticks: Vec<(f64, (f64, f64))>,
    /// Labels beside the bar's ends (`Colorbar::label_ends`).
    pub ends: bool,
    /// The NA swatch and its label anchor (vertical middle; left end for a
    /// vertical bar, centre-top for a horizontal one), with `Colorbar::na`.
    pub na: Option<(Rect, (f64, f64))>,
    /// Bar plus labels.
    pub size: (f64, f64),
}

impl ColorbarLayout {
    /// The same layout moved by `(dx, dy)`.
    pub fn translated(&self, (dx, dy): (f64, f64)) -> ColorbarLayout {
        let along = if self.horizontal { dx } else { dy };
        ColorbarLayout {
            ends: self.ends,
            bar: Rect {
                x: self.bar.x + dx,
                y: self.bar.y + dy,
                ..self.bar
            },
            horizontal: self.horizontal,
            ticks: self
                .ticks
                .iter()
                .map(|(tick, (x, y))| (tick + along, (x + dx, y + dy)))
                .collect(),
            na: self.na.map(|(swatch, (x, y))| {
                (
                    Rect {
                        x: swatch.x + dx,
                        y: swatch.y + dy,
                        ..swatch
                    },
                    (x + dx, y + dy),
                )
            }),
            size: self.size,
        }
    }
}

/// The label of the NA swatch.
const NA_LABEL: &str = "NA";

/// Lay out the bar, labels and NA swatch with the bar's top-left corner at `origin`.
/// The swatch is one key (the bar's thickness by a key length) half a key after
/// everything else: below a vertical bar's labels, right of a horizontal one's.
pub(super) fn layout_colorbar(
    colorbar: &Colorbar,
    origin: (f64, f64),
    horizontal: bool,
    theme: &Theme,
) -> ColorbarLayout {
    let mut layout = layout_bar(colorbar, origin, horizontal, theme);
    if colorbar.na.is_none() {
        return layout;
    }
    let (key_width, key_height) = theme.legend_key_size;
    let bar = layout.bar;
    let label = Label::plain(NA_LABEL);
    let (label_width, band) = theme.legend_text.map_or((0.0, 0.0), |style| {
        (
            measure_label(&label.text, None, None, style.size, style.font),
            text_band(style),
        )
    });
    if horizontal {
        let gap = key_width / 2.0;
        let swatch = Rect {
            x: origin.0 + layout.size.0 + gap,
            y: bar.y,
            width: key_width,
            height: bar.height,
        };
        let anchor = (swatch.x + key_width / 2.0, bar.y + bar.height + band / 3.0);
        let label_height = if theme.legend_text.is_some() {
            bar.height + band / 3.0 + band
        } else {
            0.0
        };
        let overhang = (label_width - key_width).max(0.0) / 2.0;
        let swatch = Rect {
            x: swatch.x + overhang,
            ..swatch
        };
        layout.na = Some((swatch, (anchor.0 + overhang, anchor.1)));
        layout.size = (
            layout.size.0 + gap + key_width.max(label_width),
            layout.size.1.max(label_height),
        );
    } else {
        let gap = key_height / 2.0;
        let swatch = Rect {
            x: bar.x,
            y: origin.1 + layout.size.1 + gap,
            width: bar.width,
            height: key_height,
        };
        let label_gap = theme.legend_text.map_or(0.0, |style| {
            measure("_", style.size, style.font).width / 2.0
        });
        let anchor = (
            swatch.x + swatch.width + label_gap,
            swatch.y + key_height / 2.0,
        );
        layout.na = Some((swatch, anchor));
        layout.size = (
            layout.size.0.max(swatch.width + label_gap + label_width),
            layout.size.1 + gap + key_height,
        );
    }
    layout
}

/// [`layout_colorbar`] without the NA swatch.
fn layout_bar(
    colorbar: &Colorbar,
    origin: (f64, f64),
    horizontal: bool,
    theme: &Theme,
) -> ColorbarLayout {
    let lines = |label: &Label| label_lines(label).len() as f64;
    // lets-plot `ColorBarComponentSpec`: the bar runs `BAR_LENGTH_IN_KEYS` key
    // widths (horizontal) or heights (vertical), one key thick.
    let (key_width, key_height) = theme.legend_key_size;
    // lets-plot sizes the bar by its text estimates (lines x font size, estimated
    // widths), not the real font: this keeps bar lengths identical.
    let (estimated_height, estimated_width) = theme.legend_text.map_or((0.0, 0.0), |style| {
        let face = style.font.face;
        (
            colorbar
                .breaks
                .iter()
                .map(|(_, label)| lines(label) * style.size)
                .fold(0.0, f64::max),
            colorbar
                .breaks
                .iter()
                .flat_map(|(_, label)| label_lines(label))
                .map(|line| estimated_label_width(&line, style.size, face))
                .fold(0.0, f64::max),
        )
    });
    let slots = (colorbar.breaks.len() + 1) as f64;
    let (width, height) = if horizontal {
        (
            colorbar
                .bar_width
                .unwrap_or_else(|| (BAR_LENGTH_IN_KEYS * key_width).max(estimated_width * slots)),
            colorbar.bar_height.unwrap_or(key_height),
        )
    } else {
        (
            colorbar.bar_width.unwrap_or(key_width),
            colorbar
                .bar_height
                .unwrap_or_else(|| (BAR_LENGTH_IN_KEYS * key_height).max(estimated_height * slots)),
        )
    };
    let bar = Rect {
        x: origin.0,
        y: origin.1,
        width,
        height,
    };
    let (low, high) = colorbar.domain;
    let reverse = colorbar.reverse;
    let fraction = |value: f64| {
        let at = if high > low {
            (value - low) / (high - low)
        } else {
            0.5
        };
        if reverse { 1.0 - at } else { at }
    };
    let Some(style) = theme.legend_text else {
        // No labels: ticks only.
        let ticks = colorbar
            .breaks
            .iter()
            .map(|(value, _)| {
                let tick = tick_position(bar, horizontal, fraction(*value));
                (tick, (tick, tick))
            })
            .collect();
        return ColorbarLayout {
            ends: false,
            bar,
            horizontal,
            ticks,
            na: None,
            size: (width, height),
        };
    };
    let band = text_band(style);
    if colorbar.label_ends {
        return layout_end_labels(colorbar, bar, horizontal, style, band, &fraction);
    }
    if horizontal {
        // lets-plot: labels a third of a line below the bar, centred on their ticks.
        let distance = band / 3.0;
        let label_height = colorbar
            .breaks
            .iter()
            .map(|(_, label)| lines(label) * band)
            .fold(0.0, f64::max);
        // Labels centred on the end ticks can reach past the bar; reserve that room
        // so they do not run into the title on the left (lets-plot overlaps them,
        // deviations.md).
        let (overhang_left, overhang_right) =
            colorbar
                .breaks
                .iter()
                .fold((0.0_f64, 0.0_f64), |(left, right), (value, label)| {
                    // Breaks outside the domain get no room (their ticks are off the bar).
                    let at = fraction(*value);
                    if !(0.0..=1.0).contains(&at) {
                        return (left, right);
                    }
                    let x = tick_position(bar, true, at);
                    let half = label_lines(label)
                        .iter()
                        .map(|line| {
                            measure_label(
                                &line.text,
                                line.superscript.as_deref(),
                                line.suffix.as_deref(),
                                style.size,
                                style.font,
                            )
                        })
                        .fold(0.0, f64::max)
                        / 2.0;
                    (
                        left.max(half - (x - bar.x)),
                        right.max(x + half - (bar.x + width)),
                    )
                });
        let bar = Rect {
            x: bar.x + overhang_left,
            ..bar
        };
        let ticks = colorbar
            .breaks
            .iter()
            .map(|(value, _)| {
                let x = tick_position(bar, true, fraction(*value));
                (x, (x, bar.y + height + distance))
            })
            .collect();
        return ColorbarLayout {
            ends: false,
            bar,
            horizontal,
            ticks,
            na: None,
            size: (
                overhang_left + width + overhang_right,
                height + distance + label_height,
            ),
        };
    }
    let label_gap = measure("_", style.size, style.font).width / 2.0;
    let label_width = colorbar
        .breaks
        .iter()
        .flat_map(|(_, label)| label_lines(label))
        .map(|line| {
            measure_label(
                &line.text,
                line.superscript.as_deref(),
                line.suffix.as_deref(),
                style.size,
                style.font,
            )
        })
        .fold(0.0, f64::max);
    let ticks = colorbar
        .breaks
        .iter()
        .map(|(value, _)| {
            let y = tick_position(bar, false, fraction(*value));
            (y, (bar.x + width + label_gap, y))
        })
        .collect();
    ColorbarLayout {
        ends: false,
        bar,
        horizontal,
        ticks,
        na: None,
        size: (width + label_gap + label_width, height),
    }
}

/// [`layout_colorbar`] with `label_position="ends"`: the lowest break's label
/// before the bar (left, or below a vertical bar) and the highest's after it
/// (right, or above), half a line from the bar. Other breaks keep their ticks only.
fn layout_end_labels(
    colorbar: &Colorbar,
    bar: Rect,
    horizontal: bool,
    style: crate::theme::TextStyle,
    band: f64,
    fraction: &dyn Fn(f64) -> f64,
) -> ColorbarLayout {
    let inside = |value: f64| (0.0..=1.0).contains(&fraction(value));
    let pick = |better: fn(f64, f64) -> bool| {
        colorbar
            .breaks
            .iter()
            .enumerate()
            .filter(|(_, (value, _))| inside(*value))
            .fold(None, |best: Option<(usize, f64)>, (index, (value, _))| {
                // By place on the bar, so a reversed bar keeps low at the start.
                let at = fraction(*value);
                match best {
                    Some((_, kept)) if !better(at, kept) => best,
                    _ => Some((index, at)),
                }
            })
            .map(|(index, _)| index)
    };
    let low = pick(|value, kept| value < kept);
    let high = pick(|value, kept| value > kept).filter(|high| Some(*high) != low);
    let block = |index: Option<usize>| {
        index.map_or((0.0, 0.0), |index| {
            let lines = label_lines(&colorbar.breaks[index].1);
            let width = lines
                .iter()
                .map(|line| {
                    measure_label(
                        &line.text,
                        line.superscript.as_deref(),
                        line.suffix.as_deref(),
                        style.size,
                        style.font,
                    )
                })
                .fold(0.0, f64::max);
            (width, lines.len() as f64 * band)
        })
    };
    let (low_size, high_size) = (block(low), block(high));
    let gap = |size: (f64, f64)| if size.0 > 0.0 { band / 2.0 } else { 0.0 };
    // The title sits right before the graph: keep a wider gap from the left label.
    let lead = if horizontal && low_size.0 > 0.0 && !colorbar.title.trim().is_empty() {
        band * 0.75
    } else {
        0.0
    };
    let (origin_x, origin_y) = (bar.x + lead, bar.y);
    let (bar, size, low_anchor, high_anchor) = if horizontal {
        let height = bar.height.max(low_size.1).max(high_size.1);
        let bar = Rect {
            x: origin_x + low_size.0 + gap(low_size),
            y: origin_y + (height - bar.height) / 2.0,
            ..bar
        };
        let middle = bar.y + bar.height / 2.0;
        (
            bar,
            (
                lead + low_size.0 + gap(low_size) + bar.width + gap(high_size) + high_size.0,
                height,
            ),
            (bar.x - gap(low_size), middle),
            (bar.x + bar.width + gap(high_size), middle),
        )
    } else {
        let width = bar.width.max(low_size.0).max(high_size.0);
        let bar = Rect {
            x: origin_x + (width - bar.width) / 2.0,
            y: origin_y + high_size.1 + gap(high_size),
            ..bar
        };
        let centre = bar.x + bar.width / 2.0;
        (
            bar,
            (
                width,
                high_size.1 + gap(high_size) + bar.height + gap(low_size) + low_size.1,
            ),
            (centre, bar.y + bar.height + gap(low_size)),
            (centre, origin_y),
        )
    };
    let ticks = colorbar
        .breaks
        .iter()
        .enumerate()
        .map(|(index, (value, _))| {
            let anchor = if Some(index) == low {
                low_anchor
            } else if Some(index) == high {
                high_anchor
            } else {
                (f64::NAN, f64::NAN)
            };
            (tick_position(bar, horizontal, fraction(*value)), anchor)
        })
        .collect();
    ColorbarLayout {
        ends: true,
        bar,
        horizontal,
        ticks,
        na: None,
        size,
    }
}

/// Where a domain fraction falls on the bar: low at the left (horizontal) or the
/// bottom (vertical), with the end padding kept free.
fn tick_position(bar: Rect, horizontal: bool, fraction: f64) -> f64 {
    let length = if horizontal { bar.width } else { bar.height };
    let usable = (length - 2.0 * BAR_END_PADDING).max(0.0);
    if horizontal {
        bar.x + BAR_END_PADDING + usable * fraction
    } else {
        bar.y + BAR_END_PADDING + usable * (1.0 - fraction)
    }
}

/// Draw segments, ticks and labels.
pub(super) fn draw_colorbar(
    colorbar: &Colorbar,
    layout: &ColorbarLayout,
    theme: &Theme,
) -> Vec<Node> {
    // Snap the bar onto whole units (moving ticks and labels with it, by less than
    // half a unit): a bar off the pixel grid gets a faint, light anti-aliased
    // column along its edges that reads as white peeking out.
    let snap = |value: f64| {
        if value.is_finite() {
            value.round() - value
        } else {
            0.0
        }
    };
    let snapped = layout.translated((snap(layout.bar.x), snap(layout.bar.y)));
    let layout = &snapped;
    let bar = layout.bar;
    let horizontal = layout.horizontal;
    let mut nodes = Vec::new();
    // Each segment runs one step past the next segment's start, so later ones
    // cover the seams of earlier ones; the first is longer by the end padding
    // (lets-plot `addColorBar`). lets-plot runs every segment to the bar's far end
    // instead, which stacks dozens of anti-aliased edges on a bar that does not
    // sit on whole pixels and lets lighter colors bleed down its side. Vertical
    // bars start at the top (high values), horizontal ones at the left (low values).
    let count = colorbar.colors.len().max(1);
    let length = if horizontal { bar.width } else { bar.height };
    let step = (length - 2.0 * BAR_END_PADDING) / count as f64;
    let colors: Vec<Option<Color>> = if horizontal {
        colorbar
            .colors
            .iter()
            .rev()
            .map(|color| Some(*color))
            .collect()
    } else {
        colorbar.colors.iter().map(|color| Some(*color)).collect()
    };
    // Segment `index` starts here; the last two end at the bar's far end.
    let start_of = |index: usize| -> f64 {
        if index == 0 {
            0.0
        } else {
            BAR_END_PADDING + step * index as f64
        }
    };
    // Translucent segments (`alpha`) would darken where they overlap: they abut.
    // Their edges sit on whole units, so no anti-aliased seam shows between them.
    let opaque = colorbar.colors.iter().all(|color| color.alpha == u8::MAX);
    let overlap = if opaque { 2 } else { 1 };
    let edge = |at: f64| if opaque { at } else { at.round() };
    let mut rects = Vec::with_capacity(count);
    for index in 0..colors.len() {
        let start = edge(start_of(index));
        let end = if index + overlap >= colors.len() {
            length
        } else {
            edge(start_of(index + overlap))
        };
        rects.push(if horizontal {
            Rect {
                x: bar.x + start,
                y: bar.y,
                width: end - start,
                height: bar.height,
            }
        } else {
            Rect {
                x: bar.x,
                y: bar.y + start,
                width: bar.width,
                height: end - start,
            }
        });
    }
    nodes.push(Node::Rects(Rects {
        rects,
        fill: PerItem::Varying(colors),
        stroke: None,
    }));
    if let (Some(color), Some((swatch, anchor))) = (colorbar.na, layout.na) {
        nodes.push(Node::Rects(Rects {
            rects: vec![swatch],
            fill: PerItem::Uniform(Some(color)),
            stroke: None,
        }));
        if let Some(style) = theme.legend_text {
            let (y, align) = if horizontal {
                (anchor.1 + text_band(style) / 2.0, HorizontalAlign::Middle)
            } else {
                (anchor.1, HorizontalAlign::Start)
            };
            nodes.push(label_node(
                &Label::plain(NA_LABEL),
                style,
                (anchor.0, y),
                align,
                0.0,
            ));
        }
    }
    if let Some(line) = theme.legend_ticks {
        let length = theme.legend_ticks_length;
        let (low, high) = colorbar.domain;
        let tolerance = (high - low).abs() * 1e-9;
        let hidden = |value: f64| {
            (!colorbar.limit_ticks.0 && (value - low).abs() <= tolerance)
                || (!colorbar.limit_ticks.1 && (value - high).abs() <= tolerance)
        };
        for ((tick, _), (value, _)) in layout.ticks.iter().zip(&colorbar.breaks) {
            if hidden(*value) {
                continue;
            }
            let marks = if horizontal {
                [
                    ((*tick, bar.y), (*tick, bar.y + length)),
                    (
                        (*tick, bar.y + bar.height - length),
                        (*tick, bar.y + bar.height),
                    ),
                ]
            } else {
                [
                    ((bar.x, *tick), (bar.x + length, *tick)),
                    (
                        (bar.x + bar.width - length, *tick),
                        (bar.x + bar.width, *tick),
                    ),
                ]
            };
            for (start, end) in marks {
                nodes.push(Node::Path(Path {
                    points: vec![start, end],
                    closed: false,
                    fill: None,
                    stroke: Some(line.stroke()),
                }));
            }
        }
    }
    if let Some(style) = theme.legend_text {
        let band = text_band(style);
        for ((_, anchor), (_, label)) in layout.ticks.iter().zip(&colorbar.breaks) {
            let lines = label_lines(label);
            if layout.ends {
                if !anchor.0.is_finite() {
                    continue; // a middle break: tick only
                }
                // Horizontal: centred on the bar, flush to it; vertical: stacked
                // down from the anchor, centred on the bar.
                let (first, horizontal_align) = if horizontal {
                    let align = if anchor.0 <= bar.x {
                        HorizontalAlign::End
                    } else {
                        HorizontalAlign::Start
                    };
                    (anchor.1 - band * (lines.len() as f64 - 1.0) / 2.0, align)
                } else {
                    (anchor.1 + band / 2.0, HorizontalAlign::Middle)
                };
                for (index, line) in lines.iter().enumerate() {
                    nodes.push(label_node(
                        line,
                        style,
                        (anchor.0, first + band * index as f64),
                        horizontal_align,
                        0.0,
                    ));
                }
                continue;
            }
            // Vertical: lines centred as a block on the tick. Horizontal: lines
            // stacked down from the anchor, centred on the tick.
            let (first, horizontal_align) = if horizontal {
                (anchor.1 + band / 2.0, HorizontalAlign::Middle)
            } else {
                (
                    anchor.1 - band * (lines.len() as f64 - 1.0) / 2.0,
                    HorizontalAlign::Start,
                )
            };
            for (index, line) in lines.iter().enumerate() {
                nodes.push(label_node(
                    line,
                    style,
                    (anchor.0, first + band * index as f64),
                    horizontal_align,
                    0.0,
                ));
            }
        }
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bins_are_centred_and_run_top_down() {
        assert_eq!(Colorbar::bin_values((0.0, 4.0), 2), [3.0, 1.0]);
        assert_eq!(Colorbar::bin_values((0.0, 4.0), 1).len(), 2);
    }

    #[test]
    fn ticks_run_up_vertical_bars_and_right_along_horizontal_ones() {
        let bar = Rect {
            x: 10.0,
            y: 20.0,
            width: 104.0,
            height: 54.0,
        };
        assert_eq!(tick_position(bar, false, 0.0), 72.0);
        assert_eq!(tick_position(bar, false, 1.0), 22.0);
        assert_eq!(tick_position(bar, true, 0.0), 12.0);
        assert_eq!(tick_position(bar, true, 0.5), 62.0);
    }

    #[test]
    fn segments_sit_on_whole_units_and_overlap_one_step() {
        let colorbar = Colorbar {
            title: "v".into(),
            domain: (0.0, 10.0),
            breaks: vec![(0.0, Label::plain("0")), (10.0, Label::plain("10"))],
            colors: vec![Color::WHITE; 4],
            bar_width: Some(10.0),
            bar_height: Some(44.0),
            label_ends: false,
            na: None,
            reverse: false,
            limit_ticks: (true, true),
            style: Default::default(),
        };
        let theme = Theme::default();
        let layout = layout_colorbar(&colorbar, (0.0, 0.0), false, &theme).translated((3.4, 5.6));
        let nodes = draw_colorbar(&colorbar, &layout, &theme);
        let Some(Node::Rects(segments)) = nodes.first() else {
            panic!("the bar segments come first");
        };
        let (x, y) = (layout.bar.x.round(), layout.bar.y.round());
        // 44 long, 2 padding at each end: four steps of 10.
        let spans: Vec<(f64, f64, f64, f64)> = segments
            .rects
            .iter()
            .map(|rect| (rect.x - x, rect.y - y, rect.width, rect.height))
            .collect();
        assert_eq!(
            spans,
            vec![
                (0.0, 0.0, 10.0, 22.0),
                (0.0, 12.0, 10.0, 20.0),
                (0.0, 22.0, 10.0, 22.0),
                (0.0, 32.0, 10.0, 12.0),
            ]
        );
    }

    #[test]
    fn na_swatch_follows_the_bar_and_grows_the_size() {
        let mut colorbar = Colorbar {
            title: "v".into(),
            domain: (0.0, 10.0),
            breaks: vec![(0.0, Label::plain("0")), (10.0, Label::plain("10"))],
            colors: vec![Color::WHITE; DEFAULT_BINS],
            bar_width: None,
            bar_height: None,
            label_ends: false,
            na: None,
            reverse: false,
            limit_ticks: (true, true),
            style: Default::default(),
        };
        let theme = Theme::default();
        let plain = |horizontal| layout_colorbar(&colorbar, (0.0, 0.0), horizontal, &theme);
        let (vertical, horizontal) = (plain(false), plain(true));
        assert!(vertical.na.is_none() && horizontal.na.is_none());
        colorbar.na = Some(Color::WHITE);
        let with_na = layout_colorbar(&colorbar, (0.0, 0.0), false, &theme);
        let (swatch, (label_x, label_y)) = with_na.na.expect("vertical swatch");
        assert_eq!((swatch.x, swatch.width, swatch.height), (0.0, 23.0, 23.0));
        assert_eq!(swatch.y, vertical.size.1 + 11.5);
        assert!(label_x > swatch.width && label_y == swatch.y + 11.5);
        assert_eq!(with_na.size.1, vertical.size.1 + 11.5 + 23.0);
        let with_na = layout_colorbar(&colorbar, (0.0, 0.0), true, &theme);
        let (swatch, _) = with_na.na.expect("horizontal swatch");
        assert!(swatch.x >= horizontal.size.0 + 11.5 && swatch.y == with_na.bar.y);
        assert!(with_na.size.0 >= horizontal.size.0 + 11.5 + 23.0);
        let moved = with_na.translated((5.0, 7.0)).na.expect("moved swatch").0;
        assert_eq!((moved.x, moved.y), (swatch.x + 5.0, swatch.y + 7.0));
        // Drawing snaps the bar (and the swatch with it) onto whole units.
        let bar = with_na.bar;
        let shift = (bar.x.round() - bar.x, bar.y.round() - bar.y);
        let drawn = with_na.translated(shift).na.expect("snapped swatch").0;
        let nodes = draw_colorbar(&colorbar, &with_na, &theme);
        assert!(
            nodes
                .iter()
                .any(|node| matches!(node, Node::Rects(rects) if rects.rects == vec![drawn]))
        );
    }

    #[test]
    fn horizontal_colorbars_are_five_keys_long_with_labels_below() {
        let colorbar = Colorbar {
            title: "v".into(),
            domain: (0.0, 10.0),
            breaks: vec![(0.0, Label::plain("0")), (10.0, Label::plain("10"))],
            colors: vec![Color::WHITE; DEFAULT_BINS],
            bar_width: None,
            bar_height: None,
            label_ends: false,
            na: None,
            reverse: false,
            limit_ticks: (true, true),
            style: Default::default(),
        };
        let theme = Theme::default();
        let layout = layout_colorbar(&colorbar, (0.0, 0.0), true, &theme);
        assert_eq!((layout.bar.width, layout.bar.height), (115.0, 23.0));
        assert!(layout.horizontal);
        let (first, (label_x, label_y)) = layout.ticks[0];
        assert_eq!((first, label_x), (layout.bar.x + 2.0, layout.bar.x + 2.0));
        assert!(label_y > 23.0 && layout.size.1 > label_y);
        let moved = layout.translated((5.0, 7.0));
        assert_eq!(moved.ticks[0].0, layout.bar.x + 7.0);
        let vertical = layout_colorbar(&colorbar, (0.0, 0.0), false, &theme);
        assert_eq!((vertical.bar.width, vertical.bar.height), (23.0, 115.0));
        assert_eq!(
            vertical.translated((5.0, 7.0)).ticks[0].0,
            115.0 - 2.0 + 7.0
        );
    }

    #[test]
    fn horizontal_end_labels_reaching_past_the_bar_get_room() {
        let colorbar = Colorbar {
            title: "RNA".into(),
            domain: (0.0, 1.0),
            breaks: vec![
                (0.0, Label::plain("Minimum")),
                (1.0, Label::plain("Maximum")),
            ],
            colors: vec![Color::WHITE; DEFAULT_BINS],
            bar_width: Some(80.0),
            bar_height: Some(8.0),
            label_ends: false,
            na: None,
            reverse: false,
            limit_ticks: (true, true),
            style: Default::default(),
        };
        let theme = Theme::default();
        let layout = layout_colorbar(&colorbar, (0.0, 0.0), true, &theme);
        let style = theme.legend_text.expect("default theme has legend text");
        let half = measure("Minimum", style.size, style.font).width / 2.0;
        // The first label starts at the layout's left edge, not before it.
        assert!((layout.ticks[0].1.0 - half).abs() < 1e-9);
        assert!(layout.bar.x > 0.0);
        // The last label ends at the layout's right edge.
        let right = layout.ticks[1].1.0 + measure("Maximum", style.size, style.font).width / 2.0;
        assert!((layout.size.0 - right).abs() < 1e-9);
        // Breaks outside the domain add no room.
        let outside = Colorbar {
            title: "RNA".into(),
            domain: (0.0, 1.0),
            breaks: vec![(-100.0, Label::plain("far left")), (0.5, Label::plain("a"))],
            colors: vec![Color::WHITE; DEFAULT_BINS],
            bar_width: Some(80.0),
            bar_height: Some(8.0),
            label_ends: false,
            na: None,
            reverse: false,
            limit_ticks: (true, true),
            style: Default::default(),
        };
        let kept = layout_colorbar(&outside, (0.0, 0.0), true, &theme);
        assert_eq!((kept.bar.x, kept.size.0), (0.0, 80.0));
        // Labels that fit inside the bar add no room.
        let short = Colorbar {
            breaks: vec![(0.5, Label::plain("a"))],
            ..colorbar
        };
        let fitted = layout_colorbar(&short, (0.0, 0.0), true, &theme);
        assert_eq!((fitted.bar.x, fitted.size.0), (0.0, 80.0));
    }

    #[test]
    fn end_labels_sit_beside_the_bar_and_middle_breaks_lose_theirs() {
        let colorbar = Colorbar {
            title: "RNA".into(),
            domain: (0.0, 1.0),
            breaks: vec![
                (0.0, Label::plain("Min")),
                (0.5, Label::plain("mid")),
                (1.0, Label::plain("Max")),
            ],
            colors: vec![Color::WHITE; DEFAULT_BINS],
            bar_width: Some(80.0),
            bar_height: Some(8.0),
            label_ends: true,
            na: None,
            reverse: false,
            limit_ticks: (true, true),
            style: Default::default(),
        };
        let theme = Theme::default();
        let style = theme.legend_text.expect("default theme has legend text");
        let width = |text: &str| measure(text, style.size, style.font).width;
        let layout = layout_colorbar(&colorbar, (0.0, 0.0), true, &theme);
        assert!(layout.ends);
        let [(_, low), (_, middle), (_, high)] = layout.ticks[..] else {
            panic!("three breaks")
        };
        // Low label right-aligned before the bar, high label after it, both on the
        // bar's middle; the middle break has a tick only.
        assert!(low.0 < layout.bar.x && low.0 - width("Min") > 0.0);
        assert!(high.0 > layout.bar.x + layout.bar.width);
        assert!((low.1 - (layout.bar.y + layout.bar.height / 2.0)).abs() < 1e-9);
        assert_eq!(low.1, high.1);
        assert!(middle.0.is_nan());
        assert!((layout.size.0 - (high.0 + width("Max"))).abs() < 1e-9);
        // Vertical: high above the bar, low below.
        let vertical = layout_colorbar(&colorbar, (0.0, 0.0), false, &theme);
        let (low, high) = (vertical.ticks[0].1, vertical.ticks[2].1);
        assert!(high.1 < vertical.bar.y);
        assert!(low.1 > vertical.bar.y + vertical.bar.height);
        assert!((vertical.size.1 - (low.1 + text_band(style))).abs() < 1e-9);
        // Breaks outside the domain are never end labels.
        let outside = Colorbar {
            breaks: vec![(-5.0, Label::plain("far")), (0.5, Label::plain("only"))],
            ..colorbar
        };
        let single = layout_colorbar(&outside, (0.0, 0.0), true, &theme);
        assert!(single.ticks[0].1.0.is_nan());
        assert!(single.ticks[1].1.0 < single.bar.x);
    }
}
