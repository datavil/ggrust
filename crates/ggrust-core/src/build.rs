//! Plot builder: typed spec + Arrow data -> [`Scene`].
//!
//! Stages: map every layer ([`map_layer`]) -> resolve the position scales (kinds,
//! discrete levels) -> stat and position per layer ([`compute_layer`]) -> train
//! style scales on the computed data -> resolve styles ([`style_layer`]) -> train
//! position scales -> layout -> draw. Layer stages own their warnings (no shared
//! `&mut Vec`), so they can run in parallel later.
//!
//! Features parsed by `spec` but not built yet are reported as warnings by
//! [`report_unimplemented`]; remove an entry there when its phase lands.

use std::collections::{BTreeMap, BTreeSet};

use crate::coord::{self, Coord, Panel};
use crate::data::as_discrete::{DiscreteOrder, as_discrete};
use crate::data::{
    Column, Computed, ContinuousColumn, DataFrame, LayerData, SourceRows, group_index, inline,
};
use crate::facet::{Facets, LayerFacetColumns, grid_variables};
use crate::geom::{self, DrawLayer, Geom};
use crate::layout::facet::{FacetGrid, FacetLayout, FacetPanel, draw_strips, layout_facets};
use crate::layout::zoom::{Zoom, ZoomPlot, zoom_tile};
use crate::layout::{
    Colorbar, DEFAULT_BINS, KeyGrid, KeyLayer, Legend, LegendBox, LegendKey, PANEL_MIN_SIZE,
    PlotLayout, Titles, draw_axes, draw_legends, draw_panel_background, draw_panel_border,
    draw_titles, layout_plot, legend_budget, legend_origin, legend_space, measure_legends,
    wrap_headings,
};
use crate::parallel;
use crate::parameters::Parameters;
use crate::position::{self, Position};
use crate::scale::datetime::TemporalKind;
use crate::scale::format::Label;
use crate::scale::number_format::{ExponentFormat, NumberFormat};
use crate::scale::style::spec::{StyleScaleSpec, style_scale_specs};
use crate::scale::style::{LegendEntry, StyleScale, default_mapper};
use crate::scale::{AxisScaleSpec, PositionScale, PositionScales, position_scales};
use crate::scene::{Group, LAYER_ROLE, Node, PerItem, Rect, Rects, Scene};
use crate::spec::{
    Aesthetic, DataSource, GuideOptions, GuideSpec, LayerSpec, Mapping, MappingAnnotation,
    MappingValue, Options, PlotSpec, SeriesAnnotation, TooltipsSpec, Value,
};
use crate::stat::groups::rows_by_group;
use crate::stat::{self, Stat, StatContext};
use crate::style::{StyleValues, Styles, parse_constant};
use crate::theme::{LegendPosition, Margins, Theme};
use crate::tooltip::content::{ContentSource, layer_content};
use crate::tooltip::{Interaction, LayerInteraction};
use crate::{Error, Result};

/// Plot size when `ggsize` is not given (lets-plot default).
pub const DEFAULT_SIZE: (f64, f64) = (600.0, 400.0);

/// Data frames of a plot: the plot-level frame and one optional frame per layer.
#[derive(Debug, Clone, Default)]
pub struct PlotData {
    /// `ggplot(data, ...)`.
    pub plot: Option<DataFrame>,
    /// `geom_*(data=...)`, by layer index; shorter than the layer list is fine.
    pub layers: Vec<Option<DataFrame>>,
}

impl PlotData {
    /// Resolve the spec's frame references (`{"frame": i}`) against `frames`.
    /// Frames are shared, not copied (Arrow buffers are reference counted).
    pub fn from_frames(spec: &PlotSpec, frames: &[DataFrame]) -> Result<Self> {
        let resolve = |source: &Option<DataSource>, path: &str| match source {
            Some(DataSource::Frame { index, columns }) => {
                let frame = frames.get(*index).cloned();
                frame
                    .map(|frame| Some(frame.with_source_columns(columns)))
                    .ok_or_else(|| {
                        Error::spec(
                            path,
                            format!(
                                "frame {index} was referenced but only {} were passed",
                                frames.len()
                            ),
                        )
                    })
            }
            Some(DataSource::Inline) | None => Ok(None),
        };
        Ok(PlotData {
            plot: resolve(&spec.data, "data")?,
            layers: spec
                .layers
                .iter()
                .enumerate()
                .map(|(index, layer)| resolve(&layer.data, &format!("layers[{index}].data")))
                .collect::<Result<_>>()?,
        })
    }
}

/// A layer stage's failure, with the warnings the stage collected before it; the
/// caller appends them after earlier layers' warnings, so notes stay in order.
type StageResult<T> = std::result::Result<T, (Error, Vec<String>)>;

/// A built plot.
#[derive(Debug, Clone)]
pub struct BuiltPlot {
    /// The drawing.
    pub scene: Scene,
    /// Hover tooltips, when asked for (HTML output).
    pub interaction: Option<Interaction>,
    /// Things the user should know: ignored options, removed rows, ...
    pub warnings: Vec<String>,
}

/// Build a single plot at its own size (`ggsize`, or the default).
pub fn build_plot(spec: &PlotSpec, data: &PlotData) -> Result<BuiltPlot> {
    build_plot_with(spec, data, false)
}

/// [`build_plot`], with the hover tooltips of HTML output when `interactive`.
pub fn build_plot_with(spec: &PlotSpec, data: &PlotData, interactive: bool) -> Result<BuiltPlot> {
    let plot = prepare_plot(spec, data)?;
    let (width, height) = plot.size;
    let area = Rect {
        x: 0.0,
        y: 0.0,
        width,
        height,
    };
    let arrangement = plot.arrange(area, PANEL_MIN_SIZE)?;
    let nodes = plot.draw(&arrangement)?;
    let scene = Scene {
        width,
        height,
        background: plot.background(),
        nodes,
    };
    scene.validate()?;
    let mut warnings = plot.warnings.clone();
    let interaction = if interactive {
        let mut interaction = Interaction {
            zoom: Zoom::from_spec(spec.ggtoolbar.as_ref(), "ggtoolbar")?,
            ..Interaction::default()
        };
        plot.interaction(&arrangement, &mut interaction, &mut warnings)?;
        if let Some(zoom) = &interaction.zoom {
            zoom.check_layer_labels("ggtoolbar")?;
        }
        Some(interaction)
    } else {
        None
    };
    Ok(BuiltPlot {
        scene,
        interaction,
        warnings,
    })
}

/// Map, compute and style a plot: everything but its layout (see [`PreparedPlot`]).
pub fn prepare_plot(spec: &PlotSpec, data: &PlotData) -> Result<PreparedPlot> {
    match inline_mapping_vectors(spec, data)? {
        Some((spec, data)) => prepare_columns(&spec, &data),
        None => prepare_columns(spec, data),
    }
}

/// `aes(x=[1, 2, 3])`: the plot and layer mappings with their data vectors turned
/// into columns of their data (see [`crate::data::inline`]). `None` when no
/// mapping holds one.
fn inline_mapping_vectors(
    spec: &PlotSpec,
    data: &PlotData,
) -> Result<Option<(PlotSpec, PlotData)>> {
    let has_vector = |mapping: &crate::spec::Mapping| {
        mapping
            .iter()
            .any(|(aesthetic, value)| inline::is_vector(*aesthetic, value))
    };
    if !has_vector(&spec.mapping) && !spec.layers.iter().any(|layer| has_vector(&layer.mapping)) {
        return Ok(None);
    }
    // Every data column name, sent or not (lets-plot avoids them all).
    let source_columns = std::iter::once(&spec.data)
        .chain(spec.layers.iter().map(|layer| &layer.data))
        .flat_map(|source| match source {
            Some(DataSource::Frame { columns, .. }) => columns.as_slice(),
            _ => &[],
        });
    let mut used_names: BTreeSet<String> = data
        .plot
        .iter()
        .chain(data.layers.iter().flatten())
        .flat_map(|frame| frame.column_names().map(str::to_owned))
        .chain(source_columns.cloned())
        .collect();
    let mut spec = spec.clone();
    let mut data = data.clone();
    if let Some(frame) = inline::inline_vectors(
        &mut spec.mapping,
        data.plot.as_ref(),
        &mut used_names,
        "mapping",
    )? {
        data.plot = Some(frame);
    }
    for (index, layer) in spec.layers.iter_mut().enumerate() {
        let path = format!("layers[{index}].mapping");
        let frame = data.layers.get(index).and_then(Option::as_ref);
        if let Some(frame) =
            inline::inline_vectors(&mut layer.mapping, frame, &mut used_names, &path)?
        {
            if data.layers.len() <= index {
                data.layers.resize(index + 1, None);
            }
            data.layers[index] = Some(frame);
        }
    }
    Ok(Some((spec, data)))
}

/// [`prepare_plot`] once every mapping names columns.
fn prepare_columns(spec: &PlotSpec, data: &PlotData) -> Result<PreparedPlot> {
    let mut warnings = Vec::new();
    report_unimplemented(spec, &mut warnings);
    let coord = resolve_coord(spec, &mut warnings)?;
    let scales =
        position_scales(spec, &mut warnings).map_err(|error| error.with_notes(&warnings))?;

    // Faceting columns with the option that names each (for column errors).
    let facet_variables: Vec<(String, &'static str)> = match &spec.facet {
        None => Vec::new(),
        Some(facet) => match facet.name.as_str() {
            "wrap" => facet
                .facets
                .as_ref()
                .map(|facets| {
                    facets
                        .as_slice()
                        .iter()
                        .map(|name| (name.clone(), "facet.facets"))
                        .collect()
                })
                .unwrap_or_default(),
            "grid" => {
                let names = grid_variables(facet).map_err(|error| error.with_notes(&warnings))?;
                let x_defined = facet.x.is_some();
                names
                    .into_iter()
                    .enumerate()
                    .map(|(index, name)| {
                        (
                            name,
                            if index == 0 && x_defined {
                                "facet.x"
                            } else {
                                "facet.y"
                            },
                        )
                    })
                    .collect()
            }
            other => {
                return Err(
                    unknown("facet", "facet.name", other, false, &["wrap", "grid"])
                        .with_notes(&warnings),
                );
            }
        },
    };
    let mut mapped = Vec::with_capacity(spec.layers.len());
    for (index, layer) in spec.layers.iter().enumerate() {
        match map_layer(index, layer, spec, data, &facet_variables) {
            Ok(mut layer) => {
                warnings.append(&mut layer.warnings);
                mapped.push(layer);
            }
            Err((error, mut layer_warnings)) => {
                warnings.append(&mut layer_warnings);
                return Err(error.with_notes(&warnings));
            }
        }
    }
    let facets = match &spec.facet {
        Some(facet) => {
            let layers: Vec<(&LayerFacetColumns, usize)> = mapped
                .iter()
                .map(|layer| (&layer.facet_columns, layer.row_count))
                .collect();
            let facets = match facet.name.as_str() {
                "grid" => Facets::grid(facet, &layers),
                _ => Facets::wrap(facet, &layers),
            }
            .map_err(|error| error.with_notes(&warnings))?;
            if facets.tiles.is_empty() {
                return Err(Error::spec(
                    "facet.facets",
                    "no panel to draw: every row has a missing value in a faceting column",
                )
                .with_notes(&warnings));
            }
            Some(facets)
        }
        None => None,
    };
    let (width, height) = match &spec.ggsize {
        Some(size) if size.width > 0.0 && size.height > 0.0 => (size.width, size.height),
        Some(size) => {
            return Err(Error::spec(
                "ggsize",
                format!(
                    "width and height must be positive, got {} x {}",
                    size.width, size.height
                ),
            )
            .with_notes(&warnings));
        }
        // lets-plot `PlotSizeHelper.defaultFacetedPlotSize`: panels shrink less
        // than the grid grows.
        None => match &facets {
            Some(facets) => {
                let (columns, rows) = (facets.columns as f64, facets.rows as f64);
                (
                    DEFAULT_SIZE.0 * (0.5 + 0.5 / columns) * columns,
                    DEFAULT_SIZE.1 * (0.5 + 0.5 / rows) * rows,
                )
            }
            None => DEFAULT_SIZE,
        },
    };
    let axis_scales = axis_scale(
        &scales.x,
        &mapped,
        Aesthetic::is_x_position,
        "x",
        &mut warnings,
    )
    .and_then(|x| {
        axis_scale(
            &scales.y,
            &mapped,
            Aesthetic::is_y_position,
            "y",
            &mut warnings,
        )
        .map(|y| (x, y))
    });
    let (mut x_scale, mut y_scale) = axis_scales.map_err(|error| error.with_notes(&warnings))?;
    // Every number label of the plot writes exponents as the theme says.
    let exponent_format = crate::theme::exponent_format(spec.theme.as_ref())
        .map_err(|error| error.with_notes(&warnings))?;
    x_scale.set_exponent_format(exponent_format);
    y_scale.set_exponent_format(exponent_format);
    for layer in mapped.iter().filter(|layer| layer.geom.includes_zero_y()) {
        match layer.y_oriented {
            true => x_scale.set_include_zero(),
            false => y_scale.set_include_zero(),
        }
    }
    if let Some(coord_spec) = &spec.coord {
        for (limits, scale, key) in [
            (coord_spec.xlim, &mut x_scale, "xlim"),
            (coord_spec.ylim, &mut y_scale, "ylim"),
        ] {
            let Some(limits) = limits else { continue };
            // lets-plot rejects reversed or equal coord limits ("Invalid coord ylim").
            if let [Some(low), Some(high)] = limits
                && low >= high
            {
                return Err(Error::spec(
                    format!("coord.{key}"),
                    format!("expected [low, high] with low < high, got [{low}, {high}]"),
                )
                .with_notes(&warnings));
            }
            scale.set_coord_limits(limits);
        }
    }
    let mut aspect = coord.fixes_aspect().then(|| {
        spec.coord
            .as_ref()
            .and_then(|coord_spec| coord_spec.ratio)
            .unwrap_or(1.0)
    });
    if let Some(ratio) = aspect.filter(|ratio| !(ratio.is_finite() && *ratio > 0.0)) {
        warnings.push(format!(
            "at 'coord.ratio': expected a positive number, got {ratio}; the panel keeps its \
             shape"
        ));
    }
    if aspect.is_some()
        && let Some(facets) = &facets
        && (facets.free_x || facets.free_y)
    {
        warnings.push(
            "at 'facet.scales': a fixed aspect ratio (coord_fixed, raster) with free facet \
             scales is not supported yet; the panels keep their shape"
                .to_owned(),
        );
        aspect = None;
    }
    let context = StatContext {
        x_range: overall_range(&mapped.iter().collect::<Vec<_>>(), Aesthetic::X, &x_scale),
        y_range: overall_range(&mapped.iter().collect::<Vec<_>>(), Aesthetic::Y, &y_scale),
    };
    // Per panel: its layers (stats and positions run per panel, as in lets-plot),
    // its position scales before training (free axes: discrete levels of the
    // panel's scale group only, in the plot's order) and its stat context: on a
    // free axis the range of its scale group (the panel in facet_wrap, the grid
    // column or row in facet_grid), as ggplot2 (lets-plot bins every panel over
    // the whole plot's range, #1443).
    let level_order = facets.as_ref().map(|_| style_level_order(&mapped));
    let parallel = parallel::is_worth(mapped.iter().map(|layer| layer.row_count).sum());
    // Scale group of every tile on each axis (see `Tile::x_scale_group`).
    let (x_groups, y_groups): (Vec<usize>, Vec<usize>) = match &facets {
        None => (vec![0], vec![0]),
        Some(facets) => facets
            .tiles
            .iter()
            .map(|tile| (tile.x_scale_group, tile.y_scale_group))
            .unzip(),
    };
    let (tiles, mut x_group_scales, mut y_group_scales): (
        Vec<TileSetup>,
        Vec<PositionScale>,
        Vec<PositionScale>,
    ) = match &facets {
        None => (
            vec![(mapped, x_scale.clone(), y_scale.clone(), context)],
            vec![x_scale.clone()],
            vec![y_scale.clone()],
        ),
        Some(facets) => {
            let rows: Vec<Vec<Vec<u32>>> = mapped
                .iter()
                .map(|layer| facets.rows_by_tile(&layer.facet_columns, layer.row_count))
                .collect();
            let tile_layers: Vec<Vec<MappedLayer>> = (0..facets.tiles.len())
                .map(|tile| {
                    mapped
                        .iter()
                        .zip(&rows)
                        .map(|(layer, rows)| split_layer(layer, &rows[tile]))
                        .collect()
                })
                .collect();
            // Per scale group: its scale before training and its stat range.
            let group_scales = |groups: &[usize],
                                shared: &PositionScale,
                                free: bool,
                                on_axis: fn(Aesthetic) -> bool,
                                aesthetic: Aesthetic,
                                shared_range: Option<(f64, f64)>| {
                let count = groups.iter().max().map_or(0, |max| max + 1);
                (0..count)
                    .map(|group| {
                        if !free {
                            return (shared.clone(), shared_range);
                        }
                        let members: Vec<&MappedLayer> = tile_layers
                            .iter()
                            .zip(groups)
                            .filter(|(_, member)| **member == group)
                            .flat_map(|(layers, _)| layers)
                            .collect();
                        (
                            panel_scale(shared, &members, on_axis),
                            overall_range(&members, aesthetic, shared),
                        )
                    })
                    .collect::<Vec<_>>()
            };
            let x_groups_setup = group_scales(
                &x_groups,
                &x_scale,
                facets.free_x,
                Aesthetic::is_x_position,
                Aesthetic::X,
                context.x_range,
            );
            let y_groups_setup = group_scales(
                &y_groups,
                &y_scale,
                facets.free_y,
                Aesthetic::is_y_position,
                Aesthetic::Y,
                context.y_range,
            );
            let tiles = tile_layers
                .into_iter()
                .zip(x_groups.iter().zip(&y_groups))
                .map(|(layers, (x_group, y_group))| {
                    let (tile_x, x_range) = x_groups_setup[*x_group].clone();
                    let (tile_y, y_range) = y_groups_setup[*y_group].clone();
                    (layers, tile_x, tile_y, StatContext { x_range, y_range })
                })
                .collect();
            (
                tiles,
                x_groups_setup.into_iter().map(|(scale, _)| scale).collect(),
                y_groups_setup.into_iter().map(|(scale, _)| scale).collect(),
            )
        }
    };
    // Tiles and layers compute in parallel; results are taken in order, so
    // warnings and the first error are those of a sequential run.
    let results = parallel::ordered_map(tiles, parallel, |(layers, tile_x, tile_y, context)| {
        let results = parallel::ordered_map(layers, parallel, |layer| {
            compute_layer(layer, [&tile_x, &tile_y], &context)
        });
        (results, tile_x, tile_y)
    });
    let mut computed_tiles = Vec::with_capacity(results.len());
    let mut tile_warnings = Vec::new();
    for (results, tile_x, tile_y) in results {
        let mut computed = Vec::with_capacity(results.len());
        for result in results {
            match result {
                Ok(mut layer) => {
                    tile_warnings.append(&mut layer.warnings);
                    computed.push(layer);
                }
                Err(error) => {
                    warnings.extend(merge_tile_warnings(tile_warnings));
                    return Err(error.with_notes(&warnings));
                }
            }
        }
        computed_tiles.push((computed, tile_x, tile_y));
    }
    warnings.extend(merge_tile_warnings(tile_warnings));
    let style_specs = style_scale_specs(spec, &mut warnings);
    let all_computed: Vec<&ComputedLayer> = computed_tiles
        .iter()
        .flat_map(|(layers, _, _)| layers)
        .collect();
    let mut style_scales = train_style_scales(&all_computed, &style_specs, &mut warnings)
        .map_err(|error| error.with_notes(&warnings))?;
    for scale in style_scales.values_mut() {
        scale.set_exponent_format(exponent_format);
    }
    // Panels split the data, which can reorder discrete levels (a stat keeps only
    // a panel's levels): keep the data's order, as without facets.
    if let Some(order) = &level_order {
        for (aesthetic, scale) in &mut style_scales {
            if let Some(levels) = order.get(aesthetic) {
                scale.order_levels_by(levels);
            }
        }
    }
    // Styles map in parallel too, taken in order as above.
    let styled = parallel::ordered_map(computed_tiles, parallel, |(computed, tile_x, tile_y)| {
        let results = parallel::ordered_map(computed, parallel, |layer| {
            style_layer(layer, &style_scales, exponent_format)
        });
        (results, tile_x, tile_y)
    });
    let mut prepared_tiles = Vec::with_capacity(styled.len());
    let mut tile_warnings = Vec::new();
    for ((results, tile_x, tile_y), (x_group, y_group)) in
        styled.into_iter().zip(x_groups.iter().zip(&y_groups))
    {
        let mut layers = Vec::with_capacity(results.len());
        for result in results {
            match result {
                Ok((layer, mut layer_warnings)) => {
                    tile_warnings.append(&mut layer_warnings);
                    layers.push(layer);
                }
                Err((error, mut layer_warnings)) => {
                    tile_warnings.append(&mut layer_warnings);
                    warnings.extend(merge_tile_warnings(tile_warnings));
                    return Err(error.with_notes(&warnings));
                }
            }
        }
        // A free axis trains its scale group on every panel of the group.
        train_positions(
            &layers,
            &mut x_group_scales[*x_group],
            &mut y_group_scales[*y_group],
        );
        train_positions(&layers, &mut x_scale, &mut y_scale);
        prepared_tiles.push((layers, tile_x, tile_y));
    }
    warnings.extend(merge_tile_warnings(tile_warnings));
    // Shared axes take the scale trained on every panel, free ones their group's.
    let (free_x, free_y) = facets
        .as_ref()
        .map_or((false, false), |facets| (facets.free_x, facets.free_y));
    for ((_, tile_x, tile_y), (x_group, y_group)) in prepared_tiles
        .iter_mut()
        .zip(x_groups.iter().zip(&y_groups))
    {
        *tile_x = if free_x {
            x_group_scales[*x_group].clone()
        } else {
            x_scale.clone()
        };
        *tile_y = if free_y {
            y_group_scales[*y_group].clone()
        } else {
            y_scale.clone()
        };
    }

    // lets-plot: a single coord limit beyond the other end of the data range is an
    // error ("Incompatible x-limits and the data range").
    for (_, tile_x, tile_y) in &prepared_tiles {
        for (scale, key) in [(tile_x, "xlim"), (tile_y, "ylim")] {
            if scale.coord_limit_beyond_data() {
                return Err(Error::spec(
                    format!("coord.{key}"),
                    "the given limit lies beyond the other end of the data \
                     range; give both limits",
                )
                .with_notes(&warnings));
            }
        }
    }
    let theme = Theme::from_spec(spec.theme.as_ref(), &mut warnings)
        .map_err(|error| error.with_notes(&warnings))?;
    // Per layer: whether it runs in y orientation (the same in every panel).
    let orientations: Vec<bool> = prepared_tiles
        .first()
        .map(|(layers, _, _)| layers.iter().map(|layer| layer.y_oriented).collect())
        .unwrap_or_default();
    let titles = plot_titles(spec, &scales, &orientations);
    // Legends: each layer once, from the first panel where it has rows.
    let legend_layers: Vec<&PreparedLayer> = (0..spec.layers.len())
        .filter_map(|index| {
            let mut candidates = prepared_tiles
                .iter()
                .filter_map(|(layers, _, _)| layers.get(index));
            let first = candidates.clone().next();
            candidates
                .find(|layer| layer.data.row_count() > 0)
                .or(first)
        })
        .collect();
    let legends = assemble_legends(spec, &legend_layers, &style_scales, &style_specs)
        .map_err(|error| error.with_notes(&warnings))?;
    let tooltip_setups = tooltip_setups(spec, &titles, &style_specs, &orientations);
    // coord_flip: the screen axes take the other scale, title and theme elements.
    let flipped = spec
        .coord
        .as_ref()
        .is_some_and(|coord| coord.flip || coord.name == "flip");
    let (theme, titles) = if flipped {
        let titles = Titles {
            x: titles.y.clone(),
            y: titles.x.clone(),
            ..titles
        };
        (theme.flipped(), titles)
    } else {
        (theme, titles)
    };
    Ok(PreparedPlot {
        tiles: prepared_tiles,
        facets,
        theme,
        titles,
        coord,
        aspect,
        flipped,
        legends,
        size: (width, height),
        tooltip_setups,
        warnings,
    })
}

