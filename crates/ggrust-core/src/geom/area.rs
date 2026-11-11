//! `geom_area`, `geom_density` and `geom_ribbon`: filled bands along x, as
//! lets-plot's `AreaGeom`, `DensityGeom` and `RibbonGeom`.
//!
//! Rows sort by x within a group (stable) and split where a bound is missing;
//! single-row pieces are dropped. Each band is filled from its first row (`alpha`
//! on the fill only) and outlined by its edge lines, split where their stroke
//! style changes (lets-plot `splitByStyle`).
//!
//! - Area: from `y` down to `ymin`, which setup sets to 0 so `position="stack"`
//!   (the default) lifts it to the stack below. Groups are drawn last group first,
//!   each as its bands then its top lines. With a `quantile` column (from
//!   `stat="density"`) a group whose fill varies splits into its quantile bands,
//!   so `fill="..quantile.."` colors each band (one fill: drawn whole, no seams); `quantile_lines` draws a line from
//!   the top to the base at each quantile. The y axis includes 0.
//! - Density: an area with `stat="density"` and `position="identity"`.
//! - Ribbon: from `ymin` to `ymax`. All bands first, then all `ymax` lines, then
//!   all `ymin` lines.
//!
//! `outline_type` (ggplot2) picks the edge lines: `upper` (area and density
//! default), `lower`, `both` (ribbon default) or `full` (the closed band).
//!
//! What it does not do (yet): split the fill where a mapped fill varies within a
//! group (lets-plot does; ggrust fills from the first row), resampling in non-linear coordinates, `flat`, horizontal ribbons.

use super::area_ridges::STYLE_DEFAULTS;
use super::bar::{draw_key, key_size};
use super::hover::VertexBuilder;
use super::lines;
use super::quantiles::{quantile_line_rows, split_by_band};
use super::{DrawLayer, Geom};
use crate::Result;
use crate::coord::Panel;
use crate::data::{Column, ContinuousColumn, LayerData};
use crate::parameters::Parameters;
use crate::position::DefaultPosition;
use crate::scene::{Node, Path, Rect};
use crate::spec::Aesthetic;
use crate::stat::ydensity_quantiles;
use crate::style::{StyleDefault, Styles};
use crate::tooltip::{HoverTargets, TooltipBehavior};

/// Areas under a line.
pub struct AreaGeom;

/// Bands between two lines.
pub struct RibbonGeom;

/// Kernel density areas (`geom_density`: an area with `stat="density"` and
/// `position="identity"`).
pub struct DensityGeom;

impl Geom for AreaGeom {
    fn name(&self) -> &'static str {
        "area"
    }

    fn default_position(&self) -> DefaultPosition {
        DefaultPosition::named("stack")
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn includes_zero_y(&self) -> bool {
        true
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Ymin]
    }

    fn setup(&self, mut data: LayerData, _parameters: &Parameters) -> Result<LayerData> {
        let zeros = vec![0.0; data.row_count()];
        data.insert(
            Aesthetic::Ymin,
            Column::Continuous(ContinuousColumn::from_values(zeros)),
        )?;
        Ok(data)
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["quantile_lines", "quantiles", "outline_type"]
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        draw_areas(layer, panel)
    }

    /// lets-plot `xUnivariateFunction(HOVER)` over the top line.
    fn tooltip_behavior(&self) -> TooltipBehavior {
        TooltipBehavior::ALONG_X
    }

    fn hover_targets(&self, layer: &DrawLayer, panel: &Panel) -> Result<Option<HoverTargets>> {
        Ok(super::hover::vertices(layer, panel))
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        draw_key(styles, cell)
    }
}

