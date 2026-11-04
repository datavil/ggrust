//! Callouts of the ggforce marks: a label box (bold `label` over a wrapped plain
//! `description`), placed outside every mark and connected to its mark by a line.
//!
//! Follows ggforce 0.5 (MIT): `labelboxGrob` sizes (grid text heights: 0.718 em
//! a line, 1.2 x `lineheight` em between lines, a 0.229 em descent gap under a lone
//! label), `place_labels` (the label centre is the point nearest the mark's anchor
//! where the box, grown by 2 mm, stays in the panel, clear of every mark grown by
//! `label_buffer`, of filtered-out points and of earlier labels grown by 10 mm), and
//! the `straight` / `elbow` connectors with `con_border` and `con_cap`.
//! The placement search runs on a pixel grid of the panel (ggforce clips polygons).
//!
//! What it does not do: plotmath or markdown, `con_arrow`, `x0` / `y0` anchors.

use super::mark_shape::{Point, closest_on_outline, contains};
use crate::scene::{
    Color, HorizontalAlign, Node, Path, PerItem, Rect, Rects, Stroke, Text, VerticalAlign,
};
use crate::text::{self, Font};

/// Pixels per millimetre (96 px per inch).
pub(super) const PIXELS_PER_MM: f64 = 96.0 / 25.4;
/// Pixels per point (ggplot2 font sizes are points).
pub(super) const PIXELS_PER_POINT: f64 = 96.0 / 72.0;
/// grid's height of one text line, in font sizes.
const LINE_ASCENT: f64 = 0.718;
/// grid's distance between lines, in font sizes per unit of `lineheight`.
const LINE_SPACING: f64 = 1.2;
/// The gap under a label without description (grid font descent), in font sizes.
const DESCENT: f64 = 0.229;
/// Placed labels keep this far from later ones (ggforce: 10 mm).
const LABEL_GAP_MM: f64 = 10.0;
/// Label boxes grow by this before placement (ggforce: 2 mm).
const LABEL_PADDING_MM: f64 = 2.0;
/// Half the side of the box around a filtered-out point (ggforce: 2 mm).
const GHOST_MM: f64 = 2.0;
/// Placement grid cells (at most) per panel.
const PLACEMENT_CELLS: f64 = 400_000.0;

/// The look of one of the two texts.
#[derive(Debug, Clone)]
pub(super) struct TextStyle {
    /// Font size in pixels.
    pub(super) size: f64,
    pub(super) font: Font,
    pub(super) color: Color,
    pub(super) line_height: f64,
}

/// How connectors are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ConnectorType {
    Elbow,
    Straight,
    None,
}

/// Which label border the connector draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Border {
    /// The edge of the label the connector starts from.
    One,
    /// The whole box outline.
    All,
    None,
}

/// Callout options of a layer, in pixels.
#[derive(Debug, Clone)]
pub(super) struct LabelOptions {
    /// Top, right, bottom, left.
    pub(super) margin: [f64; 4],
    pub(super) width: Option<f64>,
    pub(super) minimum_width: f64,
    /// For the label and the description.
    pub(super) hjust: [f64; 2],
    pub(super) label: TextStyle,
    pub(super) description: TextStyle,
    pub(super) fill: Option<Color>,
    pub(super) buffer: f64,
    pub(super) connector: ConnectorType,
    pub(super) border: Border,
    pub(super) cap: f64,
    pub(super) stroke: Option<Stroke>,
}

/// A measured label box.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct LabelBox {
    label_lines: Vec<String>,
    description_lines: Vec<String>,
    /// Width of the text column (box width without the side margins).
    text_width: f64,
    label_height: f64,
    separator: f64,
    pub(super) width: f64,
    pub(super) height: f64,
}

fn text_width(line: &str, style: &TextStyle) -> f64 {
    text::measure(line, style.size, style.font).width
}

