//! Nested (bracket) axis guide (`guide_axis_nested()`, plan 10.3), legendry's
//! `guide_axis_nested()` with one level of groups.
//!
//! What it does: past an axis's labels (or across the panel, `side="opposite"`),
//! draws one bracket (or box) per group spanning the group's breaks, with the
//! group's name beside it. Groups are given as levels (`{name: [levels]}`) or as
//! `key_range_manual(start, end, name)`; both resolve to a span of axis positions.
//! Measure and draw share [`extent`] and [`nodes`], so the room the layout keeps
//! is the room the drawing takes.
//!
//! What it does not do: split labels automatically (`key_range_auto`), stack
//! several levels, or draw legendry's `fence` type. In facets the opposite side
//! is dropped by the build (with a warning).

use std::sync::Arc;

use super::side::AxisEdge;
use super::{LINE_BREAK, label_node, label_sizes, rect_node};
use crate::scale::PositionScale;
use crate::scale::format::Label;
use crate::scene::{Group, HorizontalAlign, Node, Path, Rect, VerticalAlign};
use crate::spec::Options;
#[cfg(test)]
use crate::theme::{Dash, LineStyle};
use crate::theme::{TextStyle, Theme};
use crate::{Error, Result};

/// What marks a group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NestedType {
    /// A bracket beside the group's breaks, its name past it.
    #[default]
    Bracket,
    /// A box over the group's span holding its name.
    Box,
}

/// legendry's bracket shapes (`bracket_line()`, `bracket_square()`, `bracket_chevron()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BracketShape {
    /// A straight line in the middle of the bracket's room (legendry's default).
    #[default]
    Line,
    /// A bar with tips pointing back to the axis.
    Square,
    /// A V pointing to the name.
    Chevron,
}

impl BracketShape {
    /// The shape as legendry gives it: (along 0..1, across 0..1) with across 1
    /// nearest the axis.
    fn points(self) -> &'static [(f64, f64)] {
        match self {
            BracketShape::Line => &[(0.0, 0.5), (1.0, 0.5)],
            BracketShape::Square => &[(1.0, 1.0), (1.0, 0.0), (0.0, 0.0), (0.0, 1.0)],
            BracketShape::Chevron => &[(0.0, 1.0), (0.5, 0.0), (1.0, 1.0)],
        }
    }
}

/// Where the guide goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NestedSide {
    /// Past the axis's labels, on every side the axis is drawn.
    #[default]
    Axis,
    /// Across the panel from the axis (top for x, right for y by default).
    Opposite,
}

/// One end (or member) of a group: a category or a data value.
#[derive(Debug, Clone, PartialEq)]
pub enum KeyBound {
    /// A category of a discrete axis.
    Level(String),
    /// A data value (continuous axes, or numbers on a discrete axis).
    Value(f64),
}

/// One group of the key.
#[derive(Debug, Clone, PartialEq)]
pub struct NestedGroup {
    /// The name drawn beside the bracket (`\n` already turned into line breaks).
    pub name: String,
    /// The group's levels, or its start and end.
    pub bounds: Vec<KeyBound>,
    /// A `key_range_manual()` range: drawn only when both ends are on the axis.
    pub needs_all: bool,
}

/// Options of a `guide_axis_nested()`.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisNested {
    /// The groups, in key order.
    pub groups: Vec<NestedGroup>,
    /// Bracket or box.
    pub kind: NestedType,
    /// The bracket's shape.
    pub bracket: BracketShape,
    /// Past the labels or across the panel.
    pub side: NestedSide,
    /// Positions a range reaches past its first and last category (discrete axes).
    pub pad_discrete: f64,
    /// Name angle, degrees (0: horizontal).
    pub angle: f64,
}

/// legendry's `pad_discrete` default for brackets and boxes.
pub const DEFAULT_PAD_DISCRETE: f64 = 0.4;

impl AxisNested {
    /// Whether the guide sits across the panel on the secondary (top, right)
    /// side of an axis drawn on `sides` (only for [`NestedSide::Opposite`]).
    pub fn opposite_on_secondary(&self, sides: crate::scale::AxisSides) -> Option<bool> {
        (self.side == NestedSide::Opposite).then_some(sides.primary)
    }

