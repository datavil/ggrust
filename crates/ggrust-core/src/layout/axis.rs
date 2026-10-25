//! Choosing axis breaks that fit: port of lets-plot's flexible break layout.
//!
//! The first break count comes from the space a typical label ("0000") needs; while
//! neighbouring labels overlap, the count is re-estimated from the real labels.

use crate::scale::ContinuousPositionScale;
use crate::scale::breaks::linear_breaks;
use crate::scale::format::format_breaks;
use crate::text::measure;
use crate::theme::TextStyle;

/// lets-plot `AxisLabelsLayout.INITIAL_TICK_LABEL`.
const INITIAL_TICK_LABEL: &str = "0000";
/// lets-plot `MIN_TICK_LABEL_DISTANCE_AUTO`: spacing assumed when estimating counts.
const LABEL_DISTANCE_FOR_COUNT: f64 = 20.0;
/// lets-plot `MIN_TICK_LABEL_DISTANCE`: labels closer than this overlap.
const MIN_LABEL_DISTANCE: f64 = 8.0;

/// Breaks of one axis with their labels.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisBreaks {
    /// Break values in data units.
    pub values: Vec<f64>,
    /// Formatted labels, one per value.
    pub labels: Vec<String>,
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
    let size = text.map_or(13.0, |text| text.size);
    let extent = |label: &str| match orientation {
        Orientation::Horizontal => {
            text.map_or(0.0, |text| measure(label, text.size, text.face).width)
        }
        Orientation::Vertical => size,
    };
    let count = |label_extent: f64| {
        ((axis_length / (label_extent + LABEL_DISTANCE_FOR_COUNT)).floor() as usize).max(1)
    };
    let mut target = count(extent(INITIAL_TICK_LABEL));
    loop {
        let values = linear_breaks(low, high, target);
        let labels = format_breaks(&values);
        let extents: Vec<f64> = labels.iter().map(|label| extent(label)).collect();
        let positions: Vec<f64> = values
            .iter()
            .map(|&value| scale.map(value) * axis_length)
            .collect();
        let overlaps = text.is_some()
            && positions
                .windows(2)
                .zip(extents.windows(2))
                .any(|(position, extent)| {
                    (position[1] - position[0]).abs()
                        < (extent[0] + extent[1]) / 2.0 + MIN_LABEL_DISTANCE
                });
        if !overlaps {
            return AxisBreaks { values, labels };
        }
        let widest = extents.iter().copied().fold(0.0, f64::max);
        let next = count(widest);
        if next >= target {
            return AxisBreaks { values, labels };
        }
        target = next;
    }
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

    #[test]
    fn reference_plot_breaks_match_lets_plot() {
        let theme = Theme::default();
        let x = choose_breaks(
            &scale(1.0, 4.5),
            549.96,
            Orientation::Horizontal,
            theme.axis_text_x,
        );
        assert_eq!(x.labels, ["1", "1.5", "2", "2.5", "3", "3.5", "4", "4.5"]);
        let y = choose_breaks(
            &scale(1.0, 4.0),
            338.0,
            Orientation::Vertical,
            theme.axis_text_y,
        );
        assert_eq!(y.labels, ["1", "1.5", "2", "2.5", "3", "3.5", "4"]);
    }

    #[test]
    fn long_labels_reduce_the_count() {
        let theme = Theme::default();
        let breaks = choose_breaks(
            &scale(0.0, 1.2e6),
            150.0,
            Orientation::Horizontal,
            theme.axis_text_x,
        );
        let widths: Vec<f64> = breaks
            .labels
            .iter()
            .map(|label| measure(label, 13.0, crate::text::FontFace::Plain).width)
            .collect();
        assert!(breaks.values.len() >= 2, "{breaks:?}");
        for pair in breaks.values.windows(2).zip(widths.windows(2)) {
            let (values, widths) = pair;
            let gap = (values[1] - values[0]) / 1.32e6 * 150.0;
            assert!(gap >= (widths[0] + widths[1]) / 2.0, "{breaks:?}");
        }
    }

    #[test]
    fn tiny_axes_still_get_a_break() {
        let breaks = choose_breaks(&scale(0.0, 10.0), 5.0, Orientation::Vertical, None);
        assert!(!breaks.values.is_empty());
    }
}