impl Geom for RibbonGeom {
    fn name(&self) -> &'static str {
        "ribbon"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Ymin, Aesthetic::Ymax]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["outline_type"]
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let (Some(x), Some(low), Some(high)) = (
            layer.data.continuous(Aesthetic::X),
            layer.data.continuous(Aesthetic::Ymin),
            layer.data.continuous(Aesthetic::Ymax),
        ) else {
            return Ok(Vec::new());
        };
        let outline = OutlineType::parse(&layer.parameters, OutlineType::Both)?;
        let groups = sorted_groups(layer, x);
        let mut nodes = Vec::new();
        for rows in &groups {
            for piece in pieces(rows, |row| finite(low, row) && finite(high, row)) {
                nodes.push(band_node(layer, panel, &piece, x, high, low)?);
                if outline == OutlineType::Full {
                    nodes.extend(full_outline(layer, panel, &piece, x, high, low)?);
                }
            }
        }
        // Each edge splits on its own bound only (lets-plot `TO_LOCATION_X_YMAX`).
        for bound in outline.bounds(high, low) {
            for rows in &groups {
                for piece in pieces(rows, |row| finite(bound, row)) {
                    nodes.extend(edge(layer, panel, &piece, x, bound)?);
                }
            }
        }
        Ok(nodes)
    }

    /// lets-plot: a target at every row's band middle, `ymax`/`ymin` first.
    fn tooltip_behavior(&self) -> TooltipBehavior {
        TooltipBehavior {
            lookup: Some(crate::tooltip::Lookup::Nearest),
            axis: (true, false),
            side: &[Aesthetic::Ymax, Aesthetic::Ymin],
            ..TooltipBehavior::NONE
        }
    }

    fn hover_targets(&self, layer: &DrawLayer, panel: &Panel) -> Result<Option<HoverTargets>> {
        let (Some(x_column), Some(low), Some(high)) = (
            layer.data.continuous(Aesthetic::X),
            layer.data.continuous(Aesthetic::Ymin),
            layer.data.continuous(Aesthetic::Ymax),
        ) else {
            return Ok(None);
        };
        let mut targets = VertexBuilder::default();
        for row in 0..layer.data.row_count() {
            let (Some(value_x), Some(value_low), Some(value_high)) =
                (x_column.get(row), low.get(row), high.get(row))
            else {
                continue;
            };
            let middle = panel.map(value_x, (value_low + value_high) / 2.0);
            targets.push(middle, layer.data.group_of(row), row);
        }
        Ok(Some(targets.finish()))
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        draw_key(styles, cell)
    }
}

impl Geom for DensityGeom {
    fn name(&self) -> &'static str {
        "density"
    }

    fn default_stat(&self) -> &'static str {
        "density"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["quantile_lines", "quantiles", "outline_type"]
    }

    fn includes_zero_y(&self) -> bool {
        true
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        AreaGeom.setup_aesthetics()
    }

    fn setup(&self, data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        AreaGeom.setup(data, parameters)
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        draw_areas(layer, panel)
    }

    fn tooltip_behavior(&self) -> TooltipBehavior {
        AreaGeom.tooltip_behavior()
    }

    fn hover_targets(&self, layer: &DrawLayer, panel: &Panel) -> Result<Option<HoverTargets>> {
        AreaGeom.hover_targets(layer, panel)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        draw_key(styles, cell)
    }
}

