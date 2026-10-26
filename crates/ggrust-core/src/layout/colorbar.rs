//! Colorbars for continuous color scales (lets-plot `ColorBarAssembler`,
//! `ColorBarComponentLayout`, `ColorBarComponent`): a bar of `bins` color segments
//! with ticks at both edges. Vertical: high values at the top, labels at the right.
//! Horizontal (legends at the top or bottom): high values at the right, labels below.

use super::axis::estimated_label_width;
use super::{label_node, text_band};
use crate::scale::format::Label;
use crate::scene::{Color, HorizontalAlign, Node, Path, PerItem, Rect, Rects, Stroke};
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
    /// anchor (vertical: left end, vertical middle; horizontal: centre, top).
    pub ticks: Vec<(f64, (f64, f64))>,
    /// Bar plus labels.
    pub size: (f64, f64),
}

impl ColorbarLayout {
    /// The same layout moved by `(dx, dy)`.
    pub fn translated(&self, (dx, dy): (f64, f64)) -> ColorbarLayout {
        let along = if self.horizontal { dx } else { dy };
        ColorbarLayout {
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
            size: self.size,
        }
    }
}

/// Lay out the bar and labels with the bar's top-left corner at `origin`.
pub(super) fn layout_colorbar(
    colorbar: &Colorbar,
    origin: (f64, f64),
    horizontal: bool,
    theme: &Theme,
) -> ColorbarLayout {
    let lines = |label: &Label| label_lines(label).len() as f64;
    let key = theme.legend_key_size;
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
                .unwrap_or_else(|| (BAR_LENGTH_IN_KEYS * key).max(estimated_width * slots)),
            colorbar.bar_height.unwrap_or(key),
        )
    } else {
        (
            colorbar.bar_width.unwrap_or(key),
            colorbar
                .bar_height
                .unwrap_or_else(|| (BAR_LENGTH_IN_KEYS * key).max(estimated_height * slots)),
        )
    };
    let bar = Rect {
        x: origin.0,
        y: origin.1,
        width,
        height,
    };
    let (low, high) = colorbar.domain;
    let fraction = |value: f64| {
        if high > low {
            (value - low) / (high - low)
        } else {
            0.5
        }
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
            bar,
            horizontal,
            ticks,
            size: (width, height),
        };
    };
    let band = text_band(style);
    if horizontal {
        // lets-plot: labels a third of a line below the bar, centred on their ticks.
        let distance = band / 3.0;
        let label_height = colorbar
            .breaks
            .iter()
            .map(|(_, label)| lines(label) * band)
            .fold(0.0, f64::max);
        let ticks = colorbar
            .breaks
            .iter()
            .map(|(value, _)| {
                let x = tick_position(bar, true, fraction(*value));
                (x, (x, bar.y + height + distance))
            })
            .collect();
        return ColorbarLayout {
            bar,
            horizontal,
            ticks,
            size: (width, height + distance + label_height),
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
        bar,
        horizontal,
        ticks,
        size: (width + label_gap + label_width, height),
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
    let bar = layout.bar;
    let horizontal = layout.horizontal;
    let mut nodes = Vec::new();
    // Each segment runs to the bar's far end, so later ones cover earlier ones
    // without seams; the first is longer by the end padding (lets-plot
    // `addColorBar`). Vertical bars start at the top (high values), horizontal ones
    // at the left (low values).
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
    let mut start = 0.0;
    let mut rects = Vec::with_capacity(count);
    for index in 0..colors.len() {
        rects.push(if horizontal {
            Rect {
                x: bar.x + start,
                y: bar.y,
                width: bar.width - start,
                height: bar.height,
            }
        } else {
            Rect {
                x: bar.x,
                y: bar.y + start,
                width: bar.width,
                height: bar.height - start,
            }
        });
        start += step + if index == 0 { BAR_END_PADDING } else { 0.0 };
    }
    nodes.push(Node::Rects(Rects {
        rects,
        fill: PerItem::Varying(colors),
        stroke: None,
    }));
    if let Some(line) = theme.legend_ticks {
        let length = theme.legend_ticks_length;
        for (tick, _) in &layout.ticks {
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
                    stroke: Some(Stroke::solid(line.color, line.width)),
                }));
            }
        }
    }
    if let Some(style) = theme.legend_text {
        let band = text_band(style);
        for ((_, anchor), (_, label)) in layout.ticks.iter().zip(&colorbar.breaks) {
            let lines = label_lines(label);
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
    fn horizontal_colorbars_are_five_keys_long_with_labels_below() {
        let colorbar = Colorbar {
            title: "v".into(),
            domain: (0.0, 10.0),
            breaks: vec![(0.0, Label::plain("0")), (10.0, Label::plain("10"))],
            colors: vec![Color::WHITE; DEFAULT_BINS],
            bar_width: None,
            bar_height: None,
        };
        let theme = Theme::default();
        let layout = layout_colorbar(&colorbar, (0.0, 0.0), true, &theme);
        assert_eq!((layout.bar.width, layout.bar.height), (115.0, 23.0));
        assert!(layout.horizontal);
        let (first, (label_x, label_y)) = layout.ticks[0];
        assert_eq!((first, label_x), (2.0, 2.0));
        assert!(label_y > 23.0 && layout.size.1 > label_y);
        let moved = layout.translated((5.0, 7.0));
        assert_eq!(moved.ticks[0].0, 7.0);
        let vertical = layout_colorbar(&colorbar, (0.0, 0.0), false, &theme);
        assert_eq!((vertical.bar.width, vertical.bar.height), (23.0, 115.0));
        assert_eq!(
            vertical.translated((5.0, 7.0)).ticks[0].0,
            115.0 - 2.0 + 7.0
        );
    }
}
