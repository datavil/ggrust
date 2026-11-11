//! Faceted layout (lets-plot `FacetedPlotLayout`, `FixedScalesTilesLayouter`,
//! `FreeScalesTilesLayouter`): panels in a grid, strips above them (every panel of
//! a `facet_wrap`, the top row of a `facet_grid`) and row strips right of the right
//! column (`facet_grid`), their text turned to read top to bottom.
//!
//! Every panel has the same size, except with `facet_grid(space="free")`, where
//! column widths and row heights follow their scale ranges (ggplot2). The space left by titles and legends is split as
//! for a single plot, minus the strips, the axes and `panel_spacing` between panels.
//! A grid row takes the strips of its panel with the most labels.
//! A grid column is as wide as its widest shown y axis, a grid row's x axis band as
//! thick as its thickest; the last panel of an unfinished column draws its x axis in
//! the empty space below it, as lets-plot does. With shared (fixed) scales the
//! breaks are chosen once, for the first panel, and reused.
//!
//! What it does not do: strip text angles other than lets-plot's defaults, and
//! `coord_fixed` with free scales.

use super::*;

/// One panel to lay out.
#[derive(Debug, Clone)]
pub struct FacetPanel<'a> {
    /// The panel's x scale (shared ones are the same scale for every panel).
    pub x_scale: &'a PositionScale,
    /// The panel's y scale.
    pub y_scale: &'a PositionScale,
    /// Strip labels, top first.
    pub labels: &'a [Label],
    /// Row strip label right of the panel (`facet_grid`).
    pub row_label: Option<&'a Label>,
    /// Grid column.
    pub column: usize,
    /// Grid row.
    pub row: usize,
    /// The sides the panel draws its x axis on (bottom: the last panel of its
    /// column or free x; top: the first row or free x; each only where the
    /// scale's `position` puts the axis).
    pub x_axis: AxisSides,
    /// The sides the panel draws its y axis on (left: the first column or free
    /// y; right: the last panel of its row or free y).
    pub y_axis: AxisSides,
    /// The last panel of its grid column.
    pub is_bottom: bool,
    /// The last panel of its grid row.
    pub is_row_end: bool,
}

/// The panels of a faceted plot.
#[derive(Debug, Clone)]
pub struct FacetGrid<'a> {
    /// Grid columns.
    pub columns: usize,
    /// Grid rows.
    pub rows: usize,
    /// Panels in any order.
    pub panels: Vec<FacetPanel<'a>>,
    /// Every panel has the same x scale.
    pub shared_x: bool,
    /// Every panel has the same y scale.
    pub shared_y: bool,
    /// `facet_grid(space=)`: grid columns (x) and rows (y) sized in proportion to
    /// their scale's expanded range, as ggplot2.
    pub free_space: (bool, bool),
}

/// One laid-out panel.
#[derive(Debug, Clone, PartialEq)]
pub struct TileLayout {
    /// The panel, its breaks and its x axis band (`legend_space` and `content` as
    /// the plot's).
    pub layout: PlotLayout,
    /// The sides the x and y axes are drawn on.
    pub axes: [AxisSides; 2],
    /// Strip labels, top first.
    pub labels: Vec<Label>,
    /// Text lines each strip has room for (the most lines at its place in the
    /// row), top first.
    pub strip_lines: Vec<usize>,
    /// Row strip label right of the panel.
    pub row_label: Option<Label>,
    /// Text lines the row strips have room for (the row label with most lines).
    pub row_label_lines: usize,
}

/// A laid-out faceted plot.
#[derive(Debug, Clone, PartialEq)]
pub struct FacetLayout {
    /// The whole plot: `panel` is the union of the panels, `axis_bands` the outer
    /// rows' and columns'. Titles and legends are placed from it.
    pub plot: PlotLayout,
    /// The panels, in [`FacetGrid::panels`] order.
    pub tiles: Vec<TileLayout>,
}

