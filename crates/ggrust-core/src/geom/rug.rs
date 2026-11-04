//! `geom_rug` (ggplot2 `GeomRug`): a short tick at the panel edge for every x
//! and/or y value, as a marginal distribution.
//!
//! `sides` picks the edges (any of `"trbl"`, default `"bl"`): x values tick
//! the bottom/top edge, y values the left/right edge. `length` is the tick
//! length as a share of the panel (ggplot2 `unit(0.03, "npc")`): of its
//! height for x ticks, of its width for y ticks. Under `coord_flip` the sides
//! follow the axes, as ggplot2 (`b`/`t` of data x become `l`/`r`).
//! `outside=true` draws the ticks away from the panel; the panel clips its
//! layers (ggrust has no `clip="off"`), so they are not visible (Python warns).
//!
//! Line width comes from `linewidth` (ggplot2's name), in ggrust line units.
//!
//! What it does not do (yet): keep ticks on the panel edge under HTML zoom
//! (they move with the panel like other marks), `lineend`.

use super::lines;
use super::point::POINT_UNIT_SIZE;
use super::{DrawLayer, Geom};
use crate::coord::Panel;
use crate::parameters::Parameters;
use crate::scene::{Node, Path, Rect, Stroke};
use crate::spec::{Aesthetic, Value};
use crate::style::{StyleDefault, Styles};
use crate::{Error, Result};

/// ggplot2 rug style in ggrust's line defaults: pen color, `linewidth` 0.5.
const RUG_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Color, StyleDefault::PEN),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Linewidth, StyleDefault::Number(0.5)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

/// ggplot2's default tick length: 3% of the panel.
const DEFAULT_LENGTH: f64 = 0.03;

/// Panel edges, in screen terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

/// Rug ticks along the panel edges.
pub struct RugGeom;

impl Geom for RugGeom {
    fn name(&self) -> &'static str {
        "rug"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[]
    }

    fn optional_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        RUG_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["sides", "length", "outside"]
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let options = RugOptions::parse(&layer.parameters)?;
        let x = layer.data.continuous(Aesthetic::X);
        let y = layer.data.continuous(Aesthetic::Y);
        if x.is_none() && y.is_none() {
            return Err(Error::spec(
                layer.parameters.path_of("mapping"),
                "geom_rug needs x and/or y: map one, e.g. aes(x='value')",
            ));
        }
        let mut nodes = Vec::new();
        // (data values, data x?) pairs; each tick runs from the edge inwards.
        for (values, data_x) in [(x, true), (y, false)] {
            let Some(values) = values else { continue };
            // Data x runs horizontally unless flipped (coord_flip or y orientation).
            let horizontal = data_x != panel.flipped;
            for side in options.sides(data_x, panel.flipped) {
                for row in 0..layer.data.row_count() {
                    let Some(value) = values.get(row).filter(|value| value.is_finite()) else {
                        continue;
                    };
                    let along = match horizontal {
                        true => panel.x_pixel(value),
                        false => panel.y_pixel(value),
                    };
                    if !along.is_finite() {
                        continue;
                    }
                    let Some(stroke) = rug_stroke(layer.styles, row)? else {
                        continue;
                    };
                    nodes.push(tick(panel.rect, side, along, &options, stroke));
                }
            }
        }
        Ok(nodes)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        let width = styles.numbers(Aesthetic::Linewidth)?.get(0).max(0.0) * POINT_UNIT_SIZE;
        Ok((width + 2.0, width + 2.0))
    }

    /// ggplot2 `draw_key_path`: a horizontal line through the key.
    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        let Some(stroke) = rug_stroke(styles, 0)? else {
            return Ok(Vec::new());
        };
        let y = cell.y + cell.height / 2.0;
        Ok(vec![Node::Path(Path {
            points: vec![(cell.x, y), (cell.x + cell.width, y)],
            closed: false,
            fill: None,
            stroke: Some(stroke),
        })])
    }
}

/// The tick line of one row: `linewidth` wide, faded by `alpha`.
fn rug_stroke(styles: &Styles, row: usize) -> Result<Option<Stroke>> {
    let color = styles.colors(Aesthetic::Color)?.get(row);
    let alpha = styles.numbers(Aesthetic::Alpha)?.get(row);
    let width = styles.numbers(Aesthetic::Linewidth)?.get(row) * POINT_UNIT_SIZE;
    let linetype = styles.linetypes(Aesthetic::Linetype)?.get_ref(row);
    Ok(lines::stroke(color.with_opacity(alpha), width, linetype))
}

