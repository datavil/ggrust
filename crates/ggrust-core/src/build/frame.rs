//! Animation seam (plan step an0): draw a prepared plot again from changed layer
//! rows, with the scales, legends and titles it trained once.
//!
//! A [`PreparedPlot`] holds every layer after map, stat, position and style, plus
//! the trained position scales and the legend boxes. Drawing reads the marks only
//! from each prepared layer's `data` and `styles`; layout reads only the scales,
//! legends and titles. So a frame is a copy of the plot whose layers hold other
//! rows ([`PreparedPlot::with_layer_frames`]), laid out and drawn by the normal
//! code. An animated plot ([`super::prepare_plot_states`]) also knows the state of
//! every row ([`PreparedPlot::state_rows`]). This module does not plan frames,
//! tween values or render outputs: that is `crate::animate`.

use super::PANEL_MIN_SIZE;
use super::{PreparedLayer, PreparedPlot};
use crate::data::LayerData;
use crate::layout::Titles;
use crate::scene::{Rect, Scene};
use crate::style::Styles;
use crate::{Error, Result};

/// One layer's rows in one frame: the computed data a geom draws and the resolved
/// style of every row.
#[derive(Debug, Clone)]
pub(crate) struct LayerFrame {
    /// Position and other computed columns, in data space (after stat and position).
    pub data: LayerData,
    /// Colors, sizes, shapes, ... per row, already mapped by the trained scales.
    pub styles: Styles,
}

impl LayerFrame {
    /// The frame holding only `rows` (indices into this frame), in that order.
    pub(crate) fn take(&self, rows: &[u32]) -> LayerFrame {
        LayerFrame {
            data: self.data.take(rows),
            styles: self.styles.take(rows),
        }
    }
}

impl PreparedPlot {
    /// Every layer's computed rows, per tile (facet panel) and per layer, in the
    /// order [`Self::with_layer_frames`] takes them. A frame starts from these.
    pub(crate) fn layer_frames(&self) -> Vec<Vec<LayerFrame>> {
        self.tiles
            .iter()
            .map(|(layers, _, _)| {
                layers
                    .iter()
                    .map(|layer| LayerFrame {
                        data: layer.data.clone(),
                        styles: layer.styles.clone(),
                    })
                    .collect()
            })
            .collect()
    }

    /// A copy of the plot whose layers draw `frames` (per tile, per layer, as
    /// [`Self::layer_frames`] gives them). Scales, legends, titles and theme stay
    /// as trained, so axes and legends do not move between frames.
    ///
    /// The copy has no tooltips and no interactive legend keys: those index the
    /// prepared rows, which a frame no longer has.
    ///
    /// # Errors
    ///
    /// An internal error when `frames` does not have the plot's tile and layer
    /// counts, or when a layer's varying styles do not have one value per row.
    pub(crate) fn with_layer_frames(&self, frames: Vec<Vec<LayerFrame>>) -> Result<PreparedPlot> {
        if frames.len() != self.tiles.len() {
            return Err(Error::data(format!(
                "internal: animation frame has {} panels, the plot has {}",
                frames.len(),
                self.tiles.len()
            )));
        }
        let mut tiles = Vec::with_capacity(self.tiles.len());
        for (tile, ((layers, x_scale, y_scale), tile_frames)) in
            self.tiles.iter().zip(frames).enumerate()
        {
            if tile_frames.len() != layers.len() {
                return Err(Error::data(format!(
                    "internal: animation frame has {} layers in panel {tile}, the plot has {}",
                    tile_frames.len(),
                    layers.len()
                )));
            }
            let frame_layers = layers
                .iter()
                .zip(tile_frames)
                .map(|(layer, frame)| frame_layer(layer, frame))
                .collect::<Result<Vec<_>>>()?;
            tiles.push((frame_layers, x_scale.clone(), y_scale.clone()));
        }
        Ok(PreparedPlot {
            tiles,
            bands: self.bands.clone(),
            facets: self.facets.clone(),
            theme: self.theme.clone(),
            titles: self.titles.clone(),
            coord: self.coord,
            aspect: self.aspect,
            flipped: self.flipped,
            legends: self.legends.clone(),
            style_guides: self.style_guides.clone(),
            size: self.size,
            tooltip_setups: self.tooltip_setups.clone(),
            space: self.space.clone(),
            polar: self.polar,
            state_labels: self.state_labels.clone(),
            warnings: Vec::new(),
        })
    }

    /// An animation's states, labelled as `{current_frame}` shows them; empty
    /// for a plot prepared without states.
    pub(crate) fn state_labels(&self) -> &[String] {
        self.state_labels.as_deref().unwrap_or_default()
    }

