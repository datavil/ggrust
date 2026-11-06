//! Leader-line labels for an axis's breaks (`guide_axis_mark()`, plan 9.9), the
//! layout of ComplexHeatmap's `anno_mark` expressed as an axis guide.
//!
//! What it does: spreads the labels of an axis's breaks along the axis so they do
//! not overlap (as close to their breaks as room allows, or in equal slots), and
//! draws a link from each break's true position to its label: straight out for a
//! third of the link, slanted over the middle third, straight into the label.
//!
//! What it does not do: pick the breaks (the scale's `breaks=` chooses the subset)
//! or spread again while zoomed in HTML (zoomed labels sit at their breaks, without
//! links; resetting the zoom restores the spread layout).

use super::side::{AxisEdge, LabelPlacer};
use super::{AxisBreaks, label_sizes, x_tick_band, y_tick_band};
use crate::scene::{Node, Path};
use crate::spec::Options;
use crate::theme::{Dash, LineStyle, Theme};
use crate::{Error, Result};

/// How the labels are spread along the axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Spread {
    /// Each label at its break when there is room, pushed apart only as far as
    /// needed (ComplexHeatmap `anno_mark`).
    #[default]
    Nearest,
    /// Equal slots over the whole axis, in break order.
    Even,
}

/// Options of a `guide_axis_mark()`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisMark {
    /// How the labels are spread.
    pub spread: Spread,
    /// Pixels across the axis the link takes past the ticks.
    pub link_width: f64,
    /// Pixels kept between neighbouring labels.
    pub padding: f64,
}

/// `anno_mark`'s `link_width` (5 mm) at 96 dpi, rounded.
pub const DEFAULT_LINK_WIDTH: f64 = 20.0;
/// `anno_mark`'s `padding` (1 mm) at 96 dpi, rounded.
pub const DEFAULT_PADDING: f64 = 4.0;
/// Link width when the theme hides the ticks (the default tick width).
const FALLBACK_LINK_LINE_WIDTH: f64 = 1.0;

impl Default for AxisMark {
    fn default() -> Self {
        AxisMark {
            spread: Spread::Nearest,
            link_width: DEFAULT_LINK_WIDTH,
            padding: DEFAULT_PADDING,
        }
    }
}

impl AxisMark {
    /// Options of `guide_axis_mark(...)` at `path` (the scale's `guide`).
    pub fn parse(options: &Options, path: &str) -> Result<AxisMark> {
        let mut mark = AxisMark::default();
        let at = |key: &str| format!("{path}.{key}");
        let pixels = |key: &str, value: &serde_json::Value| {
            value
                .as_f64()
                .filter(|pixels| pixels.is_finite() && *pixels >= 0.0)
                .ok_or_else(|| {
                    Error::spec(
                        at(key),
                        format!("{key} is a size in pixels (a number of 0 or more), got {value}"),
                    )
                })
        };
        for (key, value) in options {
            match key.as_str() {
                "spread" => {
                    mark.spread = match value.as_str() {
                        Some("nearest") => Spread::Nearest,
                        Some("even") => Spread::Even,
                        _ => {
                            return Err(Error::spec(
                                at(key),
                                format!("spread is 'nearest' or 'even', got {value}"),
                            ));
                        }
                    }
                }
                "link_width" => mark.link_width = pixels(key, value)?,
                "padding" => mark.padding = pixels(key, value)?,
                other => {
                    let known = ["spread", "link_width", "padding"];
                    return Err(Error::spec(
                        at(other),
                        format!(
                            "guide_axis_mark() has no option '{other}'; accepted: {}{}",
                            known.join(", "),
                            crate::suggest::did_you_mean(other, known)
                        ),
                    ));
                }
            }
        }
        Ok(mark)
    }
}

