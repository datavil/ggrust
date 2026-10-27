//! `geom_violin`: mirrored densities per x, as lets-plot's `ViolinGeom`.
//!
//! Setup turns x into `xmin`/`xmax` spans of `width` (default 1) x the x
//! resolution, so dodging moves and narrows whole violins; each point then lies
//! `violinwidth` x half that span from the centre. A violin is drawn per group
//! and x: a filled band (no outline, `alpha` on the fill) and its two sides as
//! open lines (no top or bottom edge). When fill or colour changes along a violin
//! (`fill='..quantile..'`), each quantile band is drawn on its own.
//!
//! Options: `quantile_lines` (lines across at the `quantiles`), `show_half`
//! (-1 left half, 1 right half, 0 both).
//!
//! What it does not do (yet): `orientation="y"`.

use std::collections::HashMap;

use super::bar::{draw_key, key_size};
use super::lines;
use super::{DrawLayer, Geom};
use crate::coord::Panel;
use crate::data::{Column, ContinuousColumn, LayerData, resolution};
use crate::parameters::Parameters;
use crate::position::DefaultPosition;
use crate::scene::{Color, Node, Path, Rect};
use crate::spec::Aesthetic;
use crate::stat::ydensity_quantiles;
use crate::style::{StyleDefault, Styles};
use crate::{Error, Result};

/// lets-plot violin style: pen outline, paper fill, size 0.75.
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

/// Violin plots.
pub struct ViolinGeom;