/// Lay out the panels of `grid` inside `area`. With an `aspect` ratio and shared
/// scales the panels keep it, and the grid is centred in `area`.
pub fn layout_facets(
    area: Rect,
    theme: &Theme,
    titles: &Titles,
    grid: &FacetGrid,
    legend_space: Margins,
    aspect: Option<f64>,
    panel_min_size: f64,
) -> FacetLayout {
    let layout = layout_grid(area, theme, titles, grid, legend_space, panel_min_size);
    let (Some(ratio), Some(first), Some(tile)) = (
        aspect.filter(|ratio| ratio.is_finite() && *ratio > 0.0),
        grid.panels.first(),
        layout.tiles.first(),
    ) else {
        return layout;
    };
    if !(grid.shared_x && grid.shared_y) {
        return layout;
    }
    let span = |scale: &PositionScale| {
        let (low, high) = scale.expanded_domain();
        high - low
    };
    let (x_span, y_span) = (span(first.x_scale), span(first.y_scale));
    let (width, height) = (tile.layout.panel.width, tile.layout.panel.height);
    let wanted_height = ratio * width / x_span * y_span;
    let shrunk = if wanted_height < height {
        let cut = (height - wanted_height) * grid.rows as f64;
        Rect {
            y: area.y + cut / 2.0,
            height: area.height - cut,
            ..area
        }
    } else {
        let cut = (width - height * x_span / (ratio * y_span)) * grid.columns as f64;
        Rect {
            x: area.x + cut / 2.0,
            width: area.width - cut,
            ..area
        }
    };
    let mut relaid = if shrunk.width > 0.0 && shrunk.height > 0.0 && shrunk != area {
        layout_grid(shrunk, theme, titles, grid, legend_space, panel_min_size)
    } else {
        layout
    };
    // As for a single plot: fit each panel exactly, centred in its space.
    for tile in &mut relaid.tiles {
        tile.layout = fit_ratio(tile.layout.clone(), ratio, x_span, y_span);
    }
    relaid.plot.panel = union(relaid.tiles.iter().map(|tile| tile.layout.panel));
    relaid
}