    /// Options of `guide_axis_nested(...)` at `path` (the scale's `guide`).
    /// `discrete` is the axis kind: a continuous axis takes numbers only.
    pub fn parse(options: &Options, path: &str, discrete: bool) -> Result<AxisNested> {
        let at = |key: &str| format!("{path}.{key}");
        let mut nested = AxisNested {
            groups: Vec::new(),
            kind: NestedType::default(),
            bracket: BracketShape::default(),
            side: NestedSide::default(),
            pad_discrete: DEFAULT_PAD_DISCRETE,
            angle: 0.0,
        };
        let mut key_given = false;
        for (key, value) in options {
            if value.is_null() {
                continue;
            }
            let choice = |accepted: &[&str]| {
                value
                    .as_str()
                    .filter(|name| accepted.contains(name))
                    .ok_or_else(|| {
                        Error::spec(
                            at(key),
                            format!(
                                "{key} is one of {}, got {value}{}",
                                accepted
                                    .iter()
                                    .map(|name| format!("'{name}'"))
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                value.as_str().map_or(String::new(), |name| {
                                    crate::suggest::did_you_mean(name, accepted.iter().copied())
                                })
                            ),
                        )
                    })
            };
            match key.as_str() {
                "key" => {
                    nested.groups = parse_key(value, &at(key), discrete)?;
                    key_given = true;
                }
                "type" => {
                    nested.kind = match choice(&["bracket", "box"])? {
                        "box" => NestedType::Box,
                        _ => NestedType::Bracket,
                    }
                }
                "bracket" => {
                    nested.bracket = match choice(&["line", "square", "chevron"])? {
                        "square" => BracketShape::Square,
                        "chevron" => BracketShape::Chevron,
                        _ => BracketShape::Line,
                    }
                }
                "side" => {
                    nested.side = match choice(&["axis", "opposite"])? {
                        "opposite" => NestedSide::Opposite,
                        _ => NestedSide::Axis,
                    }
                }
                "pad_discrete" => {
                    nested.pad_discrete =
                        value
                            .as_f64()
                            .filter(|pad| pad.is_finite())
                            .ok_or_else(|| {
                                Error::spec(
                                    at(key),
                                    format!("pad_discrete is a number of positions, got {value}"),
                                )
                            })?
                }
                "angle" => {
                    nested.angle = value
                        .as_f64()
                        .filter(|angle| angle.is_finite())
                        .ok_or_else(|| {
                            Error::spec(at(key), format!("angle is degrees, got {value}"))
                        })?
                }
                other => {
                    let known = ["key", "type", "bracket", "side", "pad_discrete", "angle"];
                    return Err(Error::spec(
                        at(other),
                        format!(
                            "guide_axis_nested() has no option '{other}'; accepted: {}{}",
                            known.join(", "),
                            crate::suggest::did_you_mean(other, known)
                        ),
                    ));
                }
            }
        }
        if !key_given {
            return Err(Error::spec(
                at("key"),
                "guide_axis_nested() needs key=: a mapping {group: [levels]} or \
                 key_range_manual(start, end, name)",
            ));
        }
        Ok(nested)
    }

    /// The groups as spans of axis positions (transformed units) on `scale`.
    /// Groups without any member on the axis (or a range missing an end) are dropped.
    pub fn resolve(self: &Arc<Self>, scale: &PositionScale) -> ResolvedNested {
        let position = |bound: &KeyBound| -> Option<f64> {
            match (scale, bound) {
                (PositionScale::Discrete(discrete), KeyBound::Level(level)) => discrete
                    .effective_levels()
                    .iter()
                    .position(|known| *known == level)
                    .map(|index| index as f64),
                (PositionScale::Discrete(_), KeyBound::Value(value)) => Some(*value),
                (PositionScale::Continuous(continuous), KeyBound::Value(value)) => {
                    continuous.transform.apply(*value)
                }
                (PositionScale::Continuous(_), KeyBound::Level(_)) => None,
            }
            .filter(|position| position.is_finite())
        };
        let pad = if scale.is_discrete() {
            self.pad_discrete
        } else {
            0.0
        };
        let ranges = self
            .groups
            .iter()
            .filter_map(|group| {
                let positions: Vec<Option<f64>> = group.bounds.iter().map(position).collect();
                if group.needs_all && positions.iter().any(Option::is_none) {
                    return None;
                }
                let found = positions.into_iter().flatten();
                let (low, high) = found
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), at| {
                        (low.min(at), high.max(at))
                    });
                (low <= high).then(|| NestedRange {
                    low: low - pad,
                    high: high + pad,
                    label: Label::plain(group.name.clone()),
                })
            })
            .collect();
        ResolvedNested {
            guide: Arc::clone(self),
            ranges,
        }
    }
}

