//! Grids of figures: `gggrid` (lets-plot `CompositeFigureBuildInfo`,
//! `CompositeFigureGridLayout`, `CompositeFigureGridAlignmentLayout`).
//!
//! What it does: prepare every figure of a grid once (plots and nested grids), give
//! each its cells ([`crate::layout::grid`]: relative `widths`/`heights`,
//! `hspace`/`vspace`, `fit`), lay plots out at their cell's size, align panels across
//! rows and columns (`align`), collect legends (`guides`), and draw the grid's
//! background, title, caption, tag and collected legends. Figures inherit the grid's theme as in
//! lets-plot ([`child_theme`]).
//!
//! ggrust extensions (patchwork): `design=` areas (figures span cells,
//! [`crate::layout::grid::figure_spans`]), nested grids taking part in `align`
//! ([`GridLayout::frame`]), automatic tags ([`crate::grid_tags`]) and insets
//! ([`inset`]).
//!
//! Shared axes (`sharex`/`sharey`): [`crate::grid_share`].
//!
//! What it does not do (warned): grid-level `guides(...)`.

use crate::build::{
    ALIGN_PASSES, ALIGN_TOLERANCE, Arrangement, BuiltPlot, DEFAULT_SIZE, PlotData, PreparedPlot,
    build_plot_with,
};
use crate::data::DataFrame;
use crate::grid_share;
use crate::grid_tags::{self, FigureTags};
use crate::layout::PANEL_MIN_SIZE;
use crate::layout::grid::{
    DEFAULT_SPACING, Span, cell_rects, figure_spans, shrink_to_aspect, span_rect,
};
use crate::layout::tag::{tag_bands, tag_nodes};
use crate::layout::zoom::Zoom;
use crate::layout::{
    LegendBlock, LegendBox, Titles, caption_band, caption_nodes, draw_legends, heading_nodes,
    headings_band, legend_budget, measure_themed_legends, shrink,
};
use crate::parallel;
use crate::scene::{Color, Group, Node, PerItem, Rect, Rects, Scene};
use crate::spec::{
    CaptionSpec, ElementSpec, Figure, PlotSpec, SubplotsSpec, ThemeSpec, ThemeValue,
};
use crate::theme::{LegendPosition, Margins, Theme};
use crate::tooltip::Interaction;
use crate::{Error, Result};

mod inset;
use inset::PreparedInset;

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
        Figure::Plot(spec) if spec.insets.is_empty() => {
            build_plot_with(spec, &PlotData::from_frames(spec, frames)?, interactive)
        }
        _ => {
            let mut warnings = Vec::new();
            let context = GridContext {
                frames,
                container: None,
                container_collects: false,
                container_aligns: false,
                tags: None,
                path: "",
            };
            let toolbar = match figure {
                Figure::Plot(spec) => spec.ggtoolbar.as_ref(),
                Figure::Subplots(spec) => spec.ggtoolbar.as_ref(),
            };
            let zoom = if interactive {
                crate::layout::zoom::Zoom::from_spec(toolbar, "ggtoolbar")?
            } else {
                None
            };
            let mut collected = interactive.then(|| {
                let interaction = Interaction {
                    zoom,
                    ..Interaction::default()
                };
                (interaction, Vec::new())
            });
            let at_origin = |(width, height): (f64, f64)| Rect {
                x: 0.0,
                y: 0.0,
                width,
                height,
            };
            let ((width, height), background, nodes) = match figure {
                Figure::Subplots(spec) => {
                    let (grid, _) = PreparedGrid::new(spec, &context, &mut warnings)
                        .map_err(|error| error.with_notes(&warnings))?;
                    let nodes = grid.draw(at_origin(grid.size), false, collected.as_mut())?;
                    let background = grid.theme.plot_background.map(|background| background.fill);
                    (grid.size, background, nodes)
                }
                // A plot with insets.
                Figure::Plot(spec) => {
                    let mut plot = PlotData::from_frames(spec, frames)
                        .and_then(|data| crate::build::prepare_plot(spec, &data))?;
                    warnings.append(&mut plot.warnings);
                    let insets = inset::prepare(&spec.insets, &[], &context, &mut warnings)
                        .map_err(|error| error.with_notes(&warnings))?;
                    let arrangement = plot.arrange(at_origin(plot.size), PANEL_MIN_SIZE)?;
                    let placed = Placed::Plot(&plot, Box::new(arrangement), &insets);
                    let nodes = placed.draw(collected.as_mut(), false)?;
                    (plot.size, plot.background(), nodes)
                }
            };
            let scene = Scene {
                width,
                height,
                background,
                nodes,
            };
            scene.validate()?;
            if let Some(zoom) = collected
                .as_ref()
                .and_then(|(interaction, _)| interaction.zoom.as_ref())
            {
                zoom.check_layer_labels("ggtoolbar")?;
            }
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
            Some(other) => Err(Error::spec(
                "layout.guides",
                format!("expected 'auto', 'collect' or 'keep', got '{other}'"),
            )
            .nested(path)),
        }
    }
}