impl Geom for ViolinGeom {
    fn name(&self) -> &'static str {
        "violin"
    }

    fn default_stat(&self) -> &'static str {
        "ydensity"
    }

    fn default_position(&self) -> DefaultPosition {
        DefaultPosition::dodge(0.95)
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Violinwidth]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["width", "quantile_lines", "show_half", "quantiles"]
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Xmin, Aesthetic::Xmax]
    }

    fn setup(&self, mut data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        let Some(x) = data.continuous(Aesthetic::X).cloned() else {
            return Ok(data);
        };
        let width = parameters.number("width")?.unwrap_or(1.0);
        let half = width * resolution(&x) / 2.0;
        let continuous = |values| Column::Continuous(ContinuousColumn::from_values(values));
        let location = |row: usize| x.get(row).unwrap_or(f64::NAN);
        let row_count = data.row_count();
        data.insert(
            Aesthetic::Xmin,
            continuous((0..row_count).map(|row| location(row) - half).collect()),
        )?;
        data.insert(
            Aesthetic::Xmax,
            continuous((0..row_count).map(|row| location(row) + half).collect()),
        )?;
        data.insert(Aesthetic::Width, continuous(vec![width; row_count]))?;
        Ok(data)
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let quantile_lines = layer.parameters.boolean("quantile_lines")?.unwrap_or(false);
        let show_half = layer.parameters.number("show_half")?.unwrap_or(0.0);
        let quantiles = ydensity_quantiles(&layer.parameters)?;
        let (left_sign, right_sign) = (
            if show_half > 0.0 { 0.0 } else { -1.0 },
            if show_half < 0.0 { 0.0 } else { 1.0 },
        );
        let column = |aesthetic| {
            layer.data.continuous(aesthetic).ok_or_else(|| {
                Error::InvalidData(format!("violin is missing its '{aesthetic}' column"))
            })
        };
        let (x, y, xmin, xmax, violin_width) = (
            column(Aesthetic::X)?,
            column(Aesthetic::Y)?,
            column(Aesthetic::Xmin)?,
            column(Aesthetic::Xmax)?,
            column(Aesthetic::Violinwidth)?,
        );
        let band = layer.data.continuous(Aesthetic::Quantile);
        let value = |column: &ContinuousColumn, row: usize| {
            column.get(row).filter(|value| value.is_finite())
        };
        // Rows of each violin (group and x), in order of first appearance.
        let mut violins: Vec<Vec<usize>> = Vec::new();
        let mut slot_of: HashMap<(u32, u64), usize> = HashMap::new();
        for row in 0..layer.data.row_count() {
            let defined = [x, y, xmin, xmax, violin_width]
                .iter()
                .all(|column| value(column, row).is_some());
            if !defined {
                continue;
            }
            let key = (
                layer.data.group_of(row),
                x.get(row).unwrap_or(0.0).to_bits(),
            );
            let slot = *slot_of.entry(key).or_insert_with(|| {
                violins.push(Vec::new());
                violins.len() - 1
            });
            violins[slot].push(row);
        }
        // A side of a point: centre + sign x half the (dodged) span x violinwidth.
        let side = |row: usize, sign: f64| -> (f64, f64) {
            let (low, high) = (xmin.get(row).unwrap_or(0.0), xmax.get(row).unwrap_or(0.0));
            let center = x.get(row).unwrap_or(0.0);
            let offset = (high - low) / 2.0 * sign * violin_width.get(row).unwrap_or(0.0);
            panel.map(center + offset, y.get(row).unwrap_or(0.0))
        };
        let band_of = |row: usize| band.and_then(|band| band.get(row)).unwrap_or(f64::NAN);
        let fills = layer.styles.colors(Aesthetic::Fill)?;
        let colors = layer.styles.colors(Aesthetic::Color)?;
        let alphas = layer.styles.numbers(Aesthetic::Alpha)?;

        let mut nodes = Vec::new();
        for mut rows in violins {
            // lets-plot `ordered_Y`, then bands when paint varies along the violin.
            rows.sort_by(|left, right| {
                y.get(*left)
                    .unwrap_or(0.0)
                    .total_cmp(&y.get(*right).unwrap_or(0.0))
            });
            let varies = rows.iter().any(|&row| {
                fills.get(row) != fills.get(rows[0]) || colors.get(row) != colors.get(rows[0])
            });
            let runs = if varies {
                split_by_band(&rows, band_of, y)
            } else {
                vec![rows.clone()]
            };
            for run in &runs {
                let first = run[0];
                let left: Vec<(f64, f64)> = run.iter().map(|&row| side(row, left_sign)).collect();
                let right: Vec<(f64, f64)> = run.iter().map(|&row| side(row, right_sign)).collect();
                let fill = fills.get(first).with_opacity(alphas.get(first));
                let mut outline = left.clone();
                outline.extend(right.iter().rev());
                nodes.push(Node::Path(Path {
                    points: outline,
                    closed: true,
                    fill: (fill.alpha > 0).then_some(fill),
                    stroke: None,
                }));
                let stroke = lines::stroke_with_alpha(layer.styles, first, false)?;
                for points in [left, right] {
                    if stroke.is_some() && points.len() > 1 {
                        nodes.push(Node::Path(Path {
                            points,
                            closed: false,
                            fill: None,
                            stroke: stroke.clone(),
                        }));
                    }
                }
            }
            if quantile_lines {
                for row in quantile_line_rows(&rows, band_of, y, &quantiles) {
                    let Some(stroke) = lines::stroke_with_alpha(layer.styles, row, false)? else {
                        continue;
                    };
                    nodes.push(Node::Path(Path {
                        points: vec![side(row, left_sign), side(row, right_sign)],
                        closed: false,
                        fill: None,
                        stroke: Some(stroke),
                    }));
                }
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

/// Rows sorted by (band, y), split where the band changes (lets-plot
/// `QuantilesHelper.splitByQuantiles`); a missing band joins its run.
fn split_by_band(
    rows: &[usize],
    band_of: impl Fn(usize) -> f64,
    y: &ContinuousColumn,
) -> Vec<Vec<usize>> {
    let mut sorted = rows.to_vec();
    sorted.sort_by(|left, right| {
        band_of(*left).total_cmp(&band_of(*right)).then(
            y.get(*left)
                .unwrap_or(0.0)
                .total_cmp(&y.get(*right).unwrap_or(0.0)),
        )
    });
    let mut runs: Vec<Vec<usize>> = Vec::new();
    for (index, &row) in sorted.iter().enumerate() {
        let joins = index > 0 && {
            let (previous, current) = (band_of(sorted[index - 1]), band_of(row));
            !(previous.is_finite() && current.is_finite()) || previous == current
        };
        match runs.last_mut() {
            Some(run) if joins => run.push(row),
            _ => runs.push(vec![row]),
        }
    }
    runs
}

/// lets-plot `QuantilesHelper.getQuantileLineElements`: from the top, the first
/// point of each quantile band (quantiles descending) carries its line.
fn quantile_line_rows(
    rows: &[usize],
    band_of: impl Fn(usize) -> f64,
    y: &ContinuousColumn,
    quantiles: &[f64],
) -> Vec<usize> {
    let mut sorted = rows.to_vec();
    sorted.sort_by(|left, right| {
        band_of(*left).total_cmp(&band_of(*right)).then(
            y.get(*left)
                .unwrap_or(0.0)
                .total_cmp(&y.get(*right).unwrap_or(0.0)),
        )
    });
    sorted.reverse();
    let mut descending = quantiles.to_vec();
    descending.sort_by(|left, right| right.total_cmp(left));
    let mut found = Vec::new();
    let mut index = 0;
    for quantile in descending {
        while index < sorted.len() {
            let row = sorted[index];
            index += 1;
            if band_of(row) == quantile {
                found.push(row);
                break;
            }
        }
    }
    found
}
