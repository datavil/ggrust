//! `geom_dotplot` and `geom_ydotplot`: stacks of dots one bin width wide (lets-plot
//! `DotplotGeom`, `YDotplotGeom`). Dots are sized in pixels from the bin width, so the
//! axis across the stacks carries no data: it runs 0 to 1 for `geom_dotplot`.
//!
//! A stack is cut where it would leave the panel, as lets-plot.
//!
//! What it does not do: tooltips.

use std::collections::HashMap;

use super::point::POINT_UNIT_SIZE;
use super::{DrawLayer, Geom};
use crate::coord::Panel;
use crate::data::{Column, ContinuousColumn, LayerData};
use crate::parameters::Parameters;
use crate::position::DefaultPosition;
use crate::scene::{MarkerShape, Node, PerItem, Points, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::{Error, Result};

const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Color, StyleDefault::PAPER),
    (Aesthetic::Fill, StyleDefault::BRUSH),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Stroke, StyleDefault::Number(0.75)),
];

/// lets-plot keys draw a filled circle of size 5.
const KEY_SIZE: f64 = 5.0;
const OPTIONS: &[&str] = &["dotsize", "stackratio", "stackgroups", "stackdir", "method"];

/// How dots pile up, in dots from the axis.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Stacking {
    /// Away from the axis, in the positive screen sense.
    Forward,
    Backward,
    Center,
    CenterWhole,
}

impl Stacking {
    /// Position of dot `index` of a stack of `size`, in dot steps (lets-plot
    /// `getDotCenter`).
    fn offset(self, index: usize, size: usize, stack_ratio: f64) -> f64 {
        let (index, size) = (index as f64, size as f64);
        match self {
            Stacking::Forward => index + 1.0 / (2.0 * stack_ratio),
            Stacking::Backward => -index - 1.0 / (2.0 * stack_ratio),
            Stacking::Center => index + 0.5 - size / 2.0,
            Stacking::CenterWhole => {
                let parity = if size % 2.0 == 0.0 { 0.0 } else { 0.5 };
                index + parity - size / 2.0 + 1.0 / (2.0 * stack_ratio)
            }
        }
    }
}

/// The options of a dot plot layer.
#[derive(Debug, Clone, Copy)]
struct Options {
    dot_size: f64,
    stack_ratio: f64,
    across_groups: bool,
    stacking: Stacking,
}

impl Options {
    /// `forward` and `backward` are the `stackdir` names of the two ends.
    fn parse(
        parameters: &Parameters,
        forward: &str,
        backward: &str,
        default: Stacking,
    ) -> Result<Options> {
        let stacking = match parameters.string("stackdir")?.map(str::to_lowercase) {
            None => default,
            Some(name) if name == forward => Stacking::Forward,
            Some(name) if name == backward => Stacking::Backward,
            Some(name) if name == "center" => Stacking::Center,
            Some(name) if name == "centerwhole" => Stacking::CenterWhole,
            Some(other) => {
                return Err(Error::spec(
                    parameters.path_of("stackdir"),
                    format!(
                        "expected one of '{forward}', '{backward}', 'center', 'centerwhole', \
                         got '{other}'"
                    ),
                ));
            }
        };
        let histodot = parameters
            .string("method")?
            .is_some_and(|method| method.eq_ignore_ascii_case("histodot"));
        Ok(Options {
            dot_size: parameters.number("dotsize")?.unwrap_or(1.0),
            stack_ratio: parameters.number("stackratio")?.unwrap_or(1.0),
            across_groups: parameters.boolean("stackgroups")?.unwrap_or(false) && histodot,
            stacking,
        })
    }

    /// lets-plot `boundedStackSize`: `size`, cut to what fits along `length` pixels.
    fn bounded(&self, size: usize, length: f64, bin: f64) -> usize {
        let capacity = (length / (self.dot_size * self.stack_ratio * bin)).ceil();
        if !capacity.is_finite() {
            return size;
        }
        // A negative stack ratio or dot size fits nothing, as lets-plot.
        if capacity < 0.0 {
            return 0;
        }
        let capacity = capacity as usize + 1;
        let parity = usize::from(size % 2 != capacity % 2);
        size.min(capacity + parity)
    }
}

/// Dots collected for one layer.
#[derive(Default)]
struct Dots {
    x: Vec<f64>,
    y: Vec<f64>,
    rows: Vec<usize>,
}