/// Label centres (pixels along the axis) for labels wanted at `desired`, each
/// `extents[i]` long along the axis, inside `range` (low, high), at least
/// `padding` apart. Labels keep the order of their breaks. When they need more
/// room than `range` has, they are centred on it and overflow both ends.
pub fn spread_labels(
    desired: &[f64],
    extents: &[f64],
    range: (f64, f64),
    padding: f64,
    spread: Spread,
) -> Vec<f64> {
    let count = desired.len().min(extents.len());
    let (low, high) = (range.0.min(range.1), range.0.max(range.1));
    // A degenerate panel has nowhere to spread to (and `clamp` panics on NaN).
    if !(range.0.is_finite() && range.1.is_finite()) || padding.is_nan() {
        return desired[..count].to_vec();
    }
    let mut order: Vec<usize> = (0..count).collect();
    order.sort_by(|a, b| desired[*a].total_cmp(&desired[*b]).then(a.cmp(b)));
    let slot = |index: usize| extents[index].max(0.0) + padding;
    let mut centres = vec![0.0; count];
    match spread {
        Spread::Even => {
            let step = (high - low) / count.max(1) as f64;
            for (rank, index) in order.iter().enumerate() {
                centres[*index] = low + (rank as f64 + 0.5) * step;
            }
        }
        Spread::Nearest => {
            // Clusters of touching labels, each placed where the sum of squared
            // shifts is least (the mean of its members' wanted starts), kept in
            // range, merged with the previous one while they overlap.
            struct Cluster {
                members: Vec<usize>,
                length: f64,
                /// Sum of each member's wanted start minus its offset in the cluster.
                wanted: f64,
                start: f64,
            }
            let place = |cluster: &mut Cluster| {
                let start = cluster.wanted / cluster.members.len() as f64;
                cluster.start = if cluster.length >= high - low {
                    low + (high - low - cluster.length) / 2.0
                } else {
                    start.clamp(low, high - cluster.length)
                };
            };
            let mut clusters: Vec<Cluster> = Vec::new();
            for &index in &order {
                let mut cluster = Cluster {
                    members: vec![index],
                    length: slot(index),
                    wanted: desired[index] - slot(index) / 2.0,
                    start: 0.0,
                };
                place(&mut cluster);
                while let Some(previous) = clusters.last() {
                    if previous.start + previous.length <= cluster.start + 1e-9 {
                        break;
                    }
                    let Some(mut previous) = clusters.pop() else {
                        break;
                    };
                    let offset = previous.length;
                    previous.wanted += cluster.wanted - offset * cluster.members.len() as f64;
                    previous.length += cluster.length;
                    previous.members.extend(cluster.members);
                    cluster = previous;
                    place(&mut cluster);
                }
                clusters.push(cluster);
            }
            for cluster in clusters {
                let mut start = cluster.start;
                for index in cluster.members {
                    centres[index] = start + slot(index) / 2.0;
                    start += slot(index);
                }
            }
        }
    }
    centres
}

