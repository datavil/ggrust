//! Colorbars for continuous color scales (lets-plot `ColorBarAssembler`,
//! `ColorBarComponentLayout`, `ColorBarComponent`): a vertical bar of `bins`
//! color segments, high values at the top, ticks at both edges, labels at the right.
//!
//! What it does not do (yet): horizontal colorbars.

use super::label_node;
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
    /// Per break: tick y and label anchor (left, vertical middle).
    pub ticks: Vec<(f64, (f64, f64))>,
    /// Bar plus labels.
    pub size: (f64, f64),
}

/// Lay out the bar and labels with the bar's top-left corner at `origin`.
pub(super) fn layout_colorbar(
    colorbar: &Colorbar,
    origin: (f64, f64),
    theme: &Theme,
) -> ColorbarLayout {
    let (label_height, label_gap, label_width) = match theme.legend_text {
        // lets-plot sizes the bar by its text height estimate (lines x font size),
        // not the line box: this keeps bar lengths identical.
        Some(style) => (
            colorbar
                .breaks
                .iter()
                .map(|(_, label)| label_lines(label).len() as f64 * style.size)
                .fold(0.0, f64::max),
            measure("_", style.size, style.face).width / 2.0,
            colorbar
                .breaks
                .iter()
                .flat_map(|(_, label)| label_lines(label))
                .map(|line| {
                    measure_label(
                        &line.text,
                        line.superscript.as_deref(),
                        style.size,
                        style.face,
                    )
                })
                .fold(0.0, f64::max),
        ),
        None => (0.0, 0.0, 0.0),
    };
    let width = colorbar.bar_width.unwrap_or(theme.legend_key_size);
    let height = colorbar.bar_height.unwrap_or_else(|| {
        (BAR_LENGTH_IN_KEYS * theme.legend_key_size)
            .max(label_height * (colorbar.breaks.len() + 1) as f64)
    });
    let bar = Rect {
        x: origin.0,
        y: origin.1,
        width,
        height,
    };
    // Domain onto the bar, reversed: the high end at the top.
    let (low, high) = colorbar.domain;
    let usable = (height - 2.0 * BAR_END_PADDING).max(0.0);
    let ticks = colorbar
        .breaks
        .iter()
        .map(|(value, _)| {
            let fraction = if high > low {
                (value - low) / (high - low)
            } else {
                0.5
            };
            let y = bar.y + BAR_END_PADDING + usable * (1.0 - fraction);
            (y, (bar.x + width + label_gap, y))
        })
        .collect();
    ColorbarLayout {
        bar,
        ticks,
        size: (width + label_gap + label_width, height),
    }
}

/// Draw segments, ticks and labels.
pub(super) fn draw_colorbar(
    colorbar: &Colorbar,
    layout: &ColorbarLayout,
    theme: &Theme,
) -> Vec<Node> {
    let bar = layout.bar;
    let mut nodes = Vec::new();
    // Each segment runs to the bar bottom, so later ones cover earlier ones without
    // seams; the first is longer by the end padding (lets-plot `addColorBar`).
    let count = colorbar.colors.len().max(1);
    let step = (bar.height - 2.0 * BAR_END_PADDING) / count as f64;
    let mut top = bar.y;
    let mut rects = Vec::with_capacity(count);
    for index in 0..colorbar.colors.len() {
        rects.push(Rect {
            x: bar.x,
            y: top,
            width: bar.width,
            height: bar.y + bar.height - top,
        });
        top += step + if index == 0 { BAR_END_PADDING } else { 0.0 };
    }
    nodes.push(Node::Rects(Rects {
        rects,
        fill: PerItem::Varying(colorbar.colors.iter().map(|color| Some(*color)).collect()),
        stroke: None,
    }));
    if let Some(line) = theme.legend_ticks {
        let length = theme.legend_ticks_length;
        for (y, _) in &layout.ticks {
            for (start, end) in [
                (bar.x, bar.x + length),
                (bar.x + bar.width - length, bar.x + bar.width),
            ] {
                nodes.push(Node::Path(Path {
                    points: vec![(start, *y), (end, *y)],
                    closed: false,
                    fill: None,
                    stroke: Some(Stroke::solid(line.color, line.width)),
                }));
            }
        }
    }
    if let Some(style) = theme.legend_text {
        let band = super::text_band(style);
        for ((_, anchor), (_, label)) in layout.ticks.iter().zip(&colorbar.breaks) {
            // Lines centred as a block on the tick.
            let lines = label_lines(label);
            let first = anchor.1 - band * (lines.len() as f64 - 1.0) / 2.0;
            for (index, line) in lines.iter().enumerate() {
                nodes.push(label_node(
                    line,
                    style,
                    (anchor.0, first + band * index as f64),
                    HorizontalAlign::Start,
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
}