/// [`layout_facets`] without an aspect ratio.
fn layout_grid(
    area: Rect,
    theme: &Theme,
    titles: &Titles,
    grid: &FacetGrid,
    legend_space: Margins,
    panel_min_size: f64,
) -> FacetLayout {
    let inset = theme.plot_inset;
    let content = plot_content(area, theme, titles);
    let axis_sides = grid
        .panels
        .first()
        .map_or([AxisSides::default(); 2], |panel| {
            [panel.x_scale.sides(), panel.y_scale.sides()]
        });
    let far_sides = grid.panels.first().map_or([None, None], |panel| {
        [
            panel.x_scale.secondary_on_far_side(),
            panel.y_scale.secondary_on_far_side(),
        ]
    });
    let (x_title_top, x_title_bottom) =
        axis_title_bands(theme, titles, 0, axis_sides[0], far_sides[0]);
    let (y_title_right, y_title_left) =
        axis_title_bands(theme, titles, 1, axis_sides[1], far_sides[1]);
    let top = content.y + headings_band(theme, titles) + legend_space.top + x_title_top + inset.top;
    let bottom = content.y + content.height - legend_space.bottom - x_title_bottom - inset.bottom;
    let left = content.x + legend_space.left + y_title_left + inset.left;
    let (columns, rows) = (grid.columns.max(1), grid.rows.max(1));
    // Strips above each row (its panel with the most labels), and right of the
    // grid (lets-plot `facetRowHeadTotalWidth`).
    // A strip is as tall as the label with the most lines at its place in the row
    // (lets-plot), row strips as wide as the row label with the most lines.
    let row_strip_lines: Vec<Vec<usize>> = (0..rows)
        .map(|row| {
            let mut lines: Vec<usize> = Vec::new();
            for panel in grid
                .panels
                .iter()
                .filter(|panel| panel.row.min(rows - 1) == row)
            {
                for (index, label) in panel.labels.iter().enumerate() {
                    match lines.get_mut(index) {
                        Some(most) => *most = (*most).max(line_count(label)),
                        None => lines.push(line_count(label)),
                    }
                }
            }
            lines
        })
        .collect();
    let row_strips: Vec<f64> = row_strip_lines
        .iter()
        .map(|lines| strips_height(theme, lines))
        .collect();
    let row_label_lines = grid
        .panels
        .iter()
        .filter_map(|panel| panel.row_label.map(line_count))
        .max()
        .unwrap_or(0);
    let right_strip = row_strip_width(theme, row_label_lines);
    let right =
        content.x + content.width - legend_space.right - y_title_right - inset.right - right_strip;
    let (spacing_x, spacing_y) = theme.panel_spacing;
    let column_weights = space_weights(grid, columns, grid.free_space.0, true);
    let row_weights = space_weights(grid, rows, grid.free_space.1, false);

    // lets-plot `maxHAxisThickness`: the last panel of an unfinished column (above
    // the last row) takes no room for its x axis.
    let takes_x_band =
        |panel: &FacetPanel| panel.x_axis.primary && !(panel.is_bottom && panel.row + 1 < rows);
    let takes_top_band = |panel: &FacetPanel| panel.x_axis.secondary;
    // As the bottom: the last panel of an unfinished row draws its right axis in
    // the empty space right of it.
    let takes_right_band = |panel: &FacetPanel| {
        panel.y_axis.secondary && !(panel.is_row_end && panel.column + 1 < columns)
    };
    let row_has = |takes: &dyn Fn(&FacetPanel) -> bool| -> Vec<bool> {
        (0..rows)
            .map(|row| {
                grid.panels
                    .iter()
                    .any(|panel| panel.row.min(rows - 1) == row && takes(panel))
            })
            .collect()
    };
    let row_has_band = row_has(&takes_x_band);
    let row_has_top_band = row_has(&takes_top_band);

    // Axis labels share the overflow budget among the axis bands they stack in.
    let band_rows = row_has_band
        .iter()
        .chain(&row_has_top_band)
        .filter(|has| **has)
        .count()
        .max(1);
    let mut axis_columns: Vec<(usize, bool)> = grid
        .panels
        .iter()
        .flat_map(|panel| {
            [
                panel.y_axis.primary.then_some((panel.column, false)),
                takes_right_band(panel).then_some((panel.column, true)),
            ]
        })
        .flatten()
        .collect();
    axis_columns.sort_unstable();
    axis_columns.dedup();
    let x_limit = content.height * AXIS_LABELS_MAX_SHARE / band_rows as f64;
    let y_limit = content.width * AXIS_LABELS_MAX_SHARE / axis_columns.len().max(1) as f64;

    let simple = theme.axis_text_x.map_or(0.0, text_band);
    let mut label_heights = vec![simple; rows];
    let mut kept_x: Option<Vec<AxisBreaks>> = None;
    let mut first_pass = true;
    loop {
        let bands = |has: &[bool]| -> Vec<f64> {
            (0..rows)
                .map(|row| {
                    if has[row] {
                        x_axis_band(label_heights[row], theme)
                    } else {
                        0.0
                    }
                })
                .collect()
        };
        let row_bands = bands(&row_has_band);
        let row_top_bands = bands(&row_has_top_band);
        let panel_heights = split_space(
            (bottom - top)
                - row_strips.iter().sum::<f64>()
                - row_bands.iter().sum::<f64>()
                - row_top_bands.iter().sum::<f64>()
                - (rows - 1) as f64 * spacing_y,
            &row_weights,
            panel_min_size,
        );
        let height_of = |panel: &FacetPanel| panel_heights[panel.row.min(rows - 1)];
        let y_breaks = per_panel(grid, grid.shared_y, |panel| {
            choose_breaks(
                panel.y_scale,
                height_of(panel),
                Orientation::Vertical,
                theme.axis_text_y,
                (0.0, 0.0),
            )
        });
        let y_breaks: Vec<AxisBreaks> = grid
            .panels
            .iter()
            .zip(y_breaks)
            .map(|(panel, breaks)| {
                fit_y_labels(breaks, panel.y_scale, height_of(panel), y_limit, theme)
            })
            .collect();
        let y_secondary: Vec<Option<SecondaryBreaks>> = grid
            .panels
            .iter()
            .zip(&y_breaks)
            .map(|(panel, breaks)| {
                secondary_breaks(
                    panel.y_scale,
                    breaks,
                    height_of(panel),
                    Orientation::Vertical,
                    theme.axis_text_y,
                    (0.0, 0.0),
                )
            })
            .collect();
        let mut column_widths = vec![0.0_f64; columns];
        let mut column_right_widths = vec![0.0_f64; columns];
        for ((panel, breaks), secondary) in grid.panels.iter().zip(&y_breaks).zip(&y_secondary) {
            let column = panel.column.min(columns - 1);
            if panel.y_axis.primary {
                let width = &mut column_widths[column];
                *width = width.max(y_axis_width(side_breaks(breaks, secondary, false), theme));
            }
            if takes_right_band(panel) {
                let width = &mut column_right_widths[column];
                *width = width.max(y_axis_width(side_breaks(breaks, secondary, true), theme));
            }
        }
        let panel_widths = split_space(
            (right - left)
                - column_widths.iter().sum::<f64>()
                - column_right_widths.iter().sum::<f64>()
                - (columns - 1) as f64 * spacing_x,
            &column_weights,
            panel_min_size,
        );
        let width_of = |panel: &FacetPanel| panel_widths[panel.column.min(columns - 1)];
        let x_breaks = match kept_x.take() {
            Some(breaks) => breaks,
            None => per_panel(grid, grid.shared_x, |panel| {
                // lets-plot lets x labels reach over the y axis on the left.
                let column = panel.column.min(columns - 1);
                choose_breaks(
                    panel.x_scale,
                    width_of(panel),
                    Orientation::Horizontal,
                    theme.axis_text_x,
                    (
                        column_widths[column].max(LABELS_OVERFLOW),
                        column_right_widths[column].max(LABELS_OVERFLOW),
                    ),
                )
            })
            .into_iter()
            .zip(&grid.panels)
            .map(|(breaks, panel)| {
                fit_x_labels(breaks, panel.x_scale, width_of(panel), x_limit, theme)
            })
            .collect(),
        };
        let x_secondary: Vec<Option<SecondaryBreaks>> = grid
            .panels
            .iter()
            .zip(&x_breaks)
            .map(|(panel, breaks)| {
                let column = panel.column.min(columns - 1);
                secondary_breaks(
                    panel.x_scale,
                    breaks,
                    width_of(panel),
                    Orientation::Horizontal,
                    theme.axis_text_x,
                    (
                        column_widths[column].max(LABELS_OVERFLOW),
                        column_right_widths[column].max(LABELS_OVERFLOW),
                    ),
                )
            })
            .collect();
        let mut needed = label_heights.clone();
        for ((panel, breaks), secondary) in grid.panels.iter().zip(&x_breaks).zip(&x_secondary) {
            if takes_x_band(panel) || takes_top_band(panel) {
                let row = panel.row.min(rows - 1);
                needed[row] = needed[row].max(x_labels_height(breaks, theme));
                if let Some(secondary) = secondary {
                    needed[row] = needed[row].max(x_labels_height(&secondary.breaks, theme));
                }
            }
        }
        if first_pass
            && needed
                .iter()
                .zip(&label_heights)
                .any(|(need, has)| need > has)
        {
            // Thicker x labels (tilted, two rows): lay out once more for the shorter
            // panels, keeping the x breaks and their arrangement.
            label_heights = needed;
            kept_x = Some(x_breaks);
            first_pass = false;
            continue;
        }

        let mut column_x = Vec::with_capacity(columns);
        let mut cursor = left;
        for (column, width) in column_widths.iter().enumerate() {
            if column > 0 {
                cursor += spacing_x;
            }
            cursor += width;
            column_x.push(cursor);
            cursor += panel_widths[column];
            cursor += column_right_widths[column];
        }
        let mut row_y = Vec::with_capacity(rows);
        let mut cursor = top;
        for (row, band) in row_bands.iter().enumerate() {
            if row > 0 {
                cursor += spacing_y;
            }
            cursor += row_top_bands[row];
            cursor += row_strips[row];
            row_y.push(cursor);
            cursor += panel_heights[row] + band;
        }
        let tiles: Vec<TileLayout> = grid
            .panels
            .iter()
            .zip(x_breaks)
            .zip(y_breaks)
            .zip(x_secondary.into_iter().zip(y_secondary))
            .map(|(((panel, x_breaks), y_breaks), secondary)| {
                let row = panel.row.min(rows - 1);
                let column = panel.column.min(columns - 1);
                TileLayout {
                    layout: PlotLayout {
                        content,
                        panel: Rect {
                            x: column_x[column],
                            y: row_y[row],
                            width: panel_widths[column],
                            height: panel_heights[row],
                        },
                        axis_bands: Margins {
                            top: row_top_bands[row],
                            right: column_right_widths[column],
                            bottom: x_axis_band(label_heights[row], theme),
                            left: column_widths[column],
                        },
                        axis_sides,
                        x_breaks,
                        y_breaks,
                        legend_space,
                        top_band: row_strips[row],
                        // Row strips sit between the panel and its right axis.
                        right_band: if panel.row_label.is_some() {
                            right_strip
                        } else {
                            0.0
                        },
                        dendrograms: Default::default(),
                        bands: Vec::new(),
                        secondary: [secondary.0, secondary.1],
                    },
                    axes: [panel.x_axis, panel.y_axis],
                    labels: panel.labels.to_vec(),
                    strip_lines: row_strip_lines[row].clone(),
                    row_label: panel.row_label.cloned(),
                    row_label_lines,
                }
            })
            .collect();
        let plot = PlotLayout {
            content,
            panel: union(tiles.iter().map(|tile| tile.layout.panel)),
            x_breaks: tiles
                .first()
                .map(|tile| tile.layout.x_breaks.clone())
                .unwrap_or_else(|| AxisBreaks::new(Vec::new(), Vec::new())),
            y_breaks: tiles
                .first()
                .map(|tile| tile.layout.y_breaks.clone())
                .unwrap_or_else(|| AxisBreaks::new(Vec::new(), Vec::new())),
            axis_bands: Margins {
                top: row_top_bands.first().copied().unwrap_or(0.0),
                right: column_right_widths.last().copied().unwrap_or(0.0),
                bottom: row_bands.last().copied().unwrap_or(0.0),
                left: column_widths.first().copied().unwrap_or(0.0),
            },
            axis_sides,
            legend_space,
            top_band: row_strips[0],
            right_band: right_strip,
            dendrograms: Default::default(),
            bands: Vec::new(),
            secondary: tiles
                .first()
                .map(|tile| tile.layout.secondary.clone())
                .unwrap_or_default(),
        };
        return FacetLayout { plot, tiles };
    }
}