/// Legends collected from one figure, placed as that figure would place them.
struct Collected {
    position: LegendPosition,
    justification: (f64, f64),
    boxes: Vec<LegendBox>,
    /// Per box, the theme of the figure it came from: lets-plot builds legend boxes
    /// in their figure, so each keeps its legend text, keys and spacing. Once
    /// grouped, these carry the group's arrangement too ([`arranged`]).
    box_themes: Vec<Theme>,
    /// The first figure's theme: it and the grid's theme arrange the group's block.
    theme: Theme,
}

/// `theme` with the arrangement of a collected legend group: its placement, and
/// the grid's box layout and spacing (lets-plot `LegendsBlockInfo.arrangeLegendBoxes`).
fn arranged(theme: &Theme, group: &Collected, grid_theme: &Theme) -> Theme {
    let mut theme = theme.clone();
    theme.legend_position = group.position;
    theme.legend_justification = group.justification;
    theme.legend_box_horizontal = grid_theme.legend_box_horizontal;
    theme.legend_box_just = grid_theme.legend_box_just;
    theme.legend_spacing = grid_theme.legend_spacing;
    theme
}

/// What a grid's figures see of it while they are prepared.
#[derive(Clone, Copy)]
struct GridContext<'a> {
    frames: &'a [DataFrame],
    /// The containing grid's theme options and theme.
    container: Option<(&'a ThemeSpec, &'a Theme)>,
    /// Whether the containing grid collects legends.
    container_collects: bool,
    /// Whether the containing grid aligns panels: a nested grid without its own
    /// `align` follows it.
    container_aligns: bool,
    /// The tags of this grid's figures and insets, when a containing grid numbers
    /// them ([`crate::grid_tags`]).
    tags: Option<&'a FigureTags>,
    /// Spec path of this grid (`figures[1].`), for messages.
    path: &'a str,
}

enum Element<'s> {
    Plot(Box<PreparedPlot>, Vec<PreparedInset<'s>>),
    Grid(Box<PreparedGrid<'s>>),
}

impl Element<'_> {
    /// Rows of every prepared layer of the figure.
    fn prepared_rows(&self) -> usize {
        match self {
            Element::Plot(plot, insets) => plot.prepared_rows() + inset::prepared_rows(insets),
            Element::Grid(grid) => {
                grid.elements
                    .iter()
                    .flatten()
                    .map(Element::prepared_rows)
                    .sum::<usize>()
                    + inset::prepared_rows(&grid.insets)
            }
        }
    }

    /// Size the figure has on its own (`ggsize` or the default), for `fit=False`.
    fn own_size(&self) -> (f64, f64) {
        match self {
            Element::Plot(plot, _) => plot.size,
            Element::Grid(grid) => grid.size,
        }
    }
}

