//! ggforce marks: `geom_mark_hull`, `geom_mark_ellipse` and `geom_mark_rect`
//! outline each group of points (a cluster on a UMAP), optionally with a callout.
//!
//! Follows ggforce 0.5 (MIT). Per group, in panel pixels: the concave hull, the
//! minimum enclosing ellipse or the bounding box of the points, grown by `expand`
//! with corners rounded by `radius` (see `mark_shape.rs`). A mapped `label` and/or
//! `description` gives the group a callout box outside every mark, with a
//! connector (see `mark_label.rs`). A boolean `filter` mapping keeps the rows to
//! outline; the other rows only keep callouts away. Sizes given in millimetres
//! (`expand`, `radius`, `label_buffer`, ...) are 96 px per inch, font sizes are points.
//!
//! What it does not do: `geom_mark_circle`, `x0`/`y0` anchors, `con_arrow`,
//! `subgroup` holes, tooltips.

use super::lines;
use super::mark_label::{
    self, Border, ConnectorType, LabelOptions, PIXELS_PER_MM, PIXELS_PER_POINT, TextStyle,
};
use super::mark_shape::{self, Point};
use super::point::POINT_UNIT_SIZE;
use super::{DrawLayer, Geom, PolygonGeom};
use crate::coord::Panel;
use crate::data::Column;
use crate::scene::{Color, Node, Path, Rect, Stroke};
use crate::spec::{Aesthetic, Value};
use crate::style::{LineType, StyleDefault, Styles};
use crate::text::{Font, FontFace, FontFamily};
use crate::{Error, Result};

/// ggforce defaults: black outline, no fill, alpha 0.3 (fill only), line width
/// 0.5 mm in ggplot2 units (1.42 px, so 0.65 x 2.2 px).
const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Color, StyleDefault::Color(Color::BLACK)),
    (Aesthetic::Fill, StyleDefault::Color(Color::TRANSPARENT)),
    (Aesthetic::Alpha, StyleDefault::Number(0.3)),
    (Aesthetic::Size, StyleDefault::Number(MARK_LINE_SIZE)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
    (Aesthetic::Label, StyleDefault::Text("")),
    (Aesthetic::Description, StyleDefault::Text("")),
];

/// ggplot2's 0.5 mm line (`0.5 x .pt` = 1.42 px) in ggrust size units.
const MARK_LINE_SIZE: f64 = 0.5 * 72.27 / 25.4 / POINT_UNIT_SIZE;

/// Which shape a mark draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkKind {
    /// Concave hull (`geom_mark_hull`).
    Hull,
    /// Minimum enclosing ellipse (`geom_mark_ellipse`).
    Ellipse,
    /// Bounding box (`geom_mark_rect`).
    Rect,
}

/// A ggforce mark geom.
pub struct MarkGeom {
    name: &'static str,
    kind: MarkKind,
    parameters: &'static [&'static str],
}

macro_rules! with_label_parameters {
    ($($extra:literal),*) => {
        &[
            "expand", "radius", $($extra,)*
            "label_margin", "label_width", "label_minwidth", "label_hjust",
            "label_fontsize", "label_family", "label_lineheight", "label_fontface",
            "label_fill", "label_colour", "label_buffer", "con_colour", "con_size",
            "con_type", "con_linetype", "con_border", "con_cap",
        ]
    };
}

/// `geom_mark_hull`.
pub static MARK_HULL_GEOM: MarkGeom = MarkGeom {
    name: "mark_hull",
    kind: MarkKind::Hull,
    parameters: with_label_parameters!("concavity"),
};

/// `geom_mark_ellipse`.
pub static MARK_ELLIPSE_GEOM: MarkGeom = MarkGeom {
    name: "mark_ellipse",
    kind: MarkKind::Ellipse,
    parameters: with_label_parameters!("n", "tol"),
};

/// `geom_mark_rect`.
pub static MARK_RECT_GEOM: MarkGeom = MarkGeom {
    name: "mark_rect",
    kind: MarkKind::Rect,
    parameters: with_label_parameters!(),
};

impl Geom for MarkGeom {
    fn name(&self) -> &'static str {
        self.name
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn optional_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Filter]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        self.parameters
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        draw_marks(self.kind, layer, panel)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        PolygonGeom.minimum_key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        PolygonGeom.draw_key(styles, cell)
    }
}

