//! Choosing axis breaks: port of lets-plot's flexible break layout
//! (`HorizontalFlexBreaksLabelsLayout`, `VerticalFlexBreaksLabelsLayout`,
//! `BreakLabelsLayoutUtil.getFlexBreaks`).
//!
//! Break *counts* are decided with lets-plot's text width estimator so ggrust picks
//! the same breaks as lets-plot; labels are then drawn with real font metrics.

use crate::scale::ContinuousPositionScale;
use crate::scale::breaks::linear_breaks;
use crate::scale::format::{Label, format_breaks};
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

/// Breaks of one axis with their labels.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisBreaks {
    /// Break values in data units.
    pub values: Vec<f64>,
    /// Formatted labels, one per value.
    pub labels: Vec<Label>,
}

impl AxisBreaks {
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

/// Breaks for an axis of `axis_length` pixels showing `scale`.
pub fn choose_breaks(
    scale: &ContinuousPositionScale,
    axis_length: f64,
    orientation: Orientation,
    text: Option<TextStyle>,
) -> AxisBreaks {
    let (low, high) = scale.expanded_domain();
    let breaks_for = |count: usize| {
        let values = linear_breaks(low, high, count);
        let labels = format_breaks(&values);
        AxisBreaks { values, labels }
    };
    let size = text.map_or(DEFAULT_LABEL_SIZE, |text| text.size);
    let face = text.map(|text| text.face).unwrap_or_default();
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
    scale: &ContinuousPositionScale,
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

fn estimated_label_width(label: &Label, size: f64, face: crate::text::FontFace) -> f64 {
    estimated_width(&label.text, size, face)
        + label.superscript.as_deref().map_or(0.0, |superscript| {
            estimated_width(superscript, size * SUPERSCRIPT_SCALE, face)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    fn scale(low: f64, high: f64) -> ContinuousPositionScale {
        let mut scale = ContinuousPositionScale::default();
        scale.train_range(low, high);
        scale
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
        );
        assert_eq!(texts(&x), ["1", "1.5", "2", "2.5", "3", "3.5", "4", "4.5"]);
        let y = choose_breaks(
            &scale(1.0, 4.0),
            338.0,
            Orientation::Vertical,
            theme.axis_text_y,
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
        );
        assert_eq!(texts(&y), ["0.1", "0.15", "0.2", "0.25", "0.3"]);
        let y = choose_breaks(
            &scale(-7.0, 13.0),
            338.0,
            Orientation::Vertical,
            theme.axis_text_y,
        );
        assert_eq!(texts(&y), ["-5", "0", "5", "10"]);
    }

    #[test]
    fn one_allowed_break_keeps_the_first() {
        let breaks = choose_breaks(&scale(0.0, 1.0), 20.0, Orientation::Vertical, None);
        assert_eq!(breaks.values.len(), 1);
    }

    #[test]
    fn tiny_axes_still_get_a_break() {
        let breaks = choose_breaks(&scale(0.0, 10.0), 5.0, Orientation::Horizontal, None);
        assert!(!breaks.values.is_empty());
    }
}