/// A figure placed in its cell.
enum Placed<'e, 's> {
    Plot(&'e PreparedPlot, Box<Arrangement>, &'e [PreparedInset<'s>]),
    Grid(&'e PreparedGrid<'s>, Box<GridLayout<'e, 's>>),
}

impl<'e, 's> Placed<'e, 's> {
    /// `element` laid out in `area`.
    fn new(element: &'e Element<'s>, area: Rect) -> Result<Placed<'e, 's>> {
        Ok(match element {
            Element::Plot(plot, insets) => Placed::Plot(
                plot,
                Box::new(plot.arrange(area, CELL_PANEL_MIN_SIZE)?),
                insets,
            ),
            Element::Grid(grid) => Placed::Grid(grid, Box::new(grid.layout(area)?)),
        })
    }

    fn bounds(&self) -> Rect {
        match self {
            Placed::Plot(_, arrangement, _) => arrangement.area(),
            Placed::Grid(_, layout) => layout.bounds,
        }
    }

    /// The panels' outer edges: a plot's panels, or a grid's [`GridLayout::frame`].
    fn frame(&self) -> Option<Rect> {
        match self {
            Placed::Plot(_, arrangement, _) => Some(arrangement.panel_bounds()),
            Placed::Grid(_, layout) => layout.frame(),
        }
    }

    /// Laid out again so the panels' outer edges are at `target` (`align`).
    fn align_to(&mut self, target: Rect) -> Result<()> {
        match self {
            Placed::Plot(plot, arrangement, _) => {
                **arrangement =
                    plot.arrange_panel(arrangement.area(), target, CELL_PANEL_MIN_SIZE)?;
            }
            Placed::Grid(grid, layout) => {
                **layout = grid.layout_panel(layout.bounds, target)?;
            }
        }
        Ok(())
    }

    /// Draw the figure (a plot with its background and insets). With
    /// `interaction`, also collect its tooltips.
    fn draw(
        &self,
        mut interaction: Option<&mut (Interaction, Vec<String>)>,
        fill_background: bool,
    ) -> Result<Vec<Node>> {
        let mut nodes = Vec::new();
        match self {
            Placed::Plot(plot, arrangement, insets) => {
                if fill_background && let Some(fill) = plot.background() {
                    nodes.push(Node::Rects(Rects {
                        rects: vec![arrangement.area()],
                        fill: PerItem::Uniform(Some(fill)),
                        stroke: None,
                    }));
                }
                let areas = inset::HostAreas::of_plot(plot, arrangement);
                nodes.extend(inset::draw(
                    insets,
                    &areas,
                    false,
                    interaction.as_deref_mut(),
                )?);
                nodes.extend(plot.draw(arrangement)?);
                if let Some((collected, notes)) = interaction.as_deref_mut() {
                    plot.interaction(arrangement, collected, notes)?;
                }
                nodes.extend(inset::draw(insets, &areas, true, interaction)?);
            }
            Placed::Grid(grid, layout) => {
                nodes.extend(grid.draw_layout(layout, fill_background, interaction)?)
            }
        }
        Ok(nodes)
    }
}

/// Where a grid's parts go (from [`PreparedGrid::layout`]).
struct GridLayout<'e, 's> {
    /// The area the grid fills.
    bounds: Rect,
    /// Inside the margins (and border): the tag's area.
    inner: Rect,
    /// Inside the margins and the tag's bands: titles, legends and figures.
    content: Rect,
    /// `content` below the titles: the collected legends' outer area.
    without_titles: Rect,
    /// The figures' cells, inside the inset and the collected legends.
    elements_area: Rect,
    blocks: Vec<(&'e Collected, LegendBlock<'e>, Theme)>,
    placed: Vec<Option<Placed<'e, 's>>>,
    /// The cells each placed figure spans.
    spans: &'e [Span],
}

impl GridLayout<'_, '_> {
    /// The outer edges of the grid's panels: on each side, the panels of the
    /// figures nearest that side (their shared range: the innermost edge). None
    /// without panels.
    fn frame(&self) -> Option<Rect> {
        let figures: Vec<(Span, Rect)> = self
            .placed
            .iter()
            .zip(self.spans)
            .filter_map(|(placed, span)| Some((*span, placed.as_ref()?.frame()?)))
            .collect();
        let first_column = figures.iter().map(|(span, _)| span.left).min()?;
        let last_column = figures.iter().map(|(span, _)| span.right).max()?;
        let first_row = figures.iter().map(|(span, _)| span.top).min()?;
        let last_row = figures.iter().map(|(span, _)| span.bottom).max()?;
        let edge = |keep: &dyn Fn(&Span) -> bool, value: &dyn Fn(&Rect) -> f64, inner_max: bool| {
            figures
                .iter()
                .filter(|(span, _)| keep(span))
                .map(|(_, rect)| value(rect))
                .reduce(|a, b| if inner_max { a.max(b) } else { a.min(b) })
        };
        let left = edge(&|span| span.left == first_column, &|rect| rect.x, true)?;
        let right = edge(
            &|span| span.right == last_column,
            &|rect| rect.x + rect.width,
            false,
        )?;
        let top = edge(&|span| span.top == first_row, &|rect| rect.y, true)?;
        let bottom = edge(
            &|span| span.bottom == last_row,
            &|rect| rect.y + rect.height,
            false,
        )?;
        (right > left && bottom > top).then_some(Rect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        })
    }
}

/// A grid with every figure prepared: ready to be laid out at any size.
struct PreparedGrid<'s> {
    spec: &'s SubplotsSpec,
    theme: Theme,
    titles: Titles,
    elements: Vec<Option<Element<'s>>>,
    /// The cells each element spans (`design=` areas, or one cell each).
    spans: Vec<Span>,
    /// Whether panels are aligned (`align`, else the containing grid's).
    align: bool,
    /// `+ inset_element(...)` figures of the grid itself.
    insets: Vec<PreparedInset<'s>>,
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
        if columns == 0 || rows == 0 {
            return Err(Error::spec(
                "layout",
                format!(
                    "a grid needs at least one column and row, got {} x {}",
                    layout.ncol, layout.nrow
                ),
            )
            .nested(path));
        }
        let spans = figure_spans((columns, rows), spec.figures.len(), layout.areas.as_deref())
            .map_err(|message| Error::spec("layout.areas", message).nested(path))?;
        if layout.areas.is_none() && spec.figures.len() != columns * rows {
            return Err(Error::spec(
                "layout",
                format!(
                    "{} figures do not fill a grid of {} columns and {} rows",
                    spec.figures.len(),
                    layout.ncol,
                    layout.nrow
                ),
            )
            .nested(path));
        }
        for (key, sizes) in [("widths", &layout.widths), ("heights", &layout.heights)] {
            let Some(sizes) = sizes else { continue };
            let valid = sizes.iter().all(|size| size.is_finite() && *size >= 0.0)
                && sizes.iter().sum::<f64>() > 0.0;
            if !valid {
                return Err(Error::spec(
                    format!("layout.{key}"),
                    format!("expected relative sizes >= 0 with a positive sum, got {sizes:?}"),
                )
                .nested(path));
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
                return Err(Error::spec(
                    "ggsize",
                    format!(
                        "width and height must be positive, got {} x {}",
                        size.width, size.height
                    ),
                )
                .nested(path));
            }
            // lets-plot `CompositeFigureGridLayoutBase.defaultSize`.
            None => (
                DEFAULT_SIZE.0 * (0.5 + 0.5 / columns as f64) * columns as f64,
                DEFAULT_SIZE.1 * (0.5 + 0.5 / rows as f64) * rows as f64,
            ),
        };
        grid_tags::starts_level(spec).map_err(|error| nest_error(error, path))?;
        let own_tags = match context.tags {
            Some(_) => None,
            None => {
                grid_tags::grid_tags(spec, warnings).map_err(|error| nest_error(error, path))?
            }
        };
        let (figure_tags, inset_tags) = match context.tags.or(own_tags.as_ref()) {
            Some(FigureTags::Grid(figures, insets)) => {
                (Some(figures.as_slice()), insets.as_slice())
            }
            _ => (None, &[][..]),
        };
        let figure_tag = |index: usize| figure_tags.and_then(|tags| tags.get(index));
        let align = layout.align.unwrap_or(context.container_aligns);
        let own_guides = Guides::of(spec, path)?;
        let collects = if context.container_collects {
            own_guides != Guides::Keep
        } else {
            own_guides == Guides::Collect
        };
        let mut collected = Vec::new();
        let mut elements = Vec::with_capacity(spec.figures.len());
        grid_share::validate(layout).map_err(|error| nest_error(error, path))?;
        // Plots prepare in parallel; their results are taken in figure order below.
        let rows = context.frames.iter().map(DataFrame::row_count).sum();
        let plot_specs: Vec<Option<PlotSpec>> = spec
            .figures
            .iter()
            .enumerate()
            .map(|(index, figure)| match figure {
                Some(Figure::Plot(plot_spec)) => Some(figure_plot_spec(
                    plot_spec,
                    Some((&theme_spec, &theme)),
                    figure_tag(index),
                )),
                _ => None,
            })
            .collect();
        let mut prepared: Vec<Option<Result<PreparedPlot>>> = parallel::ordered_map(
            plot_specs.iter().collect(),
            parallel::is_worth(rows),
            |plot_spec| {
                plot_spec.as_ref().map(|plot_spec| {
                    PlotData::from_frames(plot_spec, context.frames)
                        .and_then(|data| crate::build::prepare_plot(plot_spec, &data))
                })
            },
        );
        grid_share::share_axes(layout, &plot_specs, &mut prepared, context.frames)
            .map_err(|error| nest_error(error, path))?;
        for (index, figure) in spec.figures.iter().enumerate() {
            let figure_path = format!("{path}figures[{index}]");
            let child_path = format!("{figure_path}.");
            let inset_context = GridContext {
                frames: context.frames,
                container: Some((&theme_spec, &theme)),
                container_collects: false,
                container_aligns: false,
                tags: None,
                path: &child_path,
            };
            let element = match figure {
                None => None,
                Some(Figure::Plot(plot_spec)) => {
                    let mut plot = prepared[index]
                        .take()
                        .expect("every plot figure was prepared")
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
                        let boxes = std::mem::take(&mut plot.legends);
                        collected.push(Collected {
                            position: plot_theme.legend_position,
                            justification: plot_theme.legend_justification,
                            box_themes: vec![plot_theme.clone(); boxes.len()],
                            theme: plot_theme,
                            boxes,
                        });
                    }
                    let tags = match figure_tag(index) {
                        Some(FigureTags::Plot(_, insets)) => insets.as_slice(),
                        _ => &[],
                    };
                    let insets = inset::prepare(&plot_spec.insets, tags, &inset_context, warnings)?;
                    Some(Element::Plot(Box::new(plot), insets))
                }
                Some(Figure::Subplots(grid_spec)) => {
                    let child_context = GridContext {
                        frames: context.frames,
                        container: Some((&theme_spec, &theme)),
                        container_collects: collects,
                        container_aligns: align,
                        tags: figure_tag(index).filter(|tags| matches!(tags, FigureTags::Grid(..))),
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
            (group_legends(collected, &theme), Vec::new())
        };
        let title = spec.ggtitle.as_ref();
        let titles = Titles {
            title: title.and_then(|title| title.text.clone()),
            subtitle: title.and_then(|title| title.subtitle.clone()),
            caption: spec
                .caption
                .as_ref()
                .and_then(|caption| caption.text.clone()),
            tag: spec.tag.as_ref().and_then(|tag| tag.text.clone()),
            ..Default::default()
        };
        let insets = {
            let inset_context = GridContext {
                frames: context.frames,
                container: Some((&theme_spec, &theme)),
                container_collects: false,
                container_aligns: false,
                tags: None,
                path,
            };
            inset::prepare(&spec.insets, inset_tags, &inset_context, warnings)?
        };
        let grid = PreparedGrid {
            spec,
            theme,
            titles,
            elements,
            spans,
            align,
            insets,
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
        interaction: Option<&mut (Interaction, Vec<String>)>,
    ) -> Result<Vec<Node>> {
        let layout = self.layout(bounds)?;
        self.draw_layout(&layout, fill_background, interaction)
    }

    /// Lay the grid out in `bounds`: lets-plot `CompositeFigureBuildInfo.layoutedByOuterSize`:
    /// margins (and the border) -> tag bands -> titles -> inset -> collected legends
    /// -> cells, then panels aligned (`align`).
    fn layout<'e>(&'e self, bounds: Rect) -> Result<GridLayout<'e, 's>> {
        let theme = &self.theme;
        let layout = &self.spec.layout;
        let border_width = theme
            .plot_background
            .and_then(|background| background.border)
            .map_or(0.0, |border| border.width);
        let inner = shrink(
            bounds,
            theme.plot_margin.plus(Margins::uniform(border_width)),
        );
        let content = shrink(inner, tag_bands(theme, &self.titles.tag));
        let headings = headings_band(theme, &self.titles);
        let caption = caption_band(theme, &self.titles);
        let without_titles = Rect {
            y: content.y + headings,
            height: (content.height - headings - caption).max(1.0),
            ..content
        };
        let blocks = self.measure_legend_groups(shrink(without_titles, theme.plot_inset))?;
        let elements_area = shrink(
            shrink(without_titles, theme.plot_inset),
            legend_space(&blocks, theme.legend_box_spacing),
        );
        let columns = layout.ncol as usize;
        let cells = cell_rects(
            elements_area,
            (columns, layout.nrow as usize),
            (
                layout.hspace.unwrap_or(DEFAULT_SPACING),
                layout.vspace.unwrap_or(DEFAULT_SPACING),
            ),
            layout.widths.as_deref(),
            layout.heights.as_deref(),
        );
        let fit = layout.fit.unwrap_or(true);
        let mut placed = Vec::with_capacity(self.elements.len());
        for (element, span) in self.elements.iter().zip(&self.spans) {
            let Some(element) = element else {
                placed.push(None);
                continue;
            };
            let cell = span_rect(&cells, columns, *span);
            let cell = if fit {
                cell
            } else {
                shrink_to_aspect(cell, element.own_size())
            };
            placed.push(Some(Placed::new(element, cell)?));
        }
        if self.align {
            align(&mut placed, &self.spans)?;
        }
        Ok(GridLayout {
            bounds,
            inner,
            content,
            without_titles,
            elements_area,
            blocks,
            placed,
            spans: &self.spans,
        })
    }

    /// [`PreparedGrid::layout`] with the panels' outer edges ([`GridLayout::frame`])
    /// at `target`, as [`PreparedPlot::arrange_panel`]: the area is moved by the
    /// edges' distance from `target` until they meet.
    fn layout_panel<'e>(&'e self, area: Rect, target: Rect) -> Result<GridLayout<'e, 's>> {
        let mut area = area;
        let mut layout = self.layout(area)?;
        for _ in 0..ALIGN_PASSES {
            let Some(frame) = layout.frame() else {
                break;
            };
            let left = target.x - frame.x;
            let top = target.y - frame.y;
            let right = (target.x + target.width) - (frame.x + frame.width);
            let bottom = (target.y + target.height) - (frame.y + frame.height);
            if [left, top, right, bottom]
                .iter()
                .all(|distance| distance.abs() < ALIGN_TOLERANCE)
            {
                break;
            }
            let moved = Rect {
                x: area.x + left,
                y: area.y + top,
                width: area.width + right - left,
                height: area.height + bottom - top,
            };
            if !(moved.width > 0.0 && moved.height > 0.0) {
                break;
            }
            area = moved;
            layout = self.layout(area)?;
        }
        Ok(layout)
    }

