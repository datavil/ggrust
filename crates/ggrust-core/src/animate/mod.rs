//! Animations (R gganimate, MIT): a plot with a transition becomes a sequence of
//! frames drawn by the normal layout and draw code.
//!
//! [`Animation::new`] prepares the plot once with the transition's state column
//! ([`crate::build::prepare_plot_states`]: stats and positions per state, scales
//! and legends trained once on all states), plans the output frames
//! ([`timeline`]) and parses the label templates ([`labels`]). Each scene frame
//! is then a copy of the prepared plot holding the rows of the states it shows
//! ([`crate::build::PreparedPlot::with_layer_frames`]), with its titles filled in.
//! Writers (`render::to_gif`, `render::to_apng`, `render::to_html_animation`)
//! draw frames through [`FrameSource`]. A frame shows whole states, or two
//! states tweened ([`tween`]) at the frame's progress with the plot's
//! `ease_aes()` ([`ease`]). Rows without a partner move in or out by the
//! plot's enter and exit effects ([`transmute`]); a shadow ([`shadow`]) adds
//! rows of other frames behind each frame's rows.

mod align;
mod along;
mod ease;
mod labels;
mod manual;
mod reveal;
mod shadow;
mod spec;
mod states;
mod time;
pub(crate) mod timeline;
mod transition;
mod transmute;
mod tween;

pub use spec::{AnimateOptions, AnimationSpec};
pub use timeline::Timeline;

use crate::build::frame::LayerFrame;
use crate::build::{PlotData, PreparedPlot, prepare_plot_states};
use crate::layout::Titles;
use crate::render::FrameSource;
use crate::scene::Scene;
use crate::spec::PlotSpec;
use crate::{Error, Result};
use align::Shape;
use along::TimeClass;
use ease::Easing;
use labels::{FRAME_VARIABLES, Template};
use reveal::RevealRows;
use shadow::{Shadow, stack};
use time::{TimeShow, kept_states, time_show};
use timeline::{Settings, r_number};
use transition::{FrameShow, StateSet, Transition};
use transmute::Transmuters;
use tween::{Drawn, Phase};