/// Shape options of a layer, in pixels.
struct ShapeOptions {
    expand: f64,
    radius: f64,
    concavity: f64,
    vertices: usize,
    tolerance: f64,
}

fn spec_error(layer: &DrawLayer, name: &str, message: String) -> Error {
    Error::spec(layer.parameters.path_of(name), message)
}

/// A non-negative length in millimetres, as pixels.
fn millimetres(layer: &DrawLayer, name: &str, default: f64) -> Result<f64> {
    let value = layer.parameters.number(name)?.unwrap_or(default);
    if value < 0.0 {
        return Err(spec_error(
            layer,
            name,
            format!("{name} must be a non-negative length in millimetres, got {value}"),
        ));
    }
    Ok(value * PIXELS_PER_MM)
}

fn shape_options(kind: MarkKind, layer: &DrawLayer) -> Result<ShapeOptions> {
    let expand = millimetres(layer, "expand", 5.0)?;
    let default_radius = match kind {
        MarkKind::Ellipse => expand / PIXELS_PER_MM,
        _ => 2.5,
    };
    let radius = millimetres(layer, "radius", default_radius)?;
    let concavity = match kind {
        MarkKind::Hull => {
            let value = layer.parameters.number("concavity")?.unwrap_or(2.0);
            if value <= 0.0 {
                return Err(spec_error(
                    layer,
                    "concavity",
                    format!(
                        "concavity must be a positive number (1 is very concave, larger \
                         values approach the convex hull), got {value}"
                    ),
                ));
            }
            value
        }
        _ => 2.0,
    };
    let (vertices, tolerance) = match kind {
        MarkKind::Ellipse => {
            let n = layer.parameters.number("n")?.unwrap_or(100.0);
            if n < 3.0 || n.fract() != 0.0 || n > 100_000.0 {
                return Err(spec_error(
                    layer,
                    "n",
                    format!("n must be a whole number of ellipse points from 3 to 100000, got {n}"),
                ));
            }
            let tolerance = layer.parameters.number("tol")?.unwrap_or(0.01);
            if tolerance <= 0.0 {
                return Err(spec_error(
                    layer,
                    "tol",
                    format!("tol must be a positive tolerance (0.01 by default), got {tolerance}"),
                ));
            }
            (n as usize, tolerance)
        }
        _ => (100, 0.01),
    };
    Ok(ShapeOptions {
        expand,
        radius,
        concavity,
        vertices,
        tolerance,
    })
}

/// One value, or the label's and the description's (ggforce recycles a vector
/// of two).
fn pair<'a>(layer: &'a DrawLayer, name: &str) -> Result<Option<[&'a Value; 2]>> {
    match layer.parameters.value(name) {
        None => Ok(None),
        Some(Value::Array(values)) => match values.as_slice() {
            [one] => Ok(Some([one, one])),
            [first, second] => Ok(Some([first, second])),
            _ => Err(spec_error(
                layer,
                name,
                format!(
                    "{name} takes one value, or two (the label's and the description's), \
                     got {} values",
                    values.len()
                ),
            )),
        },
        Some(value) => Ok(Some([value, value])),
    }
}

fn pair_numbers(layer: &DrawLayer, name: &str, default: [f64; 2]) -> Result<[f64; 2]> {
    let Some(values) = pair(layer, name)? else {
        return Ok(default);
    };
    let mut numbers = [0.0; 2];
    for (number, value) in numbers.iter_mut().zip(values) {
        *number = value
            .as_f64()
            .filter(|number| number.is_finite())
            .ok_or_else(|| {
                spec_error(
                    layer,
                    name,
                    format!("expected a finite number, got {value}"),
                )
            })?;
    }
    Ok(numbers)
}

fn pair_texts<'a>(layer: &'a DrawLayer, name: &str, default: [&'a str; 2]) -> Result<[&'a str; 2]> {
    let Some(values) = pair(layer, name)? else {
        return Ok(default);
    };
    let mut texts = default;
    for (text, value) in texts.iter_mut().zip(values) {
        *text = value
            .as_str()
            .ok_or_else(|| spec_error(layer, name, format!("expected a string, got {value}")))?;
    }
    Ok(texts)
}