    /// Per tile and per layer, the rows of every state (indices into the
    /// layer's prepared rows, see [`Self::layer_frames`]); `None` for a layer
    /// without states, drawn whole in every frame.
    pub(crate) fn state_rows(&self) -> Vec<Vec<Option<Vec<Vec<u32>>>>> {
        let count = self.state_labels().len();
        self.tiles
            .iter()
            .map(|(layers, _, _)| {
                layers
                    .iter()
                    .map(|layer| {
                        let states = layer.states.as_ref()?;
                        let mut rows = vec![Vec::new(); count];
                        for (row, state) in states.iter().enumerate() {
                            if let Some(rows) = rows.get_mut(*state as usize) {
                                rows.push(row as u32);
                            }
                        }
                        Some(rows)
                    })
                    .collect()
            })
            .collect()
    }

    /// The plot's screen titles (title, subtitle, caption, axes).
    pub(crate) fn titles(&self) -> &Titles {
        &self.titles
    }

    /// Replace the titles (an animation frame's filled label templates).
    pub(crate) fn set_titles(&mut self, titles: Titles) {
        self.titles = titles;
    }

    /// Lay the plot out at its own size and draw it, as `build_plot` does for
    /// static output (no tooltips).
    pub(crate) fn frame_scene(&self) -> Result<Scene> {
        let (width, height) = self.size;
        let area = Rect {
            x: 0.0,
            y: 0.0,
            width,
            height,
        };
        let arrangement = self.arrange(area, PANEL_MIN_SIZE)?;
        let scene = Scene {
            width,
            height,
            metadata: true,
            background: self.background(),
            nodes: self.draw(&arrangement, false)?,
        };
        scene.validate()?;
        Ok(scene)
    }
}

/// `layer` drawing `frame`'s rows: every option and setting of the layer, with
/// row-indexed extras (tooltip columns, legend keys) dropped.
fn frame_layer(layer: &PreparedLayer, frame: LayerFrame) -> Result<PreparedLayer> {
    let rows = frame.data.row_count();
    if !frame.styles.fits(rows) {
        return Err(Error::data(format!(
            "internal: animation frame of {} has {rows} rows but styles of another length",
            layer.path
        )));
    }
    Ok(PreparedLayer {
        unadjusted: frame.data.clone(),
        data: frame.data,
        styles: frame.styles,
        tooltip_columns: Vec::new(),
        keys: None,
        ..layer.clone_settings()
    })
}