/// One tick at `along` (pixel along the edge) on `side` of `rect`.
fn tick(rect: Rect, side: Side, along: f64, options: &RugOptions, stroke: Stroke) -> Node {
    let direction = if options.outside { -1.0 } else { 1.0 };
    let (left, top) = (rect.x, rect.y);
    let (right, bottom) = (rect.x + rect.width, rect.y + rect.height);
    let vertical_length = options.length * rect.height * direction;
    let horizontal_length = options.length * rect.width * direction;
    let points = match side {
        Side::Bottom => vec![(along, bottom), (along, bottom - vertical_length)],
        Side::Top => vec![(along, top), (along, top + vertical_length)],
        Side::Left => vec![(left, along), (left + horizontal_length, along)],
        Side::Right => vec![(right, along), (right - horizontal_length, along)],
    };
    Node::Path(Path {
        points,
        closed: false,
        fill: None,
        stroke: Some(stroke),
    })
}

/// Parsed `sides`, `length` and `outside`.
#[derive(Debug, Clone, PartialEq)]
struct RugOptions {
    /// Requested sides in data terms (`b`/`t` for x, `l`/`r` for y), as given.
    sides: Vec<Side>,
    length: f64,
    outside: bool,
}

impl RugOptions {
    fn parse(parameters: &Parameters) -> Result<RugOptions> {
        let sides = match parameters.value("sides") {
            None | Some(Value::Null) => "bl".to_owned(),
            Some(Value::String(text)) => text.clone(),
            Some(other) => {
                return Err(Error::spec(
                    parameters.path_of("sides"),
                    format!(
                        "sides must be a string of 't', 'r', 'b', 'l' (e.g. 'bl'), got {other}"
                    ),
                ));
            }
        };
        let mut parsed = Vec::new();
        for letter in sides.chars() {
            let side = match letter {
                't' => Side::Top,
                'r' => Side::Right,
                'b' => Side::Bottom,
                'l' => Side::Left,
                other => {
                    return Err(Error::spec(
                        parameters.path_of("sides"),
                        format!(
                            "unknown side '{other}' in sides='{sides}'; use any of 't' (top), \
                             'r' (right), 'b' (bottom), 'l' (left), e.g. 'bl'"
                        ),
                    ));
                }
            };
            if !parsed.contains(&side) {
                parsed.push(side);
            }
        }
        let length = parameters.number("length")?.unwrap_or(DEFAULT_LENGTH);
        if length < 0.0 {
            return Err(Error::spec(
                parameters.path_of("length"),
                format!(
                    "length must be a share of the panel, 0 or more (ggplot2 default 0.03), \
                     got {length}"
                ),
            ));
        }
        let outside = parameters.boolean("outside")?.unwrap_or(false);
        Ok(RugOptions {
            sides: parsed,
            length,
            outside,
        })
    }

    /// Screen sides that ticks of data x (`data_x`) or data y go to. ggplot2
    /// reads `b`/`t` for x and `l`/`r` for y, then under `coord_flip` maps the
    /// letters `tblr` to `rlbt` (so data x ticks the left edge).
    fn sides(&self, data_x: bool, flipped: bool) -> Vec<Side> {
        // Screen edges the ticks of this axis can sit on.
        let edges = match data_x != flipped {
            true => [Side::Bottom, Side::Top],
            false => [Side::Left, Side::Right],
        };
        self.sides
            .iter()
            .map(|side| match (flipped, side) {
                (false, side) => *side,
                (true, Side::Top) => Side::Right,
                (true, Side::Bottom) => Side::Left,
                (true, Side::Left) => Side::Bottom,
                (true, Side::Right) => Side::Top,
            })
            .filter(|side| edges.contains(side))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(sides: &str) -> RugOptions {
        let mut parsed = Vec::new();
        for letter in sides.chars() {
            parsed.push(match letter {
                't' => Side::Top,
                'r' => Side::Right,
                'b' => Side::Bottom,
                _ => Side::Left,
            });
        }
        RugOptions {
            sides: parsed,
            length: 0.03,
            outside: false,
        }
    }

    #[test]
    fn sides_follow_the_axes() {
        let all = options("trbl");
        assert_eq!(all.sides(true, false), vec![Side::Top, Side::Bottom]);
        assert_eq!(all.sides(false, false), vec![Side::Right, Side::Left]);
        // coord_flip: x ticks go to the left/right edges.
        assert_eq!(options("b").sides(true, true), vec![Side::Left]);
        assert_eq!(options("l").sides(false, true), vec![Side::Bottom]);
        assert!(options("l").sides(true, false).is_empty());
    }

    #[test]
    fn ticks_have_the_panel_share_as_length() {
        let rect = Rect {
            x: 10.0,
            y: 20.0,
            width: 200.0,
            height: 100.0,
        };
        let stroke = Stroke::solid(crate::scene::Color::BLACK, 1.0);
        let mut rug = options("b");
        let Node::Path(path) = tick(rect, Side::Bottom, 50.0, &rug, stroke.clone()) else {
            panic!("a path")
        };
        assert_eq!(path.points, vec![(50.0, 120.0), (50.0, 117.0)]);
        rug.outside = true;
        let Node::Path(path) = tick(rect, Side::Left, 70.0, &rug, stroke) else {
            panic!("a path")
        };
        assert_eq!(path.points, vec![(10.0, 70.0), (4.0, 70.0)]);
    }
}