fn parse_color(layer: &DrawLayer, name: &str, text: &str) -> Result<Color> {
    crate::style::GeomPalette::STANDARD
        .parse_color(text)
        .map_err(|message| spec_error(layer, name, message))
}

fn choice<'a>(
    layer: &DrawLayer,
    name: &str,
    accepted: &'a [&'a str],
    default: &'a str,
) -> Result<&'a str> {
    let Some(value) = layer.parameters.string(name)? else {
        return Ok(default);
    };
    accepted
        .iter()
        .copied()
        .find(|candidate| *candidate == value)
        .ok_or_else(|| {
            spec_error(
                layer,
                name,
                format!(
                    "unknown {name} '{value}'{}; accepted: {}",
                    crate::suggest::did_you_mean(value, accepted.iter().copied()),
                    accepted.join(", ")
                ),
            )
        })
}

fn label_options(layer: &DrawLayer) -> Result<LabelOptions> {
    let parameters = &layer.parameters;
    let margin = match parameters.value("label_margin") {
        None => [2.0 * PIXELS_PER_MM; 4],
        Some(Value::Array(values)) if values.len() == 4 => {
            let mut margin = [0.0; 4];
            for (side, value) in margin.iter_mut().zip(values) {
                let number = value
                    .as_f64()
                    .filter(|number| number.is_finite() && *number >= 0.0);
                *side = number.ok_or_else(|| {
                    spec_error(
                        layer,
                        "label_margin",
                        format!("expected non-negative millimetres, got {value}"),
                    )
                })? * PIXELS_PER_MM;
            }
            margin
        }
        Some(Value::Array(values)) => {
            return Err(spec_error(
                layer,
                "label_margin",
                format!(
                    "label_margin takes one length or four (top, right, bottom, left, in \
                     millimetres), got {} values",
                    values.len()
                ),
            ));
        }
        Some(_) => [millimetres(layer, "label_margin", 2.0)?; 4],
    };
    let width = match parameters.number("label_width")? {
        None => None,
        Some(_) => Some(millimetres(layer, "label_width", 0.0)?),
    };
    let minimum_width = millimetres(layer, "label_minwidth", 50.0)?;
    let hjust = pair_numbers(layer, "label_hjust", [0.0, 0.0])?;
    let sizes = pair_numbers(layer, "label_fontsize", [12.0, 12.0])?;
    if let Some(size) = sizes.iter().find(|size| **size <= 0.0) {
        return Err(spec_error(
            layer,
            "label_fontsize",
            format!("label_fontsize must be a positive size in points, got {size}"),
        ));
    }
    let heights = pair_numbers(layer, "label_lineheight", [1.0, 1.0])?;
    if let Some(height) = heights.iter().find(|height| **height < 0.0) {
        return Err(spec_error(
            layer,
            "label_lineheight",
            format!(
                "label_lineheight must be a non-negative multiple of the font size, got {height}"
            ),
        ));
    }
    let faces = pair_texts(layer, "label_fontface", ["bold", "plain"])?;
    let families = pair_texts(layer, "label_family", ["sans", "sans"])?;
    let colours = pair_texts(layer, "label_colour", ["black", "black"])?;
    let style = |index: usize| -> Result<TextStyle> {
        Ok(TextStyle {
            size: sizes[index] * PIXELS_PER_POINT,
            font: Font {
                family: FontFamily::from_name(families[index]).0,
                face: FontFace::from_name(faces[index]),
            },
            color: parse_color(layer, "label_colour", colours[index])?,
            line_height: heights[index].max(0.0),
        })
    };
    let fill = match parameters.value("label_fill") {
        None => Some(Color::WHITE),
        Some(Value::String(text)) => {
            Some(parse_color(layer, "label_fill", text)?).filter(|color| color.alpha > 0)
        }
        Some(value) => {
            return Err(spec_error(
                layer,
                "label_fill",
                format!("expected a color string, got {value}"),
            ));
        }
    };
    let connector = match choice(layer, "con_type", &["elbow", "straight", "none"], "elbow")? {
        "elbow" => ConnectorType::Elbow,
        "straight" => ConnectorType::Straight,
        _ => ConnectorType::None,
    };
    let border = match choice(layer, "con_border", &["one", "all", "none"], "one")? {
        "one" => Border::One,
        "all" => Border::All,
        _ => Border::None,
    };
    let connector_color = parameters.color("con_colour")?.unwrap_or(Color::BLACK);
    let connector_size = parameters.number("con_size")?.unwrap_or(MARK_LINE_SIZE);
    if connector_size < 0.0 {
        return Err(spec_error(
            layer,
            "con_size",
            format!("con_size must be a non-negative line width, got {connector_size}"),
        ));
    }
    let linetype = match parameters.value("con_linetype") {
        None => LineType(Vec::new()),
        Some(value) => {
            LineType::parse(value).map_err(|message| spec_error(layer, "con_linetype", message))?
        }
    };
    let stroke: Option<Stroke> = lines::stroke(
        connector_color,
        connector_size.max(0.0) * POINT_UNIT_SIZE,
        &linetype,
    );
    Ok(LabelOptions {
        margin,
        width,
        minimum_width,
        hjust,
        label: style(0)?,
        description: style(1)?,
        fill,
        buffer: millimetres(layer, "label_buffer", 10.0)?,
        connector,
        border,
        cap: millimetres(layer, "con_cap", 3.0)?,
        stroke,
    })
}

