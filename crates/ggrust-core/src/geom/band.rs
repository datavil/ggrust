//! `geom_band`: a band across the whole panel per row, between `xmin` and `xmax`
//! (a vertical band) or between `ymin` and `ymax` (a horizontal band).
//!
//! Follows lets-plot `BandGeom`: the fill rectangle spans the panel and has no
//! outline; the two edges are drawn as lines in `color`, `size` x 2.2 px and
//! `linetype`. `alpha` applies to the fill only. Pen fill at 10% opacity by
//! default, as `geom_rect`. A row with a missing bound draws nothing; swapped
//! bounds draw the same band (the check would run on transformed values, where
//! `scale_*_reverse` swaps them). Under `coord_flip` an x band becomes a horizontal stripe.
//!
//! What it does not do (yet): tooltips (lets-plot shows them on the band).

use super::bar::{draw_key, key_size};
use super::lines;
use super::{DrawLayer, Geom};
use crate::coord::Panel;
use crate::scene::{Node, Path, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles, ThemeColor};
use crate::{Error, Result};

/// lets-plot band style: pen edges, pen fill at 10% opacity, size 0.75.
const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Color, StyleDefault::PEN),
    (Aesthetic::Fill, StyleDefault::Themed(ThemeColor::Pen, 26)),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Size, StyleDefault::Number(0.75)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

/// The band's bounds: one pair, mapped or constant.
const BOUNDS: &[Aesthetic] = &[
    Aesthetic::Xmin,
    Aesthetic::Xmax,
    Aesthetic::Ymin,
    Aesthetic::Ymax,
];

/// Bands across the panel.
pub struct BandGeom;

impl Geom for BandGeom {
    fn name(&self) -> &'static str {
        "band"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[]
    }

    fn optional_aesthetics(&self) -> &'static [Aesthetic] {
        BOUNDS
    }

    fn constant_positions(&self) -> &'static [Aesthetic] {
        BOUNDS
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let has = |aesthetic| layer.data.continuous(aesthetic).is_some();
        let x_bounds = has(Aesthetic::Xmin) || has(Aesthetic::Xmax);
        let y_bounds = has(Aesthetic::Ymin) || has(Aesthetic::Ymax);
        let (low, high) = match (x_bounds, y_bounds) {
            (true, false) => (Aesthetic::Xmin, Aesthetic::Xmax),
            (false, true) => (Aesthetic::Ymin, Aesthetic::Ymax),
            (true, true) => {
                return Err(Error::spec(
                    layer.parameters.path_of("mapping"),
                    "geom_band takes either xmin/xmax (a vertical band) or ymin/ymax \
                     (a horizontal band), not both; use geom_rect for a rectangle"
                        .to_owned(),
                ));
            }
            (false, false) => return Ok(Vec::new()),
        };
        let (Some(lows), Some(highs)) = (layer.data.continuous(low), layer.data.continuous(high))
        else {
            return Ok(Vec::new());
        };
        // The band's edges run across the screen when its data axis is drawn
        // vertically: y bounds unflipped, x bounds under coord_flip.
        let edges_horizontal = y_bounds != panel.flipped;
        let fills = layer.styles.colors(Aesthetic::Fill)?;
        let alphas = layer.styles.numbers(Aesthetic::Alpha)?;
        let area = panel.rect;
        let mut nodes = Vec::new();
        for row in 0..layer.data.row_count() {
            let (Some(start), Some(end)) = (lows.get(row), highs.get(row)) else {
                continue;
            };
            if !(start.is_finite() && end.is_finite()) {
                continue;
            }
            let pixel = |value: f64| {
                if edges_horizontal {
                    panel.y_pixel(value)
                } else {
                    panel.x_pixel(value)
                }
            };
            let (first, second) = (pixel(start), pixel(end));
            if !(first.is_finite() && second.is_finite()) {
                continue;
            }
            let rect = if edges_horizontal {
                Rect {
                    x: area.x,
                    y: first.min(second),
                    width: area.width,
                    height: (second - first).abs(),
                }
            } else {
                Rect {
                    x: first.min(second),
                    y: area.y,
                    width: (second - first).abs(),
                    height: area.height,
                }
            };
            let fill = fills.get(row).with_opacity(alphas.get(row));
            if fill.alpha > 0 {
                nodes.push(Node::Path(Path {
                    points: corners(rect),
                    closed: true,
                    fill: Some(fill),
                    stroke: None,
                }));
            }
            let Some(stroke) = lines::stroke_with_alpha(layer.styles, row, false)? else {
                continue;
            };
            for position in [first, second] {
                let points = if edges_horizontal {
                    vec![(area.x, position), (area.x + area.width, position)]
                } else {
                    vec![(position, area.y + area.height), (position, area.y)]
                };
                nodes.push(Node::Path(Path {
                    points,
                    closed: false,
                    fill: None,
                    stroke: Some(stroke.clone()),
                }));
            }
        }
        Ok(nodes)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        draw_key(styles, cell)
    }
}

/// The corners of `rect`, clockwise from the top left.
fn corners(rect: Rect) -> Vec<(f64, f64)> {
    vec![
        (rect.x, rect.y),
        (rect.x + rect.width, rect.y),
        (rect.x + rect.width, rect.y + rect.height),
        (rect.x, rect.y + rect.height),
    ]
}
