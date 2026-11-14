//! `coord_3d` in the plot builder: which layers may draw, the z scale, the axis
//! titles, and drawing a 3D panel (see [`crate::coord3d`]).
//!
//! The x and y scales are the usual ones; z gets its own continuous scale
//! (`scale_z_continuous()`), trained on every panel. Layer z values are scaled
//! (transformed) here, as x and y are by the stat stage.
//!
//! What it does not do: tooltips, zoom, the interactive legend, `rasterize()`,
//! stats other than identity (an error), free z scales across facets.

use super::{PreparedLayer, axis_title, layer_stat, merged_mapping};
use crate::coord::{Cartesian, Panel};
use crate::coord3d::{self, Projector, Scales3d, View, identity_scale};
use crate::data::Column;
use crate::geom::{DrawLayer, surface};
use crate::layout::Titles;
use crate::layout::box3d::{self, Titles3d};
use crate::parameters::Parameters;
use crate::scale::number_format::ExponentFormat;
use crate::scale::{AxisScaleSpec, PositionScale};
use crate::scene::{Group, LAYER_ROLE, Node, Rect};
use crate::spec::{Aesthetic, PlotSpec};
use crate::theme::Theme;
use crate::{Error, Result};

/// What a `coord_3d` plot keeps beyond a 2D one.
#[derive(Clone)]
pub(super) struct Space3d {
    pub view: View,
    /// z scale, trained.
    pub z: PositionScale,
    /// Axis titles drawn on the box (the 2D layout gets none).
    pub titles: Titles3d,
}

/// Whether the plot asks for `coord_3d()`.
pub(super) fn is_3d(spec: &PlotSpec) -> bool {
    spec.coord.as_ref().is_some_and(|coord| coord.name == "3d")
}