/// Rows kept by a `filter` mapping (missing values drop the row, as ggforce).
fn kept_rows(layer: &DrawLayer) -> Result<Option<Vec<bool>>> {
    let Some(column) = layer.data.get(Aesthetic::Filter) else {
        return Ok(None);
    };
    let Column::Discrete(column) = column else {
        return Err(Error::spec(
            layer.parameters.path_of("mapping.filter"),
            "filter must map a boolean column (True keeps the row in the mark), got a \
             numeric column; compare it first, e.g. a column `frame['x'] > 0`",
        ));
    };
    let mut kept = Vec::with_capacity(column.len());
    for row in 0..column.len() {
        match column.label(row) {
            Some("true") => kept.push(true),
            Some("false") | None => kept.push(false),
            Some(other) => {
                return Err(Error::spec(
                    layer.parameters.path_of("mapping.filter"),
                    format!(
                        "filter must map a boolean column (True keeps the row in the mark), \
                         got the value '{other}'"
                    ),
                ));
            }
        }
    }
    Ok(Some(kept))
}

/// A mapped text that is missing counts as no text (ggforce skips `NA` labels).
fn text_at<'a>(layer: &'a DrawLayer, aesthetic: Aesthetic, row: usize) -> Result<&'a str> {
    if let Some(Column::Discrete(column)) = layer.data.get(aesthetic)
        && column.label(row).is_none()
    {
        return Ok("");
    }
    if let Some(Column::Continuous(column)) = layer.data.get(aesthetic)
        && !column.get(row).is_some_and(f64::is_finite)
    {
        return Ok("");
    }
    Ok(layer.styles.texts(aesthetic)?.get_ref(row).as_str())
}

/// The outline of one group's points, before expansion.
fn group_outline(kind: MarkKind, points: &[Point], options: &ShapeOptions) -> Vec<Point> {
    match kind {
        MarkKind::Hull => mark_shape::concave_hull(points, options.concavity),
        MarkKind::Ellipse => {
            let ellipse = mark_shape::enclosing_ellipse(points, options.tolerance);
            mark_shape::ellipse_polygon(&ellipse, options.vertices)
        }
        MarkKind::Rect => {
            let (mut low, mut high) = (
                (f64::INFINITY, f64::INFINITY),
                (f64::NEG_INFINITY, f64::NEG_INFINITY),
            );
            for point in points {
                low = (low.0.min(point.0), low.1.min(point.1));
                high = (high.0.max(point.0), high.1.max(point.1));
            }
            mark_shape::unique_points(&[low, (low.0, high.1), high, (high.0, low.1)])
        }
    }
}

