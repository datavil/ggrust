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
use crate::data::{Column, ContinuousColumn, DataFrame, LayerData, group_index};
use crate::geom::{self, DrawLayer, Geom};
use crate::layout::{Titles, draw_axes_and_titles, draw_panel_background, layout_plot};
use crate::parameters::Parameters;
use crate::position::{self, Position};
use crate::scale::style::{StyleScale, default_mapper};
use crate::scale::{AxisScaleSpec, PositionScale, PositionScales, position_scales};
use crate::scene::{Group, Node, Rect, Scene};
use crate::spec::{
    Aesthetic, DataSource, GuideOptions, GuideSpec, LayerSpec, Mapping, MappingValue, Options,
    PlotSpec,
};
use crate::stat::{self, Stat};
use crate::style::{Styles, parse_constant};
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

impl PlotData {
    /// Resolve the spec's frame references (`{"frame": i}`) against `frames`.
    /// Frames are shared, not copied (Arrow buffers are reference counted).
    pub fn from_frames(spec: &PlotSpec, frames: &[DataFrame]) -> Result<Self> {
        let resolve = |source: Option<DataSource>, path: &str| match source {
            Some(DataSource::Frame(index)) => {
                frames.get(index).cloned().map(Some).ok_or_else(|| {
                    Error::InvalidSpec(format!(
                        "at '{path}': frame {index} was referenced but only {} were passed",
                        frames.len()
                    ))
                })
            }
            Some(DataSource::Inline) | None => Ok(None),
        };
        Ok(PlotData {
            plot: resolve(spec.data, "data")?,
            layers: spec
                .layers
                .iter()
                .enumerate()
                .map(|(index, layer)| resolve(layer.data, &format!("layers[{index}].data")))
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
    let scales =
        position_scales(spec, &mut warnings).map_err(|error| error.with_notes(&warnings))?;

    let mut mapped = Vec::with_capacity(spec.layers.len());
    for (index, layer) in spec.layers.iter().enumerate() {
        match map_layer(index, layer, spec, data) {
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
    let mut computed = Vec::with_capacity(mapped.len());
    for layer in mapped {
        match compute_layer(layer, [&x_scale, &y_scale]) {
            Ok(mut layer) => {
                warnings.append(&mut layer.warnings);
                computed.push(layer);
            }
            Err(error) => return Err(error.with_notes(&warnings)),
        }
    }
    let style_scales = train_style_scales(&computed, &mut warnings)
        .map_err(|error| error.with_notes(&warnings))?;
    let mut layers = Vec::with_capacity(computed.len());
    for layer in computed {
        match style_layer(layer, &style_scales) {
            Ok((layer, mut layer_warnings)) => {
                warnings.append(&mut layer_warnings);
                layers.push(layer);
            }
            Err((error, mut layer_warnings)) => {
                warnings.append(&mut layer_warnings);
                return Err(error.with_notes(&warnings));
            }
        }
    }

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
    let titles = plot_titles(spec, &scales);
    let area = Rect {
        x: 0.0,
        y: 0.0,
        width,
        height,
    };
    let layout = layout_plot(area, &theme, &titles, &x_scale, &y_scale);
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
            parameters: Parameters::new(&layer.options, &layer.path),
        };
        layer_nodes.extend(layer.geom.draw(&draw_layer, &panel)?);
    }
    let mut nodes = draw_panel_background(&layout, &theme, &panel);
    nodes.push(Node::Group(Group {
        clip: Some(layout.panel),
        role: Some("panel"),
        nodes: layer_nodes,
    }));
    nodes.extend(draw_axes_and_titles(&layout, &theme, &titles, &panel));
    let scene = Scene {
        width,
        height,
        background: theme.plot_background,
        nodes,
    };
    scene.validate()?;
    Ok(BuiltPlot { scene, warnings })
}

/// Title, subtitle and axis titles. Axis titles follow lets-plot: `labs(x=...)`
/// first, then the scale's `name`, otherwise `defaultScaleName` (see
/// [`default_axis_title`]).
fn plot_titles(spec: &PlotSpec, scales: &PositionScales) -> Titles {
    let axis_title = |aesthetic: Aesthetic, scale_name: &Option<String>| {
        let from_labs = match spec.guides.get(aesthetic.as_str()) {
            Some(GuideSpec::Full(options)) => options.title.clone(),
            _ => None,
        };
        from_labs
            .or_else(|| scale_name.clone())
            .unwrap_or_else(|| default_axis_title(spec, aesthetic))
    };
    let title = spec.ggtitle.as_ref();
    Titles {
        title: title.and_then(|title| title.text.clone()),
        subtitle: title.and_then(|title| title.subtitle.clone()),
        x: Some(axis_title(Aesthetic::X, &scales.x.name)),
        y: Some(axis_title(Aesthetic::Y, &scales.y.name)),
    }
}

/// lets-plot `PlotConfigUtil.defaultScaleName` for a position axis: the distinct
/// columns mapped to it across layers, ignoring one named like the aesthetic; none
/// or several -> the aesthetic name (`x`), exactly one -> that column.
fn default_axis_title(spec: &PlotSpec, aesthetic: Aesthetic) -> String {
    let mut labels: Vec<String> = Vec::new();
    for layer in &spec.layers {
        let (mapping, _) = merged_mapping(spec, layer);
        if let Some(name) = mapping.get(&aesthetic).and_then(MappingValue::as_column)
            && name != aesthetic.as_str()
            && !labels.iter().any(|label| label == name)
        {
            labels.push(name.to_owned());
        }
    }
    match labels.as_slice() {
        [only] => only.clone(),
        _ => aesthetic.as_str().to_owned(),
    }
}

/// Whether a guide entry is only an axis title from `labs(x=...)` / `labs(y=...)`,
/// which the layout already uses.
fn is_axis_title_only(key: &str, guide: &GuideSpec) -> bool {
    let GuideSpec::Full(options) = guide else {
        return false;
    };
    // Everything but the title (and keys reported as unsupported elsewhere) is at its
    // default, so a new guide option cannot be missed here.
    let title_only = GuideOptions {
        title: options.title.clone(),
        unsupported: options.unsupported.clone(),
        ..GuideOptions::default()
    };
    matches!(key, "x" | "y") && **options == title_only
}

/// One layer after stat and position, ready to draw.
struct PreparedLayer {
    /// Layer options with aliases normalised (`colour` -> `color`).
    options: Options,
    path: String,
    geom: &'static dyn Geom,
    data: LayerData,
    styles: Styles,
}

/// Stage 1 of a layer: look up geom/stat/position, validate options, fetch the
/// mapped columns and compute groups. Positions stay as in the data: the position
/// scales are known only after every layer is mapped.
fn map_layer<'a>(
    index: usize,
    spec: &'a LayerSpec,
    plot: &PlotSpec,
    data: &PlotData,
) -> StageResult<MappedLayer<'a>> {
    let mut warnings = Vec::new();
    map_layer_into(index, spec, plot, data, &mut warnings).map_err(|error| (error, warnings))
}

fn map_layer_into<'a>(
    index: usize,
    spec: &'a LayerSpec,
    plot: &PlotSpec,
    data: &PlotData,
    warnings: &mut Vec<String>,
) -> Result<MappedLayer<'a>> {
    let path = format!("layers[{index}]");
    let at = |key: &str| format!("{path}.{key}");
    let geom = geom::geom(&spec.geom)
        .ok_or_else(|| unknown("geom", &at("geom"), &spec.geom, &geom::names()))?;
    let stat_name = spec.stat.as_deref().unwrap_or(geom.default_stat());
    let stat = stat::stat(stat_name)
        .ok_or_else(|| unknown("stat", &at("stat"), stat_name, &stat::names()))?;
    let (position_name, position_options) = match &spec.position {
        Some(position) => (position.name.as_str(), position.parameters.clone()),
        None => (geom.default_position(), Default::default()),
    };
    let position = position::position(position_name).ok_or_else(|| {
        unknown(
            "position",
            &at("position"),
            position_name,
            &position::names(),
        )
    })?;

    let options = normalize_options(&spec.parameters, &path, warnings);
    validate_parameters(geom, stat, &options, &path, warnings);
    validate_position_options(position, &position_options, &at("position"), warnings);

    let (mut mapping, inherited) = merged_mapping(plot, spec);
    // lets-plot (`DataConfigUtil`): an aesthetic given as a constant wins over its
    // mapping, and the mapped column is not even looked up.
    mapping.retain(|aesthetic, _| {
        options
            .get(aesthetic.as_str())
            .is_none_or(|value| value.is_null())
    });
    let frames = LayerFrames {
        layer: data.layers.get(index).and_then(Option::as_ref),
        plot: data.plot.as_ref(),
    };
    let mut columns = Vec::new();
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
        if !geom_uses(geom, *aesthetic) {
            warnings.push(format!(
                "at '{mapping_path}': geom_{} does not use aesthetic '{aesthetic}' (yet); \
                 the mapping was ignored",
                geom.name()
            ));
            continue;
        }
        let column = frames.column(name, &mapping_path)?;
        if matches!(column, Column::Discrete(_))
            && !aesthetic.is_x_position()
            && !aesthetic.is_y_position()
            && !default_grouping.iter().any(|(seen, _)| seen == name)
        {
            default_grouping.push((name, column.clone()));
        }
        columns.push((*aesthetic, column, mapping_path));
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
    let unbound = columns.is_empty();
    Ok(MappedLayer {
        spec,
        path,
        geom,
        stat,
        position,
        position_options,
        options,
        // lets-plot drops the data of a layer with no mapped columns: one row.
        row_count: if unbound { 1 } else { frames.row_count() },
        columns,
        group_index: if unbound {
            None
        } else {
            group_index(&grouping.iter().collect::<Vec<_>>())
        },
        warnings: std::mem::take(warnings),
    })
}

