//! Grids of figures: `gggrid` (lets-plot `CompositeFigureBuildInfo`,
//! `CompositeFigureGridLayout`, `CompositeFigureGridAlignmentLayout`).
//!
//! What it does: prepare every figure of a grid once (plots and nested grids), give
//! each a cell ([`crate::layout::grid`]: relative `widths`/`heights`,
//! `hspace`/`vspace`, `fit`), lay plots out at their cell's size, align panels across
//! rows and columns (`align`), collect legends (`guides`), and draw the grid's
//! background, title and collected legends. Figures inherit the grid's theme as in
//! lets-plot ([`child_theme`]).
//!
//! What it does not do (warned): shared axes (`sharex`/`sharey`), captions, tags and
//! grid-level `guides(...)`.

use crate::build::{Arrangement, BuiltPlot, DEFAULT_SIZE, PlotData, PreparedPlot, build_plot_with};
use crate::data::DataFrame;
use crate::layout::grid::{DEFAULT_SPACING, cell_rects, shrink_to_aspect};
use crate::layout::{
    LegendBlock, LegendBox, Titles, draw_legends, heading_nodes, headings_band, measure_legends,
    shrink,
};
use crate::scene::{Color, Group, Node, PerItem, Rect, Rects, Scene};
use crate::spec::{ElementSpec, Figure, SubplotsSpec, ThemeSpec, ThemeValue};
use crate::theme::{LegendPosition, Margins, Theme};
use crate::tooltip::Interaction;
use crate::{Error, Result};

/// Build a figure: a single plot, or a grid of figures.
pub fn build_figure(figure: &Figure, frames: &[DataFrame]) -> Result<BuiltPlot> {
    build_figure_with(figure, frames, false)
}

/// [`build_figure`], with the hover tooltips of every plot when `interactive`
/// (HTML output).
pub fn build_figure_with(
    figure: &Figure,
    frames: &[DataFrame],
    interactive: bool,
) -> Result<BuiltPlot> {
    match figure {
        Figure::Plot(spec) => {
            build_plot_with(spec, &PlotData::from_frames(spec, frames)?, interactive)
        }
        Figure::Subplots(spec) => {
            let mut warnings = Vec::new();
            let context = GridContext {
                frames,
                container: None,
                container_collects: false,
                path: "",
            };
            let (grid, _) = PreparedGrid::new(spec, &context, &mut warnings)
                .map_err(|error| error.with_notes(&warnings))?;
            let (width, height) = grid.size;
            let bounds = Rect {
                x: 0.0,
                y: 0.0,
                width,
                height,
            };
            let mut collected = interactive.then(|| (Interaction::default(), Vec::new()));
            let nodes = grid.draw(bounds, false, collected.as_mut())?;
            let scene = Scene {
                width,
                height,
                background: grid.theme.plot_background.map(|background| background.fill),
                nodes,
            };
            scene.validate()?;
            let interaction = collected.map(|(interaction, notes)| {
                for note in notes {
                    if !warnings.contains(&note) {
                        warnings.push(note);
                    }
                }
                interaction
            });
            Ok(BuiltPlot {
                scene,
                interaction,
                warnings,
            })
        }
    }
}

/// `gggrid(guides=...)`: lets-plot `GuidesSharingMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Guides {
    /// Collect when the containing grid collects; then pass legends up to it.
    Auto,
    /// Collect the figures' legends and draw them once, beside the grid.
    Collect,
    /// Every figure keeps its legends.
    Keep,
}

impl Guides {
    fn of(spec: &SubplotsSpec, path: &str) -> Result<Guides> {
        match spec
            .layout
            .guides
            .as_deref()
            .map(str::to_lowercase)
            .as_deref()
        {
            None | Some("auto") => Ok(Guides::Auto),
            Some("collect") => Ok(Guides::Collect),
            Some("keep") => Ok(Guides::Keep),
            Some(other) => Err(Error::InvalidSpec(nest(
                path,
                &format!("at 'layout.guides': expected 'auto', 'collect' or 'keep', got '{other}'"),
            ))),
        }
    }
}

