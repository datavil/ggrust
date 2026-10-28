//! Axis labels with a theme `angle`, `hjust` or `vjust`: port of lets-plot's
//! `HorizontalRotatedLabelsLayout` (bottom axis) and `VerticalRotatedLabelsLayout`
//! (left axis).
//!
//! Each label's bounding box is rotated (`BreakLabelsLayoutUtil.rotatedLabelBounds`)
//! and justified inside a band as thick as the largest box; the label is then drawn
//! from an anchor point with text alignments chosen so that, for example,
//! `angle=45` labels end at their tick.
//!
//! What it does not do: top or right axes (ggrust draws none yet).

use crate::scene::{HorizontalAlign, VerticalAlign};
use crate::theme::TextStyle;

/// Below this, a sine or cosine counts as zero (lets-plot `1e-6`).
const EPSILON: f64 = 1e-6;

/// Where and how one rotated label is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Anchor offset from the label's reference point: (tick x, top of the label
    /// band) on the x axis, (right edge of the label band, tick y) on the y axis.
    pub offset: (f64, f64),
    /// Horizontal text alignment at the anchor.
    pub horizontal: HorizontalAlign,
    /// Vertical text alignment at the anchor.
    pub vertical: VerticalAlign,
    /// Counter-clockwise angle in degrees.
    pub angle: f64,
}

/// Whether labels drawn with `style` take the rotated layout (lets-plot
/// `DefaultAxisTheme.rotateLabels`: any of angle, hjust, vjust is given).
pub fn rotates(style: TextStyle) -> bool {
    style.angle.is_finite() || style.hjust.is_finite() || style.vjust.is_finite()
}

/// The theme angle, 0 when unset.
pub fn angle(style: TextStyle) -> f64 {
    if style.angle.is_finite() {
        style.angle
    } else {
        0.0
    }
}

/// Size of the box around a label of `size` (width, height) turned by `degrees`
/// (lets-plot `rotatedLabelBounds`, kept as is: not the exact bounding box).
pub fn rotated_size((width, height): (f64, f64), degrees: f64) -> (f64, f64) {
    let (sin, cos) = degrees.to_radians().sin_cos();
    (
        ((width * cos).powi(2) + (height * sin).powi(2)).sqrt(),
        ((height * cos).powi(2) + (width * sin).powi(2)).sqrt(),
    )
}

struct Angle {
    sin: f64,
    is_vertical: bool,
    is_upside_down: bool,
    is_horizontal: bool,
}

impl Angle {
    fn new(degrees: f64) -> Self {
        let (sin, cos) = degrees.to_radians().sin_cos();
        let is_upside_down = cos < 0.0;
        Angle {
            sin,
            is_vertical: cos.abs() < EPSILON,
            is_upside_down,
            is_horizontal: sin.abs() < EPSILON && !is_upside_down,
        }
    }
}