/// A plot with its stats, scales and legends computed, ready to be laid out at any
/// size and drawn. A grid lays a plot out in its cell, and may lay it out again
/// around an aligned panel ([`PreparedPlot::arrange_panel`]) without recomputing
/// anything (see [`crate::grid`]).
pub struct PreparedPlot {
    /// Per panel: its layers and its x and y scales (data axes, not screen axes).
    tiles: Vec<(Vec<PreparedLayer>, PositionScale, PositionScale)>,
    facets: Option<Facets>,
    /// Screen theme and titles (swapped under `coord_flip`).
    theme: Theme,
    titles: Titles,
    coord: &'static dyn Coord,
    aspect: Option<f64>,
    flipped: bool,
    /// Legend boxes. A grid collecting legends (`guides="collect"`) takes them out,
    /// and the plot then keeps no space for them.
    pub legends: Vec<LegendBox>,
    /// `ggsize`, or the default size.
    pub size: (f64, f64),
    /// Per layer: what its tooltips read from the spec.
    tooltip_setups: Vec<LayerTooltipSetup>,
    /// Things the user should know: ignored options, removed rows, ...
    pub warnings: Vec<String>,
}

/// What a layer's tooltips read from the spec (the same in every panel).
struct LayerTooltipSetup {
    tooltips: Option<TooltipsSpec>,
    /// Variable bound to each aesthetic: a column name or `..stat..`.
    bindings: BTreeMap<Aesthetic, String>,
    /// Scale name of bound positions and style aesthetics (axis title, legend title).
    labels: BTreeMap<Aesthetic, String>,
}

/// lets-plot tooltip anchors (`TooltipConfig.readAnchor`).
const TOOLTIP_ANCHORS: [&str; 9] = [
    "top_left",
    "top_center",
    "top_right",
    "middle_left",
    "middle_center",
    "middle_right",
    "bottom_left",
    "bottom_center",
    "bottom_right",
];

/// The tooltip setup of every layer: bound variables (mapped columns, then the
/// stat's default mappings) and their labels, keyed as the layer's data (x and y
/// swapped in a y-oriented layer, see [`layer_orientation`]).
fn tooltip_setups(
    spec: &PlotSpec,
    titles: &Titles,
    style_specs: &BTreeMap<Aesthetic, StyleScaleSpec>,
    orientations: &[bool],
) -> Vec<LayerTooltipSetup> {
    spec.layers
        .iter()
        .enumerate()
        .map(|(index, layer)| {
            let y_oriented = orientations.get(index).copied().unwrap_or(false);
            let oriented = |aesthetic: Aesthetic| match y_oriented {
                true => aesthetic.flip_orientation(),
                false => aesthetic,
            };
            let (mapping, _) = merged_mapping(spec, layer);
            let mut bindings = BTreeMap::new();
            for (aesthetic, value) in &mapping {
                let constant = layer
                    .parameters
                    .get(aesthetic.as_str())
                    .is_some_and(|value| !value.is_null());
                if *aesthetic == Aesthetic::Group || constant {
                    continue;
                }
                if let Some(name) = value.as_column() {
                    bindings.insert(oriented(*aesthetic), name.to_owned());
                }
            }
            if let Some(stat) = layer_stat(layer) {
                for (aesthetic, computed) in stat.default_mapping() {
                    if sets_constant(layer, oriented(*aesthetic)) {
                        continue;
                    }
                    bindings
                        .entry(*aesthetic)
                        .or_insert_with(|| format!("..{}..", computed.as_str()));
                }
            }
            let labels = bindings
                .keys()
                .filter_map(|&aesthetic| {
                    // lets-plot scale name: guide title (`labs`), then `name`, then default.
                    let guide_title = match spec.guides.get(aesthetic.as_str()) {
                        Some(GuideSpec::Full(options)) => options.title.clone(),
                        _ => None,
                    };
                    let label = match oriented(aesthetic) {
                        Aesthetic::X => titles.x.clone(),
                        Aesthetic::Y => titles.y.clone(),
                        _ if aesthetic.kind().style().is_some() => guide_title.or_else(|| {
                            Some(legend_title(spec, aesthetic, style_specs.get(&aesthetic)))
                        }),
                        _ => None,
                    };
                    label.map(|label| (aesthetic, label))
                })
                .collect();
            LayerTooltipSetup {
                tooltips: layer.tooltips.clone(),
                bindings,
                labels,
            }
        })
        .collect()
}

/// One panel with its background, grid, border and axes: the HTML runtime's
/// zoom finds a panel's pieces inside its `tile` group.
fn tile_group(nodes: Vec<Node>) -> Node {
    Node::Group(Group {
        clip: None,
        role: Some("tile"),
        anchor: None,
        nodes,
    })
}

/// Where a plot's panels, axes and titles go (from [`PreparedPlot::arrange`]).
pub struct Arrangement {
    area: Rect,
    layout: Arranged,
    /// The titles, wrapped to the plot width.
    titles: Titles,
}

enum Arranged {
    Single(PlotLayout),
    Facets(FacetLayout),
}

impl Arrangement {
    /// The area the plot fills.
    pub fn area(&self) -> Rect {
        self.area
    }

    /// The data panel (the union of the panels of a faceted plot).
    pub fn panel_bounds(&self) -> Rect {
        self.plot_layout().panel
    }

    fn plot_layout(&self) -> &PlotLayout {
        match &self.layout {
            Arranged::Single(layout) => layout,
            Arranged::Facets(layout) => &layout.plot,
        }
    }
}

/// Moves of the area smaller than this (px) end [`PreparedPlot::arrange_panel`].
const ALIGN_TOLERANCE: f64 = 0.01;

/// Re-layouts [`PreparedPlot::arrange_panel`] tries before it gives up converging.
const ALIGN_PASSES: usize = 4;

impl PreparedPlot {
    /// The screen theme (legend position and justification for a grid collecting
    /// legends).
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// The plot background fill (`None`: transparent or blank).
    pub fn background(&self) -> Option<crate::scene::Color> {
        self.theme.plot_background.map(|background| background.fill)
    }

    /// What the layers are drawn over: the panel background fill, else the plot
    /// background fill, else white (lets-plot's default `halo_color`).
    fn layer_background(&self) -> crate::scene::Color {
        self.theme
            .panel_background
            .or(self.theme.plot_background)
            .map_or(crate::scene::Color::WHITE, |background| background.fill)
    }