/// `key=` as the Python side writes it: a list of `{"name", "levels"}` (mapping
/// form) or `{"name", "start", "end"}` (`key_range_manual()`).
fn parse_key(value: &serde_json::Value, path: &str, discrete: bool) -> Result<Vec<NestedGroup>> {
    let shape = || {
        Error::spec(
            path,
            format!(
                "key is a list of groups ({{\"name\", \"levels\"}} or {{\"name\", \"start\", \
                 \"end\"}}), got {value}"
            ),
        )
    };
    let entries = value.as_array().ok_or_else(shape)?;
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let at = format!("{path}[{index}]");
            let entry = entry.as_object().ok_or_else(shape)?;
            let name = match entry.get("name") {
                None | Some(serde_json::Value::Null) => String::new(),
                Some(serde_json::Value::String(name)) => {
                    name.replace('\n', &LINE_BREAK.to_string())
                }
                Some(other) => other.to_string(),
            };
            let bound = |value: &serde_json::Value, at: &str| -> Result<KeyBound> {
                match value {
                    serde_json::Value::String(level) if discrete => {
                        Ok(KeyBound::Level(level.clone()))
                    }
                    serde_json::Value::String(level) => Err(Error::spec(
                        at,
                        format!(
                            "the axis is continuous, so group bounds are numbers; got the \
                             category '{level}' (map a discrete column, or give numbers)"
                        ),
                    )),
                    serde_json::Value::Number(number) => number
                        .as_f64()
                        .map(KeyBound::Value)
                        .ok_or_else(|| Error::spec(at, format!("expected a number, got {number}"))),
                    other => Err(Error::spec(
                        at,
                        format!("a group bound is a category or a number, got {other}"),
                    )),
                }
            };
            if let Some(levels) = entry.get("levels") {
                let levels = levels.as_array().ok_or_else(|| {
                    Error::spec(
                        format!("{at}.levels"),
                        format!("levels is a list of categories, got {levels}"),
                    )
                })?;
                let bounds = levels
                    .iter()
                    .enumerate()
                    .map(|(position, level)| bound(level, &format!("{at}.levels[{position}]")))
                    .collect::<Result<Vec<_>>>()?;
                return Ok(NestedGroup {
                    name,
                    bounds,
                    needs_all: false,
                });
            }
            let end_of = |key: &str| {
                entry
                    .get(key)
                    .ok_or_else(|| {
                        Error::spec(
                            format!("{at}.{key}"),
                            "a range group needs both 'start' and 'end' (or give 'levels')",
                        )
                    })
                    .and_then(|value| bound(value, &format!("{at}.{key}")))
            };
            Ok(NestedGroup {
                name,
                bounds: vec![end_of("start")?, end_of("end")?],
                needs_all: true,
            })
        })
        .collect()
}

/// One group as a span of axis positions.
#[derive(Debug, Clone, PartialEq)]
pub struct NestedRange {
    /// Lower end (transformed units, padding included).
    pub low: f64,
    /// Upper end.
    pub high: f64,
    /// The group's name.
    pub label: Label,
}

/// A guide resolved on its scale: what measuring and drawing need.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedNested {
    /// The options.
    pub guide: Arc<AxisNested>,
    /// The groups found on the axis.
    pub ranges: Vec<NestedRange>,
}