fn draw_marks(kind: MarkKind, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
    let (Some(x), Some(y)) = (
        layer.data.continuous(Aesthetic::X),
        layer.data.continuous(Aesthetic::Y),
    ) else {
        return Ok(Vec::new());
    };
    let shape = shape_options(kind, layer)?;
    let labels = label_options(layer)?;
    let kept = kept_rows(layer)?;
    let group_index = layer.data.group_index();
    let group_count = layer.data.group_count().max(1);
    let mut rows_by_group: Vec<Vec<usize>> = vec![Vec::new(); group_count];
    let mut ghosts: Vec<Point> = Vec::new();
    for row in 0..layer.data.row_count() {
        let (px, py) = panel.map(
            x.get(row).unwrap_or(f64::NAN),
            y.get(row).unwrap_or(f64::NAN),
        );
        if !(px.is_finite() && py.is_finite()) {
            continue;
        }
        if kept.as_ref().is_some_and(|kept| !kept[row]) {
            ghosts.push((px, py));
            continue;
        }
        let group = group_index.map_or(0, |groups| groups[row] as usize);
        if group >= rows_by_group.len() {
            rows_by_group.resize(group + 1, Vec::new());
        }
        rows_by_group[group].push(row);
    }
    let mut marks: Vec<(usize, Vec<Point>)> = Vec::new();
    for rows in &rows_by_group {
        let Some(&first) = rows.first() else { continue };
        let points: Vec<Point> = rows
            .iter()
            .map(|&row| {
                panel.map(
                    x.get(row).unwrap_or(f64::NAN),
                    y.get(row).unwrap_or(f64::NAN),
                )
            })
            .collect();
        let outline = group_outline(kind, &points, &shape);
        let expanded = mark_shape::expand_outline(&outline, shape.expand, shape.radius);
        if !expanded.is_empty() {
            marks.push((first, expanded));
        }
    }
    let mut nodes = Vec::with_capacity(marks.len());
    for (first, outline) in &marks {
        let alpha = layer.styles.numbers(Aesthetic::Alpha)?.get(*first);
        let fill = layer.styles.colors(Aesthetic::Fill)?.get(*first);
        // ggplot2 `fill_alpha`: a missing fill stays missing.
        let fill = (fill.alpha > 0).then(|| fill.with_opacity(alpha));
        nodes.push(Node::Path(Path {
            points: outline.clone(),
            closed: true,
            fill: fill.filter(|fill| fill.alpha > 0),
            stroke: lines::stroke_with_alpha(layer.styles, *first, false)?,
        }));
    }
    // Callouts, after every mark.
    let mut boxes = Vec::with_capacity(marks.len());
    for (first, _) in &marks {
        let label = text_at(layer, Aesthetic::Label, *first)?;
        let description = text_at(layer, Aesthetic::Description, *first)?;
        boxes.push(mark_label::measure_box(label, description, &labels));
    }
    if boxes.iter().all(Option::is_none) {
        return Ok(nodes);
    }
    let outlines: Vec<Vec<Point>> = marks.iter().map(|(_, outline)| outline.clone()).collect();
    let anchors: Vec<Point> = outlines
        .iter()
        .map(|outline| bounding_center(outline))
        .collect();
    let sizes: Vec<Option<(f64, f64)>> = boxes
        .iter()
        .map(|label_box| {
            label_box
                .as_ref()
                .map(|label_box| (label_box.width, label_box.height))
        })
        .collect();
    let centers = mark_label::place_labels(
        &sizes,
        &outlines,
        &anchors,
        &ghosts,
        panel.rect,
        labels.buffer,
    );
    let anchor_divisor = if kind == MarkKind::Rect { 3.0 } else { 2.0 };
    let mut connectors = Vec::new();
    for ((label_box, center), outline) in boxes.iter().zip(&centers).zip(&outlines) {
        let (Some(label_box), Some(center)) = (label_box, center) else {
            continue;
        };
        nodes.extend(mark_label::draw_box(label_box, *center, &labels));
        let points = mark_label::connector(
            *center,
            (label_box.width, label_box.height),
            outline,
            anchor_divisor,
            &labels,
        );
        connectors.extend(mark_label::draw_connector(points, &labels));
    }
    nodes.extend(connectors);
    Ok(nodes)
}

fn bounding_center(outline: &[Point]) -> Point {
    let (mut low, mut high) = (
        (f64::INFINITY, f64::INFINITY),
        (f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for point in outline {
        low = (low.0.min(point.0), low.1.min(point.1));
        high = (high.0.max(point.0), high.1.max(point.1));
    }
    ((low.0 + high.0) / 2.0, (low.1 + high.1) / 2.0)
}
