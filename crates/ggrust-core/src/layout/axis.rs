//! Choosing axis breaks: port of lets-plot's flexible break layout
//! (`HorizontalFlexBreaksLabelsLayout`, `VerticalFlexBreaksLabelsLayout`,
//! `BreakLabelsLayoutUtil.getFlexBreaks`).
//!
//! Break *counts* are decided with lets-plot's text width estimator so ggrust picks
//! the same breaks as lets-plot; labels are then drawn with real font metrics.

use crate::scale::PositionScale;
use crate::scale::format::Label;
use crate::scene::SUPERSCRIPT_SCALE;
use crate::text::estimated_width;
use crate::theme::TextStyle;

/// lets-plot `AxisLabelsLayout.INITIAL_TICK_LABEL`.
const INITIAL_TICK_LABEL: &str = "0000";
/// lets-plot `MIN_TICK_LABEL_DISTANCE_AUTO`: spacing assumed when estimating counts.
const LABEL_DISTANCE_FOR_COUNT: f64 = 20.0;
/// lets-plot `MIN_TICK_LABEL_DISTANCE`: labels closer than this overlap.
const MIN_LABEL_DISTANCE: f64 = 8.0;
/// Font size used for estimates when axis text is blank (lets-plot keeps a label
/// spec for counting either way).
const DEFAULT_LABEL_SIZE: f64 = 13.0;
/// lets-plot `H_AXIS_LABELS_EXPAND`: how far labels may reach past the axis ends.
pub const LABELS_OVERFLOW: f64 = 10.0;
/// lets-plot `HorizontalMultilineLabelsLayout.MIN_DISTANCE`: gap between labels on a row.
const TWO_ROWS_MIN_DISTANCE: f64 = 60.0;
/// lets-plot `HorizontalTiltedLabelsLayout.ROTATION_DEGREE`.
pub const TILT_DEGREES: f64 = 30.0;
/// lets-plot `MIN_DISTANCE` of the tilted and vertical layouts.
const ROTATED_MIN_DISTANCE: f64 = 5.0;

/// How the labels of a horizontal axis are placed.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum LabelArrangement {
    /// One row, centred under the ticks.
    #[default]
    Simple,
    /// Staggered over rows; the row of every label (0 or 1).
    TwoRows(Vec<usize>),
    /// Rotated by [`TILT_DEGREES`], ending at the tick.
    Tilted,
    /// Rotated by 90 degrees, ending at the tick.
    Vertical,
}

/// Breaks of one axis with their labels.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisBreaks {
    /// Break positions in transformed units.
    pub values: Vec<f64>,
    /// Formatted labels, one per value.
    pub labels: Vec<Label>,
    /// Label placement (horizontal axes; vertical axes are always simple).
    pub arrangement: LabelArrangement,
}

impl AxisBreaks {
    fn new(values: Vec<f64>, labels: Vec<Label>) -> Self {
        AxisBreaks {
            values,
            labels,
            arrangement: LabelArrangement::Simple,
        }
    }

    fn first_only(mut self) -> Self {
        self.values.truncate(1);
        self.labels.truncate(1);
        self
    }
}

/// Whether labels run along the axis (x: widths) or stack across it (y: heights).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// Horizontal axis: labels side by side.
    Horizontal,
    /// Vertical axis: labels stacked.
    Vertical,
}

/// Breaks for an axis of `axis_length` pixels showing `scale`. `overflow` is how
/// far (px) labels of a horizontal axis may reach before its start and past its end.
pub fn choose_breaks(
    scale: &PositionScale,
    axis_length: f64,
    orientation: Orientation,
    text: Option<TextStyle>,
    overflow: (f64, f64),
) -> AxisBreaks {
    let size = text.map_or(DEFAULT_LABEL_SIZE, |text| text.size);
    let face = text.map(|text| text.font.face).unwrap_or_default();
    if scale.is_discrete() {
        // lets-plot fixed breaks: every category is a break.
        let breaks = scale.breaks(0);
        let mut breaks = AxisBreaks::new(breaks.positions, breaks.labels);
        if orientation == Orientation::Horizontal && text.is_some() {
            let ticks: Vec<f64> = breaks
                .values
                .iter()
                .map(|value| scale.map(*value) * axis_length)
                .collect();
            let widths: Vec<f64> = breaks
                .labels
                .iter()
                .map(|label| estimated_label_width(label, size, face))
                .collect();
            let span = (-overflow.0, axis_length + overflow.1);
            breaks.arrangement = arrange_fixed_labels(&ticks, &widths, size, span);
        }
        return breaks;
    }
    let breaks_for = |count: usize| {
        let breaks = scale.breaks(count);
        AxisBreaks::new(breaks.positions, breaks.labels)
    };
    let count_for = |label_extent: f64| {
        ((axis_length / (label_extent + LABEL_DISTANCE_FOR_COUNT)).floor() as usize).max(1)
    };
    match orientation {
        // lets-plot: a label line box is font-size tall; no overlap re-check.
        Orientation::Vertical => flex_breaks(&breaks_for, count_for(size)),
        Orientation::Horizontal => {
            let width = |label: &Label| estimated_label_width(label, size, face);
            let mut target = count_for(estimated_width(INITIAL_TICK_LABEL, size, face));
            let mut breaks = flex_breaks(&breaks_for, target);
            while text.is_some() && overlaps(&breaks, scale, axis_length, &width) {
                let widest = breaks.labels.iter().map(&width).fold(0.0, f64::max);
                let next = count_for(widest);
                if next >= target {
                    break;
                }
                target = next;
                breaks = flex_breaks(&breaks_for, target);
            }
            breaks
        }
    }
}

