//! Insets: `plot + inset_element(other, left, bottom, right, top, ...)` (patchwork
//! `inset_element`). A ggrust extension: lets-plot has no insets.
//!
//! What it does: prepare inset figures (plots or grids, with their own insets) as
//! the host's grid prepares its figures (the same theme inheritance, tags from
//! [`crate::grid_tags`]), place each at its fractions of the host's panels, plot
//! area (panels, axes and axis titles) or whole area, and draw it under the host's
//! content (above its background) or over it, clipped to its area by default.
//!
//! What it does not do: collect inset legends into a grid's, or align inset panels
//! with anything.

use crate::Result;
use crate::build::{Arrangement, PlotData, PreparedPlot, prepare_plot};
use crate::grid_tags::FigureTags;
use crate::scene::{Group, Node, Rect};
use crate::spec::{Figure, InsetAlign, InsetSpec};
use crate::tooltip::Interaction;

use super::{Element, GridContext, Placed, PreparedGrid, figure_plot_spec, nest, nest_error};

/// An inset figure, ready to be laid out.
pub(super) struct PreparedInset<'s> {
    element: Element<'s>,
    align: InsetAlign,
    /// `(left, bottom, right, top)` fractions of the host area.
    edges: [f64; 4],
    on_top: bool,
    clip: bool,
}

/// The areas of a host that insets are placed in (patchwork `align_to`).
pub(super) struct HostAreas {
    /// The panels (`"panel"`).
    pub panel: Rect,
    /// Panels, axes and axis titles (`"plot"`).
    pub plot: Rect,
    /// Everything (`"full"`).
    pub full: Rect,
}

impl HostAreas {
    /// The areas of a plot laid out by `arrangement`.
    pub fn of_plot(plot: &PreparedPlot, arrangement: &Arrangement) -> HostAreas {
        HostAreas {
            panel: arrangement.panel_bounds(),
            plot: plot.plot_area(arrangement),
            full: arrangement.area(),
        }
    }
}

/// Rows of every prepared layer of the insets.
pub(super) fn prepared_rows(insets: &[PreparedInset]) -> usize {
    insets
        .iter()
        .map(|inset| inset.element.prepared_rows())
        .sum()
}

/// Prepare the insets of a figure. `context` is the host's: its container theme,
/// and its path (`figures[0].`) for messages; `tags` the insets' tags.
pub(super) fn prepare<'s>(
    insets: &'s [InsetSpec],
    tags: &[FigureTags],
    context: &GridContext,
    warnings: &mut Vec<String>,
) -> Result<Vec<PreparedInset<'s>>> {
    let mut prepared = Vec::with_capacity(insets.len());
    for (index, inset) in insets.iter().enumerate() {
        let path = format!("{}insets[{index}]", context.path);
        let (align, edges) = inset
            .placement()
            .map_err(|error| nest_error(error, &format!("{path}.")))?;
        let figure_path = format!("{path}.figure");
        let child_path = format!("{figure_path}.");
        let tag = tags.get(index);
        let element = match &inset.figure {
            Figure::Plot(plot_spec) => {
                let spec = figure_plot_spec(plot_spec, context.container, tag);
                let mut plot = PlotData::from_frames(&spec, context.frames)
                    .and_then(|data| prepare_plot(&spec, &data))
                    .map_err(|error| nest_error(error, &figure_path))?;
                warnings.extend(
                    plot.warnings
                        .drain(..)
                        .map(|warning| nest(&figure_path, &warning)),
                );
                let inner_tags = match tag {
                    Some(FigureTags::Plot(_, insets)) => insets.as_slice(),
                    _ => &[],
                };
                let inner_context = GridContext {
                    path: &child_path,
                    ..*context
                };
                let nested = prepare(&plot_spec.insets, inner_tags, &inner_context, warnings)?;
                Element::Plot(Box::new(plot), nested)
            }
            Figure::Subplots(grid_spec) => {
                let child_context = GridContext {
                    tags: tag.filter(|tags| matches!(tags, FigureTags::Grid(..))),
                    path: &child_path,
                    ..*context
                };
                let (grid, _) = PreparedGrid::new(grid_spec, &child_context, warnings)?;
                Element::Grid(Box::new(grid))
            }
        };
        prepared.push(PreparedInset {
            element,
            align,
            edges,
            on_top: inset.on_top.unwrap_or(true),
            clip: inset.clip.unwrap_or(true),
        });
    }
    Ok(prepared)
}

/// Lay out and draw the insets drawn over the host (`on_top`) or under it.
pub(super) fn draw(
    insets: &[PreparedInset],
    areas: &HostAreas,
    on_top: bool,
    mut interaction: Option<&mut (Interaction, Vec<String>)>,
) -> Result<Vec<Node>> {
    let mut nodes = Vec::new();
    for inset in insets.iter().filter(|inset| inset.on_top == on_top) {
        let host = match inset.align {
            InsetAlign::Panel => areas.panel,
            InsetAlign::Plot => areas.plot,
            InsetAlign::Full => areas.full,
        };
        let [left, bottom, right, top] = inset.edges;
        let rect = Rect {
            x: host.x + left * host.width,
            y: host.y + (1.0 - top) * host.height,
            width: (right - left) * host.width,
            height: (top - bottom) * host.height,
        };
        let placed = Placed::new(&inset.element, rect)?;
        let inset_nodes = placed.draw(interaction.as_deref_mut(), true)?;
        nodes.push(Node::Group(Group {
            key: None,
            clip: inset.clip.then_some(rect),
            role: Some("inset"),
            anchor: None,
            nodes: inset_nodes,
        }));
    }
    Ok(nodes)
}