/// Relative size of each grid column (`horizontal`) or row: its scale's expanded
/// range when the space is free, else 1 (ggplot2 `space="free"`).
fn space_weights(grid: &FacetGrid, count: usize, free: bool, horizontal: bool) -> Vec<f64> {
    let mut weights = vec![1.0; count];
    if !free {
        return weights;
    }
    for (index, weight) in weights.iter_mut().enumerate() {
        let panel = grid.panels.iter().find(|panel| {
            let place = if horizontal { panel.column } else { panel.row };
            place.min(count - 1) == index
        });
        if let Some(panel) = panel {
            let scale = if horizontal {
                panel.x_scale
            } else {
                panel.y_scale
            };
            let (low, high) = scale.expanded_domain();
            let span = (high - low).abs();
            if span.is_finite() && span > 0.0 {
                *weight = span;
            }
        }
    }
    weights
}

/// `total` split by `weights`, each part at least `minimum`.
fn split_space(total: f64, weights: &[f64], minimum: f64) -> Vec<f64> {
    let sum: f64 = weights.iter().sum();
    weights
        .iter()
        .map(|weight| (total * weight / sum).max(minimum))
        .collect()
}

/// The breaks drawn on one side (`far`: top / right) of a panel's axis.
fn side_breaks<'a>(
    breaks: &'a AxisBreaks,
    secondary: &'a Option<SecondaryBreaks>,
    far: bool,
) -> &'a AxisBreaks {
    match secondary {
        Some(secondary) if secondary.far_side == far => &secondary.breaks,
        _ => breaks,
    }
}

