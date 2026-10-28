//! `geom_boxplot`: boxes from `lower` to `upper` with a midline at `middle` and
//! whiskers to `ymin`/`ymax`, as lets-plot's `BoxplotGeom`. Outliers are a
//! separate point layer (`stat="boxplot_outlier"`) that the Python API adds.
//!
//! Setup turns x into `xmin`/`xmax` spans of `width` (default 0.9, or the stat's
//! `varwidth` widths) x the x resolution, so dodging moves whole boxes. Drawing
//! order as lets-plot: every box, then every midline (outline width x `fatten`,
//! default 2), then the whiskers with fences `whisker_width` (default 0.5) of the
//! box wide. `alpha` fades the fill only.
//!
//! What it does not do (yet): `orientation="y"`, `width_unit` other than `res`.

use super::lines;
use super::point::POINT_UNIT_SIZE;
use super::{DrawLayer, Geom};
use crate::coord::Panel;
use crate::data::{Column, ContinuousColumn, LayerData};
use crate::parameters::Parameters;
use crate::position::DefaultPosition;
use crate::scene::{Color, Node, Path, PerItem, Rect, Rects, Stroke};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::{Error, Result};

/// lets-plot box style: pen outline, paper fill, size 0.75.
const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (
        Aesthetic::Color,
        StyleDefault::Color(Color::rgb(0x47, 0x47, 0x47)),
    ),
    (Aesthetic::Fill, StyleDefault::Color(Color::WHITE)),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Size, StyleDefault::Number(0.75)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

/// lets-plot `BoxplotGeom.DEF_FATTEN_MIDLINE`.
const DEFAULT_FATTEN: f64 = 2.0;
/// lets-plot `BoxplotGeom.DEF_WHISKER_WIDTH`.
const DEFAULT_WHISKER_WIDTH: f64 = 0.5;

/// Box-and-whisker plots.
pub struct BoxplotGeom;

impl Geom for BoxplotGeom {
    /// lets-plot: boxes under the cursor, the x axis tooltip; the statistics are
    /// side tooltips there and the first lines of the tooltip here.
    fn tooltip_behavior(&self) -> crate::tooltip::TooltipBehavior {
        crate::tooltip::TooltipBehavior {
            axis: (true, false),
            hidden: &[Aesthetic::Y],
            side: &[
                Aesthetic::Ymax,
                Aesthetic::Upper,
                Aesthetic::Middle,
                Aesthetic::Lower,
                Aesthetic::Ymin,
            ],
            ..crate::tooltip::TooltipBehavior::AREA
        }
    }

    fn hover_targets(
        &self,
        layer: &DrawLayer,
        panel: &Panel,
    ) -> Result<Option<crate::tooltip::HoverTargets>> {
        Ok(super::hover::rectangles(
            layer,
            panel,
            (Aesthetic::Ymin, Aesthetic::Ymax),
        ))
    }