/// Legends collected from one figure, placed as that figure would place them.
struct Collected {
    position: LegendPosition,
    justification: (f64, f64),
    boxes: Vec<LegendBox>,
    /// The figure's theme: lets-plot builds legend boxes in their figure, so they keep
    /// its legend text, keys and spacing.
    theme: Theme,
}

/// What a grid's figures see of it while they are prepared.
struct GridContext<'a> {
    frames: &'a [DataFrame],
    /// The containing grid's theme options and theme.
    container: Option<(&'a ThemeSpec, &'a Theme)>,
    /// Whether the containing grid collects legends.
    container_collects: bool,
    /// Spec path of this grid (`figures[1].`), for messages.
    path: &'a str,
}

enum Element<'s> {
    Plot(Box<PreparedPlot>),
    Grid(Box<PreparedGrid<'s>>),
}

impl Element<'_> {
    /// Size the figure has on its own (`ggsize` or the default), for `fit=False`.
    fn own_size(&self) -> (f64, f64) {
        match self {
            Element::Plot(plot) => plot.size,
            Element::Grid(grid) => grid.size,
        }
    }
}

/// A figure placed in its cell.
enum Placed<'e, 's> {
    Plot(&'e PreparedPlot, Box<Arrangement>),
    Grid(&'e PreparedGrid<'s>, Rect),
}

impl Placed<'_, '_> {
    fn bounds(&self) -> Rect {
        match self {
            Placed::Plot(_, arrangement) => arrangement.area(),
            Placed::Grid(_, bounds) => *bounds,
        }
    }
}

/// A grid with every figure prepared: ready to be laid out at any size.
struct PreparedGrid<'s> {
    spec: &'s SubplotsSpec,
    theme: Theme,
    titles: Titles,
    elements: Vec<Option<Element<'s>>>,
    /// Legends this grid draws, by placement (lets-plot groups by position and
    /// justification), duplicates removed.
    legend_groups: Vec<Collected>,
    /// `ggsize`, or the default size.
    size: (f64, f64),
}