/// A planned animation of one plot.
pub struct Animation {
    prepared: PreparedPlot,
    transition: Box<dyn Transition>,
    easing: Easing,
    /// Enter and exit effects (`enter_fade()`, `exit_shrink()`, ...).
    transmuters: Transmuters,
    /// `shadow_wake()` or `shadow_mark()`.
    shadow: Option<Shadow>,
    /// Per scene frame: whether it shows a state as it is for the last time
    /// before moving on (tweenr's `raw` phase; the other state frames are
    /// `static`). Only frames that show whole states can be raw.
    raw_frames: Vec<bool>,
    /// [tile][layer]: the geom name and the user layer (0-based).
    layers: Vec<Vec<(&'static str, usize)>>,
    /// [tile][layer]: `shadow_mark()` rows, `(raw scene frame, rows)`.
    marks: Vec<Vec<MarkRows>>,
    timeline: Timeline,
    /// [tile][layer]: every prepared row.
    rows: Vec<Vec<LayerFrame>>,
    /// [tile][layer]: rows per state, `None` for a static layer.
    state_rows: Vec<Vec<Option<Vec<Vec<u32>>>>>,
    /// [tile][layer]: what `transition_time` / `transition_reveal` read per
    /// frame; empty for the other transitions, `None` for a static layer.
    along_layers: Vec<Vec<Option<AlongLayer>>>,
    /// How every state prints in labels (`{current_frame}`, `{closest_state}`,
    /// ...): dates and date-times formatted as `{frame_time}` is.
    state_labels: Vec<String>,
    /// [tile][layer]: how the layer's rows move between states.
    shapes: Vec<Vec<Shape>>,
    /// Title, subtitle and caption templates (`None`: no such title).
    templates: [Option<Template>; 3],
    /// Things the user should know (from preparing the plot).
    pub warnings: Vec<String>,
    /// Record `ggrust vX.Y.Z` in each frame's output (`Scene::metadata`).
    pub metadata: bool,
}

impl Animation {
    /// Prepare `spec` (which must have an `animation` key) and plan its frames.
    ///
    /// # Errors
    ///
    /// Spec errors for the animation options, the transition, the label
    /// templates or a plot ggrust cannot animate yet (insets); any error of
    /// preparing the plot.
    pub fn new(spec: &PlotSpec, data: &PlotData) -> Result<Animation> {
        let animation = animation_spec(spec)?;
        if !spec.insets.is_empty() {
            return Err(Error::spec(
                "insets",
                "inset_element() is not supported in an animation yet; draw the plot \
                 without insets",
            ));
        }
        let (name, options) = &animation.transition;
        let mut transition = transition::transition(name, options)?;
        let easing = Easing::parse(animation.ease.as_ref())?;
        let transmuters = Transmuters::parse(animation.enter.as_ref(), animation.exit.as_ref())?;
        let shadow = animation.shadow.as_ref().map(Shadow::parse).transpose()?;
        if let (Some(Shadow::Mark(_)), FrameShow::Reveal { .. }) =
            (&shadow, transition.frame_show(0))
        {
            return Err(Error::spec(
                "animation.shadow",
                "shadow_mark() needs frames that show whole states; transition_reveal() \
                 has none. Use shadow_wake(), or transition_states() / transition_time()",
            ));
        }
        let settings = Settings::resolve(&animation.options)?;
        let prepared = prepare_plot_states(spec, data, Some(transition.state_variable()))?;
        let states = StateSet {
            count: prepared.state_labels().len(),
            values: prepared.state_values(),
            class: TimeClass::of(spec, &transition.state_variable().name)
                .settled(&settle_values(prepared.state_values(), options)),
        };
        if states.count == 0 {
            let variable = transition.state_variable();
            return Err(Error::spec(
                &variable.path,
                format!(
                    "the column '{}' has no values to animate (empty or all missing)",
                    variable.name
                ),
            ));
        }
        let timeline = settings.plan(|requested| transition.scene_frames(&states, requested))?;
        let state_labels = state_labels(&prepared, states.class);
        let names: Vec<&str> = FRAME_VARIABLES
            .iter()
            .chain(transition.variable_names())
            .copied()
            .collect();
        let titles = prepared.titles();
        let template = |text: &Option<String>, path: &str| {
            text.as_deref()
                .map(|text| Template::parse(text, path, &names))
                .transpose()
        };
        let templates = [
            template(&titles.title, "ggtitle.text")?,
            template(&titles.subtitle, "ggtitle.subtitle")?,
            template(&titles.caption, "caption.text")?,
        ];
        let rows = prepared.layer_frames();
        let state_rows = prepared.state_rows();
        let mut warnings = prepared.warnings.clone();
        let along_layers = along_layers(&prepared, transition.as_ref(), &rows, &state_rows);
        if let FrameShow::Time { .. } = transition.frame_show(0) {
            warnings.extend(time_warnings(
                &prepared,
                transition.as_ref(),
                &along_layers,
                &state_rows,
            ));
        }
        warnings.extend(uneven_warnings(
            &prepared,
            transition.as_ref(),
            &timeline,
            &along_layers,
            &rows,
            &state_rows,
            &state_labels,
        ));
        let shapes = prepared
            .layer_names()
            .iter()
            .map(|names| names.iter().map(|(geom, _)| Shape::of(geom)).collect())
            .collect();
        let raw_frames: Vec<bool> = (0..timeline.scene_frames)
            .map(|frame| match transition.frame_show(frame) {
                FrameShow::States(states) => {
                    frame + 1 >= timeline.scene_frames
                        || transition.frame_show(frame + 1) != FrameShow::States(states)
                }
                _ => false,
            })
            .collect();
        let layers = prepared
            .layer_names()
            .iter()
            .map(|names| {
                names
                    .iter()
                    .map(|(geom, path)| (*geom, user_layer(path)))
                    .collect()
            })
            .collect();
        let mut animation = Animation {
            state_labels,
            shapes,
            rows,
            state_rows,
            along_layers,
            warnings,
            easing,
            transmuters,
            shadow,
            raw_frames,
            layers,
            marks: Vec::new(),
            prepared,
            transition,
            timeline,
            templates,
            metadata: true,
        };
        animation.marks = animation.mark_rows()?;
        Ok(animation)
    }