fn block_width(lines: &[String], style: &TextStyle) -> f64 {
    lines
        .iter()
        .map(|line| text_width(line, style))
        .fold(0.0, f64::max)
}

fn block_height(lines: &[String], style: &TextStyle) -> f64 {
    if lines.is_empty() {
        return 0.0;
    }
    style.size * (LINE_ASCENT + (lines.len() - 1) as f64 * LINE_SPACING * style.line_height)
}

/// Greedy word wrap to `width` pixels, keeping given line breaks (ggforce
/// `wrap_text`: a word longer than the width gets a line of its own).
fn wrap(text: &str, style: &TextStyle, width: f64) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split(' ').filter(|word| !word.is_empty()) {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && text_width(&candidate, style) > width {
                lines.push(std::mem::replace(&mut line, word.to_owned()));
            } else {
                line = candidate;
            }
        }
        lines.push(line);
    }
    lines
}

/// The box of a label and description (either may be empty); `None` when both
/// are empty (ggforce draws nothing then).
pub(super) fn measure_box(
    label: &str,
    description: &str,
    options: &LabelOptions,
) -> Option<LabelBox> {
    if label.is_empty() && description.is_empty() {
        return None;
    }
    let [top, right, bottom, left] = options.margin;
    let sides = right + left;
    let label_lines: Vec<String> = match (label.is_empty(), options.width) {
        (true, _) => Vec::new(),
        (false, Some(width)) => wrap(label, &options.label, width - sides),
        (false, None) => label.split('\n').map(str::to_owned).collect(),
    };
    let label_width = block_width(&label_lines, &options.label);
    // ggforce labelboxGrob, including its use of the label width with no
    // description (the minimum width only applies to wrapping descriptions).
    let mut final_width = match options.width {
        Some(width) => width.max(options.minimum_width) - sides,
        None if label_width > options.minimum_width - sides => label_width + sides,
        None => label_width.max(options.minimum_width) - sides,
    };
    let description_lines = if description.is_empty() {
        if options.width.is_none() {
            final_width = label_width;
        }
        Vec::new()
    } else {
        let lines = wrap(description, &options.description, final_width);
        if options.width.is_none() {
            let description_width = final_width.min(block_width(&lines, &options.description));
            final_width = label_width.max(description_width);
        }
        lines
    };
    // A word longer than the wrap width widens the box instead of overflowing it.
    final_width = final_width
        .max(block_width(&label_lines, &options.label))
        .max(block_width(&description_lines, &options.description));
    let label_height = block_height(&label_lines, &options.label);
    let description_height = block_height(&description_lines, &options.description);
    let separator = match (label_height > 0.0, description_height > 0.0) {
        (true, true) => top,
        (true, false) => DESCENT * options.label.size,
        _ => 0.0,
    };
    Some(LabelBox {
        label_lines,
        description_lines,
        text_width: final_width,
        label_height,
        separator,
        width: final_width + sides,
        height: top + bottom + label_height + separator + description_height,
    })
}

/// Squared Euclidean distance transform (Felzenszwalb and Huttenlocher) of one row
/// or column, in place.
fn distance_transform_1d(values: &mut [f64], scratch: &mut Vec<f64>, hull: &mut Vec<usize>) {
    let n = values.len();
    scratch.clear();
    scratch.extend_from_slice(values);
    let f = &*scratch;
    hull.clear();
    let mut boundaries: Vec<f64> = Vec::with_capacity(n + 1);
    for q in 0..n {
        if !f[q].is_finite() {
            continue;
        }
        while let Some(&p) = hull.last() {
            let s = ((f[q] + (q * q) as f64) - (f[p] + (p * p) as f64)) / (2.0 * (q - p) as f64);
            if boundaries.last().is_some_and(|last| s <= *last) {
                hull.pop();
                boundaries.pop();
            } else {
                boundaries.push(s);
                break;
            }
        }
        if hull.is_empty() {
            boundaries.clear();
        }
        hull.push(q);
    }
    if hull.is_empty() {
        return;
    }
    // boundaries[k] separates hull[k] and hull[k + 1].
    let mut k = 0;
    for (q, value) in values.iter_mut().enumerate() {
        while k + 1 < hull.len() && boundaries[k] < q as f64 {
            k += 1;
        }
        let p = hull[k];
        let d = q as f64 - p as f64;
        *value = d * d + f[p];
    }
}