/// Size of each name across the axis (px) in `text`, turned by `angle`.
fn name_extents(nested: &ResolvedNested, text: TextStyle, horizontal: bool) -> Vec<f64> {
    let labels: Vec<Label> = nested
        .ranges
        .iter()
        .map(|range| range.label.clone())
        .collect();
    let (sin, cos) = nested.guide.angle.to_radians().sin_cos();
    label_sizes(&labels, text)
        .into_iter()
        .zip(&labels)
        .map(|((width, height), label)| {
            if label.text.is_empty() {
                return 0.0;
            }
            if horizontal {
                (width * sin).abs() + (height * cos).abs()
            } else {
                (width * cos).abs() + (height * sin).abs()
            }
        })
        .collect()
}

/// The axis text style and the gap kept before names, on a horizontal or vertical axis.
fn text_of(theme: &Theme, horizontal: bool) -> (Option<TextStyle>, f64) {
    if horizontal {
        (theme.axis_text_x, theme.axis_text_spacing.0)
    } else {
        (theme.axis_text_y, theme.axis_text_spacing.1)
    }
}

/// Room the guide takes across the axis (px), from the guide spacing to the
/// outer edge of the widest name. Nothing when there is no group to draw.
pub fn extent(nested: &ResolvedNested, theme: &Theme, horizontal: bool) -> f64 {
    if nested.ranges.is_empty() {
        return 0.0;
    }
    let (text, gap) = text_of(theme, horizontal);
    let names = text
        .map(|text| name_extents(nested, text, horizontal))
        .unwrap_or_default();
    let widest = names.iter().copied().fold(0.0, f64::max);
    let body = match nested.guide.kind {
        NestedType::Bracket => {
            theme.legendry_bracket_size + if widest > 0.0 { gap + widest } else { 0.0 }
        }
        NestedType::Box => widest + 2.0 * gap,
    };
    theme.legendry_guide_spacing + body
}

/// The guide's group (`nested-x` / `nested-y`) on `edge`, starting `offset` px
/// out from the edge. `pixel` maps an axis position to a pixel along the axis;
/// `span` is the panel along the axis (brackets are clipped to it; names are
/// not, the HTML runtime hides names whose group leaves it while zoomed).
pub(crate) fn nodes(
    nested: &ResolvedNested,
    edge: AxisEdge,
    offset: f64,
    theme: &Theme,
    pixel: &dyn Fn(f64) -> f64,
    span: (f64, f64),
) -> Option<Node> {
    if nested.ranges.is_empty() {
        return None;
    }
    let horizontal = edge.side.is_horizontal();
    let (text, gap) = text_of(theme, horizontal);
    let names = text
        .map(|text| name_extents(nested, text, horizontal))
        .unwrap_or_else(|| vec![0.0; nested.ranges.len()]);
    let widest = names.iter().copied().fold(0.0, f64::max);
    let start = offset + theme.legendry_guide_spacing;
    let guide = &nested.guide;
    let mut shapes = Vec::new();
    let mut labels = Vec::new();
    let size = theme.legendry_bracket_size;
    for (range, name_extent) in nested.ranges.iter().zip(&names) {
        let (from, to) = (pixel(range.low), pixel(range.high));
        if !(from.is_finite() && to.is_finite()) {
            continue;
        }
        // legendry's `oob = "squish"`: ends past the panel stop at its edge (a
        // 0.4 pad reaches past lets-plot's 0.2 discrete expansion). A group
        // wholly outside the panel is not drawn.
        let (low, high) = (span.0.min(span.1), span.0.max(span.1));
        if low.partial_cmp(&high).is_none() || from.max(to) < low || from.min(to) > high {
            continue;
        }
        let (from, to) = (from.clamp(low, high), to.clamp(low, high));
        let names_start = match guide.kind {
            NestedType::Bracket => {
                if let Some(line) = theme.legendry_bracket {
                    let points = guide
                        .bracket
                        .points()
                        .iter()
                        .map(|(along, depth)| {
                            edge.point(from + along * (to - from), start + (1.0 - depth) * size)
                        })
                        .collect();
                    shapes.push(Node::Path(Path {
                        points,
                        closed: false,
                        fill: None,
                        stroke: Some(line.stroke()),
                    }));
                }
                start + size + gap
            }
            NestedType::Box => {
                if let Some(style) = theme.legendry_box {
                    let (a, b) = (
                        edge.point(from, start),
                        edge.point(to, start + widest + 2.0 * gap),
                    );
                    shapes.push(rect_node(
                        Rect {
                            x: a.0.min(b.0),
                            y: a.1.min(b.1),
                            width: (a.0 - b.0).abs(),
                            height: (a.1 - b.1).abs(),
                        },
                        style,
                    ));
                }
                // Names centred in the box.
                start + gap + (widest - name_extent) / 2.0
            }
        };
        let Some(text) = text.filter(|_| !range.label.text.is_empty()) else {
            continue;
        };
        let centre = edge.point((from + to) / 2.0, names_start + name_extent / 2.0);
        let mut node = label_node(
            &range.label,
            text,
            centre,
            HorizontalAlign::Middle,
            guide.angle,
        );
        middle(&mut node);
        labels.push(Node::Group(Group {
            clip: None,
            role: None,
            anchor: Some(centre),
            key: None,
            nodes: vec![node],
        }));
    }
    let across = (
        start,
        start + extent(nested, theme, horizontal) - theme.legendry_guide_spacing,
    );
    let pad = theme
        .legendry_bracket
        .map_or(1.0, |line| line.width.max(1.0));
    let (a, b) = (
        edge.point(span.0, across.0 - pad),
        edge.point(span.1, across.1 + pad),
    );
    let clip = Rect {
        x: a.0.min(b.0),
        y: a.1.min(b.1),
        width: (a.0 - b.0).abs(),
        height: (a.1 - b.1).abs(),
    };
    let mut children = Vec::with_capacity(labels.len() + 1);
    if !shapes.is_empty() {
        // The inner group is what the HTML runtime scales along the axis; the
        // outer one keeps the clip in place.
        children.push(Node::Group(Group {
            clip: Some(clip),
            role: None,
            anchor: None,
            key: None,
            nodes: vec![Node::Group(Group {
                clip: None,
                role: None,
                anchor: None,
                key: None,
                nodes: shapes,
            })],
        }));
    }
    children.extend(labels);
    Some(Node::Group(Group {
        clip: None,
        role: Some(if horizontal { "nested-x" } else { "nested-y" }),
        anchor: None,
        key: None,
        nodes: children,
    }))
}