    fn legend_block(&self, area: Rect) -> Result<Option<crate::layout::LegendBlock<'_>>> {
        if self.theme.legend_position == LegendPosition::None {
            Ok(None)
        } else {
            let budget = legend_budget(&self.theme, area);
            measure_legends(&self.legends, &self.theme, Some(budget))
        }
    }

    /// The data aesthetic drawn along a screen axis (`horizontal`: x), swapped
    /// under `coord_flip`.
    pub(crate) fn screen_aesthetic(&self, horizontal: bool) -> Aesthetic {
        if horizontal != self.flipped {
            Aesthetic::X
        } else {
            Aesthetic::Y
        }
    }

    /// Every panel's scale along a screen axis (`horizontal`: x).
    pub(crate) fn screen_axis_scales(&self, horizontal: bool) -> Vec<&PositionScale> {
        let data_x = self.screen_aesthetic(horizontal) == Aesthetic::X;
        self.tiles
            .iter()
            .map(|(_, x, y)| if data_x { x } else { y })
            .collect()
    }

    /// Mutable [`Self::screen_axis_scales`].
    pub(crate) fn screen_axis_scales_mut(&mut self, horizontal: bool) -> Vec<&mut PositionScale> {
        let data_x = self.screen_aesthetic(horizontal) == Aesthetic::X;
        self.tiles
            .iter_mut()
            .map(|(_, x, y)| if data_x { x } else { y })
            .collect()
    }

    /// Rows of every prepared layer in every panel.
    pub(crate) fn prepared_rows(&self) -> usize {
        self.tiles
            .iter()
            .flat_map(|(layers, _, _)| layers)
            .map(|layer| layer.data.row_count())
            .sum()
    }

    /// Whether the plot is worth drawing and collecting tooltips in parallel.
    fn is_large(&self) -> bool {
        parallel::is_worth(self.prepared_rows())
    }

    /// Screen scales of a panel: swapped under `coord_flip`.
    fn screen_scales<'s>(
        &self,
        x: &'s PositionScale,
        y: &'s PositionScale,
    ) -> (&'s PositionScale, &'s PositionScale) {
        if self.flipped { (y, x) } else { (x, y) }
    }

    fn facet_grid<'s>(&'s self, facets: &'s Facets) -> FacetGrid<'s> {
        let (shared_x, shared_y) = if self.flipped {
            (!facets.free_y, !facets.free_x)
        } else {
            (!facets.free_x, !facets.free_y)
        };
        FacetGrid {
            columns: facets.columns,
            rows: facets.rows,
            panels: facets
                .tiles
                .iter()
                .zip(&self.tiles)
                .map(|(tile, (_, tile_x, tile_y))| {
                    let (horizontal, vertical) = self.screen_scales(tile_x, tile_y);
                    FacetPanel {
                        x_scale: horizontal,
                        y_scale: vertical,
                        labels: &tile.labels,
                        row_label: tile.row_label.as_ref(),
                        column: tile.column,
                        row: tile.row,
                        x_axis: tile.has_x_axis,
                        y_axis: tile.has_y_axis,
                        is_bottom: tile.is_bottom,
                    }
                })
                .collect(),
            shared_x,
            shared_y,
        }
    }

    /// Lay the plot out to fill `area`, with panels at least `panel_min_size` px
    /// across ([`PANEL_MIN_SIZE`] for a whole figure).
    pub fn arrange(&self, area: Rect, panel_min_size: f64) -> Result<Arrangement> {
        let legend_space = self
            .legend_block(area)?
            .map_or(Margins::uniform(0.0), |block| {
                legend_space(&self.theme, block.size)
            });
        let layout_with = |titles: &Titles| match &self.facets {
            None => {
                let (_, tile_x, tile_y) = &self.tiles[0];
                let (horizontal, vertical) = self.screen_scales(tile_x, tile_y);
                Arranged::Single(layout_plot(
                    area,
                    &self.theme,
                    titles,
                    horizontal,
                    vertical,
                    legend_space,
                    self.aspect,
                    panel_min_size,
                ))
            }
            Some(facets) => Arranged::Facets(layout_facets(
                area,
                &self.theme,
                titles,
                &self.facet_grid(facets),
                legend_space,
                self.aspect,
                panel_min_size,
            )),
        };
        let layout = layout_with(&self.titles);
        // Titles start at the panel: wrap them to the room right of it, and lay out
        // again for the taller heading band if that wrapped anything.
        let plot = match &layout {
            Arranged::Single(layout) => layout,
            Arranged::Facets(layout) => &layout.plot,
        };
        let room = plot.content.x + plot.content.width - plot.panel.x;
        let titles = wrap_headings(&self.titles, &self.theme, room);
        let layout = if titles == self.titles {
            layout
        } else {
            layout_with(&titles)
        };
        Ok(Arrangement {
            area,
            layout,
            titles,
        })
    }

    /// Lay the plot out with its panel bounds at `target` (lets-plot
    /// `layoutByGeomSize`, for `gggrid(align=True)`). Starting from `area`, the area
    /// is moved and resized by the panel's distance from `target` until it fits
    /// (breaks, and so axis widths, change with the size); a single panel without a
    /// fixed aspect ratio then takes `target` exactly.
    pub fn arrange_panel(
        &self,
        area: Rect,
        target: Rect,
        panel_min_size: f64,
    ) -> Result<Arrangement> {
        let mut area = area;
        let mut arrangement = self.arrange(area, panel_min_size)?;
        for _ in 0..ALIGN_PASSES {
            let panel = arrangement.panel_bounds();
            let left = target.x - panel.x;
            let top = target.y - panel.y;
            let right = (target.x + target.width) - (panel.x + panel.width);
            let bottom = (target.y + target.height) - (panel.y + panel.height);
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
            arrangement = self.arrange(area, panel_min_size)?;
        }
        if self.aspect.is_none()
            && let Arranged::Single(layout) = &mut arrangement.layout
        {
            layout.panel = target;
        }
        Ok(arrangement)
    }

    /// The hover tooltips of the plot as laid out by `arrangement`: one entry per
    /// layer and panel with targets and text (see [`crate::tooltip`]). Problems
    /// with tooltip options go to `warnings` (once each).
    pub fn interaction(
        &self,
        arrangement: &Arrangement,
        interaction: &mut Interaction,
        warnings: &mut Vec<String>,
    ) -> Result<()> {
        let panels: Vec<(Rect, usize)> = match &arrangement.layout {
            Arranged::Single(layout) => vec![(layout.panel, 0)],
            Arranged::Facets(layout) => layout
                .tiles
                .iter()
                .enumerate()
                .map(|(index, tile)| (tile.layout.panel, index))
                .collect(),
        };
        let axis_labels = (
            self.theme.axis_text_x.is_some(),
            self.theme.axis_text_y.is_some(),
        );
        // Panels first (scales per tile), then one job per panel and layer, run in
        // parallel; results and notes keep the panel and layer order.
        let mut jobs = Vec::new();
        for (rect, tile) in panels {
            let Some((layers, tile_x, tile_y)) = self.tiles.get(tile) else {
                continue;
            };
            let indices = toggle_indices(layers);
            for ((layer, setup), index) in layers.iter().zip(&self.tooltip_setups).zip(indices) {
                jobs.push((rect, tile_x, tile_y, index, layer, setup));
            }
        }
        let layer_interaction = |&(rect, tile_x, tile_y, layer_index, layer, setup): &(
            Rect,
            &PositionScale,
            &PositionScale,
            usize,
            &PreparedLayer,
            &LayerTooltipSetup,
        ),
                                 notes: &mut Vec<String>|
         -> Result<Option<LayerInteraction>> {
            let (horizontal, vertical) = self.screen_scales(tile_x, tile_y);
            let panel = Panel {
                rect,
                x_scale: horizontal,
                y_scale: vertical,
                coord: self.coord,
                flipped: self.flipped != layer.y_oriented,
            };
            // A y-oriented layer's data x is on the y scale.
            let (tile_x, tile_y) = match layer.y_oriented {
                true => (tile_y, tile_x),
                false => (tile_x, tile_y),
            };
            let behavior = layer.geom.tooltip_behavior();
            let Some(lookup) = behavior.lookup else {
                return Ok(None);
            };
            let draw_layer = DrawLayer {
                data: &layer.data,
                styles: &layer.styles,
                parameters: Parameters::new(&layer.options, &layer.path),
                background: self.layer_background(),
            };
            let Some(targets) = layer.geom.hover_targets(&draw_layer, &panel)? else {
                return Ok(None);
            };
            if targets.is_empty() {
                return Ok(None);
            }
            let source = ContentSource {
                tooltips: setup.tooltips.as_ref(),
                behavior,
                data: &layer.unadjusted,
                styles: &layer.styles,
                x_scale: tile_x,
                y_scale: tile_y,
                bindings: &setup.bindings,
                labels: &setup.labels,
                columns: &layer.tooltip_columns,
                flipped: panel.flipped,
                axis_labels,
                exponent_format: self.theme.exponent_format,
            };
            let content = layer_content(&source, &layer.path, notes);
            if content.is_empty() {
                return Ok(None);
            }
            let (anchor, min_width) = match &setup.tooltips {
                Some(TooltipsSpec::Lines(lines)) => {
                    let anchor = lines.tooltip_anchor.clone().filter(|anchor| {
                        let known = TOOLTIP_ANCHORS.contains(&anchor.as_str());
                        if !known {
                            notes.push(format!(
                                "at '{}.tooltips.tooltip_anchor': unknown anchor \
                                 '{anchor}'; expected one of {}; the tooltip follows \
                                 the cursor",
                                layer.path,
                                TOOLTIP_ANCHORS.join(", ")
                            ));
                        }
                        known
                    });
                    let min_width = lines
                        .tooltip_min_width
                        .filter(|width| width.is_finite() && *width > 0.0);
                    (anchor, min_width)
                }
                _ => (None, None),
            };
            Ok(Some(LayerInteraction {
                panel: LayerInteraction::panel_of(rect),
                layer: layer_index,
                lookup,
                targets,
                content,
                anchor,
                min_width,
            }))
        };
        let results = parallel::ordered_map(jobs, self.is_large(), |job| {
            let mut notes = Vec::new();
            (layer_interaction(&job, &mut notes), notes)
        });
        let mut notes = Vec::new();
        for (result, mut job_notes) in results {
            notes.append(&mut job_notes);
            if let Some(layer) = result? {
                interaction.layers.push(layer);
            }
        }
        if interaction.zoom.is_some() {
            let plot = self.zoom_plot(arrangement);
            if let Some(zoom) = interaction.zoom.as_mut() {
                zoom.plots.push(plot);
            }
        }
        for note in notes {
            if !warnings.contains(&note) {
                warnings.push(note);
            }
        }
        Ok(())
    }

    /// The zoom payload of the plot as laid out by `arrangement` (`ggtb`).
    fn zoom_plot(&self, arrangement: &Arrangement) -> ZoomPlot {
        let layouts: Vec<(&PlotLayout, (bool, bool))> = match &arrangement.layout {
            Arranged::Single(layout) => vec![(layout, (true, true))],
            Arranged::Facets(layout) => layout
                .tiles
                .iter()
                .map(|tile| (&tile.layout, tile.axes))
                .collect(),
        };
        let mut tiles = Vec::new();
        let mut scales = Vec::new();
        for ((layout, shown), (_, tile_x, tile_y)) in layouts.into_iter().zip(&self.tiles) {
            let (horizontal, vertical) = self.screen_scales(tile_x, tile_y);
            let panel = Panel {
                rect: layout.panel,
                x_scale: horizontal,
                y_scale: vertical,
                coord: self.coord,
                flipped: self.flipped,
            };
            tiles.push(zoom_tile(layout, &self.theme, &panel, shown));
            scales.push((horizontal, vertical));
        }
        let shared = (
            scales.windows(2).all(|pair| pair[0].0 == pair[1].0),
            scales.windows(2).all(|pair| pair[0].1 == pair[1].1),
        );
        let geoms = self
            .tiles
            .first()
            .map(|(layers, _, _)| {
                layers
                    .iter()
                    .filter(|layer| !layer.companion)
                    .map(|layer| layer.geom.name())
                    .collect()
            })
            .unwrap_or_default();
        ZoomPlot {
            tiles,
            shared,
            geoms,
        }
    }

    /// Draw the plot as laid out by `arrangement`: everything but the background
    /// fill (the scene's, or the grid cell's), with the background border last.
    pub fn draw(&self, arrangement: &Arrangement) -> Result<Vec<Node>> {
        let theme = &self.theme;
        let parallel = self.is_large();
        let draw_layers = |layers: &[PreparedLayer], panel: &Panel| -> Result<Vec<Node>> {
            // Layers draw in parallel and stack in layer order.
            let drawn = parallel::ordered_map(layers.iter().collect(), parallel, |layer| {
                let draw_layer = DrawLayer {
                    data: &layer.data,
                    styles: &layer.styles,
                    parameters: Parameters::new(&layer.options, &layer.path),
                    background: self.layer_background(),
                };
                layer
                    .geom
                    .draw(&draw_layer, &panel.oriented(layer.y_oriented))
            });
            let mut layer_nodes: Vec<Node> = Vec::new();
            for (nodes, layer) in drawn.into_iter().zip(layers) {
                let nodes = nodes?;
                if layer.companion
                    && let Some(Node::Group(group)) = layer_nodes.last_mut()
                {
                    group.nodes.extend(nodes);
                    continue;
                }
                layer_nodes.push(Node::Group(Group {
                    clip: None,
                    role: Some(LAYER_ROLE),
                    anchor: None,
                    nodes,
                }));
            }
            Ok(layer_nodes)
        };
        let mut nodes = Vec::new();
        match &arrangement.layout {
            Arranged::Single(layout) => {
                let (layers, tile_x, tile_y) = &self.tiles[0];
                let (horizontal, vertical) = self.screen_scales(tile_x, tile_y);
                let panel = Panel {
                    rect: layout.panel,
                    x_scale: horizontal,
                    y_scale: vertical,
                    coord: self.coord,
                    flipped: self.flipped,
                };
                let layer_nodes = draw_layers(layers, &panel)?;
                let mut tile = draw_panel_background(layout, theme, &panel);
                tile.push(Node::Group(Group {
                    clip: Some(layout.panel),
                    role: Some("panel"),
                    anchor: None,
                    nodes: layer_nodes,
                }));
                tile.extend(draw_panel_border(layout, theme));
                tile.extend(draw_axes(layout, theme, &panel, (true, true)));
                nodes.push(tile_group(tile));
            }
            Arranged::Facets(layout) => {
                // Panels draw in parallel and join in drawing order.
                let tiles = layout.tiles.iter().zip(&self.tiles).collect();
                let drawn =
                    parallel::ordered_map(tiles, parallel, |(tile, (layers, tile_x, tile_y))| {
                        let (horizontal, vertical) = self.screen_scales(tile_x, tile_y);
                        let panel = Panel {
                            rect: tile.layout.panel,
                            x_scale: horizontal,
                            y_scale: vertical,
                            coord: self.coord,
                            flipped: self.flipped,
                        };
                        let layer_nodes = draw_layers(layers, &panel)?;
                        let mut tile_nodes = draw_panel_background(&tile.layout, theme, &panel);
                        tile_nodes.push(Node::Group(Group {
                            clip: Some(tile.layout.panel),
                            role: Some("panel"),
                            anchor: None,
                            nodes: layer_nodes,
                        }));
                        tile_nodes.extend(draw_panel_border(&tile.layout, theme));
                        tile_nodes.extend(draw_axes(&tile.layout, theme, &panel, tile.axes));
                        let mut panel_nodes = vec![tile_group(tile_nodes)];
                        panel_nodes.extend(draw_strips(tile, theme));
                        Ok::<_, Error>(panel_nodes)
                    });
                for panel_nodes in drawn {
                    nodes.extend(panel_nodes?);
                }
            }
        }
        let plot_layout = arrangement.plot_layout();
        nodes.push(draw_titles(plot_layout, theme, &arrangement.titles));
        if let Some(block) = &self.legend_block(arrangement.area)? {
            let origin = legend_origin(plot_layout, theme, &arrangement.titles, block.size);
            nodes.extend(draw_legends(block, origin, theme)?);
        }
        // The plot background's border, drawn last over the whole plot (lets-plot
        // `PlotSvgComponent.backgroundBorder`).
        if let Some(border) = theme
            .plot_background
            .and_then(|background| background.border)
        {
            nodes.push(Node::Rects(Rects {
                rects: vec![arrangement.area],
                fill: PerItem::Uniform(None),
                stroke: Some(border.stroke()),
            }));
        }
        Ok(nodes)
    }
}

/// Train the position scales on prepared layers: spans of the geom setup, then
/// every position column the setup did not make.
fn train_positions(
    layers: &[PreparedLayer],
    x_scale: &mut PositionScale,
    y_scale: &mut PositionScale,
) {
    for layer in layers {
        if let Some((low, high)) = layer.x_span {
            x_scale.train_span(low, high);
        }
        if let Some((low, high)) = layer.y_span {
            y_scale.train_span(low, high);
        }
        let setup = layer.geom.setup_aesthetics();
        let data = if layer.trains_unadjusted {
            &layer.unadjusted
        } else {
            &layer.data
        };
        for aesthetic in data.aesthetics() {
            if setup.contains(&aesthetic) {
                continue;
            }
            if let Some(column) = data.continuous(aesthetic) {
                let aesthetic = match layer.y_oriented {
                    true => aesthetic.flip_orientation(),
                    false => aesthetic,
                };
                if aesthetic.is_x_position() {
                    x_scale.train(column);
                } else if aesthetic.is_y_position() {
                    y_scale.train(column);
                }
            }
        }
    }
}

/// One facet panel's part of a mapped layer: its rows of every column. Groups
/// stay the layer's (renumbered 0, 1, ... in their order), with each group's rank
/// by first appearance in the whole layer, so a panel's stat keeps the plot's
/// group order (lets-plot groups the merged stat output, not each panel's).
fn split_layer<'a>(layer: &MappedLayer<'a>, rows: &[u32]) -> MappedLayer<'a> {
    let (group_index, group_rank) = match &layer.group_index {
        None => (None, None),
        Some(groups) => {
            // Rank of every layer group by its first row.
            let count = groups.iter().max().map_or(0, |max| *max as usize + 1);
            let mut rank_of = vec![u32::MAX; count];
            let mut next = 0;
            for &group in groups {
                if rank_of[group as usize] == u32::MAX {
                    rank_of[group as usize] = next;
                    next += 1;
                }
            }
            let taken: Vec<u32> = rows.iter().map(|&row| groups[row as usize]).collect();
            let mut used = taken.clone();
            used.sort_unstable();
            used.dedup();
            let dense = taken
                .iter()
                .map(|group| used.partition_point(|known| known < group) as u32)
                .collect();
            let ranks = used.iter().map(|group| rank_of[*group as usize]).collect();
            (Some(dense), Some(ranks))
        }
    };
    MappedLayer {
        spec: layer.spec,
        path: layer.path.clone(),
        geom: layer.geom,
        stat: layer.stat,
        positions: layer.positions.clone(),
        options: layer.options.clone(),
        row_count: rows.len(),
        columns: layer
            .columns
            .iter()
            .map(|(aesthetic, column, path)| (*aesthetic, column.take(rows), path.clone()))
            .collect(),
        stat_mappings: layer.stat_mappings.clone(),
        temporal: layer.temporal.clone(),
        y_oriented: layer.y_oriented,
        group_index,
        group_rank,
        facet_columns: Vec::new(),
        tooltip_columns: layer
            .tooltip_columns
            .iter()
            .map(|(name, column)| (name.clone(), column.take(rows)))
            .collect(),
        warnings: Vec::new(),
    }
}

/// Stat output groups (numbered in the order the stat visited its input groups,
/// first appearance in the panel) renumbered by their input group's `rank`.
fn regroup_by_rank(data: &mut LayerData, visited: &[u32], rank: &[u32]) -> Result<()> {
    let Some(groups) = data.group_index() else {
        return Ok(());
    };
    let ranks: Vec<u32> = visited
        .iter()
        .map(|group| rank.get(*group as usize).copied().unwrap_or(u32::MAX))
        .collect();
    let mut order: Vec<u32> = ranks.clone();
    order.sort_unstable();
    order.dedup();
    let renumbered = groups
        .iter()
        .map(|group| {
            let rank = ranks.get(*group as usize).copied().unwrap_or(u32::MAX);
            order.partition_point(|known| *known < rank) as u32
        })
        .collect();
    data.set_group_index(Some(renumbered))
}

/// One facet panel before its stats run: its layers, its x and y scales before
/// training and its stat context.
type TileSetup<'a> = (
    Vec<MappedLayer<'a>>,
    PositionScale,
    PositionScale,
    StatContext,
);

/// A free facet axis's scale for one panel: the plot's scale (kind, transform,
/// limits) with only the discrete levels the panel's rows use, in the plot's order
/// (lets-plot `adjustFreeDisctereHDomainsByTile`).
fn panel_scale(
    shared: &PositionScale,
    layers: &[&MappedLayer],
    on_axis: fn(Aesthetic) -> bool,
) -> PositionScale {
    let PositionScale::Discrete(discrete) = shared else {
        return shared.clone();
    };
    let mut used = BTreeSet::new();
    for (_, column, _) in layers
        .iter()
        .flat_map(|layer| &layer.columns)
        .filter(|(aesthetic, _, _)| on_axis(*aesthetic))
    {
        if let Column::Discrete(column) = column {
            for row in 0..column.len() {
                if let Some(label) = column.label(row) {
                    used.insert(label.to_owned());
                }
            }
        }
    }
    PositionScale::Discrete(discrete.restricted_to(&used))
}

/// Discrete levels of the style aesthetics in the data's order (layer by layer),
/// before facets split the rows.
fn style_level_order(layers: &[MappedLayer]) -> BTreeMap<Aesthetic, Vec<String>> {
    let mut order: BTreeMap<Aesthetic, Vec<String>> = BTreeMap::new();
    for (aesthetic, column, _) in layers.iter().flat_map(|layer| &layer.columns) {
        let Column::Discrete(column) = column else {
            continue;
        };
        let levels = order.entry(*aesthetic).or_default();
        for level in column.levels() {
            if !levels.contains(level) {
                levels.push(level.clone());
            }
        }
    }
    order
}