/// The part of the segment `a`-`b` inside `bounds` (Liang-Barsky), so a huge mark
/// costs no more to rasterize than the panel.
fn clip_segment(a: Point, b: Point, bounds: Rect) -> Option<(Point, Point)> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut low, mut high) = (0.0f64, 1.0f64);
    for (p, q) in [
        (-dx, a.0 - bounds.x),
        (dx, bounds.x + bounds.width - a.0),
        (-dy, a.1 - bounds.y),
        (dy, bounds.y + bounds.height - a.1),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                low = low.max(t);
            } else {
                high = high.min(t);
            }
        }
    }
    (low <= high).then_some({
        (
            (a.0 + low * dx, a.1 + low * dy),
            (a.0 + high * dx, a.1 + high * dy),
        )
    })
}

/// The grid of label centres over a panel.
struct PlacementGrid {
    bounds: Rect,
    spacing: f64,
    columns: usize,
    rows: usize,
    /// Prefix sums of blocked cells, `(rows + 1) x (columns + 1)`.
    blocked_sums: Vec<u32>,
}

impl PlacementGrid {
    fn new(bounds: Rect, marks: &[Vec<Point>], ghosts: &[Point], buffer: f64) -> Self {
        let spacing = 1f64.max((bounds.width * bounds.height / PLACEMENT_CELLS).sqrt());
        let columns = ((bounds.width / spacing).ceil() as usize).max(1);
        let rows = ((bounds.height / spacing).ceil() as usize).max(1);
        let center = |column: usize, row: usize| {
            (
                bounds.x + (column as f64 + 0.5) * spacing,
                bounds.y + (row as f64 + 0.5) * spacing,
            )
        };
        let cell_of = |point: Point| -> Option<(usize, usize)> {
            let column = ((point.0 - bounds.x) / spacing).floor();
            let row = ((point.1 - bounds.y) / spacing).floor();
            (column >= 0.0 && row >= 0.0 && (column as usize) < columns && (row as usize) < rows)
                .then_some((column as usize, row as usize))
        };
        // Seeds: cells on or inside a mark.
        let mut field = vec![f64::INFINITY; columns * rows];
        for mark in marks {
            let n = mark.len();
            for index in 0..n {
                let Some((a, b)) = clip_segment(mark[index], mark[(index + 1) % n], bounds) else {
                    continue;
                };
                let length = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
                let steps = (2.0 * length / spacing).ceil() as usize + 1;
                for step in 0..=steps {
                    let t = step as f64 / steps as f64;
                    if let Some((column, row)) =
                        cell_of((a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1)))
                    {
                        field[row * columns + column] = 0.0;
                    }
                }
            }
            if n >= 3 {
                for row in 0..rows {
                    for column in 0..columns {
                        let cell = &mut field[row * columns + column];
                        if cell.is_infinite() && contains(mark, center(column, row)) {
                            *cell = 0.0;
                        }
                    }
                }
            }
        }
        // Distance (in cells) from every cell to the nearest mark cell.
        let (mut scratch, mut hull) = (Vec::new(), Vec::new());
        let mut column_values = vec![0.0; rows];
        for column in 0..columns {
            for row in 0..rows {
                column_values[row] = field[row * columns + column];
            }
            distance_transform_1d(&mut column_values, &mut scratch, &mut hull);
            for row in 0..rows {
                field[row * columns + column] = column_values[row];
            }
        }
        for row in field.chunks_mut(columns) {
            distance_transform_1d(row, &mut scratch, &mut hull);
        }
        // Blocked: within the buffer of a mark (a cell of slack), or near a ghost.
        let reach = (buffer + spacing) / spacing;
        let mut blocked: Vec<bool> = field.iter().map(|value| *value <= reach * reach).collect();
        let ghost = GHOST_MM * PIXELS_PER_MM + spacing / 2.0;
        for point in ghosts {
            let low = cell_of((point.0 - ghost, point.1 - ghost));
            let high = cell_of((point.0 + ghost, point.1 + ghost));
            let clamp = |value: f64, limit: usize| (value.max(0.0) as usize).min(limit - 1);
            let (column_low, row_low) = low.unwrap_or((
                clamp(((point.0 - ghost - bounds.x) / spacing).floor(), columns),
                clamp(((point.1 - ghost - bounds.y) / spacing).floor(), rows),
            ));
            let (column_high, row_high) = high.unwrap_or((
                clamp(((point.0 + ghost - bounds.x) / spacing).floor(), columns),
                clamp(((point.1 + ghost - bounds.y) / spacing).floor(), rows),
            ));
            for row in row_low..=row_high {
                for column in column_low..=column_high {
                    blocked[row * columns + column] = true;
                }
            }
        }
        let mut blocked_sums = vec![0u32; (rows + 1) * (columns + 1)];
        for row in 0..rows {
            for column in 0..columns {
                blocked_sums[(row + 1) * (columns + 1) + column + 1] =
                    u32::from(blocked[row * columns + column])
                        + blocked_sums[row * (columns + 1) + column + 1]
                        + blocked_sums[(row + 1) * (columns + 1) + column]
                        - blocked_sums[row * (columns + 1) + column];
            }
        }
        PlacementGrid {
            bounds,
            spacing,
            columns,
            rows,
            blocked_sums,
        }
    }

    /// Whether any blocked cell meets the box `center +/- half` (a cell of slack).
    fn hits_blocked(&self, center: Point, half: Point) -> bool {
        let index = |value: f64, limit: usize| (value.max(0.0) as usize).min(limit);
        let column_low = index(
            ((center.0 - half.0 - self.bounds.x) / self.spacing).floor() - 1.0,
            self.columns,
        );
        let column_high = index(
            ((center.0 + half.0 - self.bounds.x) / self.spacing).floor() + 2.0,
            self.columns,
        );
        let row_low = index(
            ((center.1 - half.1 - self.bounds.y) / self.spacing).floor() - 1.0,
            self.rows,
        );
        let row_high = index(
            ((center.1 + half.1 - self.bounds.y) / self.spacing).floor() + 2.0,
            self.rows,
        );
        let width = self.columns + 1;
        let sum = self.blocked_sums[row_high * width + column_high]
            + self.blocked_sums[row_low * width + column_low]
            - self.blocked_sums[row_low * width + column_high]
            - self.blocked_sums[row_high * width + column_low];
        sum > 0
    }
}