/// Areas of every group, last group first (lets-plot `sortedByDescending(group)`).
/// A group splits into its quantile bands (lets-plot `splitByQuantiles`), each
/// drawn as its bands then its top lines; `quantile_lines` then drops a line
/// from the top to the base at each quantile.
fn draw_areas(layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
    let (Some(x), Some(top), Some(base)) = (
        layer.data.continuous(Aesthetic::X),
        layer.data.continuous(Aesthetic::Y),
        layer.data.continuous(Aesthetic::Ymin),
    ) else {
        return Ok(Vec::new());
    };
    let quantile_lines = layer.parameters.boolean("quantile_lines")?.unwrap_or(false);
    let quantiles = ydensity_quantiles(&layer.parameters)?;
    let outline = OutlineType::parse(&layer.parameters, OutlineType::Upper)?;
    let band = layer.data.continuous(Aesthetic::Quantile);
    let band_of = |row: usize| band.and_then(|band| band.get(row)).unwrap_or(f64::NAN);
    let mut nodes = Vec::new();
    for rows in sorted_groups(layer, x).into_iter().rev() {
        // Bands with one fill are drawn whole: adjacent pieces leave
        // anti-aliasing seams (lets-plot #985).
        let runs = match band.is_some() && !same_fill(layer, &rows)? {
            true => split_by_band(&rows, band_of, x),
            false => vec![rows.clone()],
        };
        for run in &runs {
            let pieces = pieces(run, |row| finite(top, row) && finite(base, row));
            for piece in &pieces {
                nodes.push(band_node(layer, panel, piece, x, top, base)?);
            }
            for piece in &pieces {
                if outline == OutlineType::Full {
                    nodes.extend(full_outline(layer, panel, piece, x, top, base)?);
                }
                for bound in outline.bounds(top, base) {
                    nodes.extend(edge(layer, panel, piece, x, bound)?);
                }
            }
        }
        if quantile_lines && band.is_some() {
            for row in quantile_line_rows(&rows, band_of, x, &quantiles) {
                let (Some(value_x), Some(value_top), Some(value_base)) =
                    (x.get(row), top.get(row), base.get(row))
                else {
                    continue;
                };
                let Some(stroke) = lines::stroke_with_alpha(layer.styles, row, false)? else {
                    continue;
                };
                nodes.push(Node::Path(Path {
                    points: vec![
                        panel.map(value_x, value_top),
                        panel.map(value_x, value_base),
                    ],
                    closed: false,
                    fill: None,
                    stroke: Some(stroke),
                }));
            }
        }
    }
    Ok(nodes)
}

/// Whether every row has the fill and alpha of the first.
fn same_fill(layer: &DrawLayer, rows: &[usize]) -> Result<bool> {
    let (fill, alpha) = (
        layer.styles.colors(Aesthetic::Fill)?,
        layer.styles.numbers(Aesthetic::Alpha)?,
    );
    let Some(&first) = rows.first() else {
        return Ok(true);
    };
    let alpha_of = |row: usize| alpha.get(row).to_bits();
    Ok(rows
        .iter()
        .all(|&row| fill.get(row) == fill.get(first) && alpha_of(row) == alpha_of(first)))
}

pub(super) fn finite(column: &ContinuousColumn, row: usize) -> bool {
    column.get(row).is_some_and(f64::is_finite)
}

/// Rows with a finite x per group, in group order, each sorted by x (stable).
pub(super) fn sorted_groups(layer: &DrawLayer, x: &ContinuousColumn) -> Vec<Vec<usize>> {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for row in 0..layer.data.row_count() {
        if !finite(x, row) {
            continue;
        }
        let group = layer.data.group_of(row) as usize;
        if groups.len() <= group {
            groups.resize_with(group + 1, Vec::new);
        }
        groups[group].push(row);
    }
    for rows in &mut groups {
        rows.sort_by(|left, right| {
            x.get(*left)
                .unwrap_or(0.0)
                .total_cmp(&x.get(*right).unwrap_or(0.0))
        });
    }
    groups.retain(|rows| !rows.is_empty());
    groups
}

/// Runs of rows where `valid` holds, of at least two rows (lets-plot
/// `splitByNull`, then single points dropped).
pub(super) fn pieces(rows: &[usize], valid: impl Fn(usize) -> bool) -> Vec<Vec<usize>> {
    rows.split(|&row| !valid(row))
        .filter(|run| run.len() >= 2)
        .map(<[usize]>::to_vec)
        .collect()
}

/// The filled band of `piece`: `upper` forward, `lower` back.
pub(super) fn band_node(
    layer: &DrawLayer,
    panel: &Panel,
    piece: &[usize],
    x: &ContinuousColumn,
    upper: &ContinuousColumn,
    lower: &ContinuousColumn,
) -> Result<Node> {
    let at = |column: &ContinuousColumn, row: usize| {
        panel.map(x.get(row).unwrap_or(0.0), column.get(row).unwrap_or(0.0))
    };
    let mut points: Vec<(f64, f64)> = piece.iter().map(|&row| at(upper, row)).collect();
    points.extend(piece.iter().rev().map(|&row| at(lower, row)));
    let first = piece[0];
    let alpha = layer.styles.numbers(Aesthetic::Alpha)?.get(first);
    let fill = layer
        .styles
        .colors(Aesthetic::Fill)?
        .get(first)
        .with_opacity(alpha);
    Ok(Node::Path(Path {
        points,
        closed: true,
        fill: (fill.alpha > 0).then_some(fill),
        stroke: None,
    }))
}

