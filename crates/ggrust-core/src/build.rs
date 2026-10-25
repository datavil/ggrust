//! Plot builder: typed spec + Arrow data -> [`Scene`].
//!
//! Pipeline per layer: look up geom/stat/position, validate options, map columns
//! to aesthetics, run stat and position, then train position scales across layers,
//! lay out the panel, resolve styles and let each geom draw.
//!
//! Features parsed by `spec` but not built yet are reported as warnings by
//! [`report_unimplemented`]; remove an entry there when its phase lands.

use std::collections::{BTreeMap, BTreeSet};

use crate::coord::{self, Coord, Panel};
use crate::data::{Column, DataFrame, LayerData};
use crate::geom::{self, DrawLayer, Geom, StyleDefault, StyleValues};
use crate::layout::{Titles, draw_axes_and_titles, draw_panel_background, layout_plot};
use crate::parameters::Parameters;
use crate::position;
use crate::scale::ContinuousPositionScale;
use crate::scene::{Group, Node, PerItem, Scene};
use crate::spec::{Aesthetic, GuideSpec, LayerSpec, Mapping, MappingValue, PlotSpec};
use crate::stat;
use crate::theme::Theme;
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

/// A built plot.
#[derive(Debug, Clone)]
pub struct BuiltPlot {
    /// The drawing.
    pub scene: Scene,
    /// Things the user should know: ignored options, removed rows, ...
    pub warnings: Vec<String>,
}

/// Build a single plot.
pub fn build_plot(spec: &PlotSpec, data: &PlotData) -> Result<BuiltPlot> {
    let mut warnings = Vec::new();
    report_unimplemented(spec, &mut warnings);
    let (width, height) = match &spec.ggsize {
        Some(size) if size.width > 0.0 && size.height > 0.0 => (size.width, size.height),
        Some(size) => {
            return Err(Error::InvalidSpec(format!(
                "at 'ggsize': width and height must be positive, got {} x {}",
                size.width, size.height
            )));
        }
        None => DEFAULT_SIZE,
    };
    let coord = resolve_coord(spec, &mut warnings)?;

    let layers = spec
        .layers
        .iter()
        .enumerate()
        .map(|(index, layer)| prepare_layer(index, layer, spec, data, &mut warnings))
        .collect::<Result<Vec<_>>>()?;

    let mut x_scale = ContinuousPositionScale::default();
    let mut y_scale = ContinuousPositionScale::default();
    for layer in &layers {
        for aesthetic in layer.data.aesthetics() {
            if let Some(column) = layer.data.continuous(aesthetic) {
                if aesthetic.is_x_position() {
                    x_scale.train(column);
                } else if aesthetic.is_y_position() {
                    y_scale.train(column);
                }
            }
        }
    }

    let theme = Theme::default();
    let titles = plot_titles(spec);
    let layout = layout_plot(width, height, &theme, &titles, &x_scale, &y_scale);
    let panel = Panel {
        rect: layout.panel,
        x_scale: &x_scale,
        y_scale: &y_scale,
        coord,
    };
    let mut layer_nodes = Vec::new();
    for layer in &layers {
        let draw_layer = DrawLayer {
            data: &layer.data,
            styles: &layer.styles,
            parameters: Parameters::new(&layer.spec.parameters, &layer.path),
        };
        layer_nodes.extend(layer.geom.draw(&draw_layer, &panel)?);
    }
    let mut nodes = draw_panel_background(&layout, &theme, &x_scale, &y_scale);
    nodes.push(Node::Group(Group {
        clip: Some(layout.panel),
        role: Some("panel"),
        nodes: layer_nodes,
    }));
    nodes.extend(draw_axes_and_titles(
        &layout, &theme, &titles, &x_scale, &y_scale,
    ));
    let scene = Scene {
        width,
        height,
        background: theme.plot_background,
        nodes,
    };
    Ok(BuiltPlot { scene, warnings })
}

/// Title, subtitle and axis titles. Axis titles follow lets-plot: `labs(x=...)`
/// first, otherwise the column mapped to the axis (plot mapping, then layers).
fn plot_titles(spec: &PlotSpec) -> Titles {
    let axis_title = |aesthetic: Aesthetic| {
        let from_labs = match spec.guides.get(aesthetic.as_str()) {
            Some(GuideSpec::Full(options)) => options.title.clone(),
            _ => None,
        };
        from_labs.or_else(|| {
            std::iter::once(&spec.mapping)
                .chain(spec.layers.iter().map(|layer| &layer.mapping))
                .find_map(|mapping| mapping.get(&aesthetic)?.as_column().map(str::to_owned))
        })
    };
    let title = spec.ggtitle.as_ref();
    Titles {
        title: title.and_then(|title| title.text.clone()),
        subtitle: title.and_then(|title| title.subtitle.clone()),
        x: axis_title(Aesthetic::X),
        y: axis_title(Aesthetic::Y),
    }
}