/// Label centres (ggforce `place_labels`), one per box in order; `None` for no
/// box or no room. `anchors` are the points labels stay close to (the centres of
/// the marks' bounding boxes).
pub(super) fn place_labels(
    sizes: &[Option<(f64, f64)>],
    marks: &[Vec<Point>],
    anchors: &[Point],
    ghosts: &[Point],
    bounds: Rect,
    buffer: f64,
) -> Vec<Option<Point>> {
    if sizes.iter().all(Option::is_none) {
        return vec![None; sizes.len()];
    }
    let grid = PlacementGrid::new(bounds, marks, ghosts, buffer);
    let padding = LABEL_PADDING_MM * PIXELS_PER_MM;
    let gap = LABEL_GAP_MM * PIXELS_PER_MM;
    let mut placed: Vec<(Point, Point)> = Vec::new();
    let mut result = Vec::with_capacity(sizes.len());
    let (columns, rows) = (grid.columns, grid.rows);
    let mut feasible = vec![false; columns * rows];
    for (size, anchor) in sizes.iter().zip(anchors) {
        let Some((width, height)) = *size else {
            result.push(None);
            continue;
        };
        let half = (width / 2.0 + padding, height / 2.0 + padding);
        for row in 0..rows {
            for column in 0..columns {
                let center = (
                    bounds.x + (column as f64 + 0.5) * grid.spacing,
                    bounds.y + (row as f64 + 0.5) * grid.spacing,
                );
                let inside = center.0 - half.0 >= bounds.x
                    && center.0 + half.0 <= bounds.x + bounds.width
                    && center.1 - half.1 >= bounds.y
                    && center.1 + half.1 <= bounds.y + bounds.height;
                feasible[row * columns + column] = inside
                    && !grid.hits_blocked(center, half)
                    && placed.iter().all(|(other, other_half)| {
                        (center.0 - other.0).abs() >= half.0 + other_half.0 + gap
                            || (center.1 - other.1).abs() >= half.1 + other_half.1 + gap
                    });
            }
        }
        // The free cell on the edge of the free region nearest the anchor.
        let mut best: Option<(f64, Point)> = None;
        for row in 0..rows {
            for column in 0..columns {
                if !feasible[row * columns + column] {
                    continue;
                }
                let edge = column == 0
                    || row == 0
                    || column + 1 == columns
                    || row + 1 == rows
                    || !feasible[row * columns + column - 1]
                    || !feasible[row * columns + column + 1]
                    || !feasible[(row - 1) * columns + column]
                    || !feasible[(row + 1) * columns + column];
                if !edge {
                    continue;
                }
                let center = (
                    bounds.x + (column as f64 + 0.5) * grid.spacing,
                    bounds.y + (row as f64 + 0.5) * grid.spacing,
                );
                let distance = (center.0 - anchor.0).powi(2) + (center.1 - anchor.1).powi(2);
                if best.is_none_or(|(nearest, _)| distance < nearest) {
                    best = Some((distance, center));
                }
            }
        }
        let center = best.map(|(_, center)| center);
        if let Some(center) = center {
            placed.push((center, half));
        }
        result.push(center);
    }
    result
}