/// Under `coord_3d`, every layer must be a geom that draws in 3D; without it,
/// a mapped `z` is noted as unused.
pub(super) fn check_layers(spec: &PlotSpec, warnings: &mut Vec<String>) -> Result<()> {
    let three_d = is_3d(spec);
    for (index, layer) in spec.layers.iter().enumerate() {
        let maps_z = merged_mapping(spec, layer).0.contains_key(&Aesthetic::Z);
        let draws_3d = coord3d::GEOMS.contains(&layer.geom.as_str());
        if !three_d && layer.geom == "surface" {
            return Err(Error::spec(
                format!("layers[{index}].geom"),
                "geom_surface draws only under coord_3d(); add coord_3d() to the plot",
            ));
        }
        if three_d && !draws_3d {
            return Err(Error::spec(
                format!("layers[{index}].geom"),
                format!(
                    "geom_{} cannot draw under coord_3d(); supported: {}. Remove the layer \
                     or drop coord_3d()",
                    layer.geom,
                    coord3d::GEOMS
                        .iter()
                        .map(|name| format!("geom_{name}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
        if three_d && let Some(stat) = layer_stat(layer).filter(|stat| stat.name() != "identity") {
            // A stat's output rows have no z: it would draw them all at one depth.
            return Err(Error::spec(
                format!("layers[{index}].stat"),
                format!(
                    "stat '{}' cannot carry z under coord_3d(); use stat='identity' (the \
                     default of geom_point and geom_path) or compute the values first",
                    stat.name()
                ),
            ));
        }
        if three_d && layer.rasterize.is_some() {
            warnings.push(format!(
                "at 'layers[{index}].rasterize': coord_3d() draws layers as vectors; ignored"
            ));
        }
        if !three_d && draws_3d && maps_z {
            warnings.push(format!(
                "at 'layers[{index}].mapping.z': geom_{} draws z only under coord_3d(); \
                 the mapping was ignored",
                layer.geom
            ));
        }
    }
    Ok(())
}

/// Build and train the z scale on every layer's z, scale the z values, and take
/// the axis titles off the 2D layout. Errors when a layer has no z.
pub(super) fn prepare(
    spec: &PlotSpec,
    z_spec: &AxisScaleSpec,
    tiles: &mut [(Vec<PreparedLayer>, PositionScale, PositionScale)],
    titles: &mut Titles,
    orientations: &[bool],
    exponent_format: ExponentFormat,
    warnings: &mut Vec<String>,
) -> Result<Space3d> {
    let coord_spec = spec.coord.as_ref().ok_or_else(|| {
        Error::spec(
            "coord",
            "internal: coord_3d preparation without a coord spec",
        )
    })?;
    let view = View::from_spec(coord_spec)?;
    if coord_spec.xlim.is_some() || coord_spec.ylim.is_some() {
        warnings.push("at 'coord': coord_3d() takes no xlim/ylim; ignored".to_owned());
    }
    if z_spec.requests_discrete() {
        return Err(Error::spec(
            "scales",
            "coord_3d() needs a continuous z axis; use scale_z_continuous()",
        ));
    }
    let mut z = z_spec.build(false, warnings)?;
    z.set_exponent_format(exponent_format);
    for (layers, _, _) in tiles.iter_mut() {
        for layer in layers.iter_mut() {
            let path = format!("{}.mapping.z", layer.path);
            let column = match layer.data.get(Aesthetic::Z) {
                Some(column) => column.clone(),
                None if layer.data.row_count() == 0 => continue,
                None => {
                    return Err(Error::spec(
                        &layer.path,
                        format!(
                            "geom_{} under coord_3d() requires aesthetic z (mapped to a \
                             column or given as a constant); missing: z",
                            layer.geom.name()
                        ),
                    ));
                }
            };
            if matches!(column, Column::Discrete(_)) {
                return Err(Error::spec(
                    path,
                    "coord_3d() needs numbers on z, got a column of categories; map a \
                     numeric column",
                ));
            }
            let scaled = z
                .positions(column)
                .map_err(|message| Error::spec(&path, message))?;
            if let Column::Continuous(values) = &scaled {
                z.train(values);
            }
            layer.data.insert(Aesthetic::Z, scaled)?;
        }
    }
    z.attach_labels()?;
    let z_title = axis_title(spec, Aesthetic::Z, &z_spec.name, orientations);
    let titles3d = Titles3d {
        x: titles.x.take(),
        y: titles.y.take(),
        z: Some(z_title),
    };
    Ok(Space3d {
        view,
        z,
        titles: titles3d,
    })
}

/// The theme the 2D layout sees: no axis ticks, labels or lines, so the panel
/// takes the room the box draws its own axes in.
pub(super) fn layout_theme(theme: &Theme) -> Theme {
    Theme {
        axis_text_x: None,
        axis_text_y: None,
        axis_ticks_x: None,
        axis_ticks_y: None,
        axis_minor_ticks_x: None,
        axis_minor_ticks_y: None,
        axis_line_x: None,
        axis_line_y: None,
        ..theme.clone()
    }
}

/// One 3D panel: the box's back panes, the layers back to front, then the box's
/// edges, ticks and labels.
pub(super) fn draw_tile(
    space: &Space3d,
    layers: &[PreparedLayer],
    (tile_x, tile_y): (&PositionScale, &PositionScale),
    panel: Rect,
    theme: &Theme,
    background: crate::scene::Color,
) -> Result<Vec<Node>> {
    let scales = Scales3d {
        x: tile_x,
        y: tile_y,
        z: &space.z,
    };
    let room = box3d::label_room(&scales, theme, &space.titles);
    let projector = Projector::fit(space.view, box3d::box_rect(panel, room));
    let mut nodes = box3d::draw_back(&projector, &scales, theme);
    let identity = identity_scale();
    let flat = Panel {
        rect: panel,
        x_scale: &identity,
        y_scale: &identity,
        coord: &Cartesian,
        flipped: false,
    };
    for layer in layers.iter().filter(|layer| layer.band.is_none()) {
        if layer.geom.name() == "surface" {
            let draw_layer = DrawLayer {
                data: &layer.data,
                styles: &layer.styles,
                parameters: Parameters::new(&layer.options, &layer.path),
                background,
            };
            nodes.push(Node::Group(Group {
                key: None,
                clip: None,
                role: Some(LAYER_ROLE),
                anchor: None,
                nodes: surface::draw_3d(&draw_layer, &scales, &projector, &layer.path)?,
            }));
            continue;
        }
        let by_group = layer.geom.name() == "path";
        let (data, styles) = coord3d::project_layer(
            &layer.data,
            &layer.styles,
            &scales,
            &projector,
            panel,
            by_group,
        )?;
        let draw_layer = DrawLayer {
            data: &data,
            styles: &styles,
            parameters: Parameters::new(&layer.options, &layer.path),
            background,
        };
        let marks = layer.geom.draw(&draw_layer, &flat)?;
        nodes.push(Node::Group(Group {
            key: None,
            clip: None,
            role: Some(LAYER_ROLE),
            anchor: None,
            nodes: marks,
        }));
    }
    nodes.extend(box3d::draw_front(&projector, &scales, theme, &space.titles));
    Ok(nodes)
}
