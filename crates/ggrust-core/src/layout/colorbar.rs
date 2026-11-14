//! Colorbars for continuous color scales (lets-plot `ColorBarAssembler`,
//! `ColorBarComponentLayout`, `ColorBarComponent`): a bar of `bins` color segments
//! with ticks at both edges. Vertical: high values at the top, labels at the right.
//! Horizontal (legends at the top or bottom): high values at the right, labels below.
//! Optional (ggrust, after legendry's `guide_colbar`): triangle caps past either
//! end (matplotlib `extend`) and minor ticks between the breaks.

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
#[derive(Clone)]
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
    /// `guide_colorbar(shape="triangle", show=)`: a triangle cap past the low
    /// (first) and high (second) end, filled with that end's color.
    pub caps: (bool, bool),
    /// `guide_colorbar(cap_size=)`: cap length in pixels; `None`: equilateral on
    /// the bar's thickness.
    pub cap_size: Option<f64>,
    /// `guide_colorbar(minor_ticks=True)`: minor tick values (transformed units),
    /// from [`minor_values`]; empty draws none.
    pub minor: Vec<f64>,
}

/// Minor tick values of a colorbar (transformed units) inside `domain`: for a
/// log10 scale 2 to 9 times each power of ten (matplotlib `LogLocator`), else
/// halfway between neighbouring breaks and half a step past the outer ones
/// (lets-plot `AxisUtil.minorDomainBreaks` for even breaks). Values on a break
/// are left out. Log10 ranges over 50 decades get none.
pub fn minor_values(domain: (f64, f64), breaks: &[f64], log10: bool) -> Vec<f64> {
    let (low, high) = (domain.0.min(domain.1), domain.0.max(domain.1));
    if !(low.is_finite() && high.is_finite()) {
        return Vec::new();
    }
    let tolerance = (high - low).abs() * 1e-9;
    let inside = |value: f64| value >= low - tolerance && value <= high + tolerance;
    let mut sorted: Vec<f64> = breaks
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect();
    sorted.sort_by(f64::total_cmp);
    sorted.dedup();
    let on_break = |value: f64| {
        sorted
            .iter()
            .any(|known| (known - value).abs() <= tolerance)
    };
    let mut values = Vec::new();
    if log10 {
        // Past 50 decades minor ticks are an unreadable smear (and many nodes).
        if high - low > 50.0 {
            return Vec::new();
        }
        let mut decade = low.floor();
        while decade <= high.ceil() {
            for factor in 2..=9 {
                values.push(decade + f64::from(factor).log10());
            }
            decade += 1.0;
        }
    } else if let [first, second, ..] = sorted[..] {
        values.push(first - (second - first) / 2.0);
        for pair in sorted.windows(2) {
            values.push((pair[0] + pair[1]) / 2.0);
        }
        let last = sorted.len() - 1;
        values.push(sorted[last] + (sorted[last] - sorted[last - 1]) / 2.0);
    }
    values
        .into_iter()
        .filter(|value| inside(*value) && !on_break(*value))
        .collect()
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
    /// Minor tick positions along the bar (y, or x when horizontal).
    pub minor: Vec<f64>,
    /// Triangle caps at the bar's start (left, or top) and end (right, or bottom).
    pub caps: (bool, bool),
    /// Length of a cap along the bar.
    pub cap: f64,
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
            minor: self.minor.iter().map(|tick| tick + along).collect(),
            caps: self.caps,
            cap: self.cap,
            size: self.size,
        }
    }

    /// The length of a triangle cap along the bar.
    pub fn cap_length(&self) -> f64 {
        self.cap
    }
}

/// The length of `colorbar`'s triangle caps on a bar `thickness` across: its
/// `cap_size`, else equilateral.
fn cap_length(colorbar: &Colorbar, thickness: f64) -> f64 {
    colorbar
        .cap_size
        .unwrap_or_else(|| thickness * 3f64.sqrt() / 2.0)
}

/// How far ticks reach out of the bar at its (bottom or right, top or left) edge:
/// the negative part of `legend_ticks_length` (ggplot2 3.5), 0 without ticks.
fn outward_ticks(theme: &Theme) -> (f64, f64) {
    if theme.legend_ticks.is_none() {
        return (0.0, 0.0);
    }
    let (first, second) = theme.legend_ticks_length;
    ((-first).max(0.0), (-second).max(0.0))
}