/// ggforce `get_end_points` (y up): where a connector to `target` leaves the box.
fn end_point(low: Point, high: Point, target: Point) -> Point {
    let (xmin, ymin) = (low.0 - target.0, low.1 - target.1);
    let (xmax, ymax) = (high.0 - target.0, high.1 - target.1);
    let closer_x = if xmin.abs() < xmax.abs() {
        low.0
    } else {
        high.0
    };
    let closer_y = if ymin.abs() < ymax.abs() {
        low.1
    } else {
        high.1
    };
    if ymin < 0.0 && ymax > 0.0 {
        // Beside the target: the near vertical edge.
        let x = if xmin < 0.0 { high.0 } else { low.0 };
        return (x, closer_y);
    }
    if xmin < 0.0 && xmax > 0.0 {
        let y = if ymin < 0.0 { high.1 } else { low.1 };
        return (closer_x, y);
    }
    let x = if xmin < 0.0 { high.0 } else { low.0 };
    let y = if ymin < 0.0 { high.1 } else { low.1 };
    (x, y)
}

/// ggforce `elbow` (y up): the bend between the label end and the target.
fn elbow(start: Point, target: Point) -> Point {
    let (x, y) = (start.0 - target.0, start.1 - target.1);
    let quarter = std::f64::consts::FRAC_PI_4;
    let angle = y.atan2(x).rem_euclid(2.0 * std::f64::consts::PI);
    let bin = (angle / quarter).floor() as i64;
    // ggforce compares the remainder with 0.5 radians (not half a bin).
    let lower = angle.rem_euclid(quarter) < 0.5;
    let offset = match (bin % 4, lower) {
        (0, true) => (x - y, 0.0),
        (0, false) => (y, y),
        (1, true) => (x, x),
        (1, false) => (0.0, y - x),
        (2, true) => (0.0, y + x),
        (2, false) => (x, -x),
        (3, true) => (-y, y),
        _ => (x + y, 0.0),
    };
    (offset.0 + target.0, offset.1 + target.1)
}