    /// Draw a laid out grid (see [`PreparedGrid::draw`]).
    fn draw_layout(
        &self,
        grid_layout: &GridLayout,
        fill_background: bool,
        mut interaction: Option<&mut (Interaction, Vec<String>)>,
    ) -> Result<Vec<Node>> {
        let theme = &self.theme;
        let bounds = grid_layout.bounds;
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
        let placed = &grid_layout.placed;
        let union = placed
            .iter()
            .flatten()
            .map(Placed::bounds)
            .reduce(union)
            .unwrap_or(grid_layout.elements_area);
        let inset_areas = inset::HostAreas {
            panel: grid_layout.frame().unwrap_or(union),
            plot: grid_layout.elements_area,
            full: bounds,
        };
        nodes.extend(inset::draw(
            &self.insets,
            &inset_areas,
            false,
            interaction.as_deref_mut(),
        )?);
        // Figures draw in parallel, each collecting its own tooltips; they merge
        // back in drawing order.
        let empty_interaction = interaction.as_deref().map(|(collected, _)| Interaction {
            layers: Vec::new(),
            zoom: collected.zoom.as_ref().map(|zoom| Zoom {
                plots: Vec::new(),
                ..zoom.clone()
            }),
        });
        let rows = self
            .elements
            .iter()
            .flatten()
            .map(Element::prepared_rows)
            .sum();
        let figures = placed.iter().flatten().collect();
        let drawn = parallel::ordered_map(figures, parallel::is_worth(rows), |figure| {
            let mut figure_interaction = empty_interaction
                .clone()
                .map(|collected| (collected, Vec::new()));
            let figure_nodes = figure.draw(figure_interaction.as_mut(), true)?;
            Ok::<_, Error>((figure_nodes, figure_interaction))
        });
        for result in drawn {
            let (figure_nodes, figure_interaction) = result?;
            if let (Some(collected), Some(figure)) =
                (interaction.as_deref_mut(), figure_interaction)
            {
                merge_interaction(collected, figure);
            }
            nodes.push(Node::Group(Group {
                clip: None,
                role: Some("figure"),
                anchor: None,
                nodes: figure_nodes,
            }));
        }
        nodes.extend(heading_nodes(
            grid_layout.content.y,
            (union.x, union.width),
            theme,
            &self.titles,
        ));
        nodes.extend(caption_nodes(
            grid_layout.without_titles.y + grid_layout.without_titles.height,
            (union.x, union.width),
            theme,
            &self.titles,
        ));
        for (group, block, block_theme) in &grid_layout.blocks {
            let origin = legend_origin(union, grid_layout.without_titles, block.size, group);
            nodes.extend(draw_legends(block, origin, block_theme)?);
        }
        nodes.extend(tag_nodes(theme, &self.titles.tag, grid_layout.inner, union));
        nodes.extend(inset::draw(&self.insets, &inset_areas, true, interaction)?);
        Ok(nodes)
    }