impl<'s> PreparedGrid<'s> {
    /// Prepare the grid and its figures. Returns the legends passed up to the
    /// containing grid (`guides="auto"` inside a grid that collects).
    fn new(
        spec: &'s SubplotsSpec,
        context: &GridContext,
        warnings: &mut Vec<String>,
    ) -> Result<(PreparedGrid<'s>, Vec<Collected>)> {
        let path = context.path;
        let layout = &spec.layout;
        let (columns, rows) = (layout.ncol as usize, layout.nrow as usize);
        if columns == 0 || rows == 0 || spec.figures.len() != columns * rows {
            return Err(Error::InvalidSpec(nest(
                path,
                &format!(
                    "at 'layout': {} figures do not fill a grid of {} columns and {} rows",
                    spec.figures.len(),
                    layout.ncol,
                    layout.nrow
                ),
            )));
        }
        for (key, sizes) in [("widths", &layout.widths), ("heights", &layout.heights)] {
            let Some(sizes) = sizes else { continue };
            let valid = sizes.iter().all(|size| size.is_finite() && *size >= 0.0)
                && sizes.iter().sum::<f64>() > 0.0;
            if !valid {
                return Err(Error::InvalidSpec(nest(
                    path,
                    &format!(
                        "at 'layout.{key}': expected relative sizes >= 0 with a positive sum, \
                         got {sizes:?}"
                    ),
                )));
            }
        }
        report_grid_unsupported(spec, path, warnings);
        let theme_spec = match context.container {
            Some((container, container_theme)) => {
                child_theme(container, container_theme, spec.theme.as_ref())
            }
            None => spec.theme.clone().unwrap_or_default(),
        };
        let mut theme_warnings = Vec::new();
        let theme = Theme::from_spec(Some(&theme_spec), &mut theme_warnings)
            .map_err(|error| nest_error(error, path))?;
        warnings.extend(theme_warnings.iter().map(|warning| nest(path, warning)));
        let size = match &spec.ggsize {
            Some(size) if size.width > 0.0 && size.height > 0.0 => (size.width, size.height),
            Some(size) => {
                return Err(Error::InvalidSpec(nest(
                    path,
                    &format!(
                        "at 'ggsize': width and height must be positive, got {} x {}",
                        size.width, size.height
                    ),
                )));
            }
            // lets-plot `CompositeFigureGridLayoutBase.defaultSize`.
            None => (
                DEFAULT_SIZE.0 * (0.5 + 0.5 / columns as f64) * columns as f64,
                DEFAULT_SIZE.1 * (0.5 + 0.5 / rows as f64) * rows as f64,
            ),
        };
        let own_guides = Guides::of(spec, path)?;
        let collects = if context.container_collects {
            own_guides != Guides::Keep
        } else {
            own_guides == Guides::Collect
        };
        let mut collected = Vec::new();
        let mut elements = Vec::with_capacity(spec.figures.len());
        for (index, figure) in spec.figures.iter().enumerate() {
            let figure_path = format!("{path}figures[{index}]");
            let element = match figure {
                None => None,
                Some(Figure::Plot(plot_spec)) => {
                    let mut plot_spec = plot_spec.clone();
                    plot_spec.theme =
                        Some(child_theme(&theme_spec, &theme, plot_spec.theme.as_ref()));
                    let mut plot = PlotData::from_frames(&plot_spec, context.frames)
                        .and_then(|data| crate::build::prepare_plot(&plot_spec, &data))
                        .map_err(|error| nest_error(error, &figure_path))?;
                    warnings.extend(
                        plot.warnings
                            .drain(..)
                            .map(|warning| nest(&figure_path, &warning)),
                    );
                    let plot_theme = plot.theme().clone();
                    let fixed = matches!(
                        plot_theme.legend_position,
                        LegendPosition::Right
                            | LegendPosition::Left
                            | LegendPosition::Top
                            | LegendPosition::Bottom
                    );
                    if collects && fixed && !plot.legends.is_empty() {
                        collected.push(Collected {
                            position: plot_theme.legend_position,
                            justification: plot_theme.legend_justification,
                            theme: plot_theme,
                            boxes: std::mem::take(&mut plot.legends),
                        });
                    }
                    Some(Element::Plot(Box::new(plot)))
                }
                Some(Figure::Subplots(grid_spec)) => {
                    let child_path = format!("{figure_path}.");
                    let child_context = GridContext {
                        frames: context.frames,
                        container: Some((&theme_spec, &theme)),
                        container_collects: collects,
                        path: &child_path,
                    };
                    let (grid, mut passed_up) =
                        PreparedGrid::new(grid_spec, &child_context, warnings)?;
                    collected.append(&mut passed_up);
                    Some(Element::Grid(Box::new(grid)))
                }
            };
            elements.push(element);
        }
        let (legend_groups, passed_up) = if collects && own_guides == Guides::Auto {
            (Vec::new(), collected)
        } else {
            (group_legends(collected), Vec::new())
        };
        let title = spec.ggtitle.as_ref();
        let titles = Titles {
            title: title.and_then(|title| title.text.clone()),
            subtitle: title.and_then(|title| title.subtitle.clone()),
            x: None,
            y: None,
        };
        let grid = PreparedGrid {
            spec,
            theme,
            titles,
            elements,
            legend_groups,
            size,
        };
        Ok((grid, passed_up))
    }