/// The connector polyline (panel pixels, y down) from a label box at `center`
/// with full size `size` to the mark `outline`; `anchor_divisor` is ggforce's
/// `anchor_mod` (2 for hulls and ellipses, 3 for rectangles). Empty for
/// `con_type="none"`.
pub(super) fn connector(
    center: Point,
    size: (f64, f64),
    outline: &[Point],
    anchor_divisor: f64,
    options: &LabelOptions,
) -> Vec<Point> {
    if options.connector == ConnectorType::None || outline.is_empty() {
        return Vec::new();
    }
    // The outline point nearest one of the corners of the (shrunk) box.
    let (dx, dy) = (size.0 / anchor_divisor, size.1 / anchor_divisor);
    let corners = [
        (center.0 - dx, center.1 - dy),
        (center.0 + dx, center.1 - dy),
        (center.0 + dx, center.1 + dy),
        (center.0 - dx, center.1 + dy),
    ];
    let target = corners
        .iter()
        .map(|corner| closest_on_outline(outline, *corner))
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map_or(center, |(_, point)| point);
    // ggforce works with y up.
    let up = |point: Point| (point.0, -point.1);
    let (half_width, half_height) = (size.0 / 2.0, size.1 / 2.0);
    let center_up = up(center);
    let low = (center_up.0 - half_width, center_up.1 - half_height);
    let high = (center_up.0 + half_width, center_up.1 + half_height);
    let target_up = up(target);
    let start = end_point(low, high, target_up);
    let mut points = vec![start];
    if options.connector == ConnectorType::Elbow {
        points.push(elbow(start, target_up));
    }
    points.push(target_up);
    if options.border == Border::One {
        let other_x = if start.0 == low.0 { high.0 } else { low.0 };
        points.insert(0, (other_x, start.1));
    }
    // End cap: stop `cap` short of the mark (never past the previous point).
    let count = points.len();
    let (from, to) = (points[count - 2], points[count - 1]);
    let length = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
    let keep = if length > 0.0 {
        ((length - options.cap) / length).max(0.0)
    } else {
        0.0
    };
    points[count - 1] = (
        from.0 + (to.0 - from.0) * keep,
        from.1 + (to.1 - from.1) * keep,
    );
    points.into_iter().map(up).collect()
}

/// The label box at `center`: background (with the connector's outline for
/// `con_border="all"`), then the label and description lines.
pub(super) fn draw_box(label_box: &LabelBox, center: Point, options: &LabelOptions) -> Vec<Node> {
    let mut nodes = Vec::new();
    let left = center.0 - label_box.width / 2.0;
    let top = center.1 - label_box.height / 2.0;
    let border = match options.border {
        Border::All => options.stroke.clone(),
        _ => None,
    };
    if options.fill.is_some() || border.is_some() {
        nodes.push(Node::Rects(Rects {
            rects: vec![Rect {
                x: left,
                y: top,
                width: label_box.width,
                height: label_box.height,
            }],
            fill: PerItem::Uniform(options.fill),
            stroke: border,
        }));
    }
    let [margin_top, _, _, margin_left] = options.margin;
    let column_left = left + margin_left;
    let mut write = |lines: &[String], style: &TextStyle, hjust: f64, block_top: f64| {
        for (index, line) in lines.iter().enumerate() {
            let width = text_width(line, style);
            let baseline = block_top
                + style.size * (LINE_ASCENT + index as f64 * LINE_SPACING * style.line_height);
            nodes.push(Node::Text(Text {
                content: line.clone(),
                superscript: None,
                suffix: None,
                x: column_left + hjust * (label_box.text_width - width),
                y: baseline,
                size: style.size,
                font: style.font,
                color: style.color,
                horizontal: HorizontalAlign::Start,
                vertical: VerticalAlign::Baseline,
                angle: 0.0,
                halo: None,
            }));
        }
    };
    let label_top = top + margin_top;
    write(
        &label_box.label_lines,
        &options.label,
        options.hjust[0],
        label_top,
    );
    let description_top = label_top + label_box.label_height + label_box.separator;
    write(
        &label_box.description_lines,
        &options.description,
        options.hjust[1],
        description_top,
    );
    nodes
}