    /// [tile][layer] `shadow_mark()` rows: the rows of every raw frame (see
    /// [`Self::raw_frames`]; `transition_time`: the frames of a layer's times),
    /// with the mark's style overrides. Empty without a mark.
    fn mark_rows(&self) -> Result<Vec<Vec<MarkRows>>> {
        let Some(Shadow::Mark(mark)) = &self.shadow else {
            return Ok(Vec::new());
        };
        let mut tiles = Vec::with_capacity(self.rows.len());
        for (tile, layers) in self.rows.iter().enumerate() {
            let mut marks = Vec::with_capacity(layers.len());
            for layer in 0..layers.len() {
                let mut raws = Vec::new();
                if !self.shadow_excludes(tile, layer) {
                    for frame in 0..self.timeline.scene_frames {
                        let raw = match self.transition.frame_show(frame) {
                            FrameShow::States(_) => self.raw_frames[frame],
                            FrameShow::Time { position } => matches!(
                                time_show(self.time_anchors(tile, layer), position),
                                TimeShow::State(_)
                            ),
                            FrameShow::Tween { .. } | FrameShow::Reveal { .. } => false,
                        };
                        if !raw {
                            continue;
                        }
                        if let Some(drawn) = self.drawn(tile, layer, frame)? {
                            let mut rows = drawn.frame;
                            mark.restyle(&mut rows)?;
                            raws.push((frame, rows));
                        }
                    }
                }
                marks.push(raws);
            }
            tiles.push(marks);
        }
        Ok(tiles)
    }

    /// Whether layer `layer` of tile `tile` never gets a shadow: a static layer
    /// or one in the shadow's `exclude_layer`.
    fn shadow_excludes(&self, tile: usize, layer: usize) -> bool {
        let is_static = self
            .state_rows
            .get(tile)
            .and_then(|layers| layers.get(layer))
            .is_none_or(Option::is_none);
        let user_layer = self
            .layers
            .get(tile)
            .and_then(|layers| layers.get(layer))
            .map_or(0, |(_, user)| *user);
        is_static
            || self
                .shadow
                .as_ref()
                .is_some_and(|shadow| shadow.excludes(user_layer))
    }

    /// `transition_time` anchors of a layer (empty for other transitions).
    fn time_anchors(&self, tile: usize, layer: usize) -> &[(i64, usize)] {
        match self
            .along_layers
            .get(tile)
            .and_then(|layers| layers.get(layer))
            .and_then(Option::as_ref)
        {
            Some(AlongLayer::Time(anchors)) => anchors.as_slice(),
            _ => &[],
        }
    }

    /// The planned output frames and speed.
    pub fn timeline(&self) -> &Timeline {
        &self.timeline
    }

    /// The scene of output frame `frame` (0-based).
    ///
    /// # Errors
    ///
    /// A spec error at `animation.frame` past the last frame; any draw error.
    pub fn output_scene(&self, frame: usize) -> Result<Scene> {
        match self.timeline.frames.get(frame) {
            Some(scene_frame) => self.scene_frame(*scene_frame),
            None => Err(Error::spec(
                "animation.frame",
                format!(
                    "frame {frame} is past the last frame; the animation has {} frames \
                     (0 to {})",
                    self.timeline.frames.len(),
                    self.timeline.frames.len().saturating_sub(1)
                ),
            )),
        }
    }