impl Dots {
    fn into_node(self, styles: &Styles, diameter: f64) -> Result<Node> {
        let (colors, fills) = (
            styles.colors(Aesthetic::Color)?,
            styles.colors(Aesthetic::Fill)?,
        );
        let (alphas, strokes) = (
            styles.numbers(Aesthetic::Alpha)?,
            styles.numbers(Aesthetic::Stroke)?,
        );
        let each = |row: &usize| (fills.get(*row), colors.get(*row), alphas.get(*row));
        Ok(Node::Points(Points {
            shape: PerItem::Uniform(MarkerShape::Circle),
            size: PerItem::Uniform(diameter),
            fill: PerItem::Varying(
                self.rows
                    .iter()
                    .map(|row| {
                        let (fill, _, alpha) = each(row);
                        Some(fill.with_opacity(alpha))
                    })
                    .collect(),
            ),
            stroke: PerItem::Varying(
                self.rows
                    .iter()
                    .map(|row| {
                        let (_, color, alpha) = each(row);
                        Some(color.with_opacity(alpha))
                    })
                    .collect(),
            ),
            stroke_width: PerItem::Varying(
                self.rows
                    .iter()
                    .map(|row| strokes.get(*row) * POINT_UNIT_SIZE)
                    .collect(),
            ),
            x: self.x,
            y: self.y,
            keys: None,
        }))
    }
}

/// The rows of a layer ready to draw, as (row, stack size).
fn drawable(layer: &DrawLayer, columns: [&ContinuousColumn; 3]) -> Vec<(usize, usize)> {
    let [position, stack_size, bin_width] = columns;
    (0..layer.data.row_count())
        .filter_map(|row| {
            let size = stack_size.get(row).filter(|size| size.is_finite())?;
            bin_width.get(row).filter(|width| width.is_finite())?;
            position.get(row).filter(|value| value.is_finite())?;
            Some((row, size as usize))
        })
        .collect()
}

/// Rows grouped by the bits of `key(row)`, groups in first-appearance order.
fn stacks_by(
    rows: Vec<(usize, usize)>,
    key: impl Fn(usize) -> Vec<u64>,
) -> Vec<Vec<(usize, usize)>> {
    let mut slots: HashMap<Vec<u64>, usize> = HashMap::new();
    let mut stacks: Vec<Vec<(usize, usize)>> = Vec::new();
    for item in rows {
        let slot = *slots.entry(key(item.0)).or_insert_with(|| {
            stacks.push(Vec::new());
            stacks.len() - 1
        });
        stacks[slot].push(item);
    }
    stacks
}

fn columns<'a>(layer: &DrawLayer<'a>) -> Option<[&'a ContinuousColumn; 3]> {
    Some([
        layer.data.continuous(Aesthetic::X)?,
        layer.data.continuous(Aesthetic::Stacksize)?,
        layer.data.continuous(Aesthetic::Binwidth)?,
    ])
}

fn at(column: &ContinuousColumn, row: usize) -> f64 {
    column.get(row).unwrap_or(f64::NAN)
}

/// The domain span of each dot's position: one resolution wide. lets-plot sizes it
/// by `binwidth`, but its dry run leaves that aesthetic at the default of 1.
fn span_columns(
    data: &mut LayerData,
    position: Aesthetic,
    low: Aesthetic,
    high: Aesthetic,
) -> Result<()> {
    let Some(values) = data.continuous(position).cloned() else {
        return Ok(());
    };
    let half = data.resolution(position) / 2.0;
    let rows = 0..data.row_count();
    let continuous = |values| Column::Continuous(ContinuousColumn::from_values(values));
    let lows = rows.clone().map(|row| at(&values, row) - half).collect();
    let highs = rows.map(|row| at(&values, row) + half).collect();
    data.insert(low, continuous(lows))?;
    data.insert(high, continuous(highs))?;
    Ok(())
}

const X_SPANS: &[Aesthetic] = &[Aesthetic::Xmin, Aesthetic::Xmax];

const SPANS: &[Aesthetic] = &[
    Aesthetic::Xmin,
    Aesthetic::Xmax,
    Aesthetic::Ymin,
    Aesthetic::Ymax,
];

fn key_size(styles: &Styles) -> Result<(f64, f64)> {
    let side = KEY_SIZE * POINT_UNIT_SIZE
        + 2.0 * styles.numbers(Aesthetic::Stroke)?.get(0) * POINT_UNIT_SIZE
        + 2.0;
    Ok((side, side))
}