/// Warnings of every facet panel's layers, each once: "removed N rows" notes of
/// one layer are summed over the panels.
fn merge_tile_warnings(warnings: Vec<String>) -> Vec<String> {
    /// A "removed N rows" note split around its count.
    struct Removed {
        head: String,
        tail: String,
        count: usize,
    }
    let mut merged: Vec<std::result::Result<Removed, String>> = Vec::new();
    for warning in warnings {
        let removed = warning.split_once(": removed ").and_then(|(head, rest)| {
            let (count, tail) = rest.split_once(' ')?;
            Some(Removed {
                head: head.to_owned(),
                tail: tail.to_owned(),
                count: count.parse().ok()?,
            })
        });
        match removed {
            Some(removed) => match merged.iter_mut().find_map(|known| match known {
                Ok(known) if known.head == removed.head && known.tail == removed.tail => {
                    Some(known)
                }
                _ => None,
            }) {
                Some(known) => known.count += removed.count,
                None => merged.push(Ok(removed)),
            },
            None if merged
                .iter()
                .any(|known| known.as_ref().err() == Some(&warning)) => {}
            None => merged.push(Err(warning)),
        }
    }
    merged
        .into_iter()
        .map(|warning| match warning {
            Ok(Removed { head, tail, count }) => format!("{head}: removed {count} {tail}"),
            Err(text) => text,
        })
        .collect()
}

/// Title, subtitle and axis titles. Axis titles follow lets-plot: `labs(x=...)`
/// first, then the scale's `name`, otherwise `defaultScaleName` (see
/// [`default_axis_title`]).
fn plot_titles(spec: &PlotSpec, scales: &PositionScales, orientations: &[bool]) -> Titles {
    let axis_title = |aesthetic: Aesthetic, scale_name: &Option<String>| {
        let from_labs = match spec.guides.get(aesthetic.as_str()) {
            Some(GuideSpec::Full(options)) => options.title.clone(),
            _ => None,
        };
        from_labs
            .or_else(|| scale_name.clone())
            .unwrap_or_else(|| default_scale_name(spec, aesthetic, orientations))
    };
    let title = spec.ggtitle.as_ref();
    Titles {
        title: title.and_then(|title| title.text.clone()),
        subtitle: title.and_then(|title| title.subtitle.clone()),
        caption: spec
            .caption
            .as_ref()
            .and_then(|caption| caption.text.clone()),
        x: Some(axis_title(Aesthetic::X, &scales.x.name)),
        y: Some(axis_title(Aesthetic::Y, &scales.y.name)),
    }
}

/// lets-plot `PlotConfigUtil.defaultScaleName`: the distinct columns mapped to the
/// aesthetic across layers, ignoring one named like the aesthetic. None -> the
/// aesthetic name; several -> joined with ", " (the aesthetic name for x and y).
fn default_scale_name(spec: &PlotSpec, aesthetic: Aesthetic, orientations: &[bool]) -> String {
    let mut labels: Vec<String> = Vec::new();
    for (index, layer) in spec.layers.iter().enumerate() {
        // A y-oriented layer's stat maps its variables to the swapped aesthetic.
        let stat_aesthetic = match orientations.get(index).copied().unwrap_or(false) {
            true => aesthetic.flip_orientation(),
            false => aesthetic,
        };
        let (mapping, inherited) = merged_mapping(spec, layer);
        let label = discrete_annotation(spec, layer, aesthetic, inherited.contains(&aesthetic))
            .and_then(|annotation| annotation.parameters.label.clone());
        // A stat variable is labelled by its name (`..count..` -> `count`), also
        // when the stat maps it by default (y of `geom_bar`).
        let name: Option<String> = match mapping.get(&aesthetic) {
            Some(value) => value
                .as_column()
                .map(|name| match Computed::from_mapping(name) {
                    Some(computed) => computed.as_str().to_owned(),
                    None => name.to_owned(),
                }),
            None if sets_constant(layer, aesthetic) => None,
            None => layer_stat(layer).and_then(|stat| {
                stat.default_mapping()
                    .iter()
                    .find(|(mapped, _)| *mapped == stat_aesthetic)
                    .map(|(_, computed)| computed.as_str().to_owned())
            }),
        };
        let name = label.or(name);
        if let Some(name) = name
            && name != aesthetic.as_str()
            && !labels.contains(&name)
        {
            labels.push(name);
        }
    }
    match labels.as_slice() {
        [] => aesthetic.as_str().to_owned(),
        [_, _, ..] if matches!(aesthetic, Aesthetic::X | Aesthetic::Y) => {
            aesthetic.as_str().to_owned()
        }
        _ => labels.join(", "),
    }
}

/// The stat a layer runs: its own, else its geom's default. `None` for unknown
/// names (reported when the layer is mapped).
/// Whether the layer sets `aesthetic` as a constant (`size=5`), which wins over
/// the stat's default mapping, as in lets-plot.
fn sets_constant(layer: &LayerSpec, aesthetic: Aesthetic) -> bool {
    layer
        .parameters
        .get(aesthetic.as_str())
        .is_some_and(|value| !value.is_null())
}

fn layer_stat(layer: &LayerSpec) -> Option<&'static dyn Stat> {
    let name = match &layer.stat {
        Some(name) => name.as_str(),
        None => geom::geom(&layer.geom)?.default_stat(),
    };
    stat::stat(name)
}

/// Whether a guide entry is only an axis title from `labs(x=...)` / `labs(y=...)`,
/// which the layout already uses.
fn is_axis_title_only(key: &str, guide: &GuideSpec) -> bool {
    let GuideSpec::Full(options) = guide else {
        return false;
    };
    matches!(key, "x" | "y") && is_title_only(options)
}

/// Whether guide options hold only a title (and keys reported as unsupported
/// elsewhere): every other option is at its default, so none can be missed.
fn is_title_only(options: &GuideOptions) -> bool {
    let title_only = GuideOptions {
        title: options.title.clone(),
        unsupported: options.unsupported.clone(),
        ..GuideOptions::default()
    };
    *options == title_only
}

/// One layer after stat and position, ready to draw.
struct PreparedLayer {
    /// Layer options with aliases normalised (`colour` -> `color`).
    options: Options,
    path: String,
    geom: &'static dyn Geom,
    data: LayerData,
    /// See [`ComputedLayer::unadjusted`].
    unadjusted: LayerData,
    /// See [`ComputedLayer::tooltip_columns`].
    tooltip_columns: Vec<(String, Column)>,
    /// See [`ComputedLayer::x_span`].
    x_span: Option<(f64, f64)>,
    /// See [`ComputedLayer::trains_unadjusted`].
    trains_unadjusted: bool,
    /// See [`ComputedLayer::y_span`].
    y_span: Option<(f64, f64)>,
    /// See [`ComputedLayer::y_oriented`].
    y_oriented: bool,
    styles: Styles,
    /// `show_legend` (default: the geom's).
    show_legend: bool,
    /// Drawn on behalf of the layer before it: the outlier points the Python
    /// `geom_boxplot` adds as a second spec layer. It shares that layer's group
    /// and toggle (`ggtb(layers=...)`), so toggle indices count the layers the
    /// user added.
    companion: bool,
}

/// The toggle index (`ggtb(layers=...)`) of every layer: companion layers take
/// the index of the layer they belong to.
fn toggle_indices(layers: &[PreparedLayer]) -> Vec<usize> {
    let mut next = 0;
    layers
        .iter()
        .map(|layer| {
            if !layer.companion || next == 0 {
                next += 1;
            }
            next - 1
        })
        .collect()
}

/// Stage 1 of a layer: look up geom/stat/position, validate options, fetch the
/// mapped columns and compute groups. Positions stay as in the data: the position
/// scales are known only after every layer is mapped.
fn map_layer<'a>(
    index: usize,
    spec: &'a LayerSpec,
    plot: &PlotSpec,
    data: &PlotData,
    facet_variables: &[(String, &'static str)],
) -> StageResult<MappedLayer<'a>> {
    let mut warnings = Vec::new();
    map_layer_into(index, spec, plot, data, facet_variables, &mut warnings)
        .map_err(|error| (error, warnings))
}

fn map_layer_into<'a>(
    index: usize,
    spec: &'a LayerSpec,
    plot: &PlotSpec,
    data: &PlotData,
    facet_variables: &[(String, &'static str)],
    warnings: &mut Vec<String>,
) -> Result<MappedLayer<'a>> {
    let path = format!("layers[{index}]");
    let at = |key: &str| format!("{path}.{key}");
    let geom = geom::geom(&spec.geom).ok_or_else(|| {
        let planned = geom::NOT_DRAWN_YET.contains(&spec.geom.as_str());
        unknown("geom", &at("geom"), &spec.geom, planned, &geom::names())
    })?;
    let stat_name = spec.stat.as_deref().unwrap_or(geom.default_stat());
    let stat = stat::stat(stat_name).ok_or_else(|| {
        let planned = stat::NOT_COMPUTED_YET.contains(&stat_name);
        unknown("stat", &at("stat"), stat_name, planned, &stat::names())
    })?;
    let positions = layer_positions(geom, stat, spec, &path)?;

    let options = normalize_options(&spec.parameters, &path, warnings);
    validate_parameters(geom, stat, &options, &path, warnings);
    for (position, position_options, position_path) in &positions {
        validate_position_options(*position, position_options, position_path, warnings);
    }

    let (mut mapping, inherited) = merged_mapping(plot, spec);
    // An aesthetic given as a constant wins over its mapping (lets-plot
    // `DataConfigUtil`). Unlike lets-plot, but as ggplot2, the mapping still
    // groups (below): `geom_boxplot(aes(fill=g), outlier_fill="white")` keeps
    // outliers per box, `geom_line(aes(color=g), color="black")` one line per g.
    let has_constant = |aesthetic: &Aesthetic| {
        options
            .get(aesthetic.as_str())
            .is_some_and(|value| !value.is_null())
    };
    let mut overridden = mapping.clone();
    overridden.retain(|aesthetic, _| has_constant(aesthetic));
    mapping.retain(|aesthetic, _| !has_constant(aesthetic));
    let frames = LayerFrames {
        layer: data.layers.get(index).and_then(Option::as_ref),
        plot: data.plot.as_ref(),
    };
    let mut columns = Vec::new();
    let mut stat_mappings = Vec::new();
    let mut temporal = Vec::new();
    // Discrete non-positional columns, the default grouping (lets-plot
    // `defaultGroupingVariables`), by column name.
    let mut default_grouping: Vec<(&str, Column)> = Vec::new();
    for (aesthetic, value) in &mapping {
        if *aesthetic == Aesthetic::Group {
            continue; // see `explicit_grouping`
        }
        let mapping_path = if inherited.contains(aesthetic) {
            format!("mapping.{aesthetic}")
        } else {
            at(&format!("mapping.{aesthetic}"))
        };
        let MappingValue::Column(name) = value else {
            warnings.push(format!(
                "at '{mapping_path}': mapping '{aesthetic}' to {} is not supported yet and \
                 was ignored",
                describe_mapping_value(value)
            ));
            continue;
        };
        // Checked before the column is fetched: lets-plot ignores an unused
        // aesthetic even when its column does not exist.
        if !geom_uses(geom, stat, *aesthetic) {
            // Plot-level x/y are meant for the layers that read them (a
            // `geom_hline` over `ggplot(aes(x, y))` is the usual case): no warning.
            if inherited.contains(aesthetic)
                && (aesthetic.is_x_position() || aesthetic.is_y_position())
            {
                continue;
            }
            warnings.push(format!(
                "at '{mapping_path}': geom_{} does not use aesthetic '{aesthetic}' (yet); \
                 the mapping was ignored",
                geom.name()
            ));
            continue;
        }
        // `..count..`: a variable the stat computes, filled in after the stat.
        if let Some(computed) = Computed::from_mapping(name) {
            stat_mappings.push((*aesthetic, computed, mapping_path));
            continue;
        }
        let column = annotated_column(
            plot,
            spec,
            &frames,
            stat.name() == "identity",
            *aesthetic,
            inherited.contains(aesthetic),
            name,
            frames.column(name, &mapping_path)?,
            &mapping_path,
            warnings,
        )?;
        if matches!(column, Column::Discrete(_))
            && !aesthetic.is_x_position()
            && !aesthetic.is_y_position()
            && !default_grouping.iter().any(|(seen, _)| seen == name)
        {
            default_grouping.push((name, column.clone()));
        }
        if (aesthetic.is_x_position() || aesthetic.is_y_position())
            && matches!(column, Column::Continuous(_))
            && let Some(kind) = temporal_kind(plot, spec, name, &mapping_path, warnings)
        {
            temporal.push((*aesthetic, kind));
        }
        columns.push((*aesthetic, column, mapping_path));
    }
    for (aesthetic, value) in &overridden {
        let Some(name) = value.as_column() else {
            continue;
        };
        if aesthetic.is_x_position()
            || aesthetic.is_y_position()
            || *aesthetic == Aesthetic::Group
            || Computed::from_mapping(name).is_some()
            || !geom_uses(geom, stat, *aesthetic)
            || default_grouping.iter().any(|(seen, _)| *seen == name)
        {
            continue;
        }
        // A missing column only groups nothing: the constant is what is drawn.
        let mapping_path = format!("mapping.{aesthetic}");
        if let Ok(column @ Column::Discrete(_)) = frames.column(name, &mapping_path) {
            default_grouping.push((name, column));
        }
    }
    let grouping = match explicit_grouping(plot, spec, &path, warnings) {
        Some(names) => names
            .iter()
            .map(|(name, mapping_path)| frames.column(name, mapping_path))
            .collect::<Result<Vec<_>>>()?,
        None => default_grouping
            .into_iter()
            .map(|(_, column)| column)
            .collect(),
    };
    let y_oriented = layer_orientation(geom, stat, &options, &columns, &at("orientation"))?;
    let unbound = columns.is_empty();
    // A layer without data columns is drawn in every panel.
    let facet_columns = facet_variables
        .iter()
        .map(|(name, path)| match frames.has(name) && !unbound {
            true => frames.column(name, path).map(Some),
            false => Ok(None),
        })
        .collect::<Result<LayerFacetColumns>>()?;
    let group_index = if unbound {
        None
    } else {
        group_index(&grouping.iter().collect::<Vec<_>>())
    };
    let mut tooltip_columns = Vec::new();
    if let Some(TooltipsSpec::Lines(lines)) = &spec.tooltips
        && !unbound
    {
        for name in crate::tooltip::referenced_variables(lines) {
            let mapped = mapping
                .values()
                .any(|value| value.as_column() == Some(&name));
            if mapped || !frames.has(&name) {
                continue;
            }
            match frames.column(&name, &at("tooltips")) {
                Ok(column) => tooltip_columns.push((name, column)),
                Err(error) => {
                    let reason = match error {
                        Error::InvalidSpec { .. } => error.reason(),
                        other => other.to_string(),
                    };
                    warnings.push(format!("{reason}; tooltips cannot show it yet"));
                    // All missing: its lines are left out without a second warning.
                    let missing = vec![None; frames.row_count()];
                    tooltip_columns.push((
                        name,
                        Column::Continuous(ContinuousColumn::from_options(missing)),
                    ));
                }
            }
        }
    }
    Ok(MappedLayer {
        spec,
        path,
        geom,
        stat,
        positions,
        options,
        // lets-plot drops the data of a layer with no mapped columns: one row.
        row_count: if unbound { 1 } else { frames.row_count() },
        columns,
        stat_mappings,
        temporal,
        y_oriented,
        group_index,
        group_rank: None,
        facet_columns,
        tooltip_columns,
        warnings: std::mem::take(warnings),
    })
}

/// Whether a layer runs in y orientation (lets-plot `LayerConfig.initYOrientation`):
/// `orientation="y"`, or, without the option, a counting or distribution layer
/// (count, summary, boxplot, ydensity stats; bar, violin, sina geoms) with a
/// discrete y and a non-discrete x. Such a layer computes and draws with x and y
/// swapped (see [`Aesthetic::flip_orientation`]): `geom_bar(aes(y="g"))` draws
/// horizontal bars. Geoms that read `orientation` themselves are left alone.
fn layer_orientation(
    geom: &dyn Geom,
    stat: &dyn Stat,
    options: &Options,
    columns: &[(Aesthetic, Column, String)],
    path: &str,
) -> Result<bool> {
    // A geom that reads the option itself (brackets) handles it.
    if geom.parameters().contains(&"orientation") {
        return Ok(false);
    }
    match options.get("orientation") {
        None | Some(Value::Null) => {}
        Some(Value::String(text)) if text.eq_ignore_ascii_case("x") => return Ok(false),
        Some(Value::String(text)) if text.eq_ignore_ascii_case("y") => return Ok(true),
        Some(other) => {
            return Err(Error::spec(
                path,
                format!("orientation must be 'x' or 'y', got {other}"),
            ));
        }
    }
    let auto = matches!(
        stat.name(),
        "count" | "summary" | "boxplot" | "boxplot_outlier" | "ydensity"
    ) || matches!(geom.name(), "bar" | "violin" | "sina");
    let discrete = |wanted: Aesthetic| {
        columns.iter().any(|(aesthetic, column, _)| {
            *aesthetic == wanted && matches!(column, Column::Discrete(_))
        })
    };
    Ok(auto && !discrete(Aesthetic::X) && discrete(Aesthetic::Y))
}

/// A layer after [`map_layer`].
struct MappedLayer<'a> {
    spec: &'a LayerSpec,
    path: String,
    geom: &'static dyn Geom,
    stat: &'static dyn Stat,
    /// Positions applied in order, with their options and option paths.
    positions: Vec<(&'static dyn Position, Options, String)>,
    /// Layer options with aliases normalised (`colour` -> `color`).
    options: Options,
    row_count: usize,
    /// Mapped columns with their mapping paths; positions not yet scaled.
    columns: Vec<(Aesthetic, Column, String)>,
    /// Aesthetics mapped to stat variables (`..count..`), with mapping paths.
    stat_mappings: Vec<(Aesthetic, Computed, String)>,
    /// Position aesthetics mapped to date-time columns (series type `datetime`,
    /// `date` or `time`): their axes get date-time breaks.
    temporal: Vec<(Aesthetic, TemporalKind)>,
    /// `orientation="y"` (see [`layer_orientation`]).
    y_oriented: bool,
    group_index: Option<Vec<u32>>,
    /// A facet panel's groups: each group's rank in the whole layer (see
    /// [`split_layer`]); `None` without facets.
    group_rank: Option<Vec<u32>>,
    /// The layer's faceting columns (see [`Facets::rows_by_tile`]).
    facet_columns: LayerFacetColumns,
    /// Unmapped columns `layer_tooltips()` reads, by name.
    tooltip_columns: Vec<(String, Column)>,
    /// Warnings of this stage.
    warnings: Vec<String>,
}