/// Centre every text of a label node vertically on its anchor.
fn middle(node: &mut Node) {
    match node {
        Node::Text(text) => text.vertical = VerticalAlign::Middle,
        Node::Group(group) => group.nodes.iter_mut().for_each(middle),
        _ => {}
    }
}

/// Default line when the theme draws brackets but nothing else is known (tests).
#[cfg(test)]
fn test_line() -> LineStyle {
    LineStyle {
        color: crate::theme::INK,
        width: 1.0,
        dash: Dash::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::side::Side;

    fn options(json: &str) -> Options {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn parses_both_key_forms_and_options() {
        let nested = AxisNested::parse(
            &options(
                r#"{"key": [{"name": "A\nB", "levels": ["a", "b"]},
                            {"name": "C", "start": "c", "end": "d"}],
                    "type": "box", "bracket": "square", "side": "opposite",
                    "pad_discrete": 0.3, "angle": 90}"#,
            ),
            "scales[0].guide",
            true,
        )
        .unwrap();
        assert_eq!(nested.groups.len(), 2);
        assert_eq!(nested.groups[0].name, format!("A{LINE_BREAK}B"));
        assert!(!nested.groups[0].needs_all);
        assert!(nested.groups[1].needs_all);
        assert_eq!(nested.kind, NestedType::Box);
        assert_eq!(nested.bracket, BracketShape::Square);
        assert_eq!(nested.side, NestedSide::Opposite);
        assert_eq!(nested.pad_discrete, 0.3);
        assert_eq!(nested.angle, 90.0);
    }

    #[test]
    fn errors_name_the_path_and_the_fix() {
        let error = |json: &str, discrete: bool| {
            AxisNested::parse(&options(json), "scales[0].guide", discrete)
                .unwrap_err()
                .to_string()
        };
        let message = error(r#"{"key": [], "typ": "box"}"#, true);
        assert!(message.contains("scales[0].guide.typ"), "{message}");
        assert!(message.contains("did you mean 'type'"), "{message}");
        let message = error(r#"{"key": [], "type": "fence"}"#, true);
        assert!(message.contains("'bracket', 'box'"), "{message}");
        let message = error(r#"{"type": "box"}"#, true);
        assert!(message.contains("needs key="), "{message}");
        let message = error(r#"{"key": [{"name": "A", "start": "a"}]}"#, true);
        assert!(message.contains("scales[0].guide.key[0].end"), "{message}");
        let message = error(r#"{"key": [{"name": "A", "levels": ["a"]}]}"#, false);
        assert!(message.contains("continuous"), "{message}");
    }

    fn discrete(levels: &[&str]) -> PositionScale {
        let mut scale = crate::scale::DiscretePositionScale::default();
        scale.set_limits(levels.iter().map(|level| level.to_string()).collect());
        PositionScale::Discrete(scale)
    }

    #[test]
    fn resolves_levels_to_padded_spans_and_drops_missing_groups() {
        let nested = Arc::new(
            AxisNested::parse(
                &options(
                    r#"{"key": [{"name": "AB", "levels": ["b", "a", "zz"]},
                                {"name": "CD", "start": "c", "end": "zz"},
                                {"name": "none", "levels": ["zz"]},
                                {"name": "D", "start": "d", "end": "d"}]}"#,
                ),
                "g",
                true,
            )
            .unwrap(),
        );
        let resolved = nested.resolve(&discrete(&["a", "b", "c", "d"]));
        let spans: Vec<(f64, f64, &str)> = resolved
            .ranges
            .iter()
            .map(|range| (range.low, range.high, range.label.text.as_str()))
            .collect();
        assert_eq!(spans, vec![(-0.4, 1.4, "AB"), (2.6, 3.4, "D")]);
    }

    #[test]
    fn extent_holds_bracket_gap_and_widest_name() {
        let nested = Arc::new(
            AxisNested::parse(
                &options(r#"{"key": [{"name": "Group", "levels": ["a"]}]}"#),
                "g",
                true,
            )
            .unwrap(),
        );
        let resolved = nested.resolve(&discrete(&["a"]));
        let theme = Theme {
            legendry_bracket: Some(test_line()),
            ..Theme::default()
        };
        let text = theme.axis_text_x.unwrap();
        let height = label_sizes(&[Label::plain("Group")], text)[0].1;
        let expected = theme.legendry_guide_spacing
            + theme.legendry_bracket_size
            + theme.axis_text_spacing.0
            + height;
        assert!((extent(&resolved, &theme, true) - expected).abs() < 1e-9);
        let empty = ResolvedNested {
            guide: nested,
            ranges: Vec::new(),
        };
        assert_eq!(extent(&empty, &theme, true), 0.0);
    }

    #[test]
    fn a_square_bracket_points_its_tips_to_the_axis() {
        let nested = Arc::new(
            AxisNested::parse(
                &options(r#"{"key": [{"name": "", "levels": ["a", "b"]}], "bracket": "square"}"#),
                "g",
                true,
            )
            .unwrap(),
        );
        let resolved = nested.resolve(&discrete(&["a", "b"]));
        let theme = Theme {
            legendry_bracket: Some(test_line()),
            legendry_guide_spacing: 0.0,
            ..Theme::default()
        };
        let edge = AxisEdge {
            side: Side::Bottom,
            base: 100.0,
        };
        let node = nodes(
            &resolved,
            edge,
            10.0,
            &theme,
            &|value| value * 10.0,
            (0.0, 50.0),
        )
        .unwrap();
        let mut paths = Vec::new();
        collect_paths(&node, &mut paths);
        let size = theme.legendry_bracket_size;
        assert_eq!(
            paths[0],
            // The low end (-4) is squished to the panel's edge (0).
            vec![
                (14.0, 110.0),
                (14.0, 110.0 + size),
                (0.0, 110.0 + size),
                (0.0, 110.0)
            ]
        );
    }

    fn collect_paths(node: &Node, paths: &mut Vec<Vec<(f64, f64)>>) {
        match node {
            Node::Path(path) => paths.push(path.points.clone()),
            Node::Group(group) => group
                .nodes
                .iter()
                .for_each(|node| collect_paths(node, paths)),
            _ => {}
        }
    }
}
