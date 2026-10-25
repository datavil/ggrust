//! Backend-agnostic drawing primitives in pixel space.
//!
//! What it does: the layout stage builds a [`Scene`] (a tree of groups and
//! batched primitives); renderers turn it into SVG, PNG or HTML.
//!
//! What it does not do: know about data, scales or themes. Every coordinate is
//! already in pixels, origin top-left, y pointing down.
//!
//! Primitives are batched (one [`Points`] node for a whole scatter layer) so
//! renderers can write uniform styles once and stream geometry.

mod color;
mod color_names;

pub use color::Color;

use crate::text::FontFace;

/// A complete drawing.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    /// Width in pixels.
    pub width: f64,
    /// Height in pixels.
    pub height: f64,
    /// Background fill of the whole drawing; `None` is transparent.
    pub background: Option<Color>,
    /// Top-level nodes, drawn in order.
    pub nodes: Vec<Node>,
}

/// One element of the scene tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Child nodes with an optional clip.
    Group(Group),
    /// Many markers sharing one batch.
    Points(Points),
    /// A polyline or polygon.
    Path(Path),
    /// Many axis-aligned rectangles.
    Rects(Rects),
    /// A line of text.
    Text(Text),
}

/// Child nodes drawn together, optionally clipped to a rectangle.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Group {
    /// Clip rectangle; children are not drawn outside it.
    pub clip: Option<Rect>,
    /// Semantic label for renderers (e.g. `"panel"`, `"legend"`), used for
    /// interactivity hooks and debugging. Not styling.
    pub role: Option<&'static str>,
    /// Children, drawn in order.
    pub nodes: Vec<Node>,
}

/// An axis-aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: f64,
    /// Top edge.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

/// A value shared by every item of a batch, or one value per item.
#[derive(Debug, Clone, PartialEq)]
pub enum PerItem<T> {
    /// The same value for every item.
    Uniform(T),
    /// One value per item; must have the batch's length.
    Varying(Vec<T>),
}

impl<T: Copy> PerItem<T> {
    /// Value of item `index`.
    ///
    /// # Panics
    /// If `Varying` is shorter than `index + 1`; batches are validated by their builder.
    pub fn get(&self, index: usize) -> T {
        match self {
            PerItem::Uniform(value) => *value,
            PerItem::Varying(values) => values[index],
        }
    }

    /// Whether `Varying` has exactly `len` items (always true for `Uniform`).
    pub fn fits(&self, len: usize) -> bool {
        match self {
            PerItem::Uniform(_) => true,
            PerItem::Varying(values) => values.len() == len,
        }
    }
}

/// Line style.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    /// Line color.
    pub color: Color,
    /// Line width in pixels.
    pub width: f64,
    /// Dash pattern in pixels (`[dash, gap, ...]`); empty is solid.
    pub dash: Vec<f64>,
}

impl Stroke {
    /// A solid line.
    pub fn solid(color: Color, width: f64) -> Self {
        Stroke {
            color,
            width,
            dash: Vec::new(),
        }
    }
}

/// Marker geometry. Size is the marker's diameter / side in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkerShape {
    /// A circle.
    Circle,
    /// An axis-aligned square.
    Square,
    /// An upward-pointing triangle.
    Triangle,
    /// A square rotated 45 degrees.
    Diamond,
}

/// A batch of markers.
#[derive(Debug, Clone, PartialEq)]
pub struct Points {
    /// Centre x of each marker.
    pub x: Vec<f64>,
    /// Centre y of each marker.
    pub y: Vec<f64>,
    /// Geometry.
    pub shape: PerItem<MarkerShape>,
    /// Diameter in pixels.
    pub size: PerItem<f64>,
    /// Fill; `None` leaves the marker hollow.
    pub fill: PerItem<Option<Color>>,
    /// Outline color; `None` draws no outline.
    pub stroke: PerItem<Option<Color>>,
    /// Outline width in pixels.
    pub stroke_width: PerItem<f64>,
}

impl Points {
    /// Number of markers.
    pub fn len(&self) -> usize {
        self.x.len()
    }

    /// Whether the batch is empty.
    pub fn is_empty(&self) -> bool {
        self.x.is_empty()
    }

    /// Whether every per-item attribute matches the number of markers.
    pub fn is_consistent(&self) -> bool {
        let len = self.len();
        self.y.len() == len
            && self.shape.fits(len)
            && self.size.fits(len)
            && self.fill.fits(len)
            && self.stroke.fits(len)
            && self.stroke_width.fits(len)
    }
}

/// A polyline (open) or polygon (closed).
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    /// Vertices.
    pub points: Vec<(f64, f64)>,
    /// Connect the last vertex back to the first.
    pub closed: bool,
    /// Interior fill (closed paths only).
    pub fill: Option<Color>,
    /// Outline.
    pub stroke: Option<Stroke>,
}

/// A batch of axis-aligned rectangles.
#[derive(Debug, Clone, PartialEq)]
pub struct Rects {
    /// Rectangles.
    pub rects: Vec<Rect>,
    /// Fill of each rectangle.
    pub fill: PerItem<Option<Color>>,
    /// Outline shared by all rectangles.
    pub stroke: Option<Stroke>,
}

impl Rects {
    /// Whether the fill matches the number of rectangles.
    pub fn is_consistent(&self) -> bool {
        self.fill.fits(self.rects.len())
    }
}

/// Horizontal placement of text relative to its anchor point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HorizontalAlign {
    /// Text starts at the anchor.
    #[default]
    Start,
    /// Text is centred on the anchor.
    Middle,
    /// Text ends at the anchor.
    End,
}

/// Vertical placement of text relative to its anchor point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalAlign {
    /// The top of the line box is at the anchor.
    Top,
    /// The centre of the line box is at the anchor.
    Middle,
    /// The baseline is at the anchor.
    #[default]
    Baseline,
    /// The bottom of the line box is at the anchor.
    Bottom,
}

/// One line of text.
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    /// Content (a single line).
    pub content: String,
    /// Anchor x.
    pub x: f64,
    /// Anchor y.
    pub y: f64,
    /// Font size in pixels.
    pub size: f64,
    /// Weight and slant.
    pub face: FontFace,
    /// Text color.
    pub color: Color,
    /// Horizontal placement.
    pub horizontal: HorizontalAlign,
    /// Vertical placement.
    pub vertical: VerticalAlign,
    /// Rotation around the anchor, degrees counter-clockwise (ggplot convention).
    pub angle: f64,
}