/// The scale of one axis: discrete when `scale_*_discrete()` asks for it or any
/// layer maps a discrete column to the axis; levels join across layers in order.
fn axis_scale(
    spec: &AxisScaleSpec,
    layers: &[MappedLayer],
    on_axis: fn(Aesthetic) -> bool,
    axis: &str,
    warnings: &mut Vec<String>,
) -> Result<PositionScale> {
    let axis_columns = || {
        layers
            .iter()
            .flat_map(|layer| &layer.columns)
            .filter(|(aesthetic, _, _)| on_axis(*aesthetic))
    };
    let discrete = spec.requests_discrete()
        || axis_columns().any(|(_, column, _)| matches!(column, Column::Discrete(_)));
    // lets-plot gives an axis of date-time columns a date-time scale.
    let column_kind = layers
        .iter()
        .flat_map(|layer| &layer.temporal)
        .find(|(aesthetic, _)| on_axis(*aesthetic))
        .map(|(_, kind)| *kind);
    let mut scale = spec.build_with(discrete, column_kind, warnings)?;
    if let PositionScale::Discrete(discrete) = &mut scale {
        for (_, column, _) in axis_columns() {
            if let Column::Discrete(column) = column {
                discrete.train_levels(column.levels());
            }
        }
        if !discrete.has_levels()
            && let Some((_, _, path)) =
                axis_columns().find(|(_, column, _)| matches!(column, Column::Continuous(_)))
        {
            return Err(Error::spec(
                path,
                format!(
                    "the {axis} axis is discrete (scale_{axis}_discrete()) but only \
                 numeric columns are mapped to it; converting numbers to categories is not \
                 supported yet, map a string column instead"
                ),
            ));
        }
    }
    Ok(scale)
}

/// Stage 2 of a layer: scale the positions (lets-plot transforms data when mapping;
/// values a scale drops become missing), add constant positions, then run the stat
/// and position adjustment.
fn compute_layer(
    layer: MappedLayer,
    [x_scale, y_scale]: [&PositionScale; 2],
    context: &StatContext,
) -> Result<ComputedLayer> {
    let MappedLayer {
        spec,
        path,
        geom,
        stat,
        positions,
        options,
        row_count,
        columns,
        stat_mappings,
        y_oriented,
        group_index,
        group_rank,
        tooltip_columns,
        mut warnings,
        ..
    } = layer;
    let at = |key: &str| format!("{path}.{key}");
    // A y-oriented layer computes with x and y swapped (lets-plot flips the
    // aesthetics around the stat, position and geom): `oriented` maps between
    // the plot's aesthetics and the layer's.
    let oriented = |aesthetic: Aesthetic| match y_oriented {
        true => aesthetic.flip_orientation(),
        false => aesthetic,
    };
    let scale_of = |aesthetic: Aesthetic| {
        let aesthetic = oriented(aesthetic);
        if aesthetic.is_x_position() {
            Some(x_scale)
        } else if aesthetic.is_y_position() {
            Some(y_scale)
        } else {
            None
        }
    };
    let swapped_context;
    let context = match y_oriented {
        true => {
            swapped_context = StatContext {
                x_range: context.y_range,
                y_range: context.x_range,
            };
            &swapped_context
        }
        false => context,
    };
    let (x_discrete, y_discrete) = match y_oriented {
        true => (y_scale.is_discrete(), x_scale.is_discrete()),
        false => (x_scale.is_discrete(), y_scale.is_discrete()),
    };
    let mut layer_data = LayerData::new(row_count);
    let mut mapping_paths = BTreeMap::new();
    for (aesthetic, column, mapping_path) in columns {
        let aesthetic = oriented(aesthetic);
        let column = match scale_of(aesthetic) {
            Some(scale) => scale
                .positions(column)
                .map_err(|message| Error::spec(&mapping_path, message))?,
            None => column,
        };
        layer_data.insert(aesthetic, column)?;
        mapping_paths.insert(aesthetic, mapping_path);
    }
    layer_data.set_group_index(group_index)?;

    // Constant positions, e.g. geom_point(x=1.5): one value repeated for every row.
    for &aesthetic in geom.required_aesthetics() {
        if layer_data.contains(aesthetic) {
            continue;
        }
        let parameters = Parameters::new(&options, &path);
        if let Some(value) = parameters.number(oriented(aesthetic).as_str())? {
            let column = ContinuousColumn::from_values(vec![value; layer_data.row_count()]);
            let column = match scale_of(aesthetic) {
                Some(scale) => scale
                    .positions(Column::Continuous(column))
                    .map_err(|message| Error::spec(at(oriented(aesthetic).as_str()), message))?,
                None => Column::Continuous(column),
            };
            layer_data.insert(aesthetic, column)?;
        }
    }

    let parameters = Parameters::new(&options, &path);
    let is_identity = stat.name() == "identity";
    if !is_identity {
        require_aesthetics(
            &format!("stat_{}", stat.name()),
            stat.required_aesthetics(),
            &layer_data,
            &path,
            y_oriented,
        )?;
        if !spec.na_rm.unwrap_or(false) {
            report_missing_values(
                &format!("stat_{}", stat.name()),
                stat.required_aesthetics(),
                false,
                &layer_data,
                &path,
                &mut warnings,
            );
        }
    }
    // A facet panel's input groups in the order the stat visits them.
    let visited: Option<Vec<u32>> = group_rank.as_ref().filter(|_| !is_identity).map(|_| {
        rows_by_group(&layer_data)
            .iter()
            .filter_map(|rows| rows.first().map(|row| layer_data.group_of(*row)))
            .collect()
    });
    let mut layer_data = stat.compute(layer_data, &parameters, context)?;
    // Stat positions the geom does not draw (a summary's ymin/ymax under
    // `geom="point"`) neither train the axes nor reach the geom, as lets-plot
    // trains only a geom's rendered aesthetics.
    if !is_identity {
        let unused: Vec<Aesthetic> = layer_data
            .aesthetics()
            .filter(|aesthetic| aesthetic.is_x_position() || aesthetic.is_y_position())
            .filter(|aesthetic| !geom_uses(geom, stat, *aesthetic))
            .collect();
        for aesthetic in unused {
            layer_data.remove(aesthetic);
        }
    }
    layer_data.set_discrete_axes(x_discrete, y_discrete);
    if let (Some(visited), Some(rank)) = (&visited, &group_rank) {
        regroup_by_rank(&mut layer_data, visited, rank)?;
    }
    // Aesthetics drawn from stat variables, which are in data units.
    let mut from_variables = BTreeSet::new();
    // Stat variables: the layer's `..name..` mappings, then the stat's defaults
    // for aesthetics the layer leaves to it (lets-plot replaces a mapped column
    // there too).
    for (aesthetic, computed, mapping_path) in stat_mappings.iter() {
        let aesthetic = &oriented(*aesthetic);
        let column = layer_data.get(*computed).cloned().ok_or_else(|| {
            Error::spec(
                mapping_path,
                format!(
                    "stat_{} does not compute '..{}..'",
                    stat.name(),
                    computed.as_str()
                ),
            )
        })?;
        layer_data.insert(*aesthetic, column)?;
        mapping_paths.insert(*aesthetic, mapping_path.clone());
        from_variables.insert(*aesthetic);
    }
    for (aesthetic, computed) in stat.default_mapping() {
        if sets_constant(spec, oriented(*aesthetic))
            || stat_mappings
                .iter()
                .any(|(mapped, _, _)| oriented(*mapped) == *aesthetic)
        {
            continue;
        }
        if let Some(column) = layer_data.get(*computed).cloned() {
            layer_data.insert(*aesthetic, column)?;
            from_variables.insert(*aesthetic);
        }
    }
    // Stat positions through the scales, as lets-plot maps stat output: variables
    // (`..count..` on y) are transformed and limited; positions the stat made in
    // axis units (bin centres) are only limited.
    if !is_identity {
        let positions: Vec<Aesthetic> = layer_data
            .aesthetics()
            .filter(|aesthetic| aesthetic.is_x_position() || aesthetic.is_y_position())
            .collect();
        for aesthetic in positions {
            let (Some(scale), Some(column)) = (
                scale_of(aesthetic),
                layer_data.continuous(aesthetic).cloned(),
            ) else {
                continue;
            };
            let column = if from_variables.contains(&aesthetic) {
                scale
                    .positions(Column::Continuous(column))
                    .map_err(|message| {
                        let path = mapping_paths.get(&aesthetic).unwrap_or(&path);
                        Error::spec(path, message)
                    })?
            } else {
                Column::Continuous(scale.within_limits(column))
            };
            layer_data.insert(aesthetic, column)?;
        }
    }
    require_aesthetics(
        &format!("geom_{}", geom.name()),
        geom.required_aesthetics(),
        &layer_data,
        &path,
        y_oriented,
    )?;
    let layer_data = geom.setup(layer_data, &parameters)?;
    let x_span = setup_span(geom, &layer_data, Aesthetic::is_x_position);
    let y_span = setup_span(geom, &layer_data, Aesthetic::is_y_position);
    let (x_span, y_span) = match y_oriented {
        true => (y_span, x_span),
        false => (x_span, y_span),
    };
    // Tooltips show values before the position adjustment (lets-plot adjusts
    // positions while drawing). Columns are shared, so this costs no copy.
    let unadjusted = layer_data.clone();
    // Data columns line up with the layer rows only when the stat kept them.
    let tooltip_columns = match (is_identity, unadjusted.source_rows()) {
        (true, SourceRows::All) => tooltip_columns,
        _ => Vec::new(),
    };
    let trains_unadjusted = positions
        .iter()
        .any(|(position, _, _)| !position.trains_axes());
    let layer_data = apply_positions(layer_data, &positions)?;

    // Missing stat output is deliberate (e.g. `threshold`); input rows the stat
    // dropped were reported above.
    if is_identity && !spec.na_rm.unwrap_or(false) {
        report_missing_values(
            &format!("geom_{}", geom.name()),
            geom.required_aesthetics(),
            // Padded steps draw x = -inf / +inf at the panel edges.
            geom.draws_infinite_x(&parameters)?,
            &layer_data,
            &path,
            &mut warnings,
        );
    }
    Ok(ComputedLayer {
        options,
        path,
        geom,
        data: layer_data,
        unadjusted,
        tooltip_columns,
        trains_unadjusted,
        x_span,
        y_span,
        y_oriented,
        mapping_paths,
        show_legend: spec.show_legend.unwrap_or(geom.shows_legend()),
        companion: layer_stat(spec).is_some_and(|stat| stat.name() == "boxplot_outlier"),
        warnings,
    })
}

/// A layer after [`compute_layer`]: positions scaled, stat and position applied.
struct ComputedLayer {
    /// See [`PreparedLayer::companion`].
    companion: bool,
    /// Layer options with aliases normalised (`colour` -> `color`).
    options: Options,
    path: String,
    geom: &'static dyn Geom,
    data: LayerData,
    /// The data before the position adjustment (tooltip values).
    unadjusted: LayerData,
    /// Unmapped columns the tooltips read, one value per layer row.
    tooltip_columns: Vec<(String, Column)>,
    /// Range of the setup's x columns before the position adjustment (see
    /// [`Geom::setup_aesthetics`]).
    /// Whether the axes train on [`Self::unadjusted`]: a position that does not
    /// move the axes (nudge, as lets-plot) ran.
    trains_unadjusted: bool,
    x_span: Option<(f64, f64)>,
    /// The same for y (ridge heights).
    y_span: Option<(f64, f64)>,
    /// The data holds x and y swapped (see [`layer_orientation`]): drawn with the
    /// panel's flip toggled, trained on the other axis.
    y_oriented: bool,
    /// Where each mapped aesthetic was mapped, for error messages.
    mapping_paths: BTreeMap<Aesthetic, String>,
    /// `show_legend` (default: the geom's).
    show_legend: bool,
    /// Warnings of this stage.
    warnings: Vec<String>,
}

/// Last layer stage: style values from the trained style scales. Returns the
/// stage's warnings next to the layer.
fn style_layer(
    layer: ComputedLayer,
    style_scales: &BTreeMap<Aesthetic, StyleScale>,
    exponent_format: ExponentFormat,
) -> StageResult<(PreparedLayer, Vec<String>)> {
    let mut warnings = Vec::new();
    let parameters = Parameters::new(&layer.options, &layer.path);
    let styles = resolve_styles(
        layer.geom,
        &layer.data,
        style_scales,
        exponent_format,
        &parameters,
        &layer.path,
        &mut warnings,
    );
    let styles = match styles {
        Ok(styles) => styles,
        Err(error) => return Err((error, warnings)),
    };
    Ok((
        PreparedLayer {
            options: layer.options,
            path: layer.path,
            geom: layer.geom,
            data: layer.data,
            unadjusted: layer.unadjusted,
            tooltip_columns: layer.tooltip_columns,
            x_span: layer.x_span,
            y_oriented: layer.y_oriented,
            trains_unadjusted: layer.trains_unadjusted,
            y_span: layer.y_span,
            styles,
            show_legend: layer.show_legend,
            companion: layer.companion,
        },
        warnings,
    ))
}

/// Layer mapping over the inherited plot mapping (unless `inherit_aes=False`), and
/// which aesthetics came from the plot (for error paths).
fn merged_mapping(plot: &PlotSpec, layer: &LayerSpec) -> (Mapping, BTreeSet<Aesthetic>) {
    let mut mapping = if inherits_aes(layer) {
        plot.mapping.clone()
    } else {
        Mapping::new()
    };
    mapping.retain(|aesthetic, _| !layer.mapping.contains_key(aesthetic));
    let inherited = mapping.keys().copied().collect();
    mapping.extend(layer.mapping.clone());
    (mapping, inherited)
}

/// The `as_discrete` annotation of an aesthetic: the plot's for an inherited
/// mapping, else the layer's.
fn discrete_annotation<'a>(
    plot: &'a PlotSpec,
    layer: &'a LayerSpec,
    aesthetic: Aesthetic,
    inherited: bool,
) -> Option<&'a MappingAnnotation> {
    let meta = if inherited {
        &plot.data_meta
    } else {
        &layer.data_meta
    };
    meta.mapping_annotations
        .iter()
        .rev()
        .find(|annotation| annotation.aes == aesthetic && annotation.annotation == "as_discrete")
}

/// Whether the scales of a style aesthetic ask for `discrete=True`: options
/// merge over the scales (lets-plot), so the last scale that sets it decides.
fn forces_discrete(plot: &PlotSpec, aesthetic: Aesthetic) -> bool {
    plot.scales
        .iter()
        .rev()
        .filter(|scale| scale.aesthetic.as_slice().contains(&aesthetic))
        .find_map(|scale| scale.discrete)
        .unwrap_or(false)
}

/// Factor levels of a column (`as_discrete(levels=...)`, ordered categoricals):
/// the layer's annotation, else the plot's.
/// The date-time kind of a column from its series annotation; a time zone
/// other than UTC is reported (labels show UTC).
fn temporal_kind(
    plot: &PlotSpec,
    layer: &LayerSpec,
    column: &str,
    mapping_path: &str,
    warnings: &mut Vec<String>,
) -> Option<TemporalKind> {
    let annotation = [&layer.data_meta, &plot.data_meta]
        .into_iter()
        .flat_map(|meta| meta.series_annotations.iter())
        .find(|annotation| annotation.column == column && annotation.series_type.is_some())?;
    let kind = match annotation.series_type.as_deref()? {
        "datetime" => TemporalKind::DateTime,
        "date" => TemporalKind::Date,
        "time" => TemporalKind::TimeOfDay,
        _ => return None,
    };
    if let Some(zone) = &annotation.time_zone
        && !matches!(zone.as_str(), "UTC" | "Etc/UTC" | "GMT" | "Z")
    {
        warnings.push(format!(
            "at '{mapping_path}': column '{column}' is in time zone '{zone}'; ggrust labels \
             date-times in UTC (convert the column to UTC, or drop its time zone to label \
             wall-clock times)"
        ));
    }
    Some(kind)
}

fn factor_levels<'a>(
    plot: &'a PlotSpec,
    layer: &'a LayerSpec,
    column: &str,
) -> Option<&'a SeriesAnnotation> {
    [&layer.data_meta, &plot.data_meta]
        .into_iter()
        .flat_map(|meta| meta.series_annotations.iter())
        .find(|annotation| annotation.column == column && annotation.factor_levels.is_some())
}