    /// Lay the grid out in `bounds` and draw it. `fill_background`: draw the
    /// background fill (a nested grid; the top grid's is the scene's). With
    /// `interaction`, also collect every plot's tooltips and their warnings.
    fn draw(
        &self,
        bounds: Rect,
        fill_background: bool,
        mut interaction: Option<&mut (Interaction, Vec<String>)>,
    ) -> Result<Vec<Node>> {
        let theme = &self.theme;
        let layout = &self.spec.layout;
        let mut nodes = Vec::new();
        let border = theme
            .plot_background
            .and_then(|background| background.border);
        if let Some(background) = theme.plot_background
            && (fill_background || border.is_some())
        {
            nodes.push(Node::Rects(Rects {
                rects: vec![bounds],
                fill: PerItem::Uniform(fill_background.then_some(background.fill)),
                stroke: border.map(|border| border.stroke()),
            }));
        }
        // lets-plot `CompositeFigureBuildInfo.layoutedByOuterSize`: margins (and the
        // border) -> titles -> inset -> collected legends -> cells.
        let border_width = border.map_or(0.0, |border| border.width);
        let content = shrink(
            bounds,
            theme.plot_margin.plus(Margins::uniform(border_width)),
        );
        let headings = headings_band(theme, &self.titles);
        let without_titles = Rect {
            y: content.y + headings,
            height: (content.height - headings).max(1.0),
            ..content
        };
        let blocks = self.measure_legend_groups()?;
        let elements_area = shrink(
            shrink(without_titles, theme.plot_inset),
            legend_space(&blocks, theme.legend_box_spacing),
        );
        let cells = cell_rects(
            elements_area,
            (layout.ncol as usize, layout.nrow as usize),
            (
                layout.hspace.unwrap_or(DEFAULT_SPACING),
                layout.vspace.unwrap_or(DEFAULT_SPACING),
            ),
            layout.widths.as_deref(),
            layout.heights.as_deref(),
        );
        let fit = layout.fit.unwrap_or(true);
        let mut placed = Vec::with_capacity(self.elements.len());
        for (element, cell) in self.elements.iter().zip(cells) {
            let Some(element) = element else {
                placed.push(None);
                continue;
            };
            let cell = if fit {
                cell
            } else {
                shrink_to_aspect(cell, element.own_size())
            };
            placed.push(Some(match element {
                Element::Plot(plot) => Placed::Plot(plot, Box::new(plot.arrange(cell)?)),
                Element::Grid(grid) => Placed::Grid(grid, cell),
            }));
        }
        if layout.align.unwrap_or(false) {
            align(&mut placed, layout.ncol as usize, layout.nrow as usize)?;
        }
        let union = placed
            .iter()
            .flatten()
            .map(Placed::bounds)
            .reduce(union)
            .unwrap_or(elements_area);
        for figure in placed.iter().flatten() {
            let mut figure_nodes = Vec::new();
            match figure {
                Placed::Plot(plot, arrangement) => {
                    if let Some(fill) = plot.background() {
                        figure_nodes.push(Node::Rects(Rects {
                            rects: vec![arrangement.area()],
                            fill: PerItem::Uniform(Some(fill)),
                            stroke: None,
                        }));
                    }
                    figure_nodes.extend(plot.draw(arrangement)?);
                    if let Some((collected, notes)) = interaction.as_deref_mut() {
                        plot.interaction(arrangement, collected, notes)?;
                    }
                }
                Placed::Grid(grid, bounds) => {
                    figure_nodes.extend(grid.draw(*bounds, true, interaction.as_deref_mut())?)
                }
            }
            nodes.push(Node::Group(Group {
                clip: None,
                role: Some("figure"),
                nodes: figure_nodes,
            }));
        }
        nodes.extend(heading_nodes(
            content.y,
            (union.x, union.width),
            theme,
            &self.titles,
        ));
        for (group, block, block_theme) in &blocks {
            let origin = legend_origin(union, without_titles, block.size, group);
            nodes.extend(draw_legends(block, origin, block_theme)?);
        }
        Ok(nodes)
    }

    /// Each legend group measured, with the theme it is measured and drawn with
    /// (its first figure's, at the group's position and justification, with the
    /// grid's box arrangement).
    fn measure_legend_groups(&self) -> Result<Vec<(&Collected, LegendBlock<'_>, Theme)>> {
        let mut blocks = Vec::new();
        for group in &self.legend_groups {
            // Boxes keep their figure's theme; the grid's theme arranges them
            // (lets-plot `LegendsBlockInfo.arrangeLegendBoxes`).
            let mut block_theme = group.theme.clone();
            block_theme.legend_position = group.position;
            block_theme.legend_justification = group.justification;
            block_theme.legend_box_horizontal = self.theme.legend_box_horizontal;
            block_theme.legend_box_just = self.theme.legend_box_just;
            block_theme.legend_spacing = self.theme.legend_spacing;
            if let Some(block) = measure_legends(&group.boxes, &block_theme)? {
                blocks.push((group, block, block_theme));
            }
        }
        Ok(blocks)
    }
}