fn draw_key(styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
    let stroke = styles.numbers(Aesthetic::Stroke)?.get(0) * POINT_UNIT_SIZE;
    let alpha = styles.numbers(Aesthetic::Alpha)?.get(0);
    Ok(vec![Node::Points(Points {
        shape: PerItem::Uniform(MarkerShape::Circle.box_aligned()),
        size: PerItem::Uniform(KEY_SIZE * POINT_UNIT_SIZE + stroke),
        fill: PerItem::Uniform(Some(
            styles.colors(Aesthetic::Fill)?.get(0).with_opacity(alpha),
        )),
        stroke: PerItem::Uniform(Some(
            styles.colors(Aesthetic::Color)?.get(0).with_opacity(alpha),
        )),
        stroke_width: PerItem::Uniform(stroke),
        x: vec![cell.x + cell.width / 2.0],
        y: vec![cell.y + cell.height / 2.0],
        keys: None,
    })])
}

/// Stacks over x.
pub struct DotplotGeom;

impl DotplotGeom {
    fn options(parameters: &Parameters) -> Result<Options> {
        Options::parse(parameters, "up", "down", Stacking::Forward)
    }
}

impl Geom for DotplotGeom {
    fn name(&self) -> &'static str {
        "dotplot"
    }

    fn default_stat(&self) -> &'static str {
        "dotplot"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Stacksize, Aesthetic::Binwidth]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        OPTIONS
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        X_SPANS
    }

    fn span_aesthetics(&self) -> &'static [Aesthetic] {
        X_SPANS
    }

    fn setup(&self, mut data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        Self::options(parameters)?;
        span_columns(&mut data, Aesthetic::X, Aesthetic::Xmin, Aesthetic::Xmax)?;
        Ok(data)
    }

    fn preferable_null_y(&self, parameters: &Parameters) -> Result<Option<(f64, f64)>> {
        Ok(Some(match Self::options(parameters)?.stacking {
            Stacking::Forward => (0.0, 1.0),
            Stacking::Backward => (-1.0, 0.0),
            Stacking::Center | Stacking::CenterWhole => (-0.5, 0.5),
        }))
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let Some([x, stack_size, bin_width]) = columns(layer) else {
            return Ok(Vec::new());
        };
        let options = Self::options(&layer.parameters)?;
        let rows = drawable(layer, [x, stack_size, bin_width]);
        let Some(&(first, _)) = rows.first() else {
            return Ok(Vec::new());
        };
        let bin = bin_pixels(panel, at(x, first), 0.0, at(bin_width, first), true);
        let flipped = panel.flipped;
        let length = if flipped {
            panel.rect.width
        } else {
            panel.rect.height
        };
        let step = options.dot_size * options.stack_ratio * bin;
        let mut dots = Dots::default();
        for stack in stacks_by(rows, |row| vec![at(x, row).to_bits()]) {
            let mut built = 0;
            for (row, size) in stack {
                let drawn = options.bounded(built + size, length, bin) - built;
                let base = panel.map(at(x, row), 0.0);
                for index in 0..drawn {
                    let id = if options.across_groups {
                        built + index
                    } else {
                        index
                    };
                    let shift = options.stacking.offset(id, size, options.stack_ratio) * step;
                    let (dot_x, dot_y) = if flipped {
                        (base.0 + shift, base.1)
                    } else {
                        (base.0, base.1 - shift)
                    };
                    dots.x.push(dot_x);
                    dots.y.push(dot_y);
                    dots.rows.push(row);
                }
                built += drawn;
            }
        }
        Ok(vec![dots.into_node(layer.styles, options.dot_size * bin)?])
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        draw_key(styles, cell)
    }
}

/// Width of a bin in pixels, along x (`along_x`) or y of the layer.
fn bin_pixels(panel: &Panel, x: f64, y: f64, width: f64, along_x: bool) -> f64 {
    let from = panel.map(x, y);
    let to = if along_x {
        panel.map(x + width, y)
    } else {
        panel.map(x, y + width)
    };
    (to.0 - from.0).hypot(to.1 - from.1)
}

/// Stacks sideways from each (x, y).
pub struct YDotplotGeom;

impl YDotplotGeom {
    fn options(parameters: &Parameters) -> Result<Options> {
        Options::parse(parameters, "right", "left", Stacking::Center)
    }
}