/// Apply `as_discrete` (or factor levels) to a mapped column: make it discrete
/// and order its levels. Columns without an annotation pass through.
#[allow(clippy::too_many_arguments)]
fn annotated_column(
    plot: &PlotSpec,
    layer: &LayerSpec,
    frames: &LayerFrames<'_>,
    is_identity: bool,
    aesthetic: Aesthetic,
    inherited: bool,
    name: &str,
    column: Column,
    mapping_path: &str,
    warnings: &mut Vec<String>,
) -> Result<Column> {
    let annotation = discrete_annotation(plot, layer, aesthetic, inherited);
    let series = factor_levels(plot, layer, name);
    if annotation.is_none() && series.is_none() {
        // `scale_color_discrete()`: numbers become categories in first appearance.
        if aesthetic.kind().style().is_some() && forces_discrete(plot, aesthetic) {
            return Ok(as_discrete(&column, DiscreteOrder::default()));
        }
        return Ok(column);
    }
    let parameters = annotation.map(|annotation| &annotation.parameters);
    let order_by_name = parameters.and_then(|parameters| parameters.order_by.as_deref());
    let order_by = match order_by_name {
        // lets-plot orders by the variable in the stat's output; a data column is
        // gone there (bar, boxplot), so the levels sort by value.
        Some(_) if !is_identity => None,
        Some(order_by) if Computed::from_mapping(order_by).is_some() => {
            warnings.push(format!(
                "at '{mapping_path}': as_discrete(order_by='{order_by}') with a stat variable \
                 is not supported yet; the levels keep their order"
            ));
            None
        }
        Some(order_by) => match frames.column(order_by, mapping_path)? {
            Column::Continuous(values) => Some(values),
            Column::Discrete(_) => {
                return Err(Error::spec(
                    mapping_path,
                    format!(
                        "as_discrete(order_by='{order_by}') needs a numeric column; \
                         '{order_by}' is discrete. Order by a numeric column, or pass \
                         levels=[...] to set the order directly"
                    ),
                ));
            }
        },
        None => None,
    };
    let levels = series
        .and_then(|series| series.factor_levels.as_deref())
        .or_else(|| annotation.and_then(|annotation| annotation.levels.as_deref()));
    let order = series
        .and_then(|series| series.order)
        .or_else(|| parameters.and_then(|parameters| parameters.order))
        .or_else(|| (order_by_name.is_some() && order_by.is_none()).then_some(-1));
    Ok(as_discrete(
        &column,
        DiscreteOrder {
            levels,
            order_by: order_by.as_ref(),
            order,
        },
    ))
}

/// `inherit_aes`, defaulting to the geom's (false for `geom_bracket`).
fn inherits_aes(layer: &LayerSpec) -> bool {
    layer
        .inherit_aes
        .unwrap_or_else(|| geom::geom(&layer.geom).is_none_or(|geom| geom.inherits_aes()))
}

/// Explicit `group` columns with their mapping paths: the plot's (when inherited)
/// followed by the layer's, without repeats, as lets-plot combines them. `None`
/// when neither maps `group`; an empty list turns grouping off.
fn explicit_grouping(
    plot: &PlotSpec,
    layer: &LayerSpec,
    path: &str,
    warnings: &mut Vec<String>,
) -> Option<Vec<(String, String)>> {
    let sources = [
        (
            plot.mapping
                .get(&Aesthetic::Group)
                .filter(|_| inherits_aes(layer)),
            "mapping.group".to_owned(),
        ),
        (
            layer.mapping.get(&Aesthetic::Group),
            format!("{path}.mapping.group"),
        ),
    ];
    let mut names: Option<Vec<(String, String)>> = None;
    for (value, mapping_path) in sources {
        let columns: &[String] = match value {
            None => continue,
            Some(MappingValue::Column(name)) => std::slice::from_ref(name),
            Some(MappingValue::Columns(names)) => names,
            Some(other) => {
                warnings.push(format!(
                    "at '{mapping_path}': mapping 'group' to {} is not supported yet and \
                     was ignored",
                    describe_mapping_value(other)
                ));
                continue;
            }
        };
        let names = names.get_or_insert_with(Vec::new);
        for name in columns {
            if !names.iter().any(|(seen, _)| seen == name) {
                names.push((name.clone(), mapping_path.clone()));
            }
        }
    }
    names
}

/// Where a layer's columns come from, following lets-plot (`DataConfigUtil`): the
/// layer's own data first; when plot data has the same number of rows, its columns
/// fill in what the layer data lacks.
struct LayerFrames<'a> {
    layer: Option<&'a DataFrame>,
    plot: Option<&'a DataFrame>,
}

impl LayerFrames<'_> {
    /// Rows of the layer: its own data, else the plot data, else one row (so
    /// constant-only layers such as `geom_point(x=1, y=2)` draw once).
    fn row_count(&self) -> usize {
        self.layer.or(self.plot).map_or(1, DataFrame::row_count)
    }

    /// Whether [`Self::column`] finds the column.
    fn has(&self, name: &str) -> bool {
        let shared_plot = self.plot.filter(|plot| match self.layer {
            Some(layer) => plot.row_count() == layer.row_count(),
            None => true,
        });
        self.layer.is_some_and(|layer| layer.contains(name))
            || shared_plot.is_some_and(|plot| plot.contains(name))
    }

    fn column(&self, name: &str, path: &str) -> Result<Column> {
        let error = |message: String| Error::spec(path, message);
        let shared_plot = self.plot.filter(|plot| match self.layer {
            Some(layer) => plot.row_count() == layer.row_count(),
            None => true,
        });
        let source = match (self.layer, shared_plot) {
            (Some(layer), _) if layer.contains(name) => layer,
            (_, Some(plot)) if plot.contains(name) => plot,
            (None, None) => {
                return Err(error(format!(
                    "column '{name}' is mapped but the layer has no data; pass data to \
                     ggplot() or to the geom"
                )));
            }
            (Some(layer), _) => {
                let note = match self.plot {
                    Some(plot) if plot.contains(name) => format!(
                        "the plot data has it, but its {} rows differ from the layer \
                         data's {} rows",
                        plot.row_count(),
                        layer.row_count()
                    ),
                    _ => String::new(),
                };
                return layer.column(name).map_err(|missing| {
                    let missing = missing.nested(path);
                    if note.is_empty() {
                        missing
                    } else {
                        missing.with_notes(&[note])
                    }
                });
            }
            (None, Some(plot)) => plot,
        };
        source.column(name).map_err(|failure| failure.nested(path))
    }
}

/// lets-plot `ConfiguredStatContext.overallRange`: the range of `aesthetic` (x or
/// y) over every layer's mapped column (axis units), with finite scale limits
/// replacing its ends.
fn overall_range(
    layers: &[&MappedLayer],
    aesthetic: Aesthetic,
    scale: &PositionScale,
) -> Option<(f64, f64)> {
    let mut range: Option<(f64, f64)> = None;
    for (_, column, _) in layers
        .iter()
        .flat_map(|layer| &layer.columns)
        .filter(|(mapped, _, _)| *mapped == aesthetic)
    {
        let Ok(Column::Continuous(positions)) = scale.positions(column.clone()) else {
            continue;
        };
        for row in 0..positions.len() {
            if let Some(value) = positions.get(row).filter(|value| value.is_finite()) {
                range = Some(range.map_or((value, value), |(low, high)| {
                    (low.min(value), high.max(value))
                }));
            }
        }
    }
    match (range, scale.limits()) {
        (Some((low, high)), [low_limit, high_limit]) => {
            Some((low_limit.unwrap_or(low), high_limit.unwrap_or(high)))
        }
        (None, [Some(low), Some(high)]) => Some((low, high)),
        (None, _) => None,
    }
}

/// Finite range of the geom's setup columns on one axis.
fn setup_span(
    geom: &dyn Geom,
    data: &LayerData,
    on_axis: fn(Aesthetic) -> bool,
) -> Option<(f64, f64)> {
    let mut span: Option<(f64, f64)> = None;
    for aesthetic in geom.span_aesthetics() {
        let Some(column) = data.continuous(*aesthetic).filter(|_| on_axis(*aesthetic)) else {
            continue;
        };
        for row in 0..column.len() {
            if let Some(value) = column.get(row).filter(|value| value.is_finite()) {
                span = Some(span.map_or((value, value), |(low, high)| {
                    (low.min(value), high.max(value))
                }));
            }
        }
    }
    span
}