/// Whether a guide entry is only an axis title from `labs(x=...)` / `labs(y=...)`,
/// which the layout already uses.
fn is_axis_title_only(key: &str, guide: &GuideSpec) -> bool {
    let GuideSpec::Full(options) = guide else {
        return false;
    };
    matches!(key, "x" | "y")
        && options.name.is_none()
        && options.ncol.is_none()
        && options.nrow.is_none()
        && options.byrow.is_none()
        && options.override_aes.is_none()
        && options.barwidth.is_none()
        && options.barheight.is_none()
        && options.nbin.is_none()
        && options.order.is_none()
        && options.manual_key.is_none()
}

/// One layer after stat and position, ready to draw.
struct PreparedLayer<'a> {
    spec: &'a LayerSpec,
    path: String,
    geom: &'static dyn Geom,
    data: LayerData,
    styles: BTreeMap<Aesthetic, StyleValues>,
}

fn prepare_layer<'a>(
    index: usize,
    spec: &'a LayerSpec,
    plot: &PlotSpec,
    data: &PlotData,
    warnings: &mut Vec<String>,
) -> Result<PreparedLayer<'a>> {
    let path = format!("layers[{index}]");
    let at = |key: &str| format!("{path}.{key}");
    let geom = geom::geom(&spec.geom)
        .ok_or_else(|| unknown("geom", &at("geom"), &spec.geom, geom::NAMES))?;
    let stat_name = spec.stat.as_deref().unwrap_or(geom.default_stat());
    let stat = stat::stat(stat_name)
        .ok_or_else(|| unknown("stat", &at("stat"), stat_name, stat::NAMES))?;
    let (position_name, position_options) = match &spec.position {
        Some(position) => (position.name.as_str(), position.parameters.clone()),
        None => (geom.default_position(), Default::default()),
    };
    let position = position::position(position_name)
        .ok_or_else(|| unknown("position", &at("position"), position_name, position::NAMES))?;

    validate_parameters(geom, stat, &spec.parameters, &path, warnings);

    let mapping = merged_mapping(plot, spec);
    let frame = data
        .layers
        .get(index)
        .and_then(Option::as_ref)
        .or(data.plot.as_ref());
    let mut layer_data = LayerData::new(frame.map_or(0, DataFrame::row_count));
    for (aesthetic, value) in &mapping {
        let mapping_path = at(&format!("mapping.{aesthetic}"));
        let MappingValue::Column(name) = value else {
            warnings.push(format!(
                "at '{mapping_path}': mapping to {value:?} is not supported yet and was ignored"
            ));
            continue;
        };
        let Some(frame) = frame else {
            return Err(Error::InvalidSpec(format!(
                "at '{mapping_path}': column '{name}' is mapped but the layer has no data; \
                 pass data to ggplot() or to the geom"
            )));
        };
        let column = frame
            .column(name)
            .map_err(|error| Error::InvalidSpec(format!("at '{mapping_path}': {error}")))?;
        if (aesthetic.is_x_position() || aesthetic.is_y_position())
            && matches!(column, Column::Discrete(_))
        {
            return Err(Error::InvalidSpec(format!(
                "at '{mapping_path}': column '{name}' is discrete; discrete positions are not \
                 supported yet"
            )));
        }
        layer_data.insert(*aesthetic, column)?;
    }

    let missing: Vec<&str> = geom
        .required_aesthetics()
        .iter()
        .filter(|aesthetic| !layer_data.contains(**aesthetic))
        .map(|aesthetic| aesthetic.as_str())
        .collect();
    if !missing.is_empty() {
        return Err(Error::InvalidSpec(format!(
            "at '{path}': geom_{} requires aesthetics {}; missing: {}",
            geom.name(),
            geom.required_aesthetics()
                .iter()
                .map(|aesthetic| aesthetic.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            missing.join(", ")
        )));
    }

    let parameters = Parameters::new(&spec.parameters, &path);
    let layer_data = stat.compute(layer_data, &parameters)?;
    let position_path = at("position");
    let layer_data = position.adjust(
        layer_data,
        &Parameters::new(&position_options, &position_path),
    )?;

    if !spec.na_rm.unwrap_or(false) {
        report_missing_positions(geom, &layer_data, &path, warnings);
    }
    let styles = resolve_styles(geom, &mapping, &parameters, &path, warnings)?;
    Ok(PreparedLayer {
        spec,
        path,
        geom,
        data: layer_data,
        styles,
    })
}

/// Layer mapping over the inherited plot mapping (unless `inherit_aes=False`).
fn merged_mapping(plot: &PlotSpec, layer: &LayerSpec) -> Mapping {
    let mut mapping = if layer.inherit_aes.unwrap_or(true) {
        plot.mapping.clone()
    } else {
        Mapping::new()
    };
    mapping.extend(layer.mapping.clone());
    mapping
}