/// lets-plot `HorizontalFixedBreaksLabelsLayout`: the first of simple, two rows,
/// tilted that neither overlaps nor leaves `span`; otherwise vertical. `ticks` are
/// label centres (px), `widths` estimated label widths, `height` the label height.
fn arrange_fixed_labels(
    ticks: &[f64],
    widths: &[f64],
    height: f64,
    span: (f64, f64),
) -> LabelArrangement {
    let ranges: Vec<(f64, f64)> = ticks
        .iter()
        .zip(widths)
        .map(|(tick, width)| (tick - width / 2.0, tick + width / 2.0))
        .collect();
    let fits = |ranges: &[(f64, f64)]| {
        ranges
            .iter()
            .all(|(left, right)| *left >= span.0 && *right <= span.1)
    };
    let connected = |a: (f64, f64), b: (f64, f64)| a.0 <= b.1 && b.0 <= a.1;

    // Simple: a label closer than MIN_LABEL_DISTANCE to the labels before it overlaps.
    let mut union: Option<(f64, f64)> = None;
    let mut overlap = false;
    for &(left, right) in &ranges {
        let padded = (
            left - MIN_LABEL_DISTANCE / 2.0,
            right + MIN_LABEL_DISTANCE / 2.0,
        );
        if let Some(bounds) = union {
            overlap = overlap || connected(bounds, padded);
            union = Some((bounds.0.min(left), bounds.1.max(right)));
        } else {
            union = Some((left, right));
        }
    }
    if !overlap && fits(&ranges) {
        return LabelArrangement::Simple;
    }

    // Two rows: each label goes on the first row it keeps 60 px away from.
    let mut rows: Vec<(f64, f64)> = Vec::new();
    let mut row_of = Vec::with_capacity(ranges.len());
    for &(left, right) in &ranges {
        let padded = (left - TWO_ROWS_MIN_DISTANCE, right + TWO_ROWS_MIN_DISTANCE);
        match rows.iter().position(|row| !connected(*row, padded)) {
            Some(index) => {
                rows[index] = (rows[index].0.min(left), rows[index].1.max(right));
                row_of.push(index);
            }
            None => {
                rows.push((left, right));
                row_of.push(rows.len() - 1);
            }
        }
    }
    if rows.len() <= 2 && fits(&ranges) {
        return LabelArrangement::TwoRows(row_of);
    }

    let tick_distance = match ticks {
        [first, second, ..] => (first - second).abs(),
        _ => f64::INFINITY,
    };
    let (sin, cos) = TILT_DEGREES.to_radians().sin_cos();
    let tilted: Vec<(f64, f64)> = ticks
        .iter()
        .zip(widths)
        .map(|(tick, width)| {
            let left = tick - (width * cos + height * sin);
            (left, left + width * cos + 2.0 * height * sin)
        })
        .collect();
    if tick_distance >= (height + ROTATED_MIN_DISTANCE) / sin && fits(&tilted) {
        return LabelArrangement::Tilted;
    }
    LabelArrangement::Vertical
}

/// lets-plot `getFlexBreaks`: shrink the requested count until at most
/// `max_count` breaks come back; a single allowed break keeps only the first.
fn flex_breaks(breaks_for: &dyn Fn(usize) -> AxisBreaks, max_count: usize) -> AxisBreaks {
    let mut breaks = breaks_for(max_count);
    if max_count == 1 && !breaks.values.is_empty() {
        return breaks.first_only();
    }
    let mut count = max_count;
    while breaks.values.len() > max_count {
        let delta = ((breaks.values.len() - max_count) / 2).max(1);
        if count <= delta + 1 {
            return breaks_for(1);
        }
        count -= delta;
        breaks = breaks_for(count);
    }
    breaks
}