/// lets-plot `CompositeFigureGridAlignmentLayout`: every plot's panel takes the
/// x range its grid column's panels share and the y range its row's panels share,
/// and the plot is laid out again around it. Nested grids are not moved and do not
/// take part.
fn align(placed: &mut [Option<Placed>], columns: usize, rows: usize) -> Result<()> {
    let panel = |index: usize| match &placed[index] {
        Some(Placed::Plot(_, arrangement)) => Some(arrangement.panel_bounds()),
        _ => None,
    };
    let intersect = |spans: &mut dyn Iterator<Item = (f64, f64)>| {
        spans.reduce(|(start, end), (other_start, other_end)| {
            (start.max(other_start), end.min(other_end))
        })
    };
    let row_spans: Vec<Option<(f64, f64)>> = (0..rows)
        .map(|row| {
            intersect(
                &mut (0..columns)
                    .filter_map(|column| panel(row * columns + column))
                    .map(|rect| (rect.y, rect.y + rect.height)),
            )
        })
        .collect();
    let column_spans: Vec<Option<(f64, f64)>> = (0..columns)
        .map(|column| {
            intersect(
                &mut (0..rows)
                    .filter_map(|row| panel(row * columns + column))
                    .map(|rect| (rect.x, rect.x + rect.width)),
            )
        })
        .collect();
    for (index, figure) in placed.iter_mut().enumerate() {
        let Some(Placed::Plot(plot, arrangement)) = figure else {
            continue;
        };
        let (Some((left, right)), Some((top, bottom))) =
            (column_spans[index % columns], row_spans[index / columns])
        else {
            continue;
        };
        // Panels that do not overlap at all leave nothing to share.
        if right <= left || bottom <= top {
            continue;
        }
        let target = Rect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        };
        **arrangement = plot.arrange_panel(arrangement.area(), target)?;
    }
    Ok(())
}

/// Space the legend blocks take on each side: per side the largest block plus
/// `spacing` (blocks on one side differ in justification, so they sit side by side).
fn legend_space(blocks: &[(&Collected, LegendBlock, Theme)], spacing: f64) -> Margins {
    let mut space = Margins::uniform(0.0);
    for (group, block, _) in blocks {
        let (width, height) = block.size;
        match group.position {
            LegendPosition::Left => space.left = space.left.max(width + spacing),
            LegendPosition::Right => space.right = space.right.max(width + spacing),
            LegendPosition::Top => space.top = space.top.max(height + spacing),
            LegendPosition::Bottom => space.bottom = space.bottom.max(height + spacing),
            LegendPosition::None | LegendPosition::Inside(..) => {}
        }
    }
    space
}

/// Top-left corner of a collected legend block (lets-plot
/// `LegendBoxesLayoutUtil.overlayLegendOriginOutsidePlot`): at the edge of `outer`
/// (the grid without titles), justified along `inner` (the figures).
fn legend_origin(inner: Rect, outer: Rect, size: (f64, f64), group: &Collected) -> (f64, f64) {
    let (justify_x, justify_y) = group.justification;
    let along_y = inner.y + (inner.height - size.1) * (1.0 - justify_y);
    let along_x = inner.x + (inner.width - size.0) * justify_x;
    match group.position {
        LegendPosition::Left => (outer.x, along_y),
        LegendPosition::Top => (along_x, outer.y),
        LegendPosition::Bottom => (along_x, outer.y + outer.height - size.1),
        // Only fixed positions are collected.
        LegendPosition::Right | LegendPosition::None | LegendPosition::Inside(..) => {
            (outer.x + outer.width - size.0, along_y)
        }
    }
}

/// Collected legends grouped by placement, in order of first appearance, each
/// group without duplicates (lets-plot `LegendsBlockInfo.arrangeLegendBoxes`).
fn group_legends(collected: Vec<Collected>) -> Vec<Collected> {
    let mut groups: Vec<Collected> = Vec::new();
    for item in collected {
        let index = match groups.iter().position(|group| {
            group.position == item.position && group.justification == item.justification
        }) {
            Some(index) => index,
            None => {
                groups.push(Collected {
                    boxes: Vec::new(),
                    ..item
                });
                groups.len() - 1
            }
        };
        let group = &mut groups[index];
        for legend_box in item.boxes {
            if !group
                .boxes
                .iter()
                .any(|kept| kept.same_content(&legend_box))
            {
                group.boxes.push(legend_box);
            }
        }
    }
    groups
}