impl Geom for YDotplotGeom {
    fn name(&self) -> &'static str {
        "ydotplot"
    }

    fn default_stat(&self) -> &'static str {
        "ydotplot"
    }

    /// lets-plot `GeomProto`: dodged by 0.95 unless groups stack together.
    fn default_position_given(&self, parameters: &Parameters) -> Result<DefaultPosition> {
        Ok(match parameters.boolean("stackgroups")? {
            Some(true) => DefaultPosition::IDENTITY,
            _ => DefaultPosition::dodge(0.95),
        })
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[
            Aesthetic::X,
            Aesthetic::Y,
            Aesthetic::Stacksize,
            Aesthetic::Binwidth,
        ]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        OPTIONS
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        SPANS
    }

    fn span_aesthetics(&self) -> &'static [Aesthetic] {
        SPANS
    }

    fn setup(&self, mut data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        Self::options(parameters)?;
        span_columns(&mut data, Aesthetic::X, Aesthetic::Xmin, Aesthetic::Xmax)?;
        span_columns(&mut data, Aesthetic::Y, Aesthetic::Ymin, Aesthetic::Ymax)?;
        Ok(data)
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let (Some([x, stack_size, bin_width]), Some(y)) =
            (columns(layer), layer.data.continuous(Aesthetic::Y))
        else {
            return Ok(Vec::new());
        };
        let options = Self::options(&layer.parameters)?;
        let rows: Vec<(usize, usize)> = drawable(layer, [x, stack_size, bin_width])
            .into_iter()
            .filter(|&(row, _)| at(y, row).is_finite())
            .collect();
        let Some(&(first, _)) = rows.first() else {
            return Ok(Vec::new());
        };
        let bin = bin_pixels(
            panel,
            at(x, first),
            at(y, first),
            at(bin_width, first),
            false,
        );
        let flipped = panel.flipped;
        let length = if flipped {
            panel.rect.height
        } else {
            panel.rect.width
        };
        let step = options.dot_size * options.stack_ratio * bin;
        let mut dots = Dots::default();
        for stack in stacks_by(rows, |row| vec![at(x, row).to_bits(), at(y, row).to_bits()]) {
            let full = stack.iter().map(|&(_, size)| size).sum();
            let bounded = options.bounded(full, length, bin);
            let mut built = 0;
            for (row, size) in stack {
                let drawn = options.bounded(built + size, length, bin) - built;
                let total = if options.across_groups {
                    bounded
                } else {
                    drawn
                };
                let base = panel.map(at(x, row), at(y, row));
                for index in 0..drawn {
                    let id = if options.across_groups {
                        built + index
                    } else {
                        index
                    };
                    let shift = options.stacking.offset(id, total, options.stack_ratio) * step;
                    let (dot_x, dot_y) = if flipped {
                        (base.0, base.1 + shift)
                    } else {
                        (base.0 + shift, base.1)
                    };
                    dots.x.push(dot_x);
                    dots.y.push(dot_y);
                    dots.rows.push(row);
                }
                built += drawn;
            }
        }
        Ok(vec![dots.into_node(layer.styles, options.dot_size * bin)?])
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        draw_key(styles, cell)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn up_starts_half_a_dot_off_the_axis() {
        assert_eq!(Stacking::Forward.offset(0, 3, 1.0), 0.5);
        assert_eq!(Stacking::Forward.offset(2, 3, 1.0), 2.5);
        assert_eq!(Stacking::Backward.offset(1, 3, 1.0), -1.5);
    }

    #[test]
    fn center_is_symmetric() {
        assert_eq!(Stacking::Center.offset(0, 2, 1.0), -0.5);
        assert_eq!(Stacking::Center.offset(1, 2, 1.0), 0.5);
        assert_eq!(Stacking::Center.offset(1, 3, 1.0), 0.0);
    }

    #[test]
    fn centerwhole_lands_on_whole_dots() {
        assert_eq!(Stacking::CenterWhole.offset(1, 3, 1.0), 0.5);
        assert_eq!(Stacking::CenterWhole.offset(1, 2, 1.0), 0.5);
    }

    #[test]
    fn a_tall_stack_is_cut_to_the_panel() {
        let options = Options {
            dot_size: 1.0,
            stack_ratio: 1.0,
            across_groups: false,
            stacking: Stacking::Forward,
        };
        assert_eq!(options.bounded(5, 100.0, 10.0), 5);
        assert_eq!(options.bounded(100, 100.0, 10.0), 12);
        assert_eq!(options.bounded(101, 100.0, 10.0), 11);
    }
}