/// Apply a layer's positions. A later position (lets-plot `CompositionPos`, only
/// geom_jitter's own jitter) reads the original data, as lets-plot's resolution
/// does, and its offsets are added to the earlier result.
fn apply_positions(
    data: LayerData,
    positions: &[(&'static dyn Position, Options, String)],
) -> Result<LayerData> {
    let Some(((first, first_options, first_path), later)) = positions.split_first() else {
        return Ok(data);
    };
    let mut result = first.adjust(data.clone(), &Parameters::new(first_options, first_path))?;
    for (position, options, option_path) in later {
        let moved = position.adjust(data.clone(), &Parameters::new(options, option_path))?;
        let aesthetics: Vec<Aesthetic> = result
            .aesthetics()
            .filter(|aesthetic| aesthetic.is_x_position() || aesthetic.is_y_position())
            .collect();
        for aesthetic in aesthetics {
            let (Some(current), Some(before), Some(after)) = (
                result.continuous(aesthetic),
                data.continuous(aesthetic),
                moved.continuous(aesthetic),
            ) else {
                continue;
            };
            let shifted = (0..current.len())
                .map(|row| {
                    let offset = after.get(row).unwrap_or(0.0) - before.get(row).unwrap_or(0.0);
                    current.get(row).map(|value| value + offset)
                })
                .collect();
            result.insert(
                aesthetic,
                Column::Continuous(ContinuousColumn::from_options(shifted)),
            )?;
        }
    }
    Ok(result)
}

/// The positions of a layer, as lets-plot `LayerConfigUtil.positionAdjustmentOptions`:
/// the geom's (or stat's) preferred position without `position=`; both, the given
/// one first, when the geom has its own positioning options (`geom_jitter(width=)`);
/// one merged position when the names agree (`geom_jitter(seed=3,
/// position="jitter")`); else the given one.
fn layer_positions(
    geom: &dyn Geom,
    stat: &dyn Stat,
    spec: &LayerSpec,
    path: &str,
) -> Result<Vec<(&'static dyn Position, Options, String)>> {
    let preferred = stat
        .default_position()
        .unwrap_or_else(|| geom.default_position());
    let preferred_options = preferred.options(&spec.parameters);
    // lets-plot `hasOwnPositionAdjustmentOptions`: geom_jitter's width or height.
    let own = preferred.from_layer.iter().any(|name| {
        *name != "seed"
            && spec
                .parameters
                .get(*name)
                .is_some_and(|value| !value.is_null())
    });
    let given_path = format!("{path}.position");
    let chain: Vec<(&str, Options, String)> = match &spec.position {
        None => vec![(preferred.name, preferred_options, path.to_owned())],
        Some(given) if own => vec![
            (given.name.as_str(), given.parameters.clone(), given_path),
            (preferred.name, preferred_options, path.to_owned()),
        ],
        Some(given) if given.name == preferred.name => {
            let mut merged = preferred_options;
            merged.extend(given.parameters.clone());
            vec![(preferred.name, merged, given_path)]
        }
        Some(given) => vec![(given.name.as_str(), given.parameters.clone(), given_path)],
    };
    chain
        .into_iter()
        .map(|(name, mut options, option_path)| {
            // Only data rows: stats may repeat x within a group (ydensity, density).
            if geom.stacks_each_row()
                && stat.name() == "identity"
                && matches!(name, "stack" | "fill")
                && options.get("mode").is_none_or(|value| value.is_null())
            {
                options.insert("mode".to_owned(), Value::from("all"));
            }
            let position = position::position(name).ok_or_else(|| {
                let planned = position::NOT_APPLIED_YET.contains(&name);
                unknown(
                    "position",
                    &format!("{path}.position"),
                    name,
                    planned,
                    &position::names(),
                )
            })?;
            Ok((position, options, option_path))
        })
        .collect()
}

/// Error unless every aesthetic in `required` has a column (mapped, constant or
/// computed). `owner` names who needs them, e.g. `geom_bar`.
/// `y_oriented`: `data` holds x and y swapped (see [`layer_orientation`]); the
/// message names the user's aesthetics.
fn require_aesthetics(
    owner: &str,
    required: &[Aesthetic],
    data: &LayerData,
    path: &str,
    y_oriented: bool,
) -> Result<()> {
    let name = |aesthetic: &Aesthetic| match y_oriented {
        true => aesthetic.flip_orientation().as_str(),
        false => aesthetic.as_str(),
    };
    let missing: Vec<&str> = required
        .iter()
        .filter(|aesthetic| !data.contains(**aesthetic))
        .map(name)
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    let orientation = if y_oriented {
        " with orientation='y'"
    } else {
        ""
    };
    Err(Error::spec(
        path,
        format!(
            "{owner} requires aesthetics {}{orientation} (mapped to columns or given as \
         constants); missing: {}",
            required.iter().map(name).collect::<Vec<_>>().join(", "),
            missing.join(", ")
        ),
    ))
}

/// Whether the layer reads an aesthetic: the geom's (required positions, styles)
/// or the stat's (e.g. `weight`).
fn geom_uses(geom: &dyn Geom, stat: &dyn Stat, aesthetic: Aesthetic) -> bool {
    stat.aesthetics().contains(&aesthetic)
        || geom.required_aesthetics().contains(&aesthetic)
        || geom.optional_aesthetics().contains(&aesthetic)
        || geom
            .style_defaults()
            .iter()
            .any(|(style, _)| *style == aesthetic)
}

fn describe_mapping_value(value: &MappingValue) -> String {
    match value {
        MappingValue::Column(name) => format!("column '{name}'"),
        MappingValue::Columns(names) => format!("columns [{}]", names.join(", ")),
        MappingValue::Other(value) => format!("the value {value}"),
    }
}

/// Layer options with lets-plot's aliases resolved: `colour` and `col` -> `color`.
fn normalize_options(options: &Options, path: &str, warnings: &mut Vec<String>) -> Options {
    let mut normalized = Options::new();
    for (name, value) in options {
        let canonical = match name.as_str() {
            "colour" | "col" => "color",
            other => other,
        };
        let value = justification_number(canonical, value).unwrap_or_else(|| value.clone());
        if normalized.insert(canonical.to_owned(), value).is_some() {
            warnings.push(format!(
                "at '{path}.{name}': '{canonical}' is given twice (as an alias too); the \
                 value of '{name}' was used"
            ));
        }
    }
    normalized
}

/// `hjust`/`vjust` given by name, as lets-plot accepts them: `left`/`bottom` 0,
/// `middle`/`center` 0.5, `right`/`top` 1. Other values are left as they are.
fn justification_number(name: &str, value: &Value) -> Option<Value> {
    let Value::String(text) = value else {
        return None;
    };
    let number = match (name, text.as_str()) {
        ("hjust", "left") | ("vjust", "bottom") => 0.0,
        ("hjust" | "vjust", "middle" | "center") => 0.5,
        ("hjust", "right") | ("vjust", "top") => 1.0,
        _ => return None,
    };
    Some(Value::from(number))
}

/// Warn about position options the position does not read.
fn validate_position_options(
    position: &dyn position::Position,
    options: &Options,
    path: &str,
    warnings: &mut Vec<String>,
) {
    for name in options.keys() {
        if !position.parameters().contains(&name.as_str()) {
            let supported = position.parameters().join(", ");
            warnings.push(format!(
                "at '{path}.{name}': position_{} does not support option '{name}'; it was \
                 ignored. Supported: {}",
                position.name(),
                if supported.is_empty() {
                    "none"
                } else {
                    &supported
                }
            ));
        }
    }
}

/// Warn about options neither the geom nor the stat reads.
fn validate_parameters(
    geom: &dyn Geom,
    stat: &dyn stat::Stat,
    options: &Options,
    path: &str,
    warnings: &mut Vec<String>,
) {
    let accepted: BTreeSet<&str> = geom
        .parameters()
        .iter()
        .copied()
        .chain(stat.parameters().iter().copied())
        .chain(
            geom.style_defaults()
                .iter()
                .map(|(aesthetic, _)| aesthetic.as_str()),
        )
        // Every layer: see `layer_orientation`.
        .chain(["orientation"])
        // Constant positions: geom_point(x=1.5).
        .chain(
            geom.required_aesthetics()
                .iter()
                .map(|aesthetic| aesthetic.as_str()),
        )
        .collect();
    for name in options.keys() {
        if !accepted.contains(name.as_str()) {
            warnings.push(format!(
                "at '{path}.{name}': geom_{} does not support option '{name}' (yet); it was \
                 ignored. Supported: {}",
                geom.name(),
                accepted.iter().copied().collect::<Vec<_>>().join(", ")
            ));
        }
    }
}

/// Style scales of mapped style aesthetics, trained over every layer (mapper and
/// domain from the merged `scale_*()` calls, else lets-plot's defaults).
fn train_style_scales(
    layers: &[&ComputedLayer],
    specs: &BTreeMap<Aesthetic, StyleScaleSpec>,
    warnings: &mut Vec<String>,
) -> Result<BTreeMap<Aesthetic, StyleScale>> {
    // Every (aesthetic, column, path) after stat and position, in layer order.
    let columns: Vec<(Aesthetic, &Column, &str)> = layers
        .iter()
        .flat_map(|layer| {
            layer.data.aesthetics().filter_map(|aesthetic| {
                let path = layer
                    .mapping_paths
                    .get(&aesthetic)
                    .map_or(layer.path.as_str(), String::as_str);
                Some((aesthetic, layer.data.get(aesthetic)?, path))
            })
        })
        .collect();
    // lets-plot: one discrete column makes the aesthetic's scale discrete.
    let discrete: BTreeSet<Aesthetic> = columns
        .iter()
        .filter(|(_, column, _)| matches!(column, Column::Discrete(_)))
        .map(|(aesthetic, _, _)| *aesthetic)
        .collect();
    let default_spec = StyleScaleSpec::default();
    let mut scales: BTreeMap<Aesthetic, StyleScale> = BTreeMap::new();
    for (aesthetic, column, path) in columns {
        if default_mapper(aesthetic).is_none() {
            continue; // reported by `resolve_styles`
        }
        let scale = match scales.entry(aesthetic) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::btree_map::Entry::Vacant(entry) => {
                let spec = specs.get(&aesthetic).unwrap_or(&default_spec);
                let mapper = spec.mapper(aesthetic, warnings)?;
                let domain = spec.discrete_domain()?;
                let scale = if discrete.contains(&aesthetic) {
                    StyleScale::discrete(mapper, domain)
                } else {
                    StyleScale::new(mapper, domain)
                };
                entry.insert(scale.with_continuous(spec.continuous_domain()))
            }
        };
        scale
            .train(column)
            .map_err(|message| Error::spec(path, message))?;
    }
    Ok(scales)
}

/// The legend entries of one aesthetic.
type AestheticEntries = (Aesthetic, Vec<LegendEntry>);

/// Discrete legends, lets-plot `PlotAssemblerUtil.createLegends`: per layer, the
/// mapped style aesthetics with a discrete scale are grouped by scale name (the
/// legend title); legends with the same title merge, and so do keys with the same
/// label (one key part per layer).
fn assemble_legends(
    spec: &PlotSpec,
    layers: &[&PreparedLayer],
    scales: &BTreeMap<Aesthetic, StyleScale>,
    specs: &BTreeMap<Aesthetic, StyleScaleSpec>,
) -> Result<Vec<LegendBox>> {
    /// lets-plot `LegendAssembler.MAX_LEGEND_LABELS`: bigger scales get no legend.
    const MAX_LEGEND_LABELS: usize = 200;
    let mut legends: Vec<Legend> = Vec::new();
    let mut colorbars: Vec<Colorbar> = Vec::new();
    for layer in layers.iter().filter(|layer| layer.show_legend) {
        // Scale name -> (aesthetic, its legend entries), in the geom's rendered order.
        let mut by_title: Vec<(String, Vec<AestheticEntries>, KeyGrid, Overrides)> = Vec::new();
        let mapped: Vec<Aesthetic> = layer.data.aesthetics().collect();
        let rendered = layer
            .geom
            .style_defaults()
            .iter()
            .map(|(aesthetic, _)| *aesthetic);
        for aesthetic in rendered.filter(|aesthetic| mapped.contains(aesthetic)) {
            let Some(scale) = scales.get(&aesthetic) else {
                continue;
            };
            let guides = Guides::of(spec, aesthetic, specs.get(&aesthetic));
            if guides.hidden() {
                continue;
            }
            let title = guides
                .title()
                .unwrap_or_else(|| legend_title(spec, aesthetic, specs.get(&aesthetic)));
            let fits_colorbar = scale.continuous_guide().is_some()
                && aesthetic.kind().style() == Some(crate::style::ValueKind::Color);
            if guides.named("colorbar") && !fits_colorbar {
                // lets-plot `checkFitsColorBar`.
                return Err(Error::spec(
                    format!("guides.{aesthetic}"),
                    format!(
                        "a colorbar needs a continuous color or fill \
                     scale; '{aesthetic}' is {}",
                        if scale.is_discrete() {
                            "discrete"
                        } else {
                            "not a color"
                        }
                    ),
                ));
            }
            // lets-plot `fitsColorBar`: continuous colors get a colorbar unless a
            // legend is asked for.
            if let Some(guide) = scale.continuous_guide()
                && fits_colorbar
                && !guides.named("legend")
            {
                let options = guides.colorbar_options(aesthetic)?;
                let bins = options.bins;
                let StyleValues::Colors(colors) =
                    scale.map_numbers(&Colorbar::bin_values(guide.domain, bins))
                else {
                    continue;
                };
                let colors = (0..bins.max(2)).map(|index| colors.get(index)).collect();
                let colorbar = Colorbar {
                    title,
                    domain: guide.domain,
                    breaks: guide.breaks,
                    colors,
                    bar_width: options.bar_width,
                    bar_height: options.bar_height,
                };
                // lets-plot: an equal colorbar under the same title is shown once; a
                // different one gets "title (aesthetic)", which replaces an earlier
                // one of that name (a map keyed by title).
                match colorbars.iter().find(|known| known.title == colorbar.title) {
                    Some(known) if same_colorbar(known, &colorbar) => {}
                    Some(_) => {
                        let renamed = Colorbar {
                            title: format!("{} ({aesthetic})", colorbar.title),
                            ..colorbar
                        };
                        match colorbars
                            .iter_mut()
                            .find(|known| known.title == renamed.title)
                        {
                            Some(known) => *known = renamed,
                            None => colorbars.push(renamed),
                        }
                    }
                    None => colorbars.push(colorbar),
                }
                continue;
            }
            let Some(entries) = scale.legend_entries() else {
                continue;
            };
            if entries.len() > MAX_LEGEND_LABELS {
                continue;
            }
            let grid = guides.legend_grid()?;
            let overrides = guides.override_aes()?;
            match by_title.iter_mut().find(|(known, _, _, _)| *known == title) {
                Some((_, group, known, known_overrides)) => {
                    group.push((aesthetic, entries));
                    *known = known.combine(grid);
                    // lets-plot: the legend's maps merge; a later aesthetic's value wins.
                    for (key, values) in overrides {
                        known_overrides.retain(|(known, _)| *known != key);
                        known_overrides.push((key, values));
                    }
                }
                None => by_title.push((title, vec![(aesthetic, entries)], grid, overrides)),
            }
        }
        for (title, mut group, grid, overrides) in by_title {
            apply_override_aes(&mut group, &overrides);
            let index = match legends.iter().position(|legend| legend.title == title) {
                Some(index) => {
                    legends[index].grid = legends[index].grid.combine(grid);
                    index
                }
                None => {
                    legends.push(Legend {
                        title,
                        keys: Vec::new(),
                        grid,
                    });
                    legends.len() - 1
                }
            };
            // lets-plot `applyOverrideAesLists`: labels of every aesthetic, distinct;
            // per label, each aesthetic's last value with that label.
            let mut labels: Vec<&Label> = Vec::new();
            for (_, entries) in &group {
                for entry in entries {
                    if !labels.contains(&&entry.label) {
                        labels.push(&entry.label);
                    }
                }
            }
            for label in labels {
                let values: Vec<(Aesthetic, &StyleValues)> = group
                    .iter()
                    .filter_map(|(aesthetic, entries)| {
                        let entry = entries.iter().rev().find(|entry| entry.label == *label)?;
                        Some((*aesthetic, &entry.value))
                    })
                    .collect();
                let key_layer = KeyLayer {
                    geom: layer.geom,
                    styles: key_styles(layer, &values)?,
                };
                let keys = &mut legends[index].keys;
                match keys.iter_mut().find(|key| key.label == *label) {
                    Some(key) => key.layers.push(key_layer),
                    None => keys.push(LegendKey {
                        label: label.clone(),
                        layers: vec![key_layer],
                    }),
                }
            }
        }
    }
    Ok(colorbars
        .into_iter()
        .map(LegendBox::Colorbar)
        .chain(legends.into_iter().map(LegendBox::Keys))
        .collect())
}

/// `guide_legend(override_aes=...)` values per aesthetic: one per key position,
/// `None` keeps the scale's value.
type Overrides = Vec<(Aesthetic, Vec<Option<StyleValues>>)>;

/// lets-plot `processOverrideAesValues`: override values apply by key position
/// within each aesthetic's entries (a list shorter than the keys repeats its last
/// value); an aesthetic the legend does not map gets entries for the first
/// aesthetic's labels.
fn apply_override_aes(group: &mut Vec<AestheticEntries>, overrides: &Overrides) {
    let longest = group
        .iter()
        .map(|(_, entries)| entries.len())
        .max()
        .unwrap_or(0);
    for (aesthetic, values) in overrides {
        let mut values = values.clone();
        if let Some(last) = values.last().cloned() {
            values.resize(longest.max(values.len()), last);
        }
        if let Some((_, entries)) = group.iter_mut().find(|(known, _)| known == aesthetic) {
            for (entry, value) in entries.iter_mut().zip(&values) {
                if let Some(value) = value {
                    entry.value = value.clone();
                }
            }
        } else if let Some((_, first)) = group.first() {
            let entries = first
                .iter()
                .zip(&values)
                .filter_map(|(entry, value)| {
                    Some(LegendEntry {
                        label: entry.label.clone(),
                        value: value.clone()?,
                    })
                })
                .collect();
            group.push((*aesthetic, entries));
        }
    }
}

fn same_colorbar(left: &Colorbar, right: &Colorbar) -> bool {
    left.domain == right.domain
        && left.breaks == right.breaks
        && left.colors == right.colors
        && left.bar_width == right.bar_width
        && left.bar_height == right.bar_height
}

/// lets-plot has no limit; this keeps a typo from allocating billions of segments.
const MAX_COLORBAR_BINS: u32 = 10_000;

/// Resolved `guide_colorbar()` options.
struct ColorbarOptions {
    bins: usize,
    bar_width: Option<f64>,
    bar_height: Option<f64>,
}

/// The guides of one aesthetic: `guides(...)` first, then the scale's own `guide`.
struct Guides<'a> {
    entries: Vec<(&'a GuideSpec, String)>,
}

impl<'a> Guides<'a> {
    fn of(spec: &'a PlotSpec, aesthetic: Aesthetic, scale: Option<&'a StyleScaleSpec>) -> Self {
        let mut entries = Vec::new();
        if let Some(guide) = spec.guides.get(aesthetic.as_str()) {
            entries.push((guide, format!("guides.{aesthetic}")));
        }
        if let Some((guide, path)) = scale.and_then(StyleScaleSpec::guide_with_path) {
            entries.push((guide, path.to_owned()));
        }
        Guides { entries }
    }

    /// `"none"` anywhere hides the legend.
    fn hidden(&self) -> bool {
        self.entries
            .iter()
            .any(|(guide, _)| matches!(guide, GuideSpec::Name(name) if name == "none"))
    }

    /// Whether the first guide with a name (`"legend"`, `guide_colorbar()`, ...) is `name`.
    fn named(&self, name: &str) -> bool {
        let named = self.entries.iter().find_map(|(guide, _)| match guide {
            GuideSpec::Name(given) => Some(given.as_str()),
            GuideSpec::Full(options) => options.name.as_deref(),
        });
        named == Some(name)
    }

    fn options(&self) -> impl Iterator<Item = (&'a GuideOptions, &str)> {
        self.entries.iter().filter_map(|(guide, path)| match guide {
            GuideSpec::Full(options) => Some((&**options, path.as_str())),
            GuideSpec::Name(_) => None,
        })
    }

    fn title(&self) -> Option<String> {
        self.options()
            .find_map(|(options, _)| options.title.clone())
    }

    /// `guide_legend(ncol, nrow, byrow)`, combined as lets-plot `LegendOptions.combine`
    /// does over the scale's guide, then `guides()`: the last count given wins
    /// (`guides()` over the scale), `byrow` from any.
    fn legend_grid(&self) -> Result<KeyGrid> {
        let mut grid = KeyGrid::default();
        let options: Vec<_> = self.options().collect();
        for (options, path) in options.into_iter().rev() {
            let count = |value: Option<u32>, key: &str| -> Result<Option<usize>> {
                match value {
                    Some(0) => Err(Error::spec(
                        format!("{path}.{key}"),
                        "expected a positive count, got 0",
                    )),
                    value => Ok(value.map(|value| value as usize)),
                }
            };
            grid = grid.combine(KeyGrid {
                columns: count(options.ncol, "ncol")?,
                rows: count(options.nrow, "nrow")?,
                by_row: options.byrow.unwrap_or(false),
            });
        }
        Ok(grid)
    }

    /// `guide_legend(override_aes=...)`: the first given, each value parsed as its
    /// aesthetic's kind. A list gives one value per key (`None` keeps the key's
    /// value); an empty list overrides nothing.
    fn override_aes(&self) -> Result<Overrides> {
        let Some((overrides, path)) = self
            .options()
            .find_map(|(options, path)| Some((options.override_aes.as_ref()?, path)))
        else {
            return Ok(Vec::new());
        };
        let mut parsed = Vec::new();
        for (key, value) in overrides {
            let at = format!("{path}.override_aes.{key}");
            let (aesthetic, kind) = match key.parse::<Aesthetic>() {
                Ok(aesthetic) => match aesthetic.kind().style() {
                    Some(kind) => (aesthetic, kind),
                    None => {
                        return Err(Error::spec(
                            &at,
                            format!(
                                "'{key}' is not a style aesthetic; legend keys take                              color, fill, size, shape, alpha, ..."
                            ),
                        ));
                    }
                },
                Err(_) => {
                    return Err(Error::spec(&at, format!("unknown aesthetic '{key}'")));
                }
            };
            let parse = |item: &Value| -> Result<Option<StyleValues>> {
                if item.is_null() {
                    return Ok(None);
                }
                parse_constant(kind, item)
                    .map(Some)
                    .map_err(|message| Error::spec(&at, message))
            };
            let values = match value {
                Value::Array(items) if items.is_empty() => vec![None],
                Value::Array(items) => items.iter().map(parse).collect::<Result<_>>()?,
                item => vec![parse(item)?],
            };
            parsed.retain(|(known, _)| *known != aesthetic);
            parsed.push((aesthetic, values));
        }
        Ok(parsed)
    }

    /// `guide_colorbar(barwidth, barheight, nbin)`, validated. As lets-plot
    /// (`GuideOptionsList.getColorBarOptions`), one colorbar guide gives all of them:
    /// `guides()` over the scale's own guide, never mixed.
    fn colorbar_options(&self, aesthetic: Aesthetic) -> Result<ColorbarOptions> {
        let Some((options, path)) = self
            .options()
            .find(|(options, _)| options.name.as_deref() == Some("colorbar"))
        else {
            return Ok(ColorbarOptions {
                bins: DEFAULT_BINS,
                bar_width: None,
                bar_height: None,
            });
        };
        let size = |value: Option<f64>, key: &str| -> Result<Option<f64>> {
            match value {
                None => Ok(None),
                Some(value) if value.is_finite() && value > 0.0 => Ok(Some(value)),
                Some(value) => Err(Error::spec(
                    format!("{path}.{key}"),
                    format!("expected a positive size in pixels, got {value}"),
                )),
            }
        };
        let bins = match options.nbin {
            None => DEFAULT_BINS,
            Some(bins) if bins <= MAX_COLORBAR_BINS => bins as usize,
            Some(bins) => {
                return Err(Error::spec(
                    format!("{path}.nbin"),
                    format!(
                        "at most {MAX_COLORBAR_BINS} bins, got {bins} \
                     ('{aesthetic}' colorbar)"
                    ),
                ));
            }
        };
        Ok(ColorbarOptions {
            bins,
            bar_width: size(options.barwidth, "barwidth")?,
            bar_height: size(options.barheight, "barheight")?,
        })
    }
}

/// One-item styles of a legend key: the legend's values, else the layer's constant
/// shape / color / fill (lets-plot derives only these from constants), else the
/// geom's legend default, else its default.
fn key_styles(layer: &PreparedLayer, values: &[(Aesthetic, &StyleValues)]) -> Result<Styles> {
    let parameters = Parameters::new(&layer.options, &layer.path);
    let mut styles = Styles::default();
    for &(aesthetic, default) in layer.geom.style_defaults() {
        let value = if let Some((_, value)) = values.iter().find(|(known, _)| *known == aesthetic) {
            (*value).clone()
        } else if let Some(constant) = parameters.value(aesthetic.as_str()).filter(|_| {
            matches!(
                aesthetic,
                Aesthetic::Shape | Aesthetic::Color | Aesthetic::Fill
            )
        }) {
            parse_constant(default.kind(), constant)
                .map_err(|message| Error::spec(parameters.path_of(aesthetic.as_str()), message))?
        } else if let Some((_, legend_default)) = layer
            .geom
            .legend_defaults()
            .iter()
            .find(|(known, _)| *known == aesthetic)
        {
            legend_default.uniform()
        } else {
            default.uniform()
        };
        styles.insert(aesthetic, value)?;
    }
    Ok(styles)
}

/// Legend title without a guide title: the scale `name`, then lets-plot's default
/// scale name.
fn legend_title(spec: &PlotSpec, aesthetic: Aesthetic, scale: Option<&StyleScaleSpec>) -> String {
    scale
        .and_then(|scale| scale.name.clone())
        .unwrap_or_else(|| default_scale_name(spec, aesthetic, &[]))
}

/// Style values per aesthetic: mapped column (through its style scale) -> constant
/// option -> geom default.
fn resolve_styles(
    geom: &dyn Geom,
    data: &LayerData,
    style_scales: &BTreeMap<Aesthetic, StyleScale>,
    exponent_format: ExponentFormat,
    parameters: &Parameters,
    path: &str,
    warnings: &mut Vec<String>,
) -> Result<Styles> {
    let mut styles = Styles::default();
    for &(aesthetic, default) in geom.style_defaults() {
        let mapped = data.get(aesthetic);
        let values = match (mapped, style_scales.get(&aesthetic)) {
            // Text labels are the column's values, not a scale's output.
            (Some(column), _) if aesthetic == Aesthetic::Label => {
                label_texts(column, exponent_format)
            }
            (Some(column), Some(scale)) => scale
                .map(column)
                .map_err(|message| Error::spec(format!("{path}.mapping.{aesthetic}"), message))?,
            // Line coefficients are drawn as they are (lets-plot has no scale for them).
            (Some(Column::Continuous(column)), None)
                if matches!(aesthetic, Aesthetic::Intercept | Aesthetic::Slope) =>
            {
                let values = (0..column.len())
                    .map(|row| column.get(row).unwrap_or(f64::NAN))
                    .collect();
                StyleValues::Numbers(PerItem::Varying(values))
            }
            // Justification is drawn as it is (lets-plot has no scale for it, #1143).
            (Some(column), None) if matches!(aesthetic, Aesthetic::Hjust | Aesthetic::Vjust) => {
                mapped_justification(aesthetic, column, path, warnings)
            }
            _ => {
                // Only mappings that reached the data; skipped ones were reported already.
                if mapped.is_some() {
                    warnings.push(format!(
                        "at '{path}.mapping.{aesthetic}': mapping data to '{aesthetic}' is not \
                         supported yet; the constant or default value was used"
                    ));
                }
                match parameters.value(aesthetic.as_str()) {
                    Some(value) => parse_constant(default.kind(), value).map_err(|message| {
                        Error::spec(parameters.path_of(aesthetic.as_str()), message)
                    })?,
                    None => default.uniform(),
                }
            }
        };
        styles.insert(aesthetic, values)?;
    }
    Ok(styles)
}

/// Per-row `hjust`/`vjust` of a mapped column (lets-plot #1143): numbers as they
/// are, names as `justification_number` reads them (case-sensitive, as
/// lets-plot), numbers in text (`"0.2"`) parsed. Missing or infinite values and unknown names
/// centre the text (0.5); lets-plot centres unknown names too and fails on
/// missing values. Unknown names are reported once per layer.
fn mapped_justification(
    aesthetic: Aesthetic,
    column: &Column,
    path: &str,
    warnings: &mut Vec<String>,
) -> StyleValues {
    const CENTRE: f64 = 0.5;
    let values = match column {
        Column::Continuous(column) => (0..column.len())
            .map(|row| {
                column
                    .get(row)
                    .filter(|value| value.is_finite())
                    .unwrap_or(CENTRE)
            })
            .collect(),
        Column::Discrete(column) => {
            let mut unknown: Vec<&str> = Vec::new();
            let values = (0..column.len())
                .map(|row| {
                    let Some(label) = column.label(row) else {
                        return CENTRE;
                    };
                    let named =
                        justification_number(aesthetic.as_str(), &Value::String(label.to_owned()))
                            .and_then(|value| value.as_f64());
                    named
                        .or_else(|| label.trim().parse::<f64>().ok().filter(|v| v.is_finite()))
                        .unwrap_or_else(|| {
                            if !unknown.contains(&label) {
                                unknown.push(label);
                            }
                            CENTRE
                        })
                })
                .collect();
            if !unknown.is_empty() {
                let accepted = if aesthetic == Aesthetic::Hjust {
                    "'left', 'middle', 'center', 'right'"
                } else {
                    "'bottom', 'middle', 'center', 'top'"
                };
                let shown: Vec<String> = unknown
                    .iter()
                    .take(5)
                    .map(|label| format!("'{label}'"))
                    .collect();
                warnings.push(format!(
                    "at '{path}.mapping.{aesthetic}': unknown {aesthetic} value(s) {}{}; \
                     those texts were centred (0.5). Accepted: numbers or {accepted}",
                    shown.join(", "),
                    if unknown.len() > 5 { ", ..." } else { "" },
                ));
            }
            values
        }
    };
    StyleValues::Numbers(PerItem::Varying(values))
}

/// A column's values as label texts: levels as they are, numbers formatted as
/// lets-plot does.
fn label_texts(column: &Column, exponent_format: ExponentFormat) -> StyleValues {
    // lets-plot formats numeric labels as `,~g` with powers of ten (`1.23457·10⁸`)
    // and writes missing values as `n/a`.
    const MISSING: &str = "n/a";
    let texts = match column {
        Column::Discrete(column) => (0..column.len())
            .map(|row| column.label(row).unwrap_or(MISSING).to_owned())
            .collect(),
        Column::Continuous(column) => {
            let format =
                NumberFormat::parse(",~g").map(|format| format.with_exponent(exponent_format));
            (0..column.len())
                .map(|row| match (column.get(row), &format) {
                    (Some(value), Some(format)) if value.is_finite() => format.apply(value),
                    (Some(value), None) if value.is_finite() => value.to_string(),
                    _ => MISSING.to_owned(),
                })
                .collect()
        }
    };
    StyleValues::Texts(PerItem::Varying(texts))
}

/// Warn when rows cannot be used because a required aesthetic is missing.
/// `owner` names who needs them, e.g. `geom_point`. With `infinite_x_drawn`
/// (padded steps), x = -inf / +inf counts as usable.
fn report_missing_values(
    owner: &str,
    required: &[Aesthetic],
    infinite_x_drawn: bool,
    data: &LayerData,
    path: &str,
    warnings: &mut Vec<String>,
) {
    let required: Vec<_> = required
        .iter()
        .filter_map(|aesthetic| Some((*aesthetic, data.continuous(*aesthetic)?)))
        .collect();
    let removed = (0..data.row_count())
        .filter(|&row| {
            required.iter().any(|(aesthetic, column)| {
                let usable = if infinite_x_drawn && *aesthetic == Aesthetic::X {
                    |value: f64| !value.is_nan()
                } else {
                    f64::is_finite
                };
                !column.get(row).is_some_and(usable)
            })
        })
        .count();
    if removed > 0 {
        warnings.push(format!(
            "at '{path}': removed {removed} rows containing missing, non-finite or out-of-limits values \
             ({owner}); pass na_rm=True to silence"
        ));
    }
}

fn resolve_coord(spec: &PlotSpec, warnings: &mut Vec<String>) -> Result<&'static dyn Coord> {
    // lets-plot: the first layer whose geom prefers a coord (rasters: fixed);
    // `coord_flip()` flips that default.
    let preferred = spec
        .layers
        .iter()
        .find_map(|layer| geom::geom(&layer.geom)?.default_coord())
        .and_then(coord::coord);
    let Some(coord_spec) = &spec.coord else {
        return Ok(preferred.unwrap_or(&coord::Cartesian));
    };
    if coord_spec.name == "flip"
        && let Some(preferred) = preferred
    {
        return Ok(preferred);
    }
    match coord::coord(&coord_spec.name) {
        Some(coord) => Ok(coord),
        None => {
            warnings.push(format!(
                "at 'coord': coord_{} is not supported yet; coord_cartesian was used. \
                 Supported: {}",
                coord_spec.name,
                coord::names().join(", ")
            ));
            Ok(&coord::Cartesian)
        }
    }
}