fn given(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

/// Labels of a bottom axis, `sizes` unrotated (width, height). Returns one
/// placement per label and the band height.
pub fn bottom(sizes: &[(f64, f64)], style: TextStyle) -> (Vec<Placement>, f64) {
    let degrees = angle(style);
    let boxes: Vec<(f64, f64)> = sizes
        .iter()
        .map(|size| rotated_size(*size, degrees))
        .collect();
    let max_height = boxes.iter().map(|(_, height)| *height).fold(0.0, f64::max);
    let a = Angle::new(degrees);
    let from_tick = a.sin < 0.0;
    let vjust = given(style.vjust).unwrap_or(1.0);
    let hjust = given(style.hjust).unwrap_or(if a.is_horizontal || a.is_vertical {
        0.5
    } else if from_tick {
        0.0
    } else {
        1.0
    });
    let horizontal = if a.is_vertical {
        match (vjust, a.sin > 0.0) {
            (1.0, true) | (0.0, false) => HorizontalAlign::End,
            (0.0, true) | (1.0, false) => HorizontalAlign::Start,
            _ => HorizontalAlign::Middle,
        }
    } else if a.is_upside_down {
        HorizontalAlign::Middle
    } else if hjust == 0.0 && (a.is_horizontal || from_tick) {
        HorizontalAlign::Start
    } else if hjust == 1.0 && (a.is_horizontal || !from_tick) {
        HorizontalAlign::End
    } else {
        HorizontalAlign::Middle
    };
    let is_corner = !a.is_horizontal && horizontal != HorizontalAlign::Middle;
    let vertical = if a.is_vertical {
        match (hjust, a.sin > 0.0) {
            (0.0, true) | (1.0, false) => VerticalAlign::Top,
            (1.0, true) | (0.0, false) => VerticalAlign::Bottom,
            _ => VerticalAlign::Middle,
        }
    } else if a.is_horizontal && vjust == 0.0 {
        VerticalAlign::Bottom
    } else if (a.is_horizontal && vjust == 1.0) || is_corner {
        VerticalAlign::Top
    } else {
        VerticalAlign::Middle
    };
    let box_y = |height: f64| max_height / 2.0 + (max_height - height) * (0.5 - vjust);
    let coefficient = match horizontal {
        HorizontalAlign::End if from_tick => 1.0,
        HorizontalAlign::Start if !from_tick => 1.0,
        HorizontalAlign::Middle => 0.5,
        _ => 0.0,
    };
    let placements = boxes
        .iter()
        .map(|&(width, height)| {
            let y = if a.is_vertical && horizontal != HorizontalAlign::Middle {
                max_height * coefficient
            } else if is_corner {
                (max_height - height) * (1.0 - vjust)
            } else if a.is_horizontal && vertical == VerticalAlign::Top {
                box_y(height) - height / 2.0
            } else if a.is_horizontal && vertical == VerticalAlign::Bottom {
                box_y(height) + height / 2.0
            } else {
                box_y(height)
            };
            let x = if a.is_vertical || horizontal != HorizontalAlign::Middle {
                0.0
            } else {
                width * (0.5 - hjust)
            };
            Placement {
                offset: (x, y),
                horizontal,
                vertical,
                angle: degrees,
            }
        })
        .collect();
    (placements, max_height)
}

/// Labels of a left axis, `sizes` unrotated (width, height). Returns one
/// placement per label and the band width.
pub fn left(sizes: &[(f64, f64)], style: TextStyle) -> (Vec<Placement>, f64) {
    let degrees = angle(style);
    let boxes: Vec<(f64, f64)> = sizes
        .iter()
        .map(|size| rotated_size(*size, degrees))
        .collect();
    let max_width = boxes.iter().map(|(width, _)| *width).fold(0.0, f64::max);
    let a = Angle::new(degrees);
    let from_tick = a.sin > 0.0;
    // Flush against the axis: the left axis's labels end at hjust 1.
    let flush = 1.0;
    let hjust = given(style.hjust).unwrap_or(flush);
    let vjust = given(style.vjust).unwrap_or(if a.is_horizontal || a.is_vertical {
        0.5
    } else if from_tick {
        0.0
    } else {
        1.0
    });
    let vertical = if a.is_vertical || a.is_upside_down {
        VerticalAlign::Middle
    } else if a.is_horizontal && vjust == 0.0 {
        VerticalAlign::Top
    } else if (a.is_horizontal && vjust == 1.0) || (vjust == 0.0 && from_tick) {
        VerticalAlign::Bottom
    } else if vjust == 1.0 && !from_tick {
        VerticalAlign::Top
    } else {
        VerticalAlign::Middle
    };
    let is_corner = !a.is_horizontal && vertical != VerticalAlign::Middle;
    let horizontal = if a.is_vertical {
        match (vjust, a.sin > 0.0) {
            (1.0, true) | (0.0, false) => HorizontalAlign::Start,
            (0.0, true) | (1.0, false) => HorizontalAlign::End,
            _ => HorizontalAlign::Middle,
        }
    } else if a.is_horizontal && hjust == 0.0 {
        HorizontalAlign::Start
    } else if (a.is_horizontal && hjust == 1.0) || is_corner {
        HorizontalAlign::End
    } else {
        HorizontalAlign::Middle
    };
    let slot_centre = -max_width / 2.0;
    let placements = boxes
        .iter()
        .map(|&(width, height)| {
            let x = if is_corner {
                (max_width - width) * (hjust - flush)
            } else if a.is_horizontal && horizontal == HorizontalAlign::Start {
                slot_centre - max_width / 2.0
            } else if a.is_horizontal && horizontal == HorizontalAlign::End {
                slot_centre + max_width / 2.0
            } else {
                slot_centre + (max_width - width) * (hjust - 0.5)
            };
            let y = if a.is_vertical || vertical != VerticalAlign::Middle {
                0.0
            } else {
                height * (0.5 - vjust)
            };
            Placement {
                offset: (x, y),
                horizontal,
                vertical,
                angle: degrees,
            }
        })
        .collect();
    (placements, max_width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Color;

    fn style(angle: f64, hjust: f64, vjust: f64) -> TextStyle {
        TextStyle {
            size: 13.0,
            color: Color::BLACK,
            font: Default::default(),
            hjust,
            vjust,
            angle,
        }
    }

    #[test]
    fn forty_five_degree_labels_end_at_the_tick() {
        let (placements, height) = bottom(
            &[(40.0, 13.0), (20.0, 13.0)],
            style(45.0, f64::NAN, f64::NAN),
        );
        let first = placements[0];
        assert_eq!(
            (first.horizontal, first.vertical),
            (HorizontalAlign::End, VerticalAlign::Top)
        );
        assert_eq!(first.offset, (0.0, 0.0));
        // The shorter label hangs from the top of the band as well.
        assert_eq!(placements[1].offset, (0.0, 0.0));
        assert!((height - rotated_size((40.0, 13.0), 45.0).1).abs() < 1e-12);
    }

    #[test]
    fn ninety_degree_labels_with_vjust_centre_in_the_band() {
        // lets-plot: hjust picks the text's vertical anchor, vjust 0.5 centres the
        // text along the band (labels do not end at the tick).
        let (placements, height) = bottom(&[(40.0, 13.0)], style(90.0, 1.0, 0.5));
        let label = placements[0];
        assert_eq!(
            (label.horizontal, label.vertical),
            (HorizontalAlign::Middle, VerticalAlign::Bottom)
        );
        assert!((label.offset.1 - 20.0).abs() < 1e-9);
        assert!((height - 40.0).abs() < 1e-9);
    }

    #[test]
    fn unrotated_hjust_moves_labels_off_centre() {
        let (placements, _) = bottom(&[(40.0, 13.0)], style(f64::NAN, 0.0, f64::NAN));
        assert_eq!(placements[0].horizontal, HorizontalAlign::Start);
        let (placements, width) = left(
            &[(40.0, 13.0), (10.0, 13.0)],
            style(f64::NAN, 0.0, f64::NAN),
        );
        assert_eq!(width, 40.0);
        assert_eq!(placements[1].horizontal, HorizontalAlign::Start);
        assert_eq!(placements[1].offset.0, -40.0);
    }
}