/// `choose` for every panel; once (for the first panel) when the scale is shared.
fn per_panel(
    grid: &FacetGrid,
    shared: bool,
    choose: impl Fn(&FacetPanel) -> AxisBreaks,
) -> Vec<AxisBreaks> {
    match grid.panels.first() {
        Some(first) if shared => vec![choose(first); grid.panels.len()],
        _ => grid.panels.iter().map(choose).collect(),
    }
}

/// Height of one strip with room for `lines` text lines: their line boxes plus
/// the label margins.
fn strip_height(style: TextStyle, theme: &Theme, lines: usize) -> f64 {
    lines.max(1) as f64 * text_band(style) + theme.strip_margin.top + theme.strip_margin.bottom
}

/// Height above each panel taken by its strips (text lines of each, top first) and
/// `strip_spacing` (lets-plot `facetColHeadTotalHeight`); 0 without strips.
fn strips_height(theme: &Theme, lines: &[usize]) -> f64 {
    match theme.strip_text {
        Some(style) if !lines.is_empty() => {
            lines
                .iter()
                .map(|lines| strip_height(style, theme, *lines))
                .sum::<f64>()
                + theme.strip_spacing
        }
        _ => 0.0,
    }
}

/// Width of one row strip with room for `lines` text lines: their line boxes plus
/// the label's left and right margins (the text turns, the margins do not).
fn row_strip_thickness(style: TextStyle, theme: &Theme, lines: usize) -> f64 {
    lines.max(1) as f64 * text_band(style) + theme.strip_margin_y.left + theme.strip_margin_y.right
}

/// Width right of the grid taken by row strips with room for `lines` text lines
/// and `strip_spacing_x`; 0 without row strips (`lines` 0).
fn row_strip_width(theme: &Theme, lines: usize) -> f64 {
    match theme.strip_text_y {
        Some(style) if lines > 0 => {
            row_strip_thickness(style, theme, lines) + theme.strip_spacing_x
        }
        _ => 0.0,
    }
}

/// Smallest rectangle around all `rects` (an empty rectangle for none).
fn union(rects: impl Iterator<Item = Rect>) -> Rect {
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    for rect in rects {
        let (left, top, right, bottom) =
            (rect.x, rect.y, rect.x + rect.width, rect.y + rect.height);
        bounds = Some(match bounds {
            None => (left, top, right, bottom),
            Some((l, t, r, b)) => (l.min(left), t.min(top), r.max(right), b.max(bottom)),
        });
    }
    let (left, top, right, bottom) = bounds.unwrap_or_default();
    Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    }
}

/// A panel's strips (background and label): stacked above it (lets-plot
/// `PlotTile.addHorizontalFacetLabels`), as wide as the panel, the label justified
/// by `hjust` inside the strip's margins; and its row strip right of it
/// (`addVerticalFacetLabels`), as tall as the panel, the label turned to read top
/// to bottom, `hjust` 0 at the top.
pub fn draw_strips(tile: &TileLayout, theme: &Theme) -> Vec<Node> {
    let mut nodes = draw_column_strips(tile, theme);
    nodes.extend(draw_row_strip(tile, theme));
    if nodes.is_empty() {
        return Vec::new();
    }
    vec![role_group("strips", nodes)]
}

/// The strips above a panel.
fn draw_column_strips(tile: &TileLayout, theme: &Theme) -> Vec<Node> {
    let Some(style) = theme.strip_text else {
        return Vec::new();
    };
    let panel = tile.layout.panel;
    let margin = theme.strip_margin;
    let hjust = if style.hjust.is_finite() {
        style.hjust
    } else {
        0.5
    };
    let mut top = panel.y - strips_height(theme, &tile.strip_lines);
    let mut nodes = Vec::new();
    for (index, label) in tile.labels.iter().enumerate() {
        let lines = tile.strip_lines.get(index).copied().unwrap_or(1);
        let height = strip_height(style, theme, lines);
        let strip = Rect {
            x: panel.x,
            y: top,
            width: panel.width,
            height,
        };
        if let Some(background) = theme.strip_background {
            nodes.push(rect_node(strip, background));
        }
        let width = label_width(label, style);
        let span = panel.width - margin.left - margin.right;
        let x = panel.x + margin.left + width / 2.0 + (span - width) * hjust;
        let y = top + margin.top + lines.max(1) as f64 * text_band(style) / 2.0;
        nodes.push(label_node(
            label,
            style,
            (x, y),
            HorizontalAlign::Middle,
            0.0,
        ));
        top += height;
    }
    nodes
}