/// A layer after [`map_layer`].
struct MappedLayer<'a> {
    spec: &'a LayerSpec,
    path: String,
    geom: &'static dyn Geom,
    stat: &'static dyn Stat,
    position: &'static dyn Position,
    position_options: Options,
    /// Layer options with aliases normalised (`colour` -> `color`).
    options: Options,
    row_count: usize,
    /// Mapped columns with their mapping paths; positions not yet scaled.
    columns: Vec<(Aesthetic, Column, String)>,
    group_index: Option<Vec<u32>>,
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
    let mut scale = spec.build(discrete, warnings)?;
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
            return Err(Error::InvalidSpec(format!(
                "at '{path}': the {axis} axis is discrete (scale_{axis}_discrete()) but only \
                 numeric columns are mapped to it; converting numbers to categories is not \
                 supported yet, map a string column instead"
            )));
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
) -> Result<ComputedLayer> {
    let MappedLayer {
        spec,
        path,
        geom,
        stat,
        position,
        position_options,
        options,
        row_count,
        columns,
        group_index,
        mut warnings,
    } = layer;
    let at = |key: &str| format!("{path}.{key}");
    let scale_of = |aesthetic: Aesthetic| {
        if aesthetic.is_x_position() {
            Some(x_scale)
        } else if aesthetic.is_y_position() {
            Some(y_scale)
        } else {
            None
        }
    };
    let mut layer_data = LayerData::new(row_count);
    let mut mapping_paths = BTreeMap::new();
    for (aesthetic, column, mapping_path) in columns {
        let column = match scale_of(aesthetic) {
            Some(scale) => scale
                .positions(column)
                .map_err(|message| Error::InvalidSpec(format!("at '{mapping_path}': {message}")))?,
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
        if let Some(value) = parameters.number(aesthetic.as_str())? {
            let column = ContinuousColumn::from_values(vec![value; layer_data.row_count()]);
            let column = match scale_of(aesthetic) {
                Some(scale) => scale
                    .positions(Column::Continuous(column))
                    .map_err(|message| {
                        Error::InvalidSpec(format!("at '{}': {message}", at(aesthetic.as_str())))
                    })?,
                None => Column::Continuous(column),
            };
            layer_data.insert(aesthetic, column)?;
        }
    }

    let missing: Vec<&str> = geom
        .required_aesthetics()
        .iter()
        .filter(|aesthetic| !layer_data.contains(**aesthetic))
        .map(|aesthetic| aesthetic.as_str())
        .collect();
    if !missing.is_empty() {
        return Err(Error::InvalidSpec(format!(
            "at '{path}': geom_{} requires aesthetics {} (mapped to columns or given as \
             constants); missing: {}",
            geom.name(),
            geom.required_aesthetics()
                .iter()
                .map(|aesthetic| aesthetic.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            missing.join(", ")
        )));
    }

    let parameters = Parameters::new(&options, &path);
    let layer_data = stat.compute(layer_data, &parameters)?;
    let position_path = at("position");
    let layer_data = position.adjust(
        layer_data,
        &Parameters::new(&position_options, &position_path),
    )?;

    if !spec.na_rm.unwrap_or(false) {
        report_missing_positions(geom, &layer_data, &path, &mut warnings);
    }
    Ok(ComputedLayer {
        options,
        path,
        geom,
        data: layer_data,
        mapping_paths,
        warnings,
    })
}

/// A layer after [`compute_layer`]: positions scaled, stat and position applied.
struct ComputedLayer {
    /// Layer options with aliases normalised (`colour` -> `color`).
    options: Options,
    path: String,
    geom: &'static dyn Geom,
    data: LayerData,
    /// Where each mapped aesthetic was mapped, for error messages.
    mapping_paths: BTreeMap<Aesthetic, String>,
    /// Warnings of this stage.
    warnings: Vec<String>,
}

/// Last layer stage: style values from the trained style scales. Returns the
/// stage's warnings next to the layer.
fn style_layer(
    layer: ComputedLayer,
    style_scales: &BTreeMap<Aesthetic, StyleScale>,
) -> StageResult<(PreparedLayer, Vec<String>)> {
    let mut warnings = Vec::new();
    let parameters = Parameters::new(&layer.options, &layer.path);
    let styles = resolve_styles(
        layer.geom,
        &layer.data,
        style_scales,
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
            styles,
        },
        warnings,
    ))
}