impl PreparedLayer {
    /// The layer's settings with empty rows: a base for [`frame_layer`], which
    /// fills in the rows (cloning the prepared rows only to drop them is waste).
    fn clone_settings(&self) -> PreparedLayer {
        PreparedLayer {
            band: self.band,
            options: self.options.clone(),
            path: self.path.clone(),
            geom: self.geom,
            data: LayerData::default(),
            unadjusted: LayerData::default(),
            tooltip_columns: Vec::new(),
            x_span: self.x_span,
            trains_unadjusted: self.trains_unadjusted,
            y_span: self.y_span,
            y_oriented: self.y_oriented,
            styles: Styles::default(),
            scale_groups: self.scale_groups.clone(),
            rasterize: self.rasterize,
            show_legend: self.show_legend,
            companion: self.companion,
            keys: None,
            states: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;

    use arrow_array::{ArrayRef, Float64Array, RecordBatch, StringArray};

    use super::*;
    use crate::build::{PlotData, prepare_plot};
    use crate::data::{Column, ContinuousColumn, DataFrame};
    use crate::render::to_png;
    use crate::scene::{Group, Node};
    use crate::spec::{Aesthetic, Figure, PlotSpec};

    /// Three states (`year` 1, 2, 3) of the same 8 items: x and y move per state,
    /// `kind` is a discrete color, `weight` a continuous size.
    fn states() -> PlotData {
        let count = 8;
        let mut x = Vec::new();
        let mut y = Vec::new();
        let mut year = Vec::new();
        let mut kind = Vec::new();
        let mut weight = Vec::new();
        for state in 1..=3 {
            for item in 0..count {
                let item_f = item as f64;
                let state_f = state as f64;
                x.push(item_f + state_f * 1.5);
                y.push((item_f * 0.9 + state_f).sin() * 3.0 + state_f * 2.0);
                year.push(state.to_string());
                kind.push(["a", "b", "c", "d"][item % 4]);
                weight.push(item_f * 10.0 + state_f);
            }
        }
        let columns: Vec<(&str, ArrayRef)> = vec![
            ("x", Arc::new(Float64Array::from(x))),
            ("y", Arc::new(Float64Array::from(y))),
            ("year", Arc::new(StringArray::from(year))),
            ("kind", Arc::new(StringArray::from(kind))),
            ("weight", Arc::new(Float64Array::from(weight))),
        ];
        let batch = RecordBatch::try_from_iter(columns).unwrap();
        PlotData {
            plot: Some(DataFrame::new(batch.schema(), vec![batch]).unwrap()),
            layers: vec![],
        }
    }

    fn plot(json: &str) -> PlotSpec {
        match Figure::from_json(json).unwrap() {
            Figure::Plot(plot) => *plot,
            Figure::Subplots(_) => panic!("expected a plot"),
        }
    }

    /// Rows `state * 8 .. state * 8 + 8` (state 0, 1, 2) of a frame.
    fn state_rows(state: u32) -> Vec<u32> {
        (state * 8..state * 8 + 8).collect()
    }

    /// The scene without the panel's marks (`role = "panel"` groups emptied).
    fn without_marks(nodes: &[Node]) -> Vec<Node> {
        nodes
            .iter()
            .map(|node| match node {
                Node::Group(group) if group.role == Some("panel") => Node::Group(Group {
                    nodes: Vec::new(),
                    ..group.clone()
                }),
                Node::Group(group) => Node::Group(Group {
                    nodes: without_marks(&group.nodes),
                    ..group.clone()
                }),
                other => other.clone(),
            })
            .collect()
    }

    /// The panel's marks (children of `role = "panel"` groups).
    fn marks(nodes: &[Node]) -> Vec<Node> {
        nodes
            .iter()
            .flat_map(|node| match node {
                Node::Group(group) if group.role == Some("panel") => group.nodes.clone(),
                Node::Group(group) => marks(&group.nodes),
                _ => Vec::new(),
            })
            .collect()
    }

    #[test]
    fn frames_redraw_changed_rows_with_fixed_axes_and_legends() {
        let spec = plot(
            r#"{"kind": "plot",
                "mapping": {"x": "x", "y": "y", "color": "kind", "size": "weight"},
                "ggsize": {"width": 500, "height": 350},
                "layers": [{"geom": "point"}]}"#,
        );
        let prepared = prepare_plot(&spec, &states()).unwrap();
        let all_rows = prepared.layer_frames();
        let layer = &all_rows[0][0];
        let first = layer.take(&state_rows(0));
        let last = layer.take(&state_rows(2));
        // A tweened frame: the first state's rows halfway to the second's (rows
        // matched by item, as an1/an2 will match them by group).
        let second = layer.take(&state_rows(1));
        let mut halfway = first.clone();
        for aesthetic in [Aesthetic::X, Aesthetic::Y] {
            let from = first.data.continuous(aesthetic).unwrap().values();
            let to = second.data.continuous(aesthetic).unwrap().values();
            let values = from.iter().zip(to).map(|(a, b)| a + (b - a) * 0.5);
            halfway
                .data
                .insert(
                    aesthetic,
                    Column::Continuous(ContinuousColumn::from_values(values.collect())),
                )
                .unwrap();
        }

        let full = prepared.frame_scene().unwrap();
        let frames: Vec<Scene> = [first, halfway, last]
            .into_iter()
            .map(|frame| {
                prepared
                    .with_layer_frames(vec![vec![frame]])
                    .unwrap()
                    .frame_scene()
                    .unwrap()
            })
            .collect();

        // Axes, grid, titles and legends: the same nodes in every frame and in the
        // static plot of all rows.
        let fixed = without_marks(&full.nodes);
        for frame in &frames {
            assert_eq!(without_marks(&frame.nodes), fixed);
        }
        // The marks differ between frames, and each frame draws 8 points.
        let frame_marks: Vec<Vec<Node>> = frames.iter().map(|frame| marks(&frame.nodes)).collect();
        assert_ne!(frame_marks[0], frame_marks[1]);
        assert_ne!(frame_marks[1], frame_marks[2]);
        assert_ne!(frame_marks[0], frame_marks[2]);
        for layer_marks in &frame_marks {
            let points: usize = layer_marks
                .iter()
                .map(|node| match node {
                    Node::Group(group) => group
                        .nodes
                        .iter()
                        .map(|node| match node {
                            Node::Points(points) => points.x.len(),
                            _ => 0,
                        })
                        .sum(),
                    _ => 0,
                })
                .sum();
            assert_eq!(points, 8);
        }

        let output = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/visual/output");
        fs::create_dir_all(&output).unwrap();
        for (index, frame) in frames.iter().enumerate() {
            let png = to_png(frame, 1.0).unwrap();
            fs::write(output.join(format!("animate_seam_frame_{index}.png")), png).unwrap();
        }
    }

    #[test]
    fn frames_must_match_the_plot_shape() {
        let spec = plot(
            r#"{"kind": "plot", "mapping": {"x": "x", "y": "y", "color": "kind"},
                "layers": [{"geom": "point"}]}"#,
        );
        let prepared = prepare_plot(&spec, &states()).unwrap();
        let error = prepared.with_layer_frames(vec![]).err().unwrap();
        assert!(error.to_string().contains("0 panels"), "{error}");
        let mut frame = prepared.layer_frames()[0][0].clone();
        // Rows cut but styles left whole.
        frame.data = frame.data.take(&state_rows(0));
        let error = prepared.with_layer_frames(vec![vec![frame]]).err().unwrap();
        assert!(error.to_string().contains("8 rows but styles"), "{error}");
    }
}
