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
//! draw frames through [`FrameSource`]. No tweening yet (step an2): frames show
//! whole states.

mod labels;
mod manual;
mod spec;
pub(crate) mod timeline;
mod transition;

pub use spec::{AnimateOptions, AnimationSpec};
pub use timeline::Timeline;

use crate::build::frame::LayerFrame;
use crate::build::{PlotData, PreparedPlot, prepare_plot_states};
use crate::layout::Titles;
use crate::render::FrameSource;
use crate::scene::Scene;
use crate::spec::PlotSpec;
use crate::{Error, Result};
use labels::{FRAME_VARIABLES, Template};
use timeline::{Settings, r_number};
use transition::Transition;

/// A planned animation of one plot.
pub struct Animation {
    prepared: PreparedPlot,
    transition: Box<dyn Transition>,
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
        let transition = transition::transition(name, options)?;
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
        Ok(Animation {
            rows: prepared.layer_frames(),
            state_rows: prepared.state_rows(),
            warnings: prepared.warnings.clone(),
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
        let states = self
            .transition
            .frame_states(frame, self.prepared.state_labels().len());
        let frames = self
            .rows
            .iter()
            .zip(&self.state_rows)
            .map(|(layers, layer_states)| {
                layers
                    .iter()
                    .zip(layer_states)
                    .map(|(all, by_state)| match by_state {
                        None => all.clone(),
                        Some(by_state) => {
                            let rows: Vec<u32> = states
                                .iter()
                                .filter_map(|state| by_state.get(*state))
                                .flatten()
                                .copied()
                                .collect();
                            all.take(&rows)
                        }
                    })
                    .collect()
            })
            .collect();
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