/// Layer mapping over the inherited plot mapping (unless `inherit_aes=False`), and
/// which aesthetics came from the plot (for error paths).
fn merged_mapping(plot: &PlotSpec, layer: &LayerSpec) -> (Mapping, BTreeSet<Aesthetic>) {
    let mut mapping = if layer.inherit_aes.unwrap_or(true) {
        plot.mapping.clone()
    } else {
        Mapping::new()
    };
    mapping.retain(|aesthetic, _| !layer.mapping.contains_key(aesthetic));
    let inherited = mapping.keys().copied().collect();
    mapping.extend(layer.mapping.clone());
    (mapping, inherited)
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
                .filter(|_| layer.inherit_aes.unwrap_or(true)),
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

    fn column(&self, name: &str, path: &str) -> Result<Column> {
        let error = |message: String| Error::InvalidSpec(format!("at '{path}': {message}"));
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
                        "; the plot data has it, but its {} rows differ from the layer \
                         data's {} rows",
                        plot.row_count(),
                        layer.row_count()
                    ),
                    _ => String::new(),
                };
                return layer
                    .column(name)
                    .map_err(|missing| error(format!("{missing}{note}")));
            }
            (None, Some(plot)) => plot,
        };
        source
            .column(name)
            .map_err(|failure| error(failure.to_string()))
    }
}