    /// Label variables of scene frame `frame` (0-based): `frame` (1-based),
    /// `nframes`, `progress` and the transition's.
    pub fn frame_variables(&self, frame: usize) -> Vec<(&'static str, String)> {
        let count = self.timeline.scene_frames;
        let mut values = vec![
            ("frame", (frame + 1).to_string()),
            ("nframes", count.to_string()),
            ("progress", r_number((frame + 1) as f64 / count as f64)),
        ];
        values.extend(self.transition.frame_variables(frame, &self.state_labels));
        values
    }

    /// The rows layer `layer` of tile `tile` draws in scene frame `frame`
    /// (before any shadow), with every row's identity and phase; `None` for a
    /// static layer (drawn whole in every frame).
    fn drawn(&self, tile: usize, layer: usize, frame: usize) -> Result<Option<Drawn>> {
        let (Some(all), Some(Some(by_state))) = (
            self.rows.get(tile).and_then(|layers| layers.get(layer)),
            self.state_rows
                .get(tile)
                .and_then(|layers| layers.get(layer)),
        ) else {
            return Ok(None);
        };
        let geom = self
            .layers
            .get(tile)
            .and_then(|layers| layers.get(layer))
            .map_or("", |(geom, _)| *geom);
        let rows_of = |state: usize| -> &[u32] { by_state.get(state).map_or(&[], Vec::as_slice) };
        let shape = self
            .shapes
            .get(tile)
            .and_then(|shapes| shapes.get(layer))
            .copied()
            .unwrap_or(Shape::Points);
        let tween = |from: usize, to: usize, progress: f64| {
            tween::tween_rows(
                &all.take(rows_of(from)),
                &all.take(rows_of(to)),
                progress,
                &self.easing,
                &self.transmuters,
                geom,
                shape,
            )
        };
        Ok(Some(match self.transition.frame_show(frame) {
            FrameShow::States(states) => {
                let rows: Vec<u32> = states
                    .iter()
                    .flat_map(|state| rows_of(*state))
                    .copied()
                    .collect();
                let phase = match self.raw_frames.get(frame) {
                    Some(true) => Phase::Raw,
                    _ => Phase::Static,
                };
                Drawn::as_is(all.take(&rows), phase)
            }
            FrameShow::Tween { from, to, progress } => tween(from, to, progress)?,
            FrameShow::Time { position } => {
                match time_show(self.time_anchors(tile, layer), position) {
                    TimeShow::Nothing => Drawn::as_is(all.take(&[]), Phase::Raw),
                    TimeShow::State(state) => Drawn::as_is(all.take(rows_of(state)), Phase::Raw),
                    TimeShow::Tween { from, to, progress } => tween(from, to, progress)?,
                }
            }
            FrameShow::Reveal {
                position,
                keep_last,
            } => {
                let along = self
                    .along_layers
                    .get(tile)
                    .and_then(|layers| layers.get(layer))
                    .and_then(Option::as_ref);
                match along {
                    Some(AlongLayer::Reveal { rows, connected }) => {
                        let rows = rows.frame(position, keep_last, *connected);
                        let phases = rows
                            .iter()
                            .map(|(from, to, progress)| match from != to && *progress > 0.0 {
                                true => Phase::Transition,
                                false => Phase::Raw,
                            })
                            .collect();
                        let mut drawn =
                            Drawn::as_is(tween::mix_rows(all, &rows, &self.easing)?, Phase::Raw);
                        drawn.phases = phases;
                        drawn
                    }
                    _ => Drawn::as_is(all.take(&[]), Phase::Raw),
                }
            }
        }))
    }