/// The caps at the bar's (start, end) on screen: left and right, or top and
/// bottom. Low values sit at the left and the bottom unless `reverse`.
fn screen_caps(colorbar: &Colorbar, horizontal: bool) -> (bool, bool) {
    let (low, high) = colorbar.caps;
    let low_first = horizontal != colorbar.reverse;
    if low_first { (low, high) } else { (high, low) }
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
    layout.cap = cap_length(
        colorbar,
        if horizontal {
            layout.bar.height
        } else {
            layout.bar.width
        },
    );
    // Breaks outside the domain (given breaks past the limits) would sit past the
    // bar's ends, over the caps: they get neither tick nor label (as axes).
    let (low, high) = (
        colorbar.domain.0.min(colorbar.domain.1),
        colorbar.domain.0.max(colorbar.domain.1),
    );
    let tolerance = (high - low).abs() * 1e-9;
    for ((tick, anchor), (value, _)) in layout.ticks.iter_mut().zip(&colorbar.breaks) {
        if !(*value >= low - tolerance && *value <= high + tolerance) {
            *tick = f64::NAN;
            *anchor = (f64::NAN, f64::NAN);
        }
    }
    layout = make_room_for_caps(layout, origin);
    layout = make_room_for_ticks(layout, origin, outward_ticks(theme));
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

/// `layout` grown (and moved) so its caps fit inside its box: labels may already
/// reach past the bar's ends, so only the part of a cap beyond them adds room.
fn make_room_for_caps(layout: ColorbarLayout, origin: (f64, f64)) -> ColorbarLayout {
    let cap = layout.cap_length();
    let (start, end) = layout.caps;
    if !(start || end) {
        return layout;
    }
    let bar = layout.bar;
    let (before, after) = if layout.horizontal {
        (
            bar.x - origin.0,
            origin.0 + layout.size.0 - (bar.x + bar.width),
        )
    } else {
        (
            bar.y - origin.1,
            origin.1 + layout.size.1 - (bar.y + bar.height),
        )
    };
    let lead = if start { (cap - before).max(0.0) } else { 0.0 };
    let trail = if end { (cap - after).max(0.0) } else { 0.0 };
    let shift = if layout.horizontal {
        (lead, 0.0)
    } else {
        (0.0, lead)
    };
    let mut moved = layout.translated(shift);
    if moved.horizontal {
        moved.size.0 += lead + trail;
    } else {
        moved.size.1 += lead + trail;
    }
    moved
}

/// `layout` grown (and moved) so ticks reaching out of the bar across its
/// thickness (`outward`: bottom or right edge, top or left edge) stay in its box.
fn make_room_for_ticks(
    layout: ColorbarLayout,
    origin: (f64, f64),
    (out_first, out_second): (f64, f64),
) -> ColorbarLayout {
    if out_first <= 0.0 && out_second <= 0.0 {
        return layout;
    }
    let bar = layout.bar;
    // Room already there before the bar (top or left) and after it.
    let (before, after) = if layout.horizontal {
        (
            bar.y - origin.1,
            origin.1 + layout.size.1 - (bar.y + bar.height),
        )
    } else {
        (
            bar.x - origin.0,
            origin.0 + layout.size.0 - (bar.x + bar.width),
        )
    };
    let lead = (out_second - before).max(0.0);
    let trail = (out_first - after).max(0.0);
    let shift = if layout.horizontal {
        (0.0, lead)
    } else {
        (lead, 0.0)
    };
    let mut moved = layout.translated(shift);
    if moved.horizontal {
        moved.size.1 += lead + trail;
    } else {
        moved.size.0 += lead + trail;
    }
    moved
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
    let caps = screen_caps(colorbar, horizontal);
    let minor_along = |bar: Rect| -> Vec<f64> {
        colorbar
            .minor
            .iter()
            .map(|value| fraction(*value))
            // Values on the domain ends may land a rounding error outside it.
            .filter(|at| (-1e-9..=1.0 + 1e-9).contains(at))
            .map(|at| tick_position(bar, horizontal, at.clamp(0.0, 1.0)))
            .collect()
    };
    let minor = minor_along(bar);
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
            minor,
            caps,
            cap: 0.0,
            size: (width, height),
        };
    };
    let band = text_band(style);
    if colorbar.label_ends {
        let title_left = super::legend::title_position(theme, horizontal)
            == crate::theme::LegendTitlePosition::Left;
        let mut layout = layout_end_labels(
            colorbar, bar, horizontal, style, band, &fraction, title_left,
        );
        layout.minor = minor_along(layout.bar);
        layout.caps = caps;
        return layout;
    }
    if horizontal {
        // lets-plot: labels a third of a line below the bar, centred on their ticks;
        // ticks reaching out of the bar push them further.
        let distance = band / 3.0 + outward_ticks(theme).0;
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
            minor: minor_along(bar),
            caps,
            cap: 0.0,
            size: (
                overhang_left + width + overhang_right,
                height + distance + label_height,
            ),
        };
    }
    let label_gap = measure("_", style.size, style.font).width / 2.0 + outward_ticks(theme).0;
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
        minor,
        caps,
        cap: 0.0,
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
    title_left: bool,
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
    // Caps sit between the bar and the end labels: the gaps grow by their length.
    let (cap_start, cap_end) = screen_caps(colorbar, horizontal);
    let cap = cap_length(colorbar, if horizontal { bar.height } else { bar.width });
    let cap_at = |shown: bool| if shown { cap } else { 0.0 };
    // Horizontal: low before (left), high after; vertical: high above (start).
    let (cap_low, cap_high) = if horizontal {
        (cap_at(cap_start), cap_at(cap_end))
    } else {
        (cap_at(cap_end), cap_at(cap_start))
    };
    let gap_low = |size: (f64, f64)| {
        if size.0 > 0.0 {
            band / 2.0 + cap_low
        } else {
            0.0
        }
    };
    let gap_high = |size: (f64, f64)| {
        if size.0 > 0.0 {
            band / 2.0 + cap_high
        } else {
            0.0
        }
    };
    // The title sits right before the graph: keep a wider gap from the left label.
    let lead = if horizontal && low_size.0 > 0.0 && title_left && !colorbar.title.trim().is_empty()
    {
        band * 0.75
    } else {
        0.0
    };
    let (origin_x, origin_y) = (bar.x + lead, bar.y);
    let (bar, size, low_anchor, high_anchor) = if horizontal {
        let height = bar.height.max(low_size.1).max(high_size.1);
        let bar = Rect {
            x: origin_x + low_size.0 + gap_low(low_size),
            y: origin_y + (height - bar.height) / 2.0,
            ..bar
        };
        let middle = bar.y + bar.height / 2.0;
        (
            bar,
            (
                lead + low_size.0
                    + gap_low(low_size)
                    + bar.width
                    + gap_high(high_size)
                    + high_size.0,
                height,
            ),
            (bar.x - gap_low(low_size), middle),
            (bar.x + bar.width + gap_high(high_size), middle),
        )
    } else {
        let width = bar.width.max(low_size.0).max(high_size.0);
        let bar = Rect {
            x: origin_x + (width - bar.width) / 2.0,
            y: origin_y + high_size.1 + gap_high(high_size),
            ..bar
        };
        let centre = bar.x + bar.width / 2.0;
        (
            bar,
            (
                width,
                high_size.1 + gap_high(high_size) + bar.height + gap_low(low_size) + low_size.1,
            ),
            (centre, bar.y + bar.height + gap_low(low_size)),
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
        minor: Vec::new(),
        caps: (false, false),
        cap: 0.0,
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

/// The outline of the bar and its caps, clockwise from the top-left corner.
fn frame_outline(
    bar: Rect,
    horizontal: bool,
    (start, end): (bool, bool),
    cap: f64,
) -> Vec<(f64, f64)> {
    let (left, top) = (bar.x, bar.y);
    let (right, bottom) = (bar.x + bar.width, bar.y + bar.height);
    let mut points = vec![(left, top)];
    if horizontal {
        let middle = top + bar.height / 2.0;
        points.push((right, top));
        if end {
            points.push((right + cap, middle));
        }
        points.push((right, bottom));
        points.push((left, bottom));
        if start {
            points.push((left - cap, middle));
        }
    } else {
        let middle = left + bar.width / 2.0;
        if start {
            points.push((middle, top - cap));
        }
        points.push((right, top));
        points.push((right, bottom));
        if end {
            points.push((middle, bottom + cap));
        }
        points.push((left, bottom));
    }
    points
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
    // Caps go first, reaching a unit under the bar (opaque bars only) so no
    // anti-aliased seam shows where they meet it.
    let tuck = if opaque { 1.0 } else { 0.0 };
    let cap = layout.cap_length();
    let ends = [
        (layout.caps.0, colors.first().copied().flatten(), true),
        (layout.caps.1, colors.last().copied().flatten(), false),
    ];
    for (shown, color, start) in ends {
        let (true, Some(color)) = (shown, color) else {
            continue;
        };
        let points = match (horizontal, start) {
            (true, true) => vec![
                (bar.x + tuck, bar.y),
                (bar.x - cap, bar.y + bar.height / 2.0),
                (bar.x + tuck, bar.y + bar.height),
            ],
            (true, false) => {
                let edge = bar.x + bar.width;
                vec![
                    (edge - tuck, bar.y),
                    (edge + cap, bar.y + bar.height / 2.0),
                    (edge - tuck, bar.y + bar.height),
                ]
            }
            (false, true) => vec![
                (bar.x, bar.y + tuck),
                (bar.x + bar.width / 2.0, bar.y - cap),
                (bar.x + bar.width, bar.y + tuck),
            ],
            (false, false) => {
                let edge = bar.y + bar.height;
                vec![
                    (bar.x, edge - tuck),
                    (bar.x + bar.width / 2.0, edge + cap),
                    (bar.x + bar.width, edge - tuck),
                ]
            }
        };
        nodes.push(Node::Path(Path {
            points,
            closed: true,
            fill: Some(color),
            stroke: None,
        }));
    }
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
    if let Some(line) = theme.legend_frame {
        nodes.push(Node::Path(Path {
            points: frame_outline(bar, horizontal, layout.caps, cap),
            closed: true,
            fill: None,
            stroke: Some(line.stroke()),
        }));
    }
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
        let (first, second) = theme.legend_ticks_length;
        let (low, high) = colorbar.domain;
        let tolerance = (high - low).abs() * 1e-9;
        let hidden = |value: f64| {
            (!colorbar.limit_ticks.0 && (value - low).abs() <= tolerance)
                || (!colorbar.limit_ticks.1 && (value - high).abs() <= tolerance)
        };
        let major = layout
            .ticks
            .iter()
            .zip(&colorbar.breaks)
            .filter(|(_, (value, _))| !hidden(*value))
            .map(|((tick, _), _)| (*tick, 1.0));
        // Minor ticks: half as long as the major ones.
        let minor = layout.minor.iter().map(|tick| (*tick, 0.5));
        for (tick, scale) in major.chain(minor).filter(|(tick, _)| tick.is_finite()) {
            // From each edge: positive lengths point into the bar, negative out.
            let (first, second) = (first * scale, second * scale);
            let marks = if horizontal {
                let bottom = bar.y + bar.height;
                [
                    (first, (tick, bottom), (tick, bottom - first)),
                    (second, (tick, bar.y), (tick, bar.y + second)),
                ]
            } else {
                let right = bar.x + bar.width;
                [
                    (first, (right, tick), (right - first, tick)),
                    (second, (bar.x, tick), (bar.x + second, tick)),
                ]
            };
            for (length, start, end) in marks {
                if length == 0.0 {
                    continue;
                }
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
            if !anchor.0.is_finite() {
                continue; // outside the bar, or a middle break of `ends`
            }
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
            caps: (false, false),
            cap_size: None,
            minor: Vec::new(),
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
            caps: (false, false),
            cap_size: None,
            minor: Vec::new(),
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
            caps: (false, false),
            cap_size: None,
            minor: Vec::new(),
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
            caps: (false, false),
            cap_size: None,
            minor: Vec::new(),
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
            caps: (false, false),
            cap_size: None,
            minor: Vec::new(),
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
            caps: (false, false),
            cap_size: None,
            minor: Vec::new(),
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

    fn plain(caps: (bool, bool), reverse: bool) -> Colorbar {
        Colorbar {
            title: "v".into(),
            domain: (0.0, 10.0),
            breaks: vec![(0.0, Label::plain("0")), (10.0, Label::plain("10"))],
            colors: vec![Color::WHITE; DEFAULT_BINS],
            bar_width: None,
            bar_height: None,
            label_ends: false,
            na: None,
            reverse,
            limit_ticks: (true, true),
            style: Default::default(),
            caps,
            cap_size: None,
            minor: Vec::new(),
        }
    }

    #[test]
    fn log10_minor_ticks_fill_each_decade_and_skip_breaks() {
        // 5 to 500: 5..9, 20..90 except 50, 200..500 (the domain ends count).
        let domain = (5f64.log10(), 500f64.log10());
        let breaks = [10f64.log10(), 50f64.log10(), 100f64.log10()];
        let values: Vec<f64> = minor_values(domain, &breaks, true)
            .into_iter()
            .map(|value| (10f64.powf(value) * 1e6).round() / 1e6)
            .collect();
        let expected = [
            5.0, 6.0, 7.0, 8.0, 9.0, 20.0, 30.0, 40.0, 60.0, 70.0, 80.0, 90.0, 200.0, 300.0, 400.0,
            500.0,
        ];
        assert_eq!(values, expected);
    }

    #[test]
    fn linear_minor_ticks_sit_between_breaks() {
        let values = minor_values((0.0, 10.0), &[2.0, 4.0, 6.0, 8.0], false);
        assert_eq!(values, vec![1.0, 3.0, 5.0, 7.0, 9.0]);
        assert!(minor_values((0.0, 10.0), &[5.0], false).is_empty());
        assert!(minor_values((f64::NAN, 1.0), &[0.5], true).is_empty());
    }

    #[test]
    fn caps_take_room_past_the_bar_ends() {
        let theme = Theme::default();
        let without = layout_colorbar(&plain((false, false), false), (0.0, 0.0), false, &theme);
        let with = layout_colorbar(&plain((true, true), false), (0.0, 0.0), false, &theme);
        let cap = with.cap_length();
        assert_eq!(cap, 23.0 * 3f64.sqrt() / 2.0);
        // Vertical: the bar moves down by one cap and the box grows by two.
        assert_eq!(with.bar.y, without.bar.y + cap);
        assert_eq!(with.size.1, without.size.1 + 2.0 * cap);
        assert_eq!(with.caps, (true, true));
        // Only the high (top) cap: the bar moves, the box grows by one cap.
        let high = layout_colorbar(&plain((false, true), false), (0.0, 0.0), false, &theme);
        assert_eq!((high.caps, high.bar.y), ((true, false), cap));
        // Reversed: the high cap is at the bottom.
        let reversed = layout_colorbar(&plain((false, true), true), (0.0, 0.0), false, &theme);
        assert_eq!((reversed.caps, reversed.bar.y), ((false, true), 0.0));
        assert_eq!(reversed.size.1, without.size.1 + cap);
        // Horizontal: labels already reach past the ends; the caps add the rest.
        let flat = layout_colorbar(&plain((true, true), false), (0.0, 0.0), true, &theme);
        assert!(flat.bar.x >= flat.cap_length());
        assert!(flat.size.0 >= flat.bar.x + flat.bar.width + flat.cap_length());
    }

    #[test]
    fn caps_and_minor_ticks_are_drawn() {
        let theme = Theme::default();
        let mut colorbar = plain((true, true), false);
        colorbar.minor = vec![5.0];
        let layout = layout_colorbar(&colorbar, (0.0, 0.0), true, &theme);
        assert_eq!(layout.minor.len(), 1);
        let nodes = draw_colorbar(&colorbar, &layout, &theme);
        let triangles = nodes
            .iter()
            .filter(|node| matches!(node, Node::Path(path) if path.closed && path.fill.is_some()))
            .count();
        assert_eq!(triangles, 2);
        let ticks = nodes
            .iter()
            .filter(|node| matches!(node, Node::Path(path) if !path.closed))
            .count();
        // Two marks (both edges) per major and minor tick.
        assert_eq!(ticks, 2 * 3);
    }

    #[test]
    fn outward_ticks_push_labels_and_grow_the_box() {
        let colorbar = plain((false, false), false);
        let ticks = Some(crate::theme::LineStyle {
            color: Color::BLACK,
            width: 1.0,
            dash: Default::default(),
        });
        let inside = Theme {
            legend_ticks: ticks,
            ..Theme::default()
        };
        let outside = Theme {
            legend_ticks: ticks,
            legend_ticks_length: (-6.0, -3.0),
            ..Theme::default()
        };
        for horizontal in [true, false] {
            let before = layout_colorbar(&colorbar, (0.0, 0.0), horizontal, &inside);
            let after = layout_colorbar(&colorbar, (0.0, 0.0), horizontal, &outside);
            let across = |layout: &ColorbarLayout| {
                if horizontal {
                    layout.size.1
                } else {
                    layout.size.0
                }
            };
            // Labels move out by 6, the top (left) side takes 3 more.
            assert_eq!(across(&after), across(&before) + 9.0);
            let label = |layout: &ColorbarLayout| {
                let anchor = layout.ticks[0].1;
                let bar = layout.bar;
                if horizontal {
                    anchor.1 - (bar.y + bar.height)
                } else {
                    anchor.0 - (bar.x + bar.width)
                }
            };
            assert!((label(&after) - label(&before) - 6.0).abs() < 1e-9);
            let nodes = draw_colorbar(&colorbar, &after, &outside);
            // Every tick mark starts on the bar edge and points out of it.
            let bar = after.bar;
            for node in &nodes {
                if let Node::Path(path) = node
                    && !path.closed
                {
                    let ((x0, y0), (x1, y1)) = (path.points[0], path.points[1]);
                    if horizontal {
                        assert_eq!(x0, x1);
                        assert!(y1 > bar.y + bar.height - 1e-9 || y1 < bar.y + 1e-9, "{y1}");
                        assert!(y0 == bar.y || y0 == bar.y + bar.height);
                    } else {
                        assert_eq!(y0, y1);
                        assert!(x1 > bar.x + bar.width - 1e-9 || x1 < bar.x + 1e-9, "{x1}");
                        assert!(x0 == bar.x || x0 == bar.x + bar.width);
                    }
                }
            }
        }
    }

    #[test]
    fn frame_outlines_the_bar_and_its_caps() {
        let bar = Rect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 10.0,
        };
        assert_eq!(
            frame_outline(bar, true, (true, true), 5.0),
            vec![
                (10.0, 20.0),
                (110.0, 20.0),
                (115.0, 25.0),
                (110.0, 30.0),
                (10.0, 30.0),
                (5.0, 25.0)
            ]
        );
        let tall = Rect {
            width: 10.0,
            height: 100.0,
            ..bar
        };
        assert_eq!(
            frame_outline(tall, false, (false, true), 5.0),
            vec![
                (10.0, 20.0),
                (20.0, 20.0),
                (20.0, 120.0),
                (15.0, 125.0),
                (10.0, 120.0)
            ]
        );
        // Drawn only with legend_frame, as one closed unfilled path.
        let mut colorbar = plain((true, true), false);
        colorbar.cap_size = Some(30.0);
        let framed = Theme {
            legend_frame: Some(crate::theme::LineStyle {
                color: Color::BLACK,
                width: 1.0,
                dash: Default::default(),
            }),
            ..Theme::default()
        };
        let layout = layout_colorbar(&colorbar, (0.0, 0.0), false, &framed);
        assert_eq!(layout.cap_length(), 30.0);
        let outlines = |theme: &Theme| {
            draw_colorbar(&colorbar, &layout, theme)
                .iter()
                .filter(
                    |node| matches!(node, Node::Path(path) if path.closed && path.fill.is_none()),
                )
                .count()
        };
        assert_eq!((outlines(&framed), outlines(&Theme::default())), (1, 0));
    }

    #[test]
    fn breaks_outside_the_domain_get_no_tick_or_label() {
        let mut colorbar = plain((true, true), false);
        colorbar.breaks.push((12.0, Label::plain("12")));
        let theme = Theme::default();
        for horizontal in [true, false] {
            let layout = layout_colorbar(&colorbar, (0.0, 0.0), horizontal, &theme);
            let (tick, anchor) = layout.ticks[2];
            assert!(tick.is_nan() && anchor.0.is_nan());
            assert!(layout.ticks[1].0.is_finite());
            let nodes = draw_colorbar(&colorbar, &layout, &theme);
            let has_label = |text: &str| {
                nodes
                    .iter()
                    .any(|node| format!("{node:?}").contains(&format!("\"{text}\"")))
            };
            assert!(has_label("10") && !has_label("12"));
        }
    }
}