/// Container theme options a figure with its own theme options still takes (lets-plot
/// `ThemeConfig.mergeUserOptions`).
const INHERITED_THEME_OPTIONS: &[&str] = &[
    "flavor",
    "legend_position",
    "legend_justification",
    "legend_direction",
    "legend_box_just",
    "plot_tag_position",
    "plot_tag_location",
    "plot_tag",
    "plot_tag_prefix",
    "plot_tag_suffix",
];

/// Theme options of a figure inside a grid (lets-plot `ThemeConfig.mergeUserOptions`):
/// a figure without theme options takes all of the grid's; one with options takes
/// the grid's theme name, flavor and legend placement under its own. Unless the
/// figure sets a flavor, its background shows the grid's fill (and is blank when the
/// grid's is) under its own background options.
fn child_theme(
    container: &ThemeSpec,
    container_theme: &Theme,
    own: Option<&ThemeSpec>,
) -> ThemeSpec {
    let own_is_empty = own.is_none_or(|own| own.name.is_none() && own.elements.is_empty());
    let mut merged = if own_is_empty {
        container.clone()
    } else {
        ThemeSpec {
            name: container.name.clone(),
            elements: container
                .elements
                .iter()
                .filter(|(key, _)| INHERITED_THEME_OPTIONS.contains(&key.as_str()))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        }
    };
    if let Some(own) = own {
        if own.name.is_some() {
            merged.name.clone_from(&own.name);
        }
        merged.elements.extend(
            own.elements
                .iter()
                .map(|(key, value)| (key.clone(), value.clone())),
        );
    }
    let own_background = own.and_then(|own| own.elements.get("plot_background"));
    if own.is_some_and(|own| own.elements.contains_key("flavor")) {
        // A flavor keeps the figure's own background.
        match own_background {
            Some(background) => {
                merged
                    .elements
                    .insert("plot_background".to_owned(), background.clone());
            }
            None => {
                merged.elements.remove("plot_background");
            }
        }
        return merged;
    }
    let background = match (container_theme.plot_background, own_background) {
        (None, _) | (_, Some(ThemeValue::Blank)) => ThemeValue::Blank,
        (Some(container_background), own_background) => {
            let mut element = match own_background {
                Some(ThemeValue::Element(element)) => (**element).clone(),
                _ => ElementSpec::default(),
            };
            if element.fill.is_none() {
                element.fill = Some(color_text(container_background.fill));
            }
            ThemeValue::Element(Box::new(element))
        }
    };
    merged
        .elements
        .insert("plot_background".to_owned(), background);
    merged
}

/// A color as theme option text (`#rrggbb`, or `rgba(...)` when not opaque).
fn color_text(color: Color) -> String {
    if color.alpha == u8::MAX {
        color.to_string()
    } else {
        format!(
            "rgba({}, {}, {}, {})",
            color.red,
            color.green,
            color.blue,
            f64::from(color.alpha) / 255.0
        )
    }
}

/// Warn about grid options that are parsed but not drawn.
fn report_grid_unsupported(spec: &SubplotsSpec, path: &str, warnings: &mut Vec<String>) {
    let mut ignored = |present: bool, feature: &str| {
        if present {
            warnings.push(nest(
                path,
                &format!("'{feature}' is not supported yet and was ignored"),
            ));
        }
    };
    ignored(spec.caption.is_some(), "caption");
    ignored(spec.ggtoolbar.is_some(), "ggtb");
    ignored(!spec.guides.is_empty(), "guides");
    let keys = spec
        .unsupported
        .keys()
        .map(String::as_str)
        .map(str::to_owned)
        .chain(
            spec.layout
                .unsupported
                .keys()
                .map(|key| format!("layout.{key}")),
        );
    for key in keys {
        warnings.push(nest(
            path,
            &format!("unsupported option '{key}' was ignored"),
        ));
    }
}