/// The links and labels of `breaks` on `edge` for `mark`, or `None` when the
/// theme hides the labels (the axis then draws plain ticks). `pixel` maps a break
/// to a pixel along the axis; `span` is the panel along the axis.
pub(super) fn mark_nodes(
    edge: AxisEdge,
    breaks: &AxisBreaks,
    theme: &Theme,
    mark: &AxisMark,
    pixel: &dyn Fn(f64) -> f64,
    span: (f64, f64),
) -> Option<Vec<Node>> {
    let placer = LabelPlacer::new(edge, breaks, theme)?;
    let horizontal = edge.side.is_horizontal();
    let text = if horizontal {
        theme.axis_text_x
    } else {
        theme.axis_text_y
    }?;
    let extents: Vec<f64> = label_sizes(&breaks.labels, text)
        .iter()
        .map(|(_, height)| *height)
        .collect();
    let desired: Vec<f64> = breaks.values.iter().map(|value| pixel(*value)).collect();
    let centres = spread_labels(&desired, &extents, span, mark.padding, mark.spread);
    let ticks = if horizontal {
        x_tick_band(theme)
    } else {
        y_tick_band(theme)
    };
    let line = edge
        .major_tick(theme)
        .map(|(line, _)| line)
        .unwrap_or(LineStyle {
            color: text.color,
            width: FALLBACK_LINK_LINE_WIDTH,
            dash: Dash::default(),
        });
    let third = mark.link_width / 3.0;
    let mut nodes = Vec::with_capacity(2 * desired.len());
    for (index, (label, (&from, &to))) in breaks
        .labels
        .iter()
        .zip(desired.iter().zip(&centres))
        .enumerate()
    {
        let points = vec![
            edge.point(from, 0.0),
            edge.point(from, ticks + third),
            edge.point(to, ticks + 2.0 * third),
            edge.point(to, ticks + mark.link_width),
        ];
        nodes.push(Node::Path(Path {
            points,
            closed: false,
            fill: None,
            stroke: Some(line.stroke()),
        }));
        nodes.push(placer.node(label, index, to));
    }
    Some(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: &[f64], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len(), "{actual:?} vs {expected:?}");
        for (a, e) in actual.iter().zip(expected) {
            assert!((a - e).abs() < 1e-9, "{actual:?} vs {expected:?}");
        }
    }

    #[test]
    fn nearest_keeps_labels_with_room_at_their_breaks() {
        let centres = spread_labels(
            &[10.0, 50.0, 90.0],
            &[10.0; 3],
            (0.0, 100.0),
            2.0,
            Spread::Nearest,
        );
        close(&centres, &[10.0, 50.0, 90.0]);
    }

    #[test]
    fn nearest_pushes_crowded_labels_apart_around_their_mean() {
        // Two labels 12 px long (with padding) wanted 4 px apart: centred on 52.
        let centres = spread_labels(
            &[50.0, 54.0],
            &[10.0; 2],
            (0.0, 100.0),
            2.0,
            Spread::Nearest,
        );
        close(&centres, &[46.0, 58.0]);
    }

    #[test]
    fn nearest_keeps_order_and_input_positions() {
        // Unsorted input (a y axis grows down the screen); results map back.
        let centres = spread_labels(
            &[54.0, 50.0],
            &[10.0; 2],
            (0.0, 100.0),
            2.0,
            Spread::Nearest,
        );
        close(&centres, &[58.0, 46.0]);
    }

    #[test]
    fn nearest_stays_inside_the_range_and_merges_pushed_clusters() {
        // Three labels at the low end are pushed up from 0; the merged cluster
        // then reaches the fourth, which joins it.
        let centres = spread_labels(
            &[0.0, 1.0, 2.0, 30.0],
            &[8.0; 4],
            (0.0, 100.0),
            2.0,
            Spread::Nearest,
        );
        close(&centres, &[5.0, 15.0, 25.0, 35.0]);
        let centres = spread_labels(
            &[99.0, 100.0],
            &[8.0; 2],
            (0.0, 100.0),
            2.0,
            Spread::Nearest,
        );
        close(&centres, &[85.0, 95.0]);
    }

    #[test]
    fn nearest_centres_labels_that_do_not_fit() {
        let centres = spread_labels(
            &[0.0, 1.0, 2.0],
            &[40.0; 3],
            (0.0, 100.0),
            0.0,
            Spread::Nearest,
        );
        close(&centres, &[10.0, 50.0, 90.0]);
    }

    #[test]
    fn a_degenerate_range_keeps_the_wanted_positions() {
        let centres = spread_labels(
            &[1.0, 2.0],
            &[10.0; 2],
            (f64::NAN, 100.0),
            2.0,
            Spread::Nearest,
        );
        close(&centres, &[1.0, 2.0]);
    }

    #[test]
    fn even_takes_equal_slots_in_break_order() {
        let centres = spread_labels(
            &[3.0, 1.0, 2.0, 0.0],
            &[5.0; 4],
            (100.0, 0.0),
            1.0,
            Spread::Even,
        );
        close(&centres, &[87.5, 37.5, 62.5, 12.5]);
    }

    #[test]
    fn options_parse_with_paths_and_suggestions() {
        let options: Options =
            serde_json::from_str(r#"{"spread": "even", "link_width": 30, "padding": 0}"#).unwrap();
        let mark = AxisMark::parse(&options, "scales[0].guide").unwrap();
        assert_eq!(mark.spread, Spread::Even);
        assert_eq!(mark.link_width, 30.0);
        assert_eq!(mark.padding, 0.0);
        let options: Options = serde_json::from_str(r#"{"spred": "even"}"#).unwrap();
        let error = AxisMark::parse(&options, "scales[0].guide")
            .unwrap_err()
            .to_string();
        assert!(error.contains("scales[0].guide.spred"), "{error}");
        assert!(error.contains("did you mean 'spread'"), "{error}");
        let options: Options = serde_json::from_str(r#"{"link_width": -1}"#).unwrap();
        let error = AxisMark::parse(&options, "g").unwrap_err().to_string();
        assert!(error.contains("pixels"), "{error}");
        let options: Options = serde_json::from_str(r#"{"spread": "tight"}"#).unwrap();
        let error = AxisMark::parse(&options, "g").unwrap_err().to_string();
        assert!(error.contains("'nearest' or 'even'"), "{error}");
    }
}
