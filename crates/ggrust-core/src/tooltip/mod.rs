//! Hover tooltips for HTML output (lets-plot `GeomInteraction`, `TooltipConfig`,
//! `LayerTargetLocator`).
//!
//! What it does: per layer and panel, the hover targets in pixel space (from the
//! geom, see [`crate::geom::Geom::hover_targets`]) and the text of every row's
//! tooltip: the general tooltip (default lines or `layer_tooltips()`), the axis
//! tooltips and the color marker. All text is formatted here; the JavaScript
//! runtime only looks targets up and shows the text ([`Interaction`] is its
//! payload).
//!
//! What it does not do: lets-plot's separate side tooltips (boxplot statistics
//! are lines of the general tooltip instead, as `disable_splitting()`), tooltip
//! theme elements, `tooltip_group`, crosshair modes, datetime formats.

pub mod content;
pub mod template;

use serde::Serialize;

pub use content::{LayerContent, TextColumn, layer_content};

use crate::scene::Rect;
use crate::spec::Aesthetic;

/// Data variables a `layer_tooltips()` reads (`variables`, `@name` fields of the
/// lines and title), without stat variables (`@..count..`).
pub fn referenced_variables(lines: &crate::spec::TooltipLines) -> Vec<String> {
    let mut names: Vec<String> = lines.variables.iter().flatten().cloned().collect();
    let templates = lines.lines.iter().flatten().chain(lines.title.iter());
    for template in templates {
        for field in template::Line::parse(template).fields() {
            if let template::Field::Variable(name) = field
                && crate::data::Computed::from_mapping(name).is_none()
                && !names.contains(name)
            {
                names.push(name.clone());
            }
        }
    }
    names
}

/// How the runtime finds the targets under the cursor (lets-plot `LookupSpec`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Lookup {
    /// The closest target within its reach (points, outlines).
    Nearest,
    /// Every target containing the cursor (tiles, boxes, polygons).
    Hover,
    /// Every target whose x range contains the cursor's x (bars, lines).
    HoverX,
}

/// What a geom's tooltips show and how its targets are found
/// (lets-plot `DefaultTooltipBehaviorFactory` + `GeomInteractionUtil`).
#[derive(Debug, Clone, Copy)]
pub struct TooltipBehavior {
    /// `None`: the geom has no tooltips.
    pub lookup: Option<Lookup>,
    /// Axis tooltips for `x` / `y` (data axes; `coord_flip` swaps the screen side).
    pub axis: (bool, bool),
    /// Aesthetics never shown in the default tooltip.
    pub hidden: &'static [Aesthetic],
    /// Statistics lets-plot shows as side tooltips (boxplot); here, the first
    /// lines of the general tooltip.
    pub side: &'static [Aesthetic],
    /// Tooltips only when `layer_tooltips()` gives lines (`geom_text`).
    pub needs_lines: bool,
    /// The color marker always shows the stroke color (lines, paths).
    pub marker_is_stroke: bool,
}

impl TooltipBehavior {
    /// No tooltips.
    pub const NONE: TooltipBehavior = TooltipBehavior {
        lookup: None,
        axis: (false, false),
        hidden: &[],
        side: &[],
        needs_lines: false,
        marker_is_stroke: false,
    };

    /// lets-plot `bivariateFunction(area = false)`: nearest target, both axis tooltips.
    pub const NEAREST_XY: TooltipBehavior = TooltipBehavior {
        lookup: Some(Lookup::Nearest),
        axis: (true, true),
        ..TooltipBehavior::NONE
    };

    /// lets-plot `bivariateFunction(area = true)`: targets under the cursor, no
    /// axis tooltips.
    pub const AREA: TooltipBehavior = TooltipBehavior {
        lookup: Some(Lookup::Hover),
        ..TooltipBehavior::NONE
    };

    /// lets-plot `xUnivariateFunction(HOVER)`: targets along the cursor's x, x
    /// axis tooltip.
    pub const ALONG_X: TooltipBehavior = TooltipBehavior {
        lookup: Some(Lookup::HoverX),
        axis: (true, false),
        ..TooltipBehavior::NONE
    };
}