    /// Each legend group measured, with the theme it is measured and drawn with
    /// (its first figure's, at the group's position and justification, with the
    /// grid's box arrangement). Blocks are fitted to `area` (the grid inside its
    /// margins, titles and inset) as a single plot's legends are: wrapped, reflowed,
    /// then cut (lets-plot#1506; lets-plot runs them off the canvas).
    fn measure_legend_groups(
        &self,
        area: Rect,
    ) -> Result<Vec<(&Collected, LegendBlock<'_>, Theme)>> {
        let mut blocks = Vec::new();
        for group in &self.legend_groups {
            // Boxes keep their figure's theme (`box_themes`); the grid's theme
            // arranges them (lets-plot `LegendsBlockInfo.arrangeLegendBoxes`).
            let block_theme = arranged(&group.theme, group, &self.theme);
            // `area` is already inside the grid's margins.
            let budget = legend_budget(
                &Theme {
                    plot_margin: Margins::uniform(0.0),
                    ..block_theme.clone()
                },
                area,
            );
            if let Some(block) = measure_themed_legends(
                &group.boxes,
                Some(&group.box_themes),
                &block_theme,
                Some(budget),
            )? {
                blocks.push((group, block, block_theme));
            }
        }
        Ok(blocks)
    }
}

/// lets-plot `CompositeFigureGridAlignmentLayout`, over spans and nested grids:
/// every figure's panels take the range its edges share with the figures that
/// start (or end) in the same column (or row), and the figure is laid out again
/// around it. A figure's panels are a plot's panels or a nested grid's outer panel
/// edges ([`GridLayout::frame`]); lets-plot leaves nested grids out.
fn align(placed: &mut [Option<Placed>], spans: &[Span]) -> Result<()> {
    let frames: Vec<Option<Rect>> = placed
        .iter()
        .map(|figure| figure.as_ref().and_then(Placed::frame))
        .collect();
    // The shared edge of the figures whose span edge is `key`: the innermost one.
    let shared = |key: &dyn Fn(&Span) -> usize,
                  value: &dyn Fn(&Rect) -> f64,
                  wanted: usize,
                  inner_max: bool| {
        frames
            .iter()
            .zip(spans)
            .filter_map(|(frame, span)| frame.filter(|_| key(span) == wanted))
            .map(|frame| value(&frame))
            .reduce(|a, b| if inner_max { a.max(b) } else { a.min(b) })
    };
    for (index, figure) in placed.iter_mut().enumerate() {
        let Some(figure) = figure else {
            continue;
        };
        if frames[index].is_none() {
            continue;
        }
        let span = spans[index];
        let edges = (
            shared(&|span| span.left, &|rect| rect.x, span.left, true),
            shared(&|span| span.top, &|rect| rect.y, span.top, true),
            shared(
                &|span| span.right,
                &|rect| rect.x + rect.width,
                span.right,
                false,
            ),
            shared(
                &|span| span.bottom,
                &|rect| rect.y + rect.height,
                span.bottom,
                false,
            ),
        );
        let (Some(left), Some(top), Some(right), Some(bottom)) = edges else {
            continue;
        };
        // Panels that do not overlap at all leave nothing to share.
        if right <= left || bottom <= top {
            continue;
        }
        figure.align_to(Rect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        })?;
    }
    Ok(())
}