/// Warn about options neither the geom nor the stat reads.
fn validate_parameters(
    geom: &dyn Geom,
    stat: &dyn stat::Stat,
    options: &crate::spec::Options,
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

/// Style values per aesthetic: mapped (not yet) -> constant option -> geom default.
fn resolve_styles(
    geom: &dyn Geom,
    mapping: &Mapping,
    parameters: &Parameters,
    path: &str,
    warnings: &mut Vec<String>,
) -> Result<BTreeMap<Aesthetic, StyleValues>> {
    let mut styles = BTreeMap::new();
    for &(aesthetic, default) in geom.style_defaults() {
        if mapping.contains_key(&aesthetic) {
            warnings.push(format!(
                "at '{path}.mapping.{aesthetic}': mapping data to '{aesthetic}' is not \
                 supported yet; the constant or default value was used"
            ));
        }
        let name = aesthetic.as_str();
        let values = match default {
            StyleDefault::Color(fallback) => StyleValues::Colors(PerItem::Uniform(
                parameters.color(name)?.unwrap_or(fallback),
            )),
            StyleDefault::Number(fallback) => StyleValues::Numbers(PerItem::Uniform(
                parameters.number(name)?.unwrap_or(fallback),
            )),
        };
        styles.insert(aesthetic, values);
    }
    Ok(styles)
}

/// Warn when rows cannot be drawn because a required position is missing.
fn report_missing_positions(
    geom: &dyn Geom,
    data: &LayerData,
    path: &str,
    warnings: &mut Vec<String>,
) {
    let required: Vec<_> = geom
        .required_aesthetics()
        .iter()
        .filter_map(|aesthetic| data.continuous(*aesthetic))
        .collect();
    let removed = (0..data.row_count())
        .filter(|&row| {
            required
                .iter()
                .any(|column| !column.get(row).is_some_and(f64::is_finite))
        })
        .count();
    if removed > 0 {
        warnings.push(format!(
            "at '{path}': removed {removed} rows containing missing or non-finite values \
             (geom_{}); pass na_rm=True to silence",
            geom.name()
        ));
    }
}

fn resolve_coord(spec: &PlotSpec, warnings: &mut Vec<String>) -> Result<&'static dyn Coord> {
    let Some(coord_spec) = &spec.coord else {
        return Ok(&coord::Cartesian);
    };
    match coord::coord(&coord_spec.name) {
        Some(coord) => Ok(coord),
        None => {
            warnings.push(format!(
                "at 'coord': coord_{} is not supported yet; coord_cartesian was used. \
                 Supported: {}",
                coord_spec.name,
                coord::NAMES.join(", ")
            ));
            Ok(&coord::Cartesian)
        }
    }
}

fn unknown(kind: &str, path: &str, name: &str, supported: &[&str]) -> Error {
    Error::InvalidSpec(format!(
        "at '{path}': unknown {kind} '{name}'; supported: {}",
        supported.join(", ")
    ))
}

/// Parsed features that the builder does not implement yet. Each is reported, never
/// silently dropped. Remove an entry when its phase (plan.md) implements it.
fn report_unimplemented(spec: &PlotSpec, warnings: &mut Vec<String>) {
    for (key, guide) in &spec.guides {
        if !is_axis_title_only(key, guide) {
            warnings.push(format!(
                "at 'guides.{key}': legends and guides are not supported yet; ignored"
            ));
        }
    }
    let mut ignored = |present: bool, feature: &str| {
        if present {
            warnings.push(format!("'{feature}' is not supported yet and was ignored"));
        }
    };
    ignored(spec.theme.is_some(), "theme");
    ignored(spec.facet.is_some(), "facet");
    ignored(!spec.scales.is_empty(), "scales");
    ignored(spec.caption.is_some(), "caption");
    ignored(spec.ggtoolbar.is_some(), "ggtb");
    ignored(
        spec.coord.as_ref().is_some_and(|coord| {
            coord.xlim.is_some() || coord.ylim.is_some() || coord.ratio.is_some() || coord.flip
        }),
        "coord limits / ratio / flip",
    );
    ignored(
        !spec.data_meta.mapping_annotations.is_empty()
            || spec
                .layers
                .iter()
                .any(|layer| !layer.data_meta.mapping_annotations.is_empty()),
        "as_discrete",
    );
    for (index, layer) in spec.layers.iter().enumerate() {
        let mut layer_ignored = |present: bool, feature: &str| {
            if present {
                warnings.push(format!(
                    "at 'layers[{index}]': '{feature}' is not supported yet and was ignored"
                ));
            }
        };
        layer_ignored(layer.tooltips.is_some(), "tooltips");
        layer_ignored(layer.labels.is_some(), "labels");
        layer_ignored(layer.show_legend.is_some(), "show_legend");
        layer_ignored(layer.manual_key.is_some(), "manual_key");
    }
    for path in spec.unsupported_options() {
        warnings.push(format!("unsupported option '{path}' was ignored"));
    }
}