/// Union of two rectangles.
fn union(left: Rect, right: Rect) -> Rect {
    let x = left.x.min(right.x);
    let y = left.y.min(right.y);
    Rect {
        x,
        y,
        width: (left.x + left.width).max(right.x + right.width) - x,
        height: (left.y + left.height).max(right.y + right.height) - y,
    }
}

/// `message` about a figure at `path` (`figures[1].`, or `figures[1]` for a plot):
/// `at 'theme.x'` becomes `at 'figures[1].theme.x'`.
fn nest(path: &str, message: &str) -> String {
    let path = path.trim_end_matches('.');
    if path.is_empty() {
        return message.to_owned();
    }
    match message.strip_prefix("at '") {
        Some(rest) => format!("at '{path}.{rest}"),
        None => format!("at '{path}': {message}"),
    }
}

/// [`nest`] for errors: specification errors take the path, others a note.
fn nest_error(error: Error, path: &str) -> Error {
    match error {
        Error::InvalidSpec(message) => Error::InvalidSpec(nest(path, &message)),
        other if path.trim_end_matches('.').is_empty() => other,
        other => other.with_notes(&[format!("in '{}'", path.trim_end_matches('.'))]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Value;

    fn theme_spec(json: &str) -> ThemeSpec {
        serde_json::from_str(json).expect("valid theme")
    }

    fn resolved(spec: &ThemeSpec) -> Theme {
        Theme::from_spec(Some(spec), &mut Vec::new()).expect("valid theme")
    }

    #[test]
    fn figure_without_theme_takes_the_whole_grid_theme() {
        let container = theme_spec(
            r#"{"name": "classic", "axis_text": {"size": 20}, "legend_position": "bottom"}"#,
        );
        let merged = child_theme(&container, &resolved(&container), None);
        assert_eq!(merged.name.as_deref(), Some("classic"));
        assert!(merged.elements.contains_key("axis_text"));
        assert!(merged.elements.contains_key("legend_position"));
    }

    #[test]
    fn figure_with_theme_takes_only_name_and_legend_placement() {
        let container = theme_spec(
            r#"{"name": "classic", "axis_text": {"size": 20}, "legend_position": "bottom"}"#,
        );
        let own = theme_spec(r#"{"axis_title": {"size": 9}}"#);
        let merged = child_theme(&container, &resolved(&container), Some(&own));
        assert_eq!(merged.name.as_deref(), Some("classic"));
        assert!(!merged.elements.contains_key("axis_text"));
        assert!(merged.elements.contains_key("axis_title"));
        assert_eq!(
            merged.elements.get("legend_position"),
            Some(&ThemeValue::Scalar(Value::String("bottom".into())))
        );
    }

    #[test]
    fn background_fill_comes_from_the_grid_unless_set() {
        let container = theme_spec(r##"{"plot_background": {"fill": "#ff0000", "size": 3}}"##);
        let merged = child_theme(&container, &resolved(&container), None);
        let Some(ThemeValue::Element(background)) = merged.elements.get("plot_background") else {
            panic!("expected a background element");
        };
        assert_eq!(background.fill.as_deref(), Some("#ff0000"));
        assert_eq!(background.size, None, "the grid's border is not inherited");
        let own = theme_spec(r#"{"plot_background": {"fill": "blue"}}"#);
        let merged = child_theme(&container, &resolved(&container), Some(&own));
        let Some(ThemeValue::Element(background)) = merged.elements.get("plot_background") else {
            panic!("expected a background element");
        };
        assert_eq!(background.fill.as_deref(), Some("blue"));
        let blank = theme_spec(r#"{"plot_background": "blank"}"#);
        let merged = child_theme(&blank, &resolved(&blank), None);
        assert_eq!(
            merged.elements.get("plot_background"),
            Some(&ThemeValue::Blank)
        );
    }

    #[test]
    fn messages_take_the_figure_path() {
        assert_eq!(
            nest("figures[1]", "at 'theme.x': bad"),
            "at 'figures[1].theme.x': bad"
        );
        assert_eq!(
            nest("figures[0].figures[2].", "oops"),
            "at 'figures[0].figures[2]': oops"
        );
        assert_eq!(nest("", "oops"), "oops");
    }
}