/// Hover targets of one layer in one panel, in scene pixels. `rows` index the
/// layer's rows (and so its [`LayerContent`]).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HoverTargets {
    /// Circles: centre and radius.
    Points {
        /// Centre x.
        #[serde(serialize_with = "rounded")]
        x: Vec<f64>,
        /// Centre y.
        #[serde(serialize_with = "rounded")]
        y: Vec<f64>,
        /// Radius (the marker's).
        #[serde(serialize_with = "rounded")]
        radius: Vec<f64>,
        /// Layer row of each point.
        rows: Vec<u32>,
    },
    /// Rectangles (bars, tiles, boxes).
    Rects {
        /// Left.
        #[serde(serialize_with = "rounded")]
        x: Vec<f64>,
        /// Top.
        #[serde(serialize_with = "rounded")]
        y: Vec<f64>,
        /// Width.
        #[serde(serialize_with = "rounded")]
        width: Vec<f64>,
        /// Height.
        #[serde(serialize_with = "rounded")]
        height: Vec<f64>,
        /// Layer row of each rectangle.
        rows: Vec<u32>,
    },
    /// Vertices of lines and outlines (violins, ridges, paths).
    Vertices {
        /// x.
        #[serde(serialize_with = "rounded")]
        x: Vec<f64>,
        /// y.
        #[serde(serialize_with = "rounded")]
        y: Vec<f64>,
        /// Which line each vertex belongs to (along-x lookups take the nearest
        /// vertex of every line).
        line: Vec<u32>,
        /// Layer row of each vertex.
        rows: Vec<u32>,
    },
    /// Closed polygons: flat `[x0, y0, x1, y1, ...]` rings.
    Polygons {
        /// One ring per polygon.
        #[serde(serialize_with = "rounded_rings")]
        rings: Vec<Vec<f64>>,
        /// Layer row of each polygon.
        rows: Vec<u32>,
    },
}

/// Pixels to 0.01 px: finer than any screen, and a much smaller payload.
fn round_pixel(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn rounded<S: serde::Serializer>(values: &[f64], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(values.iter().map(|value| round_pixel(*value)))
}

fn rounded_rings<S: serde::Serializer>(
    rings: &[Vec<f64>],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(rings.iter().map(|ring| {
        ring.iter()
            .map(|value| round_pixel(*value))
            .collect::<Vec<_>>()
    }))
}

impl HoverTargets {
    /// Whether there is no target.
    pub fn is_empty(&self) -> bool {
        match self {
            HoverTargets::Points { rows, .. }
            | HoverTargets::Rects { rows, .. }
            | HoverTargets::Vertices { rows, .. }
            | HoverTargets::Polygons { rows, .. } => rows.is_empty(),
        }
    }
}

/// Tooltips of a whole figure: the runtime's payload.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Interaction {
    /// One entry per layer and panel that has targets and something to show.
    pub layers: Vec<LayerInteraction>,
    /// The zoom/pan payload when the figure has `ggtb()`; written apart by the
    /// HTML renderer (it holds scene templates).
    #[serde(skip)]
    pub zoom: Option<crate::layout::zoom::Zoom>,
}

/// One layer in one panel.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LayerInteraction {
    /// The panel, `[x, y, width, height]` in scene pixels: axis tooltips sit on
    /// its edges, anchored tooltips in its corners.
    pub panel: [f64; 4],
    /// How targets are found.
    pub lookup: Lookup,
    /// The targets.
    pub targets: HoverTargets,
    /// Text per layer row.
    #[serde(flatten)]
    pub content: LayerContent,
    /// `layer_tooltips().anchor(...)`, e.g. `top_right`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
    /// `layer_tooltips().min_width(...)` in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_width: Option<f64>,
}

impl LayerInteraction {
    /// `[x, y, width, height]` of a rectangle.
    pub fn panel_of(rect: Rect) -> [f64; 4] {
        [rect.x, rect.y, rect.width, rect.height]
    }
}

impl Interaction {
    /// The payload as JSON that is safe inside an HTML `<script>` element: `<`,
    /// `>` and `&` are written as `<`-style escapes.
    pub fn to_script_json(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_else(|_| "{\"layers\":[]}".into());
        script_safe_json(&json)
    }
}

/// JSON text made safe inside an HTML `<script>` element: `<`, `>`, `&` and the
/// JavaScript line separators are written as `\u` escapes.
pub fn script_safe_json(json: &str) -> String {
    let mut escaped = String::with_capacity(json.len());
    for character in json.chars() {
        match character {
            '<' => escaped.push_str("\\u003c"),
            '>' => escaped.push_str("\\u003e"),
            '&' => escaped.push_str("\\u0026"),
            '\u{2028}' => escaped.push_str("\\u2028"),
            '\u{2029}' => escaped.push_str("\\u2029"),
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_json_cannot_close_the_script() {
        let mut content = LayerContent::default();
        content.lines.push(TextColumn {
            label: Some("</script><!--".into()),
            values: vec![Some("a & b > c".into())],
        });
        let interaction = Interaction {
            layers: vec![LayerInteraction {
                panel: [0.0, 0.0, 10.0, 10.0],
                lookup: Lookup::Nearest,
                targets: HoverTargets::Points {
                    x: vec![1.0],
                    y: vec![2.0],
                    radius: vec![3.0],
                    rows: vec![0],
                },
                content,
                anchor: None,
                min_width: None,
            }],
            zoom: None,
        };
        let json = interaction.to_script_json();
        assert!(!json.contains('<') && !json.contains('>') && !json.contains('&'));
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap_or_default();
        assert_eq!(parsed["layers"][0]["targets"]["kind"], "points");
        assert_eq!(parsed["layers"][0]["lines"][0]["codes"][0], 0);
        assert_eq!(parsed["layers"][0]["lines"][0]["label"], "</script><!--");
        assert_eq!(parsed["layers"][0]["lines"][0]["values"][0], "a & b > c");
    }
}
