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
//! `ease_aes()` ([`ease`]).

mod ease;
mod labels;
mod manual;
mod spec;
mod states;
pub(crate) mod timeline;
mod transition;
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
use ease::Easing;
use labels::{FRAME_VARIABLES, Template};
use timeline::{Settings, r_number};
use transition::{FrameShow, Transition};

/// A planned animation of one plot.
pub struct Animation {
    prepared: PreparedPlot,
    transition: Box<dyn Transition>,
    easing: Easing,
    timeline: Timeline,
    /// [tile][layer]: every prepared row.
    rows: Vec<Vec<LayerFrame>>,
    /// [tile][layer]: rows per state, `None` for a static layer.
    state_rows: Vec<Vec<Option<Vec<Vec<u32>>>>>,
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
        let settings = Settings::resolve(&animation.options)?;
        let prepared = prepare_plot_states(spec, data, Some(transition.state_variable()))?;
        let states = prepared.state_labels().len();
        if states == 0 {
            let variable = transition.state_variable();
            return Err(Error::spec(
                &variable.path,
                format!(
                    "the column '{}' has no values to animate (empty or all missing)",
                    variable.name
                ),
            ));
        }
        let timeline = settings.plan(|requested| transition.scene_frames(states, requested))?;
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
        warnings.extend(uneven_warnings(
            &prepared,
            transition.as_ref(),
            &timeline,
            &rows,
            &state_rows,
        ));
        Ok(Animation {
            rows,
            state_rows,
            warnings,
            easing,
            prepared,
            transition,
            timeline,
            templates,
            metadata: true,
        })
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
        values.extend(
            self.transition
                .frame_variables(frame, self.prepared.state_labels()),
        );
        values
    }

    /// Draw scene frame `frame` (0-based).
    fn scene_frame(&self, frame: usize) -> Result<Scene> {
        let show = self.transition.frame_show(frame);
        let mut frames = Vec::with_capacity(self.rows.len());
        for (layers, layer_states) in self.rows.iter().zip(&self.state_rows) {
            let mut tile = Vec::with_capacity(layers.len());
            for (all, by_state) in layers.iter().zip(layer_states) {
                let rows_of = |state: usize| -> &[u32] {
                    by_state
                        .as_ref()
                        .and_then(|by_state| by_state.get(state))
                        .map_or(&[], Vec::as_slice)
                };
                tile.push(match (by_state, &show) {
                    (None, _) => all.clone(),
                    (Some(_), FrameShow::States(states)) => {
                        let rows: Vec<u32> = states
                            .iter()
                            .flat_map(|state| rows_of(*state))
                            .copied()
                            .collect();
                        all.take(&rows)
                    }
                    (Some(_), FrameShow::Tween { from, to, progress }) => tween::tween(
                        &all.take(rows_of(*from)),
                        &all.take(rows_of(*to)),
                        *progress,
                        &self.easing,
                    )?,
                });
            }
            frames.push(tile);
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

/// One warning per layer that joins its rows (lines, polygons) and has a group
/// whose point count differs between two states it tweens: its points are
/// matched in order, so the extra ones appear or disappear at a state boundary.
fn uneven_warnings(
    prepared: &PreparedPlot,
    transition: &dyn Transition,
    timeline: &Timeline,
    rows: &[Vec<LayerFrame>],
    state_rows: &[Vec<Option<Vec<Vec<u32>>>>],
) -> Vec<String> {
    let mut pairs: Vec<(usize, usize)> = (0..timeline.scene_frames)
        .filter_map(|frame| match transition.frame_show(frame) {
            FrameShow::Tween { from, to, .. } => Some((from, to)),
            FrameShow::States(_) => None,
        })
        .collect();
    pairs.dedup();
    pairs.sort_unstable();
    pairs.dedup();
    let labels = prepared.state_labels();
    let mut warned: Vec<&str> = Vec::new();
    let mut warnings = Vec::new();
    for ((names, layers), layer_states) in prepared.layer_names().iter().zip(rows).zip(state_rows) {
        for ((geom, path), (all, by_state)) in names.iter().zip(layers.iter().zip(layer_states)) {
            let Some(by_state) = by_state else { continue };
            if !CONNECTED_GEOMS.contains(geom) || warned.contains(path) {
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
                         extra ones appear or disappear at the state boundary (gganimate \
                         adds points to match the shapes; ggrust does not yet)",
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