/// Whether the geom reads an aesthetic (required positions, styles, or `group`).
fn geom_uses(geom: &dyn Geom, aesthetic: Aesthetic) -> bool {
    geom.required_aesthetics().contains(&aesthetic)
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
        if normalized
            .insert(canonical.to_owned(), value.clone())
            .is_some()
        {
            warnings.push(format!(
                "at '{path}.{name}': '{canonical}' is given twice (as an alias too); the \
                 value of '{name}' was used"
            ));
        }
    }
    normalized
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

/// Style scales of mapped style aesthetics, trained over every layer (lets-plot
/// default mappers; `scale_color_*()` options come in Phase 1).
fn train_style_scales(
    layers: &[ComputedLayer],
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
    let mut scales: BTreeMap<Aesthetic, StyleScale> = BTreeMap::new();
    for (aesthetic, column, path) in columns {
        let Some(mapper) = default_mapper(aesthetic) else {
            continue; // reported by `resolve_styles`
        };
        scales
            .entry(aesthetic)
            .or_insert_with(|| {
                if discrete.contains(&aesthetic) {
                    StyleScale::discrete(mapper)
                } else {
                    StyleScale::new(mapper)
                }
            })
            .train(column)
            .map_err(|message| Error::InvalidSpec(format!("at '{path}': {message}")))?;
    }
    for aesthetic in scales.keys() {
        warnings.push(format!(
            "'{aesthetic}' is mapped to data, but legends are not drawn yet"
        ));
    }
    Ok(scales)
}

/// Style values per aesthetic: mapped column (through its style scale) -> constant
/// option -> geom default.
fn resolve_styles(
    geom: &dyn Geom,
    data: &LayerData,
    style_scales: &BTreeMap<Aesthetic, StyleScale>,
    parameters: &Parameters,
    path: &str,
    warnings: &mut Vec<String>,
) -> Result<Styles> {
    let mut styles = Styles::default();
    for &(aesthetic, default) in geom.style_defaults() {
        let mapped = data.get(aesthetic);
        let values = match (mapped, style_scales.get(&aesthetic)) {
            (Some(column), Some(scale)) => scale.map(column).map_err(|message| {
                Error::InvalidSpec(format!("at '{path}.mapping.{aesthetic}': {message}"))
            })?,
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
                        Error::InvalidSpec(format!(
                            "at '{}': {message}",
                            parameters.path_of(aesthetic.as_str())
                        ))
                    })?,
                    None => default.uniform(),
                }
            }
        };
        styles.insert(aesthetic, values)?;
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
            "at '{path}': removed {removed} rows containing missing, non-finite or out-of-limits values \
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
                coord::names().join(", ")
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
            map_layer(0, &spec.layers[0], spec, &cells()).map_err(|(error, _)| error)?;
        warnings.append(&mut mapped.warnings);
        let mut computed = compute_layer(mapped, [x_scale, y_scale])?;
        warnings.append(&mut computed.warnings);
        let (layer, mut layer_warnings) =
            style_layer(computed, &BTreeMap::new()).map_err(|(error, _)| error)?;
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
}