/// The row strip right of a panel (`facet_grid`).
fn draw_row_strip(tile: &TileLayout, theme: &Theme) -> Vec<Node> {
    let (Some(style), Some(label)) = (theme.strip_text_y, &tile.row_label) else {
        return Vec::new();
    };
    let panel = tile.layout.panel;
    let margin = theme.strip_margin_y;
    let strip = Rect {
        x: panel.x + panel.width + theme.strip_spacing_x,
        y: panel.y,
        width: row_strip_thickness(style, theme, tile.row_label_lines),
        height: panel.height,
    };
    let mut nodes = Vec::new();
    if let Some(background) = theme.strip_background_y {
        nodes.push(rect_node(strip, background));
    }
    let hjust = if style.hjust.is_finite() {
        style.hjust
    } else {
        0.5
    };
    let length = label_width(label, style);
    let span = panel.height - margin.top - margin.bottom;
    let x = strip.x + margin.left + tile.row_label_lines.max(1) as f64 * text_band(style) / 2.0;
    let y = panel.y + margin.top + length / 2.0 + (span - length) * hjust;
    nodes.push(label_node(
        label,
        style,
        (x, y),
        HorizontalAlign::Middle,
        -90.0,
    ));
    nodes
}

/// Width of a strip label: its widest line.
fn label_width(label: &Label, style: TextStyle) -> f64 {
    if label.text.contains(LINE_BREAK) {
        return label
            .text
            .split(LINE_BREAK)
            .map(|line| measure_label(line, None, None, style.size, style.font))
            .fold(0.0, f64::max);
    }
    measure_label(
        &label.text,
        label.superscript.as_deref(),
        label.suffix.as_deref(),
        style.size,
        style.font,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::ContinuousPositionScale;

    /// Axis sides with only the primary side drawn, when `shown`.
    fn primary(shown: bool) -> AxisSides {
        AxisSides {
            primary: shown,
            secondary: false,
        }
    }

    fn scale(high: f64) -> PositionScale {
        let mut scale = ContinuousPositionScale::default();
        scale.train_range(0.0, high);
        PositionScale::Continuous(scale)
    }

    fn panels<'a>(
        scales: &'a [PositionScale],
        labels: &'a [Label],
        count: usize,
        columns: usize,
        free: bool,
    ) -> FacetGrid<'a> {
        let rows = count.div_ceil(columns);
        let panels = (0..count)
            .map(|index| {
                let (column, row) = (index % columns, index / columns);
                let is_bottom = row + 1 == rows || index + columns >= count;
                FacetPanel {
                    x_scale: &scales[if free { index } else { 0 }],
                    y_scale: &scales[if free { index } else { 0 }],
                    labels: std::slice::from_ref(&labels[index]),
                    row_label: None,
                    column,
                    row,
                    x_axis: primary(is_bottom || free),
                    y_axis: primary(column == 0 || free),
                    is_bottom,
                    is_row_end: column + 1 == columns || index + 1 == count,
                }
            })
            .collect();
        FacetGrid {
            columns,
            rows,
            panels,
            shared_x: !free,
            shared_y: !free,
            free_space: (false, false),
        }
    }

    fn area() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width: 900.0,
            height: 600.0,
        }
    }

    #[test]
    fn wrapped_labels_make_their_row_strips_taller() {
        let scales = [scale(10.0)];
        let two_lines = format!("a{LINE_BREAK}b");
        let labels: Vec<Label> = [two_lines.as_str(), "c", "d", "e"]
            .map(Label::plain)
            .to_vec();
        let theme = Theme::default();
        let grid = panels(&scales, &labels, 4, 2, false);
        let layout = layout_facets(
            area(),
            &theme,
            &Titles::default(),
            &grid,
            Margins::uniform(0.0),
            None,
            PANEL_MIN_SIZE,
        );
        // The first row's strips have room for two lines, the second row's for one.
        assert_eq!(layout.tiles[1].strip_lines, [2]);
        assert_eq!(layout.tiles[2].strip_lines, [1]);
        let band = text_band(theme.strip_text.unwrap());
        let rects: Vec<Rect> = layout.tiles.iter().map(|tile| tile.layout.panel).collect();
        assert!((rects[0].y - (6.0 + strips_height(&theme, &[1]) + band)).abs() < 1e-9);
        // The one-line label of the tall strip is centred in it.
        let nodes = draw_strips(&layout.tiles[1], &theme);
        let Node::Group(group) = &nodes[0] else {
            panic!("expected a group")
        };
        let text = group
            .nodes
            .iter()
            .find_map(|node| match node {
                Node::Text(text) => Some(text),
                _ => None,
            })
            .unwrap();
        let strip_top = rects[1].y - strips_height(&theme, &[2]);
        let middle = strip_top + theme.strip_margin.top + band;
        assert!((text.y - middle).abs() < 1e-9, "{} {middle}", text.y);
    }

    #[test]
    fn panels_share_one_size_and_leave_room_for_strips_axes_and_spacing() {
        let scales = [scale(10.0)];
        let labels: Vec<Label> = ["a", "b", "c", "d"].map(Label::plain).to_vec();
        let theme = Theme::default();
        let grid = panels(&scales, &labels, 4, 2, false);
        let layout = layout_facets(
            area(),
            &theme,
            &Titles::default(),
            &grid,
            Margins::uniform(0.0),
            None,
            PANEL_MIN_SIZE,
        );
        let rects: Vec<Rect> = layout.tiles.iter().map(|tile| tile.layout.panel).collect();
        for rect in &rects {
            assert_eq!((rect.width, rect.height), (rects[0].width, rects[0].height));
        }
        let strip = strips_height(&theme, &[1]);
        // Panels in a row are panel_spacing apart; rows leave room for the strip.
        assert!((rects[1].x - rects[0].x - rects[0].width - 10.0).abs() < 1e-9);
        assert!((rects[2].y - rects[0].y - rects[0].height - 10.0 - strip).abs() < 1e-9);
        // Only the first column has a y axis, only the last row an x axis.
        assert_eq!(
            layout
                .tiles
                .iter()
                .map(|tile| (tile.axes[0].primary, tile.axes[1].primary))
                .collect::<Vec<_>>(),
            [(false, true), (false, false), (true, true), (true, false)]
        );
        // The grid fills the plot to the inset on the right and the bottom.
        let last = rects[3];
        assert!((last.x + last.width - (900.0 - 6.0)).abs() < 1e-9);
        let bottom = last.y + last.height + layout.plot.axis_bands.bottom;
        assert!((bottom - (600.0 - 6.0)).abs() < 1e-9, "{bottom}");
        assert_eq!(layout.plot.panel.x, rects[0].x);
        assert_eq!(layout.plot.panel.y, rects[0].y);
    }

    #[test]
    fn free_scales_give_each_column_its_widest_y_axis() {
        let scales = [scale(1.0), scale(100_000.0), scale(1.0), scale(1.0)];
        let labels: Vec<Label> = ["a", "b", "c", "d"].map(Label::plain).to_vec();
        let theme = Theme::default();
        let grid = panels(&scales, &labels, 4, 2, true);
        let layout = layout_facets(
            area(),
            &theme,
            &Titles::default(),
            &grid,
            Margins::uniform(0.0),
            None,
            PANEL_MIN_SIZE,
        );
        let rects: Vec<Rect> = layout.tiles.iter().map(|tile| tile.layout.panel).collect();
        // Column 1 holds the long labels of panel b: both its panels move right.
        assert_eq!(rects[1].x, rects[3].x);
        assert!(rects[1].x - (rects[0].x + rects[0].width) > 40.0);
        assert_ne!(
            layout.tiles[0].layout.y_breaks,
            layout.tiles[1].layout.y_breaks
        );
        // Every row has an x axis band.
        assert!(rects[2].y - (rects[0].y + rects[0].height) > 10.0 + 30.0);
    }

    #[test]
    fn coord_fixed_keeps_the_ratio_in_every_panel() {
        let scales = [scale(10.0)];
        let labels: Vec<Label> = ["a", "b", "c"].map(Label::plain).to_vec();
        let grid = panels(&scales, &labels, 3, 3, false);
        let layout = layout_facets(
            area(),
            &Theme::default(),
            &Titles::default(),
            &grid,
            Margins::uniform(0.0),
            Some(1.0),
            PANEL_MIN_SIZE,
        );
        for tile in &layout.tiles {
            let panel = tile.layout.panel;
            assert!((panel.width - panel.height).abs() < 1e-6, "{panel:?}");
        }
    }

    #[test]
    fn a_top_legend_goes_above_the_strips() {
        let scales = [scale(10.0)];
        let labels = [Label::plain("a")];
        let theme = Theme {
            legend_position: LegendPosition::Top,
            ..Theme::default()
        };
        let grid = panels(&scales, &labels, 1, 1, false);
        let space = legend_space(&theme, (50.0, 20.0));
        let layout = layout_facets(
            area(),
            &theme,
            &Titles::default(),
            &grid,
            space,
            None,
            PANEL_MIN_SIZE,
        );
        let (_, legend_y) = legend_origin(&layout.plot, &theme, &Titles::default(), (50.0, 20.0));
        let strip_top = layout.plot.panel.y - strips_height(&theme, &[1]);
        assert!(
            legend_y + 20.0 <= strip_top + 1e-9,
            "{legend_y} vs {strip_top}"
        );
    }

    #[test]
    fn grid_strips_take_the_top_row_and_the_right_column_only() {
        let scales = [scale(10.0)];
        let column_labels = [Label::plain("a"), Label::plain("b")];
        let row_labels = [Label::plain("u"), Label::plain("v")];
        let theme = Theme::default();
        let panels = (0..4)
            .map(|index| {
                let (column, row) = (index % 2, index / 2);
                FacetPanel {
                    x_scale: &scales[0],
                    y_scale: &scales[0],
                    labels: if row == 0 {
                        std::slice::from_ref(&column_labels[column])
                    } else {
                        &[]
                    },
                    row_label: (column == 1).then_some(&row_labels[row]),
                    column,
                    row,
                    x_axis: primary(row == 1),
                    y_axis: primary(column == 0),
                    is_bottom: row == 1,
                    is_row_end: column == 1,
                }
            })
            .collect();
        let grid = FacetGrid {
            columns: 2,
            rows: 2,
            panels,
            shared_x: true,
            shared_y: true,
            free_space: (false, false),
        };
        let layout = layout_facets(
            area(),
            &theme,
            &Titles::default(),
            &grid,
            Margins::uniform(0.0),
            None,
            PANEL_MIN_SIZE,
        );
        let rects: Vec<Rect> = layout.tiles.iter().map(|tile| tile.layout.panel).collect();
        // No strip between the rows: only panel_spacing.
        assert!((rects[2].y - rects[0].y - rects[0].height - 10.0).abs() < 1e-9);
        assert!((rects[0].y - (6.0 + strips_height(&theme, &[1]))).abs() < 1e-9);
        // The row strips sit between the right column and the inset.
        let style = theme.strip_text_y.unwrap();
        let width = row_strip_thickness(style, &theme, 1);
        assert!((rects[1].x + rects[1].width + width - (900.0 - 6.0)).abs() < 1e-9);
        assert_eq!(layout.plot.right_band, width);
        // The row strip text turns to read top to bottom, centred on the panel.
        let nodes = draw_strips(&layout.tiles[3], &theme);
        let Node::Group(group) = &nodes[0] else {
            panic!("expected a group")
        };
        let [Node::Text(text)] = group.nodes.as_slice() else {
            panic!("expected one text, got {:?}", group.nodes)
        };
        assert_eq!(text.angle, -90.0);
        let panel = rects[3];
        assert!(text.x > panel.x + panel.width && text.x < panel.x + panel.width + width);
        assert!((text.y - (panel.y + panel.height / 2.0)).abs() < 1e-9);
        // Panels without strips draw none.
        assert!(draw_strips(&layout.tiles[2], &theme).is_empty());
    }

    #[test]
    fn strips_sit_above_their_panel() {
        let scales = [scale(10.0)];
        let labels = [Label::plain("a")];
        let theme = Theme::default();
        let grid = panels(&scales, &labels, 1, 1, false);
        let layout = layout_facets(
            area(),
            &theme,
            &Titles::default(),
            &grid,
            Margins::uniform(0.0),
            None,
            PANEL_MIN_SIZE,
        );
        let tile = &layout.tiles[0];
        let nodes = draw_strips(tile, &theme);
        let Node::Group(group) = &nodes[0] else {
            panic!("expected a group")
        };
        // minimal2: no strip background, just the centred label.
        let [Node::Text(text)] = group.nodes.as_slice() else {
            panic!("expected one text, got {:?}", group.nodes)
        };
        let panel = tile.layout.panel;
        assert!((text.x - (panel.x + panel.width / 2.0)).abs() < 1e-9);
        assert!(text.y < panel.y && text.y > panel.y - strips_height(&theme, &[1]));
    }

    #[test]
    fn free_space_sizes_columns_by_their_scale_ranges() {
        let scales = [scale(1.0), scale(3.0)];
        let labels = [Label::plain("a".to_owned()), Label::plain("b".to_owned())];
        let mut grid = panels(&scales, &labels, 2, 2, true);
        grid.free_space = (true, false);
        let layout = layout_facets(
            area(),
            &Theme::default(),
            &Titles::default(),
            &grid,
            Margins::uniform(0.0),
            None,
            PANEL_MIN_SIZE,
        );
        let (first, second) = (layout.tiles[0].layout.panel, layout.tiles[1].layout.panel);
        let (low_1, high_1) = scales[0].expanded_domain();
        let (low_3, high_3) = scales[1].expanded_domain();
        let ratio = (high_3 - low_3) / (high_1 - low_1);
        assert!(
            (second.width / first.width - ratio).abs() < 1e-9,
            "{first:?} {second:?}"
        );
        assert_eq!(first.height, second.height);
    }
}