/// Add a figure's tooltips, zoom panels and notes to the grid's.
fn merge_interaction(
    (collected, notes): &mut (Interaction, Vec<String>),
    (figure, figure_notes): (Interaction, Vec<String>),
) {
    collected.layers.extend(figure.layers);
    if let (Some(zoom), Some(figure_zoom)) = (collected.zoom.as_mut(), figure.zoom) {
        zoom.plots.extend(figure_zoom.plots);
    }
    for note in figure_notes {
        if !notes.contains(&note) {
            notes.push(note);
        }
    }
}

/// A figure's plot spec as prepared in a grid: with the grid's theme
/// ([`child_theme`]) and its automatic tag.
fn figure_plot_spec(
    plot_spec: &PlotSpec,
    container: Option<(&ThemeSpec, &Theme)>,
    tags: Option<&FigureTags>,
) -> PlotSpec {
    let mut plot_spec = PlotSpec::clone(plot_spec);
    if let Some((container, container_theme)) = container {
        plot_spec.theme = Some(child_theme(
            container,
            container_theme,
            plot_spec.theme.as_ref(),
        ));
    }
    if let Some(FigureTags::Plot(tag, _)) = tags {
        plot_spec.tag = Some(CaptionSpec {
            text: Some(tag.clone()),
            unsupported: Default::default(),
        });
    }
    plot_spec
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
    // Kept inside `outer` along its side, as a single plot's legends (a block
    // wider than the figures was centred past both canvas edges).
    let keep_in = |position: f64, extent: f64, start: f64, span: f64| {
        position.min(start + span - extent).max(start)
    };
    let along_y = keep_in(
        inner.y + (inner.height - size.1) * (1.0 - justify_y),
        size.1,
        outer.y,
        outer.height,
    );
    let along_x = keep_in(
        inner.x + (inner.width - size.0) * justify_x,
        size.0,
        outer.x,
        outer.width,
    );
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
fn group_legends(collected: Vec<Collected>, grid_theme: &Theme) -> Vec<Collected> {
    let mut groups: Vec<Collected> = Vec::new();
    for item in collected {
        let index = match groups.iter().position(|group| {
            group.position == item.position && group.justification == item.justification
        }) {
            Some(index) => index,
            None => {
                groups.push(Collected {
                    boxes: Vec::new(),
                    box_themes: Vec::new(),
                    ..item
                });
                groups.len() - 1
            }
        };
        let group = &mut groups[index];
        for (legend_box, box_theme) in item.boxes.into_iter().zip(item.box_themes) {
            if !group
                .boxes
                .iter()
                .any(|kept| kept.same_content(&legend_box))
            {
                group.boxes.push(legend_box);
                group.box_themes.push(box_theme);
            }
        }
    }
    for group in &mut groups {
        let themes = std::mem::take(&mut group.box_themes);
        group.box_themes = themes
            .iter()
            .map(|theme| arranged(theme, group, grid_theme))
            .collect();
    }
    groups
}

/// Smallest panel of a plot in a grid cell. A ggrust deviation, as ggplot2 (patchwork):
/// panels shrink to what the cell leaves, so thin cells (annotation tracks) stay inside
/// them; lets-plot keeps its 50 px `GEOM_MIN_SIZE` and the panel runs past the cell.
const CELL_PANEL_MIN_SIZE: f64 = 1.0;

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
    error.nested(path)
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

    #[test]
    fn grid_caption_is_drawn_at_the_bottom_without_a_warning() {
        let grid = |caption: &str| {
            let json = format!(
                r#"{{"kind": "subplots", "layout": {{"name": "grid", "ncol": 2, "nrow": 1}},
                    "ggsize": {{"width": 600, "height": 300}}{caption},
                    "figures": [{{"kind": "plot"}}, {{"kind": "plot"}}]}}"#
            );
            let figure = Figure::from_json(&json).expect("valid spec");
            build_figure(&figure, &[]).expect("builds")
        };
        let built = grid(r#", "caption": {"text": "Source"}"#);
        assert!(built.warnings.is_empty(), "{:?}", built.warnings);
        let svg = crate::render::to_svg(&built.scene);
        let tail = &svg[..svg.find(">Source</text>").expect("caption drawn")];
        let y: f64 = tail[tail.rfind(" y=\"").expect("y") + 4..]
            .split('"')
            .next()
            .and_then(|value| value.parse().ok())
            .expect("number");
        assert!((280.0..300.0).contains(&y), "{y}");
        assert!(!crate::render::to_svg(&grid("").scene).contains("Source"));
    }
}