    /// `drawn` with the plot's shadow of layer `layer` of tile `tile` at scene
    /// frame `frame`.
    fn shadowed(
        &self,
        tile: usize,
        layer: usize,
        frame: usize,
        drawn: Drawn,
    ) -> Result<LayerFrame> {
        if self.shadow_excludes(tile, layer) {
            return Ok(drawn.frame);
        }
        match &self.shadow {
            None => Ok(drawn.frame),
            Some(Shadow::Wake(wake)) => {
                let geom = self.layers[tile][layer].0;
                let mut frames = Vec::new();
                for (past, at) in wake.frames(frame, self.timeline.scene_frames) {
                    if let Some(mut shadow) = self.drawn(tile, layer, past)? {
                        wake.fade(&mut shadow.frame, at, geom)?;
                        frames.push(shadow);
                    }
                }
                wake.compose(frames, drawn)
            }
            Some(Shadow::Mark(mark)) => {
                let mut parts: Vec<LayerFrame> = self
                    .marks
                    .get(tile)
                    .and_then(|layers| layers.get(layer))
                    .into_iter()
                    .flatten()
                    .filter(|(raw, rows)| mark.shows(*raw, frame) && rows.data.row_count() > 0)
                    .map(|(_, rows)| rows.clone())
                    .collect();
                if parts.is_empty() {
                    return Ok(drawn.frame);
                }
                parts.push(drawn.frame);
                stack(parts)
            }
        }
    }

    /// Draw scene frame `frame` (0-based).
    fn scene_frame(&self, frame: usize) -> Result<Scene> {
        let mut frames = Vec::with_capacity(self.rows.len());
        for (tile, layers) in self.rows.iter().enumerate() {
            let mut drawn_layers = Vec::with_capacity(layers.len());
            for (layer, all) in layers.iter().enumerate() {
                drawn_layers.push(match self.drawn(tile, layer, frame)? {
                    None => all.clone(),
                    Some(drawn) => self.shadowed(tile, layer, frame, drawn)?,
                });
            }
            frames.push(drawn_layers);
        }
        let mut plot = self.prepared.with_layer_frames(frames)?;
        if self.templates.iter().any(Option::is_some) {
            let values = self.frame_variables(frame);
            let fill = |template: &Option<Template>, fallback: &Option<String>| match template {
                Some(template) => Some(template.fill(&values)),
                None => fallback.clone(),
            };
            let titles = plot.titles();
            let filled = Titles {
                title: fill(&self.templates[0], &titles.title),
                subtitle: fill(&self.templates[1], &titles.subtitle),
                caption: fill(&self.templates[2], &titles.caption),
                ..titles.clone()
            };
            plot.set_titles(filled);
        }
        let mut scene = plot.frame_scene()?;
        scene.metadata = self.metadata;
        Ok(scene)
    }
}

impl FrameSource for Animation {
    fn scene(&self, frame: usize) -> Result<Scene> {
        self.scene_frame(frame)
    }

    fn timeline(&self) -> &[usize] {
        &self.timeline.frames
    }