/// The connector as a path node.
pub(super) fn draw_connector(points: Vec<Point>, options: &LabelOptions) -> Option<Node> {
    (points.len() >= 2 && options.stroke.is_some()).then(|| {
        Node::Path(Path {
            points,
            closed: false,
            fill: None,
            stroke: options.stroke.clone(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::FontFace;

    pub(super) fn options() -> LabelOptions {
        let style = |face| TextStyle {
            size: 12.0 * PIXELS_PER_POINT,
            font: Font::from(face),
            color: Color::BLACK,
            line_height: 1.0,
        };
        LabelOptions {
            margin: [2.0 * PIXELS_PER_MM; 4],
            width: None,
            minimum_width: 50.0 * PIXELS_PER_MM,
            hjust: [0.0, 0.0],
            label: style(FontFace::Bold),
            description: style(FontFace::Plain),
            fill: Some(Color::WHITE),
            buffer: 10.0 * PIXELS_PER_MM,
            connector: ConnectorType::Elbow,
            border: Border::One,
            cap: 3.0 * PIXELS_PER_MM,
            stroke: Some(Stroke::solid(Color::BLACK, 1.9)),
        }
    }

    #[test]
    fn box_heights_follow_ggforce() {
        let options = options();
        let millimetres = |pixels: f64| pixels / PIXELS_PER_MM;
        // ggforce: "Cluster A" alone 8.0096 mm high; with a one-line description 12.0791.
        let single = measure_box("Cluster A", "", &options).unwrap();
        assert!(
            (millimetres(single.height) - 8.0096).abs() < 0.01,
            "{}",
            millimetres(single.height)
        );
        let both = measure_box("Cluster A", "A short description", &options).unwrap();
        assert!((millimetres(both.height) - 12.0791).abs() < 0.01);
        // Without description the box is as wide as the label (no minimum width).
        assert!(single.width < 30.0 * PIXELS_PER_MM);
        // A long description wraps at the minimum width.
        let long = measure_box(
            "Cluster A",
            "A much longer description that will need to wrap over several lines of text",
            &options,
        )
        .unwrap();
        assert!(long.description_lines.len() >= 2);
        assert!(long.width <= 50.0 * PIXELS_PER_MM + 1e-9);
        assert!(measure_box("", "", &options).is_none());
        // An unbreakable word widens the box past the minimum width.
        let word = measure_box("g", &"x".repeat(60), &options).unwrap();
        let longest = text_width(&"x".repeat(60), &options.description);
        assert!((word.width - longest - 4.0 * PIXELS_PER_MM).abs() < 1e-6);
    }

    #[test]
    fn distance_transform_matches_brute_force() {
        let mut values = vec![
            f64::INFINITY,
            0.0,
            f64::INFINITY,
            f64::INFINITY,
            f64::INFINITY,
            0.0,
            f64::INFINITY,
        ];
        let (mut scratch, mut hull) = (Vec::new(), Vec::new());
        distance_transform_1d(&mut values, &mut scratch, &mut hull);
        assert_eq!(values, vec![1.0, 0.0, 1.0, 4.0, 1.0, 0.0, 1.0]);
    }

    fn square(x: f64, y: f64, side: f64) -> Vec<Point> {
        vec![(x, y), (x + side, y), (x + side, y + side), (x, y + side)]
    }

    #[test]
    fn labels_stay_clear_of_marks_and_each_other() {
        let bounds = Rect {
            x: 0.0,
            y: 0.0,
            width: 600.0,
            height: 400.0,
        };
        let marks = vec![
            square(100.0, 100.0, 80.0),
            square(300.0, 150.0, 100.0),
            square(450.0, 250.0, 60.0),
        ];
        let anchors: Vec<Point> = marks
            .iter()
            .map(|mark| ((mark[0].0 + mark[2].0) / 2.0, (mark[0].1 + mark[2].1) / 2.0))
            .collect();
        let sizes = vec![Some((90.0, 30.0)); 3];
        let buffer = 10.0 * PIXELS_PER_MM;
        let centers = place_labels(&sizes, &marks, &anchors, &[], bounds, buffer);
        let mut boxes = Vec::new();
        for (center, mark) in centers.iter().zip(&marks) {
            let center = center.expect("room for the label");
            let (half_width, half_height) = (45.0, 15.0);
            // Inside the panel.
            assert!(center.0 - half_width >= 0.0 && center.0 + half_width <= 600.0);
            assert!(center.1 - half_height >= 0.0 && center.1 + half_height <= 400.0);
            // Clear of every mark by the buffer (plus the 2 mm padding).
            for other in &marks {
                let clearance = box_distance(center, (half_width, half_height), other);
                assert!(clearance >= buffer, "{clearance}");
            }
            boxes.push((center, mark.clone()));
        }
        for i in 0..boxes.len() {
            for j in i + 1..boxes.len() {
                let (a, b) = (boxes[i].0, boxes[j].0);
                assert!(
                    (a.0 - b.0).abs() >= 90.0 || (a.1 - b.1).abs() >= 30.0,
                    "labels overlap"
                );
            }
        }
    }

    /// Smallest distance between a box and a polygon (0 when they overlap).
    fn box_distance(center: Point, half: Point, polygon: &[Point]) -> f64 {
        let corners = vec![
            (center.0 - half.0, center.1 - half.1),
            (center.0 + half.0, center.1 - half.1),
            (center.0 + half.0, center.1 + half.1),
            (center.0 - half.0, center.1 + half.1),
        ];
        let mut nearest = f64::INFINITY;
        for point in polygon {
            if contains(&corners, *point) {
                return 0.0;
            }
            nearest = nearest.min(closest_on_outline(&corners, *point).0);
        }
        for corner in &corners {
            if contains(polygon, *corner) {
                return 0.0;
            }
            nearest = nearest.min(closest_on_outline(polygon, *corner).0);
        }
        nearest
    }

    #[test]
    fn no_room_means_no_label() {
        let bounds = Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 100.0,
        };
        let marks = vec![square(10.0, 10.0, 180.0)];
        let centers = place_labels(
            &[Some((90.0, 30.0))],
            &marks,
            &[(100.0, 50.0)],
            &[],
            bounds,
            37.8,
        );
        assert_eq!(centers, vec![None]);
    }

    #[test]
    fn connectors_end_short_of_the_mark() {
        let options = options();
        let mark = square(100.0, 100.0, 80.0);
        // A label up and to the right of the mark.
        let points = connector((300.0, 40.0), (90.0, 30.0), &mark, 2.0, &options);
        // Border line, label end, elbow, capped end.
        assert_eq!(points.len(), 4);
        let end = *points.last().unwrap();
        let (distance, _) = closest_on_outline(&mark, end);
        assert!((distance - options.cap).abs() < 1e-6, "{distance}");
        // The first two points run along the bottom edge of the box.
        assert_eq!(points[0].1, 55.0);
        assert_eq!(points[1].1, 55.0);
        let straight = LabelOptions {
            connector: ConnectorType::Straight,
            border: Border::None,
            ..options.clone()
        };
        assert_eq!(
            connector((300.0, 40.0), (90.0, 30.0), &mark, 2.0, &straight).len(),
            2
        );
        let none = LabelOptions {
            connector: ConnectorType::None,
            ..options
        };
        assert!(connector((300.0, 40.0), (90.0, 30.0), &mark, 2.0, &none).is_empty());
    }
}