    fn name(&self) -> &'static str {
        "boxplot"
    }

    fn default_stat(&self) -> &'static str {
        "boxplot"
    }

    fn default_position(&self) -> DefaultPosition {
        DefaultPosition::dodge(0.95)
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[
            Aesthetic::X,
            Aesthetic::Ymin,
            Aesthetic::Lower,
            Aesthetic::Middle,
            Aesthetic::Upper,
            Aesthetic::Ymax,
        ]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["width", "fatten", "whisker_width", "width_unit"]
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Xmin, Aesthetic::Xmax]
    }

    fn span_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Xmin, Aesthetic::Xmax]
    }

    fn setup(&self, mut data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        if let Some(unit) = parameters.string("width_unit")?
            && unit != "res"
        {
            return Err(Error::InvalidSpec(format!(
                "at '{}': width_unit '{unit}' is not supported by ggrust yet; supported: res",
                parameters.path_of("width_unit")
            )));
        }
        let Some(x) = data.continuous(Aesthetic::X).cloned() else {
            return Ok(data);
        };
        // The stat's `varwidth` widths, else the option, else 0.9.
        let constant = parameters.number("width")?.unwrap_or(0.9);
        let widths = data.continuous(Aesthetic::Width).cloned();
        let width_of = |row: usize| match &widths {
            Some(widths) => widths.get(row).unwrap_or(f64::NAN),
            None => constant,
        };
        let resolution = data.resolution(Aesthetic::X);
        let half = |row: usize| width_of(row) * resolution / 2.0;
        let location = |row: usize| x.get(row).unwrap_or(f64::NAN);
        let left = (0..x.len()).map(|row| location(row) - half(row)).collect();
        let right = (0..x.len()).map(|row| location(row) + half(row)).collect();
        let widths: Vec<f64> = (0..x.len()).map(width_of).collect();
        let continuous = |values| Column::Continuous(ContinuousColumn::from_values(values));
        data.insert(Aesthetic::Xmin, continuous(left))?;
        data.insert(Aesthetic::Xmax, continuous(right))?;
        data.insert(Aesthetic::Width, continuous(widths))?;
        Ok(data)
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let fatten = layer.parameters.number("fatten")?.unwrap_or(DEFAULT_FATTEN);
        let whisker_width = layer
            .parameters
            .number("whisker_width")?
            .unwrap_or(DEFAULT_WHISKER_WIDTH);
        let column = |aesthetic| {
            layer.data.continuous(aesthetic).ok_or_else(|| {
                Error::InvalidData(format!("boxplot is missing its '{aesthetic}' column"))
            })
        };
        let [x, xmin, xmax, ymin, lower, middle, upper, ymax] = [
            Aesthetic::X,
            Aesthetic::Xmin,
            Aesthetic::Xmax,
            Aesthetic::Ymin,
            Aesthetic::Lower,
            Aesthetic::Middle,
            Aesthetic::Upper,
            Aesthetic::Ymax,
        ]
        .map(column);
        let (x, xmin, xmax, ymin, lower, middle, upper, ymax) =
            (x?, xmin?, xmax?, ymin?, lower?, middle?, upper?, ymax?);
        let finite = |column: &ContinuousColumn, row: usize| {
            column.get(row).filter(|value| value.is_finite())
        };
        let rows = 0..layer.data.row_count();
        let fills = layer.styles.colors(Aesthetic::Fill)?;
        let alphas = layer.styles.numbers(Aesthetic::Alpha)?;

        let mut boxes: Vec<Node> = Vec::new();
        for row in rows.clone() {
            let corners = [
                finite(xmin, row),
                finite(lower, row),
                finite(xmax, row),
                finite(upper, row),
            ];
            let [Some(x0), Some(y0), Some(x1), Some(y1)] = corners else {
                continue;
            };
            let (left, bottom) = panel.map(x0, y0);
            let (right, top) = panel.map(x1, y1);
            let mut rect = Rect {
                x: left.min(right),
                y: top.min(bottom),
                width: (right - left).abs(),
                height: (bottom - top).abs(),
            };
            // lets-plot `GeomHelper.toClient`: keep flat boxes visible.
            if rect.width == 0.0 {
                rect.width = 0.1;
            }
            if rect.height == 0.0 {
                rect.height = 0.1;
            }
            let fill = fills.get(row).with_opacity(alphas.get(row));
            boxes.push(Node::Rects(Rects {
                rects: vec![rect],
                fill: PerItem::Uniform((fill.alpha > 0).then_some(fill)),
                stroke: lines::stroke_with_alpha(layer.styles, row, false)?,
            }));
        }

        let mut midlines = Vec::new();
        for row in rows.clone() {
            let (Some(x0), Some(x1), Some(y)) =
                (finite(xmin, row), finite(xmax, row), finite(middle, row))
            else {
                continue;
            };
            let stroke = lines::stroke_with_alpha(layer.styles, row, false)?.map(|stroke| Stroke {
                width: stroke.width * fatten,
                dash: stroke.dash.iter().map(|length| length * fatten).collect(),
                ..stroke
            });
            midlines.push(line(panel, (x0, y), (x1, y), stroke));
        }

        let mut whiskers = Vec::new();
        for row in rows {
            let Some(center) = finite(x, row) else {
                continue;
            };
            // Half the (dodged) box width, so fences shrink with dodging.
            let half_fence = match (finite(xmin, row), finite(xmax, row)) {
                (Some(x0), Some(x1)) => (x1 - x0) / 2.0 * whisker_width,
                _ => 0.0,
            };
            let stroke = lines::stroke_with_alpha(layer.styles, row, false)?;
            for (hinge, fence) in [(lower, ymin), (upper, ymax)] {
                let (Some(hinge), Some(fence)) = (finite(hinge, row), finite(fence, row)) else {
                    continue;
                };
                whiskers.push(line(
                    panel,
                    (center, hinge),
                    (center, fence),
                    stroke.clone(),
                ));
                whiskers.push(line(
                    panel,
                    (center - half_fence, fence),
                    (center + half_fence, fence),
                    stroke.clone(),
                ));
            }
        }
        boxes.extend(midlines.into_iter().flatten());
        boxes.extend(whiskers.into_iter().flatten());
        Ok(boxes)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        let side = styles.numbers(Aesthetic::Size)?.get(0).max(0.0) * POINT_UNIT_SIZE + 2.0;
        Ok((side, side))
    }

    /// lets-plot `BoxLegendKeyElementFactory`: a box 80% of the key wide and 60%
    /// tall, a midline, and whiskers above and below (1 px key margin).
    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        const WHISKER: f64 = 0.2;
        let stroke = lines::stroke_with_alpha(styles, 0, false)?;
        let stroke_width = stroke.as_ref().map_or(0.0, |stroke| stroke.width);
        let (size_x, size_y) = (cell.width - 2.0, cell.height - 2.0);
        let width = (size_x - stroke_width) * 0.8;
        let height = size_y - stroke_width;
        let x = cell.x + 1.0 + (size_x - width) / 2.0;
        let y = cell.y + 1.0 + stroke_width / 2.0;
        let alpha = styles.numbers(Aesthetic::Alpha)?.get(0);
        let fill = styles.colors(Aesthetic::Fill)?.get(0).with_opacity(alpha);
        let middle_x = x + width / 2.0;
        let segment = |from: (f64, f64), to: (f64, f64)| {
            stroke.clone().map(|stroke| {
                Node::Path(Path {
                    points: vec![from, to],
                    closed: false,
                    fill: None,
                    stroke: Some(stroke),
                })
            })
        };
        let mut nodes = vec![Node::Rects(Rects {
            rects: vec![Rect {
                x,
                y: y + height * WHISKER,
                width,
                height: height * (1.0 - 2.0 * WHISKER),
            }],
            fill: PerItem::Uniform((fill.alpha > 0).then_some(fill)),
            stroke: stroke.clone(),
        })];
        nodes.extend(
            [
                segment((x, y + height / 2.0), (x + width, y + height / 2.0)),
                segment(
                    (middle_x, y + height * (1.0 - WHISKER)),
                    (middle_x, y + height),
                ),
                segment((middle_x, y), (middle_x, y + height * WHISKER)),
            ]
            .into_iter()
            .flatten(),
        );
        Ok(nodes)
    }
}

/// A two-point line in data units; `None` when invisible or off the coord.
fn line(panel: &Panel, from: (f64, f64), to: (f64, f64), stroke: Option<Stroke>) -> Option<Node> {
    let stroke = stroke?;
    let (from, to) = (panel.map(from.0, from.1), panel.map(to.0, to.1));
    [from.0, from.1, to.0, to.1]
        .iter()
        .all(|value| value.is_finite())
        .then(|| {
            Node::Path(Path {
                points: vec![from, to],
                closed: false,
                fill: None,
                stroke: Some(stroke),
            })
        })
}