/// `planned`: a lets-plot name that ggrust accepts in its API but does not support yet.
fn unknown(kind: &str, path: &str, name: &str, planned: bool, supported: &[&str]) -> Error {
    let (problem, suggestion) = if planned {
        (
            format!("{kind} '{name}' is not supported by ggrust yet"),
            String::new(),
        )
    } else {
        (
            format!("unknown {kind} '{name}'"),
            crate::suggest::did_you_mean(name, supported.iter().copied()),
        )
    };
    Error::spec(
        path,
        format!("{problem}; supported: {}{suggestion}", supported.join(", ")),
    )
}

/// Parsed features that the builder does not implement yet. Each is reported, never
/// silently dropped. Remove an entry when its phase (plan.md) implements it.
fn report_unimplemented(spec: &PlotSpec, warnings: &mut Vec<String>) {
    for (key, guide) in &spec.guides {
        // Built: axis titles, legend titles, `"none"` and `"legend"` for style
        // aesthetics, legend grids and colorbar sizes (see `is_built_style_guide`).
        let is_style = key
            .parse::<Aesthetic>()
            .is_ok_and(|aesthetic| default_mapper(aesthetic).is_some());
        let built = is_axis_title_only(key, guide) || (is_style && guide.is_built_style_guide());
        if !built {
            warnings.push(format!(
                "at 'guides.{key}': this guide option is not supported yet; ignored"
            ));
        }
    }
    if let Some(coord) = &spec.coord
        && coord.ratio.is_some()
        && coord.name != "fixed"
    {
        warnings.push(format!(
            "at 'coord.ratio': only coord_fixed has a ratio; ignored for coord_{}",
            coord.name
        ));
    }
    // facet_grid levels sort ascending (lets-plot's default order) for now.
    if let Some(facet) = spec.facet.as_ref().filter(|facet| facet.name == "grid") {
        for (key, order) in [("x_order", facet.x_order), ("y_order", facet.y_order)] {
            if order.is_some_and(|order| order != 1) {
                warnings.push(format!(
                    "at 'facet.{key}': only the ascending order (1) is supported yet; \
                     the levels are sorted ascending"
                ));
            }
        }
    }
    for (index, layer) in spec.layers.iter().enumerate() {
        let mut layer_ignored = |present: bool, feature: &str| {
            if present {
                warnings.push(format!(
                    "at 'layers[{index}]': '{feature}' is not supported yet and was ignored"
                ));
            }
        };
        layer_ignored(layer.labels.is_some(), "labels");
        layer_ignored(layer.manual_key.is_some(), "manual_key");
    }
    for path in spec.unsupported_options() {
        warnings.push(format!("unsupported option '{path}' was ignored"));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow_array::{ArrayRef, Float64Array, RecordBatch, StringArray};

    use super::*;
    use crate::scale::{ContinuousPositionScale, Transform};
    use crate::spec::Figure;

    fn prepare_with(
        spec: &PlotSpec,
        [x_scale, y_scale]: [&PositionScale; 2],
        warnings: &mut Vec<String>,
    ) -> Result<PreparedLayer> {
        let mut mapped =
            map_layer(0, &spec.layers[0], spec, &cells(), &[]).map_err(|(error, _)| error)?;
        warnings.append(&mut mapped.warnings);
        let mut computed = compute_layer(mapped, [x_scale, y_scale], &StatContext::default())?;
        warnings.append(&mut computed.warnings);
        let (layer, mut layer_warnings) =
            style_layer(computed, &BTreeMap::new(), ExponentFormat::default())
                .map_err(|(error, _)| error)?;
        warnings.append(&mut layer_warnings);
        Ok(layer)
    }

    fn prepare(spec: &PlotSpec, warnings: &mut Vec<String>) -> Result<PreparedLayer> {
        let scale = PositionScale::Continuous(ContinuousPositionScale::default());
        prepare_with(spec, [&scale, &scale], warnings)
    }

    fn plot(json: &str) -> PlotSpec {
        match Figure::from_json(json).unwrap() {
            Figure::Plot(plot) => *plot,
            Figure::Subplots(_) => panic!("expected a plot"),
        }
    }

    fn cells() -> PlotData {
        let columns: Vec<(&str, ArrayRef)> = vec![
            ("x", Arc::new(Float64Array::from(vec![1.0, 2.0, 3.0, 4.0]))),
            ("y", Arc::new(Float64Array::from(vec![1.0, 2.0, 3.0, 4.0]))),
            (
                "cluster",
                Arc::new(StringArray::from(vec!["T", "B", "T", "NK"])),
            ),
            (
                "sample",
                Arc::new(StringArray::from(vec!["s1", "s1", "s2", "s2"])),
            ),
            (
                "batch",
                Arc::new(Float64Array::from(vec![7.0, 7.0, 7.0, 8.0])),
            ),
        ];
        let batch = RecordBatch::try_from_iter(columns).unwrap();
        PlotData {
            plot: Some(DataFrame::new(batch.schema(), vec![batch]).unwrap()),
            layers: vec![],
        }
    }

    fn groups(json: &str) -> (Option<Vec<u32>>, Vec<String>) {
        let spec = plot(json);
        let mut warnings = Vec::new();
        let layer = prepare(&spec, &mut warnings).unwrap();
        (layer.data.group_index().map(<[u32]>::to_vec), warnings)
    }

    #[test]
    fn discrete_style_columns_group_by_default_positions_do_not() {
        let (index, _) = groups(
            r#"{"kind": "plot", "mapping": {"x": "x", "y": "y"}, "layers": [{"geom": "point"}]}"#,
        );
        assert_eq!(index, None);
        let (index, _) = groups(
            r#"{"kind": "plot", "mapping": {"x": "x", "y": "y", "color": "cluster"},
                "layers": [{"geom": "point"}]}"#,
        );
        assert_eq!(index, Some(vec![0, 1, 0, 2]));
        // Continuous style columns do not group.
        let (index, _) = groups(
            r#"{"kind": "plot", "mapping": {"x": "x", "y": "y", "size": "batch"},
                "layers": [{"geom": "point"}]}"#,
        );
        assert_eq!(index, None);
    }

    #[test]
    fn explicit_groups_combine_plot_and_layer_and_override_defaults() {
        let (index, warnings) = groups(
            r#"{"kind": "plot", "mapping": {"x": "x", "y": "y", "color": "cluster", "group": "sample"},
                "layers": [{"geom": "point", "mapping": {"group": "batch"}}]}"#,
        );
        assert!(
            warnings.iter().all(|warning| !warning.contains("group")),
            "{warnings:?}"
        );
        // (s1,7) (s1,7) (s2,7) (s2,8)
        assert_eq!(index, Some(vec![0, 0, 1, 2]));
        let (index, _) = groups(
            r#"{"kind": "plot", "mapping": {"x": "x", "y": "y", "color": "cluster", "group": "sample"},
                "layers": [{"geom": "point", "inherit_aes": false,
                            "mapping": {"x": "x", "y": "y", "group": []}}]}"#,
        );
        assert_eq!(index, None);
    }

    #[test]
    fn error_notes_keep_layer_order() {
        let spec = plot(
            r#"{"kind": "plot", "mapping": {"x": "x", "y": "y"},
                "layers": [{"geom": "point", "foo": 1},
                           {"geom": "point", "mapping": {"color": "nope"}, "bar": 2}]}"#,
        );
        let error = build_plot(&spec, &cells()).err().unwrap().to_string();
        let first = error.find("layers[0].foo").expect(&error);
        let second = error.find("layers[1].bar").expect(&error);
        assert!(first < second, "{error}");
    }

    #[test]
    fn unused_aesthetic_with_a_missing_column_only_warns() {
        let spec = plot(
            r#"{"kind": "plot", "mapping": {"x": "x", "y": "y"},
                "layers": [{"geom": "point", "mapping": {"linetype": "nope"}}]}"#,
        );
        let mut warnings = Vec::new();
        prepare(&spec, &mut warnings).unwrap();
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("does not use aesthetic 'linetype'")),
            "{warnings:?}"
        );
    }

    #[test]
    fn missing_group_column_names_the_path() {
        let spec = plot(
            r#"{"kind": "plot", "mapping": {"x": "x", "y": "y", "group": "nope"},
                "layers": [{"geom": "point"}]}"#,
        );
        let error = prepare(&spec, &mut Vec::new()).err().unwrap().to_string();
        assert!(
            error.contains("at 'mapping.group': column 'nope' not found"),
            "{error}"
        );
    }

    #[test]
    fn positions_outside_limits_or_the_transform_domain_become_missing() {
        let spec = plot(
            r#"{"kind": "plot", "mapping": {"x": "x", "y": "y"}, "layers": [{"geom": "point"}]}"#,
        );
        let mut limited = ContinuousPositionScale::default();
        limited.limits = [Some(2.0), None];
        let mut log = ContinuousPositionScale::default();
        log.transform = Transform::Log10;
        let (limited, log) = (
            PositionScale::Continuous(limited),
            PositionScale::Continuous(log),
        );
        let layer = prepare_with(&spec, [&limited, &log], &mut Vec::new()).unwrap();
        let x = layer.data.continuous(Aesthetic::X).unwrap();
        assert_eq!(
            (0..4).map(|row| x.get(row)).collect::<Vec<_>>(),
            [None, Some(2.0), Some(3.0), Some(4.0)]
        );
        let y = layer.data.continuous(Aesthetic::Y).unwrap();
        assert!((y.get(3).unwrap() - 4_f64.log10()).abs() < 1e-12);
    }

    #[test]
    fn mapped_justification_reads_numbers_and_names_per_row() {
        // lets-plot #1143: numbers as they are, names as numbers, missing and
        // unknown names centred (lets-plot fails on missing values).
        let columns: Vec<(&str, ArrayRef)> = vec![
            ("x", Arc::new(Float64Array::from(vec![1.0, 2.0, 3.0, 4.0]))),
            (
                "h",
                Arc::new(Float64Array::from(vec![
                    Some(0.0),
                    Some(-1.5),
                    None,
                    Some(f64::INFINITY),
                ])),
            ),
            (
                "v",
                Arc::new(StringArray::from(vec![
                    Some("top"),
                    Some("center"),
                    Some("TOP"),
                    None,
                ])),
            ),
            (
                "named",
                Arc::new(StringArray::from(vec!["left", "right", "middle", "0.2"])),
            ),
        ];
        let batch = RecordBatch::try_from_iter(columns).unwrap();
        let data = PlotData {
            plot: Some(DataFrame::new(batch.schema(), vec![batch]).unwrap()),
            layers: vec![],
        };
        let numbers = |json: &str| {
            let spec = plot(json);
            let mut mapped =
                map_layer(0, &spec.layers[0], &spec, &data, &[]).unwrap_or_else(|_| panic!());
            let scale = PositionScale::Continuous(ContinuousPositionScale::default());
            let mut warnings = std::mem::take(&mut mapped.warnings);
            let computed =
                compute_layer(mapped, [&scale, &scale], &StatContext::default()).unwrap();
            let (layer, layer_warnings) =
                style_layer(computed, &BTreeMap::new()).unwrap_or_else(|_| panic!());
            warnings.extend(layer_warnings);
            let read = |aesthetic| {
                let values = layer.styles.numbers(aesthetic).unwrap();
                (0..4).map(|row| values.get(row)).collect::<Vec<f64>>()
            };
            (read(Aesthetic::Hjust), read(Aesthetic::Vjust), warnings)
        };
        let (hjust, vjust, warnings) = numbers(
            r#"{"kind": "plot", "mapping": {"x": "x", "y": "x", "label": "x"},
                "layers": [{"geom": "text", "mapping": {"hjust": "h", "vjust": "v"}}]}"#,
        );
        assert_eq!(hjust, vec![0.0, -1.5, 0.5, 0.5]);
        assert_eq!(vjust, vec![1.0, 0.5, 0.5, 0.5]);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(
            warnings[0].contains("layers[0].mapping.vjust"),
            "{warnings:?}"
        );
        assert!(warnings[0].contains("'TOP'"), "{warnings:?}");
        assert!(warnings[0].contains("'bottom', 'middle', 'center', 'top'"));
        let (hjust, vjust, warnings) = numbers(
            r#"{"kind": "plot", "mapping": {"x": "x", "y": "x", "label": "x"},
                "layers": [{"geom": "label", "mapping": {"hjust": "named"}, "vjust": 0}]}"#,
        );
        assert_eq!(hjust, vec![0.0, 1.0, 0.5, 0.2]);
        assert_eq!(vjust, vec![0.0; 4]);
        assert!(warnings.is_empty(), "{warnings:?}");
    }
}