    fn fps(&self) -> f64 {
        self.timeline.fps
    }
}

/// How every state of `prepared` prints in labels: numeric states of a date,
/// date-time or time-of-day column formatted as `{frame_time}` is, the
/// prepared labels otherwise.
fn state_labels(prepared: &PreparedPlot, class: TimeClass) -> Vec<String> {
    match (prepared.state_values(), class) {
        (Some(values), TimeClass::Date | TimeClass::DateTime { .. } | TimeClass::TimeOfDay) => {
            values.iter().map(|value| class.label(*value)).collect()
        }
        _ => prepared.state_labels().to_vec(),
    }
}

/// `shadow_mark()` rows of one layer: `(raw scene frame, rows)`.
type MarkRows = Vec<(usize, LayerFrame)>;

/// The user layer (0-based) of a prepared layer from its spec path
/// (`layers[2]...`); 0 when the path has no index.
fn user_layer(path: &str) -> usize {
    path.strip_prefix("layers[")
        .and_then(|rest| rest.split(']').next())
        .and_then(|index| index.parse().ok())
        .unwrap_or(0)
}

/// Per-layer data of the time-based transitions, planned once.
enum AlongLayer {
    /// `transition_time`: `(frame position, state)` of the layer's kept states
    /// with rows in the tile, in time order (see [`time::kept_states`]).
    Time(Vec<(i64, usize)>),
    /// `transition_reveal`: the rows in reveal order, and whether the geom joins
    /// its rows (lines keep the rows already revealed).
    Reveal { rows: RevealRows, connected: bool },
}

/// [tile][layer] [`AlongLayer`]s for `transition_time` and
/// `transition_reveal`; empty for the other transitions.
fn along_layers(
    prepared: &PreparedPlot,
    transition: &dyn Transition,
    rows: &[Vec<LayerFrame>],
    state_rows: &[Vec<Option<Vec<Vec<u32>>>>],
) -> Vec<Vec<Option<AlongLayer>>> {
    let positions = transition.state_positions();
    match transition.frame_show(0) {
        FrameShow::Time { .. } => {
            let layer_count = state_rows.first().map_or(0, Vec::len);
            let kept: Vec<Vec<bool>> = (0..layer_count)
                .map(|layer| {
                    let present: Vec<bool> = (0..positions.len())
                        .map(|state| {
                            state_rows.iter().any(|tile| {
                                tile.get(layer)
                                    .and_then(Option::as_ref)
                                    .and_then(|by_state| by_state.get(state))
                                    .is_some_and(|rows| !rows.is_empty())
                            })
                        })
                        .collect();
                    kept_states(&present, positions)
                })
                .collect();
            state_rows
                .iter()
                .map(|tile| {
                    tile.iter()
                        .zip(&kept)
                        .map(|(by_state, kept)| {
                            let by_state = by_state.as_ref()?;
                            let anchors = by_state
                                .iter()
                                .enumerate()
                                .filter(|(state, rows)| kept[*state] && !rows.is_empty())
                                .map(|(state, _)| (positions[state], state))
                                .collect();
                            Some(AlongLayer::Time(anchors))
                        })
                        .collect()
                })
                .collect()
        }
        FrameShow::Reveal { .. } => prepared
            .layer_names()
            .iter()
            .zip(rows)
            .zip(state_rows)
            .map(|((names, layers), tile)| {
                names
                    .iter()
                    .zip(layers)
                    .zip(tile)
                    .map(|(((geom, _), all), by_state)| {
                        Some(AlongLayer::Reveal {
                            rows: RevealRows::new(all, by_state.as_ref()?, positions),
                            connected: CONNECTED_GEOMS.contains(geom),
                        })
                    })
                    .collect()
            })
            .collect(),
        FrameShow::States(_) | FrameShow::Tween { .. } => Vec::new(),
    }
}

/// `transition_time`: one warning per layer with times that round to a frame
/// an earlier time of the layer holds (their rows are never drawn).
fn time_warnings(
    prepared: &PreparedPlot,
    transition: &dyn Transition,
    along_layers: &[Vec<Option<AlongLayer>>],
    state_rows: &[Vec<Option<Vec<Vec<u32>>>>],
) -> Vec<String> {
    let names = prepared.layer_names();
    let Some(first) = names.first() else {
        return Vec::new();
    };
    let variable = &transition.state_variable().name;
    let mut warnings = Vec::new();
    for (layer, (geom, path)) in first.iter().enumerate() {
        let mut times = std::collections::BTreeSet::new();
        let mut drawn = std::collections::BTreeSet::new();
        for (tile, along) in state_rows.iter().zip(along_layers) {
            if let Some(Some(by_state)) = tile.get(layer) {
                times.extend(
                    by_state
                        .iter()
                        .enumerate()
                        .filter(|(_, rows)| !rows.is_empty())
                        .map(|(state, _)| state),
                );
            }
            if let Some(Some(AlongLayer::Time(anchors))) = along.get(layer) {
                drawn.extend(anchors.iter().map(|(_, state)| *state));
            }
        }
        let dropped = times.len() - drawn.len().min(times.len());
        if dropped > 0 {
            warnings.push(format!(
                "{path} (geom_{geom}): {dropped} of {} values of '{variable}' round to a \
                 frame an earlier value already holds and are not drawn (as in \
                 gganimate); raise nframes to draw more of them",
                times.len()
            ));
        }
    }
    warnings
}

/// Geoms that join their rows into one mark per group.
const CONNECTED_GEOMS: &[&str] = &[
    "line",
    "path",
    "step",
    "polygon",
    "area",
    "ribbon",
    "density",
    "area_ridges",
    "freqpoly",
    "violin",
    "half_violin",
    "smooth",
    "band",
    "slab",
    "streamline",
];

/// One warning per layer that joins its rows but is not aligned as a path or
/// polygon ([`Shape`]: areas, ribbons, densities, ...) and has a group whose
/// point count differs between two states it tweens: its points are matched in
/// order, so the extra ones appear or disappear at a state boundary.
fn uneven_warnings(
    prepared: &PreparedPlot,
    transition: &dyn Transition,
    timeline: &Timeline,
    along_layers: &[Vec<Option<AlongLayer>>],
    rows: &[Vec<LayerFrame>],
    state_rows: &[Vec<Option<Vec<Vec<u32>>>>],
    labels: &[String],
) -> Vec<String> {
    let mut pairs: Vec<(usize, usize)> = (0..timeline.scene_frames)
        .filter_map(|frame| match transition.frame_show(frame) {
            FrameShow::Tween { from, to, .. } => Some((from, to)),
            FrameShow::States(_) | FrameShow::Time { .. } | FrameShow::Reveal { .. } => None,
        })
        .collect();
    // transition_time tweens each layer between its consecutive times.
    for tile in along_layers {
        for layer in tile.iter().flatten() {
            if let AlongLayer::Time(anchors) = layer {
                pairs.extend(anchors.windows(2).map(|pair| (pair[0].1, pair[1].1)));
            }
        }
    }
    pairs.dedup();
    pairs.sort_unstable();
    pairs.dedup();
    let mut warned: Vec<&str> = Vec::new();
    let mut warnings = Vec::new();
    for ((names, layers), layer_states) in prepared.layer_names().iter().zip(rows).zip(state_rows) {
        for ((geom, path), (all, by_state)) in names.iter().zip(layers.iter().zip(layer_states)) {
            let Some(by_state) = by_state else { continue };
            if !CONNECTED_GEOMS.contains(geom)
                || Shape::of(geom) != Shape::Points
                || warned.contains(path)
            {
                continue;
            }
            let rows_of = |state: usize| by_state.get(state).map_or(&[][..], Vec::as_slice);
            for (from, to) in &pairs {
                let uneven =
                    tween::uneven_groups(&all.take(rows_of(*from)), &all.take(rows_of(*to)));
                if let Some((_, from_count, to_count)) = uneven.first() {
                    let label = |state: usize| labels.get(state).map_or("", String::as_str);
                    warnings.push(format!(
                        "{path} (geom_{geom}): a group has {from_count} points in state '{}' \
                         and {to_count} in state '{}'; points are matched in order and the \
                         extra ones appear or disappear at the state boundary (ggrust adds points \
                         to lines and polygons only)",
                        label(*from),
                        label(*to)
                    ));
                    warned.push(path);
                    break;
                }
            }
        }
    }
    warnings
}

/// The values that choose a date-time column's label format: the states and
/// the ends of `range=` (a range end with a time shows the time in labels).
fn settle_values(
    states: Option<&[f64]>,
    options: &serde_json::Map<String, serde_json::Value>,
) -> Vec<f64> {
    let range = along::parse_range(options, "animation.transition")
        .ok()
        .flatten()
        .unwrap_or([f64::NAN; 2]);
    states.unwrap_or(&[]).iter().copied().chain(range).collect()
}

/// The parsed `animation` key of `spec`.
///
/// # Errors
///
/// A spec error when the plot has no transition or the key is malformed.
pub fn animation_spec(spec: &PlotSpec) -> Result<AnimationSpec> {
    match &spec.animation {
        Some(value) => AnimationSpec::parse(value),
        None => Err(Error::spec(
            "animation",
            "the plot has no transition to animate; add one, e.g. \
             `+ transition_manual(\"year\")`",
        )),
    }
}

#[cfg(test)]
mod tests;