/// lets-plot `HorizontalSimpleLabelsLayout`: a label overlaps when its range,
/// widened by half the minimum distance on each side, touches the union of the
/// labels before it.
fn overlaps(
    breaks: &AxisBreaks,
    scale: &PositionScale,
    axis_length: f64,
    width: &dyn Fn(&Label) -> f64,
) -> bool {
    let mut union: Option<(f64, f64)> = None;
    for (value, label) in breaks.values.iter().zip(&breaks.labels) {
        let centre = scale.map(*value) * axis_length;
        let half = width(label) / 2.0;
        let (left, right) = (centre - half, centre + half);
        if let Some((union_left, union_right)) = union {
            let padding = MIN_LABEL_DISTANCE / 2.0;
            if left - padding <= union_right && union_left <= right + padding {
                return true;
            }
            union = Some((union_left.min(left), union_right.max(right)));
        } else {
            union = Some((left, right));
        }
    }
    false
}

/// lets-plot's estimated width of a label (with its superscript).
pub(super) fn estimated_label_width(label: &Label, size: f64, face: crate::text::FontFace) -> f64 {
    estimated_width(&label.text, size, face)
        + label.superscript.as_deref().map_or(0.0, |superscript| {
            estimated_width(superscript, size * SUPERSCRIPT_SCALE, face)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::ContinuousPositionScale;
    use crate::theme::Theme;

    fn scale(low: f64, high: f64) -> PositionScale {
        let mut scale = ContinuousPositionScale::default();
        scale.train_range(low, high);
        PositionScale::Continuous(scale)
    }

    fn texts(breaks: &AxisBreaks) -> Vec<String> {
        breaks.labels.iter().map(Label::to_string).collect()
    }

    #[test]
    fn reference_plot_breaks_match_lets_plot() {
        let theme = Theme::default();
        let x = choose_breaks(
            &scale(1.0, 4.5),
            549.96,
            Orientation::Horizontal,
            theme.axis_text_x,
            (0.0, 0.0),
        );
        assert_eq!(texts(&x), ["1", "1.5", "2", "2.5", "3", "3.5", "4", "4.5"]);
        let y = choose_breaks(
            &scale(1.0, 4.0),
            338.0,
            Orientation::Vertical,
            theme.axis_text_y,
            (0.0, 0.0),
        );
        assert_eq!(texts(&y), ["1", "1.5", "2", "2.5", "3", "3.5", "4"]);
    }

    #[test]
    fn flex_breaks_cut_doubled_counts_like_lets_plot() {
        // Values observed in lets-plot 4.11 to_svg() at the default size (bug hunt 0.6).
        let theme = Theme::default();
        let y = choose_breaks(
            &scale(0.1, 0.3),
            338.0,
            Orientation::Vertical,
            theme.axis_text_y,
            (0.0, 0.0),
        );
        assert_eq!(texts(&y), ["0.1", "0.15", "0.2", "0.25", "0.3"]);
        let y = choose_breaks(
            &scale(-7.0, 13.0),
            338.0,
            Orientation::Vertical,
            theme.axis_text_y,
            (0.0, 0.0),
        );
        assert_eq!(texts(&y), ["-5", "0", "5", "10"]);
    }

    #[test]
    fn one_allowed_break_keeps_the_first() {
        let breaks = choose_breaks(
            &scale(0.0, 1.0),
            20.0,
            Orientation::Vertical,
            None,
            (0.0, 0.0),
        );
        assert_eq!(breaks.values.len(), 1);
    }

    #[test]
    fn fixed_labels_fall_back_like_lets_plot() {
        let span = (-10.0, 410.0);
        let arrange =
            |ticks: &[f64], widths: &[f64]| arrange_fixed_labels(ticks, widths, 13.0, span);
        assert_eq!(
            arrange(&[50.0, 200.0, 350.0], &[40.0, 40.0, 40.0]),
            LabelArrangement::Simple
        );
        // The last label reaches 30 px past the end: only tilting keeps it inside.
        assert_eq!(
            arrange(&[50.0, 200.0, 400.0], &[40.0, 40.0, 60.0]),
            LabelArrangement::Tilted
        );
        // Neighbours overlap but every other label fits on one row.
        assert_eq!(
            arrange(&[100.0, 160.0, 220.0, 280.0], &[56.0, 56.0, 56.0, 56.0]),
            LabelArrangement::TwoRows(vec![0, 1, 0, 1])
        );
        // Ticks 20 px apart are too close to tilt (needs (13 + 5) / sin 30 = 36 px).
        let ticks: Vec<f64> = (0..20)
            .map(|index| 10.0 + 20.0 * f64::from(index))
            .collect();
        assert_eq!(arrange(&ticks, &[60.0; 20]), LabelArrangement::Vertical);
    }

    #[test]
    fn tiny_axes_still_get_a_break() {
        let breaks = choose_breaks(
            &scale(0.0, 10.0),
            5.0,
            Orientation::Horizontal,
            None,
            (0.0, 0.0),
        );
        assert!(!breaks.values.is_empty());
    }
}