/// ggplot2 `outline.type`: which edges of a band get a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OutlineType {
    /// The upper edge (area and density default).
    Upper,
    /// The lower edge.
    Lower,
    /// Upper and lower edges (ribbon default).
    Both,
    /// The closed band outline, ends included.
    Full,
}

impl OutlineType {
    const NAMES: [&'static str; 4] = ["upper", "lower", "both", "full"];

    /// `outline_type` from the layer, else `default`.
    pub(super) fn parse(parameters: &Parameters, default: OutlineType) -> Result<OutlineType> {
        match parameters.string("outline_type")? {
            None => Ok(default),
            Some("upper") => Ok(OutlineType::Upper),
            Some("lower") => Ok(OutlineType::Lower),
            Some("both") => Ok(OutlineType::Both),
            Some("full") => Ok(OutlineType::Full),
            Some(other) => Err(crate::Error::spec(
                parameters.path_of("outline_type"),
                format!(
                    "unknown outline_type '{other}'; expected one of {}{}",
                    OutlineType::NAMES.join(", "),
                    crate::suggest::did_you_mean(other, OutlineType::NAMES)
                ),
            )),
        }
    }

    /// The bounds that get an edge line, upper first (`full` draws its own).
    fn bounds<'c>(
        self,
        upper: &'c ContinuousColumn,
        lower: &'c ContinuousColumn,
    ) -> Vec<&'c ContinuousColumn> {
        match self {
            OutlineType::Upper => vec![upper],
            OutlineType::Lower => vec![lower],
            OutlineType::Both => vec![upper, lower],
            OutlineType::Full => Vec::new(),
        }
    }
}

/// The closed outline of the band of `piece` (`outline_type="full"`), stroked with
/// the first row's line style (ggplot2 draws the polygon border).
fn full_outline(
    layer: &DrawLayer,
    panel: &Panel,
    piece: &[usize],
    x: &ContinuousColumn,
    upper: &ContinuousColumn,
    lower: &ContinuousColumn,
) -> Result<Option<Node>> {
    let Some(stroke) = lines::stroke_with_alpha(layer.styles, piece[0], false)? else {
        return Ok(None);
    };
    let at = |column: &ContinuousColumn, row: usize| {
        panel.map(x.get(row).unwrap_or(0.0), column.get(row).unwrap_or(0.0))
    };
    let mut points: Vec<(f64, f64)> = piece.iter().map(|&row| at(upper, row)).collect();
    points.extend(piece.iter().rev().map(|&row| at(lower, row)));
    Ok(Some(Node::Path(Path {
        points,
        closed: true,
        fill: None,
        stroke: Some(stroke),
    })))
}

/// The edge line of `piece` along `bound`, without `alpha` (lets-plot
/// `setAlphaEnabled(false)`), split where the stroke style changes.
pub(super) fn edge(
    layer: &DrawLayer,
    panel: &Panel,
    piece: &[usize],
    x: &ContinuousColumn,
    bound: &ContinuousColumn,
) -> Result<Vec<Node>> {
    let points: Vec<(f64, f64)> = piece
        .iter()
        .map(|&row| panel.map(x.get(row).unwrap_or(0.0), bound.get(row).unwrap_or(0.0)))
        .collect();
    let strokes = piece
        .iter()
        .map(|&row| lines::stroke_with_alpha(layer.styles, row, false))
        .collect::<Result<Vec<_>>>()?;
    Ok(lines::styled_paths(&points, &strokes)
        .into_iter()
        .map(Node::Path)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pieces_split_on_invalid_rows_and_drop_single_points() {
        let rows = [0, 1, 2, 3, 4, 5];
        let runs = pieces(&rows, |row| row != 2 && row != 4);
        assert_eq!(runs, vec![vec![0, 1]]);
        assert_eq!(pieces(&rows, |_| true), vec![rows.to_vec()]);
    }
}
