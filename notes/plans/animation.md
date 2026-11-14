# Animation plan (gganimate)

Agreed with the user 2026-11-14. Requested by the user (moves gganimate out of Tier 3 "only on request"
in [extensions.md](extensions.md)). Rules: [AGENTS.md](../../AGENTS.md). Roadmap:
[plan.md](../../plan.md).

## Reference and rules

- **No lets-plot reference.** lets-plot has no animation. The reference is R
  [gganimate](https://gganimate.com) 1.0.x on top of ggplot2, with tweenr for interpolation.
  Both are MIT, so reading their source for algorithms is fine (still no line-by-line port).
- **Every step is a deviation** ("ggrust extension, lets-plot has none, ggplot2 needs
  gganimate"): one row in [deviations.md](../../deviations.md) per step, plus a row in the
  `plan.md` decisions table for the design choices below.
- **Names follow gganimate**, in Python form, composed with `+` like everything else:
  `p + transition_states("species") + ease_aes("cubic-in-out") + enter_fade()`.
  No new syntax beyond what gganimate has.
- **Parity target:** same frames as gganimate for the same spec and `nframes`: which state
  each frame shows, the tween progress, eased values, label variables. Tested against
  gganimate's `frame_vars()` and the tweened layer data from an R fixture script
  (`tests/fixtures/generate_animation_reference.R`), as done for ggpubr.

## Python API (target)

```python
from ggrust import *

p = (
    ggplot(gapminder, aes("gdp", "life_exp", size="pop", color="continent"))
    + geom_point(alpha=0.7)
    + scale_x_log10()
    + labs(title="Year: {frame_time}")
    + transition_time("year")
    + ease_aes("linear")
)

p  # notebook: animated HTML player
anim = animate(p, nframes=100, fps=10)  # Animation object, _repr_html_
anim_save("life.gif", anim)  # gif | png (APNG) | html
ggsave(p, "life.gif")  # same, default animate() settings
```

- `animate(plot, nframes=100, fps=10, duration=None, detail=1, start_pause=0,
  end_pause=0, rewind=False, width=None, height=None, dpi=None)`: gganimate's arguments
  minus `renderer`/`device` (format comes from the file name or display).
- `Animation`: frozen result; `.frame(i) -> PlotSpec`-like still for inspection,
  `.save(path)`, `_repr_html_`. One return type.
- Label templates in `labs()`/`ggtitle()`: `{frame}`, `{nframes}`, `{progress}`, plus each
  transition's variables (`{closest_state}`, `{previous_state}`, `{next_state}`,
  `{transitioning}`, `{frame_time}`, `{frame_along}`, `{current_frame}`). gganimate
  evaluates R glue; we support the variable names only, unknown names are a `SpecError`
  with a close match.
- `as_dict()` gains an `"animation"` key (transition, ease, enter/exit, shadow, view).
  Plot files (`.ggrs`) carry it.

## Engine design

```
prepare_plot (as today): map -> stat -> position -> train scales on ALL rows
  -> animate::plan     frame i -> (state a, state b, progress t)      [frames.rs]
  -> animate::tween    match rows a<->b by group, interpolate, ease   [tween.rs]
                       enter/exit rows, shadows                       [enter_exit.rs, shadow.rs]
  -> per frame (rayon): existing draw + layout -> Scene               (reuse, no copies of layout code)
  -> render: GIF | APNG | HTML player | single frame SVG/PNG
```

- New module `crates/ggrust-core/src/animate/` (spec, frames, tween, ease, one file per
  transition behind a small `Transition` trait, registry like geoms/stats).
- Stats run **once per state on the whole data**, as gganimate does (it tweens built layer
  data, it does not re-run stats per frame). Scales are trained once on all states, so
  axes and legends stay fixed (gganimate's default `view_static`).
- Interpolation is per column type: numbers linear (eased), colours in a perceptual space
  (tweenr uses Lab), discrete values switch at `t = 0.5`. Rows are matched by `group`
  (gganimate's rule; explicit `aes(group=)` decides identity).
- **The seam.** `build.rs` is 6271 lines. The plan assumes a clean split between
  "computed + scaled layer data" and "draw + layout". Step an0 checks that this seam
  exists. If animation needs edits across many unrelated files, stop and show the user
  the design before writing code (AGENTS.md).

### Outputs

| Output | How | Notes |
|---|---|---|
| HTML / notebook | Rust renders each frame's scene; one HTML page with a small player in `js/ggrust.js` (play/pause, slider, fps) | Default for `_repr_html_`. Frames as SVG strings, or PNG when the scene is heavy (canvas points). Tooltips off in animations (tier 1). |
| GIF | `gif` crate + per-frame palette quantization | New deps (`gif`, `color_quant`, both MIT/Apache): needs the user's OK. |
| APNG | `png` crate (already a dependency, has APNG support) | Truecolour, no quantization, small. |
| Single frame | `anim.frame(i)` -> SVG/PNG/PDF as today | Used by golden tests. |
| MP4 / WebM | Not planned | Would need ffmpeg; gganimate uses av/ffmpeg for this. |

## Steps

Each step: one commit on `main`, tests, rendered output inspected, bug-hunter pass,
deviations row, plan.md tick.

- [x] **an0 Spike: find the seam.** Read `prepare_plot` and the draw path, prove a frame
  can be drawn from modified computed layer data with fixed scales (throwaway test).
  Output: a short design note in this file (data model of a "state", where tweening
  plugs in). Stop and show the user if the seam is not clean.
- [ ] **an1 Frames without tweening (80 percent of the value).** `transition_manual(frames,
  cumulative=False)`, `animate()`, `Animation`, `anim_save`, `ggsave(".gif"/".png")`,
  HTML player, GIF + APNG writers, `{frame}`/`{nframes}`/`{progress}`/`{current_frame}`
  labels, `start_pause`/`end_pause`/`rewind`. Frames rendered in parallel.
  Goldens: frames 0 and last; GIF decoded and compared per frame (not bytes).
- [x] **an2 `transition_states` + tweening.** `transition_length`, `state_length`, `wrap`;
  `ease_aes()` with gganimate's easing names (linear, quadratic, cubic, quartic, quintic,
  sine, circular, exponential, elastic, back, bounce; `-in`, `-out`, `-in-out`), per
  aesthetic (`ease_aes(y="bounce-out")`). State label variables. Tween parity vs tweenr
  fixtures.
- [ ] **an3 `transition_time` and `transition_reveal`.** Time: frames spaced in time, rows
  tweened between observed times, `{frame_time}`. Reveal: paths/lines/areas grow along
  `along`, with the interpolated end point (gganimate's `keep_last`), `{frame_along}`.
- [ ] **an4 (on request) Enter/exit and shadows.** `enter_fade`/`exit_fade`, `enter_grow`/`exit_shrink`,
  `enter_appear`/`exit_disappear`, `enter_fly`/`exit_fly`, `enter_drift`/`exit_drift`,
  `enter_recolor`/`exit_recolor`; `shadow_wake`, `shadow_trail`, `shadow_mark`.
- [ ] **an5 (on request) Views.** `view_follow(fixed_x, fixed_y)`, `view_step`, `view_zoom`: per-frame
  position scales (retrain per frame instead of once). Separate step because it changes
  what is trained when.
- [ ] **an6 (on request) Performance pass.** Time box agreed with the user first. Reference: R
  gganimate + gifski on the same spec and frame count (report honestly, AGENTS rule 8).
- [ ] **On request only:** `transition_filter`, `transition_layers`, `transition_events`,
  `transition_components`, `transition_null`, facets inside animations if they need more
  than an1 gives for free, tooltips inside the HTML player, MP4.

## Decisions (user, 2026-11-14)

1. Outputs for an1: HTML player, GIF and APNG. GIF adds the `gif` and `color_quant`
   crates. No MP4.
2. A plot with a transition animates when displayed in a notebook (default `animate()`
   settings), as gganimate's `print` does.
3. HTML frames are SVG by default, PNG when the scene uses the canvas path (dense points).
4. Scope: an0 to an3 are scheduled (plan.md Phase 13). an4, an5 and an6 move to
   "on request".

## Design note (an0)

Done 2026-11-14. **The seam is clean**: no refactor of `build.rs` is needed.

### Where it is

`prepare_plot` (`build.rs`) returns a `PreparedPlot`: per tile (facet panel) a list of
`PreparedLayer` plus the tile's trained x and y `PositionScale`, and plot-wide the theme,
titles, legend boxes, coord, facets, side bands, 3D space and polar view. Everything
after it reads only that struct:

- `PreparedPlot::arrange(area, panel_min_size) -> Arrangement` reads scales, legends,
  titles and theme. It never reads layer rows.
- `PreparedPlot::draw(&Arrangement, interactive) -> Vec<Node>` reads each layer's rows
  only through `PreparedLayer::data` (`LayerData`, computed columns in data space after
  stat and position) and `PreparedLayer::styles` (`Styles`, colors/sizes/shapes per row,
  already mapped by the trained style scales). The geom draws them into the panel.
- Row-indexed extras (`unadjusted`, `tooltip_columns`, `keys`) feed only tooltips and
  the interactive legend, which animations do not have (tier 1).

So a frame is a copy of the prepared plot whose layers hold other rows, then the normal
`arrange` + `draw`. Proven by `build/frame.rs` (tests
`frames_redraw_changed_rows_with_fixed_axes_and_legends`, `frames_must_match_the_plot_shape`):
one point plot with a discrete color and a continuous size legend, three frames (state 1
rows, state 1 halfway to state 2, state 3 rows). Axes, grid, titles and legends are the
same nodes as the static plot of all rows; the marks differ; PNGs in
`tests/visual/output/animate_seam_frame_{0,1,2}.png`.

### What an0 added (all `pub(crate)`, in `crates/ggrust-core/src/build/frame.rs`)

```rust
pub(crate) struct LayerFrame { pub data: LayerData, pub styles: Styles }
impl LayerFrame { pub(crate) fn take(&self, rows: &[u32]) -> LayerFrame }

impl PreparedPlot {
    /// [tile][layer], all prepared rows.
    pub(crate) fn layer_frames(&self) -> Vec<Vec<LayerFrame>>;
    /// Copy with these rows; scales, legends, titles unchanged. Shape-checked.
    pub(crate) fn with_layer_frames(&self, frames: Vec<Vec<LayerFrame>>) -> Result<PreparedPlot>;
    /// arrange at `size` + draw, no tooltips (what build_plot does for static output).
    pub(crate) fn frame_scene(&self) -> Result<Scene>;
}
```

Plus `Clone` derives on `PreparedLayer`, `LayerTooltipSetup`, `Space3d`, `LegendBox`,
`Legend`, `LegendKey`, `KeyLayer`, `Colorbar` (attribute-only), and `Styles::fits(rows)`.
`frame.rs` has `expect(dead_code)` outside tests until `animate/` calls it: an1 removes
that attribute (the build fails when it becomes stale).

### Data model

- **State**: one level of the transition variable (`transition_manual(frames)`'s
  column, `transition_states(states)`'s column, a time value). Its rows are the computed
  rows (after stat, position and style) of every (tile, layer) that came from input rows
  of that level. Stats and positions run per state (a stack or a bin never mixes states),
  scales and legends are trained once on all states.
- **Frame**: for every (tile, layer) a `LayerFrame`. Without tweening (an1) it is the
  state's rows (`LayerFrame::take`), or all states up to it for `cumulative=True`. With
  tweening (an2) it is the rows of two states matched by `group` and interpolated at the
  eased progress, plus entering/exiting rows (an4).
- Tween **resolved styles**, not data values: `Styles` already holds hex colors and
  pixel sizes, as gganimate tweens built layer data (colors are hex there too). Numbers
  linear, colors in Lab, shapes/linetypes/texts switch at `t = 0.5`. The an0 halfway
  frame tweens x and y only, so its points keep state 1 sizes; an2 tweens styles too.

### How the state reaches computed rows (an1 builds this)

Facets already solve the same problem ("split rows, compute per part, train scales on
all"): `map_layer` carries extra columns (`facet_variables` -> `MappedLayer::facet_columns`),
then `split_layer(layer, rows)` cuts each tile's `MappedLayer` before `compute_layer`
(stat + position). Proposal for an1, all inside `build.rs` around the facet split
(lines ~560-710 today) plus the new module:

1. `prepare_plot` takes the transition variable (from `spec.animation`) and adds it to
   the carried columns like a facet variable.
2. Per tile, `split_layer` each layer once per state, `compute_layer` each part
   (the parts run in parallel with the tiles), then concatenate the parts back into one
   computed layer per tile (needs a `LayerData` concat; `Column::concat` exists in
   `data/column.rs`), recording per row its state index.
3. `PreparedLayer` gains `state_index: Option<Vec<u32>>` (one per row, `None` without
   animation). Scale training and styling then run unchanged on all rows.
4. `animate/` reads states through one accessor and builds frames:

```rust
// build/frame.rs (extends an0)
impl PreparedPlot {
    /// [tile][layer]: rows of each state, as indices into the prepared rows.
    pub(crate) fn state_rows(&self) -> Vec<Vec<Vec<Vec<u32>>>>;
    /// Copy with other titles (label templates: {frame}, {current_frame}, ...).
    pub(crate) fn with_titles(&self, titles: Titles) -> PreparedPlot; // or a parameter
}

// animate/mod.rs
pub struct AnimationSpec { transition: TransitionSpec, nframes: usize, fps: f64, ... }
pub(crate) trait Transition: Sync {
    /// Which state(s) and progress each frame shows.
    fn plan(&self, states: usize, options: &AnimationSpec) -> Vec<FramePlan>;
}
pub(crate) struct FramePlan { pub from: usize, pub to: usize, pub progress: f64, pub labels: BTreeMap<&'static str, String> }
pub fn animate(spec: &PlotSpec, data: &PlotData) -> Result<Animation>;
pub struct Animation { pub frames: Vec<Scene>, pub fps: f64, pub warnings: Vec<String> }
// per frame (rayon): layer_frames by state_rows -> with_layer_frames -> frame_scene
```

Alternative considered and rejected: a hidden facet per state (reuses tiles, but breaks
user facets, default faceted size and strips) and adding the state to `group` (stats
and stacks would still mix states at the layer level).

### Facets, gggrid, HTML canvas

- **Facets** come for free: frames are per tile, so each panel shows its own rows of
  the state and keeps its strip; a panel with no rows in a frame draws empty (as
  gganimate). Free facet scales stay as trained (fixed across frames).
- **gggrid**: `grid.rs` prepares each cell with `prepare_plot` and lays cells out with
  `arrange_panel`. gganimate cannot animate patchwork either. an1 rejects a transition
  inside `gggrid` with a `SpecError` (a later step could animate cells from the same
  frame copies, since the grid holds `PreparedPlot`s).
- **HTML canvas path**: dense point batches become canvas only in `render/html.rs`, from
  the finished `Scene`. A frame is a plain `Scene`, so every renderer works per frame; the
  player uses PNG frames when a scene has a canvas batch (decision 3).
- **Side bands, coord_flip, polar, 3D**: drawn by the same `draw` from the same layers;
  `frame_layer` keeps `band`, `y_oriented`, `companion` and the 3D z column (already scaled
  in the prepared data), so they follow the frame rows.

### Notes for an1

- Arrange per frame (cheap next to drawing); reuse one arrangement only as an an6
  optimisation, since title templates can change the heading height.
- Plot warnings come from `prepare_plot` once; frame copies carry none.
- `with_layer_frames` drops tooltips and legend keys: call `draw(.., false)` only.

## Notes (an2)

Done 2026-11-14. Ported from gganimate 1.0 / tweenr source (MIT), checked against R
for what is installed here (gganimate and tweenr are not: `distribute_frames` is copied
into `tests/fixtures/generate_animation_reference.R`, colours come from farver).

- `animate/states.rs`: `distribute_frames` (round robin of ceilings, the zero-move
  fix-up with R's recycling), then the frames of `expand_panel`: a hold of `s` frames
  and a move of `t` frames each add their count, each run replaces the last frame of
  the one before, the last frame of a move is the next state. Labels follow
  `get_frame_info` (closest state switches at linear position `j / t >= 0.5` within
  the move). With `wrap`, `nframes + 1` scene frames are planned and the timeline
  samples `nframes` of them, as gganimate (its trim to `nframes` is a no-op).
- `Transition` trait: `scene_frames(&mut self, states, requested) -> Result<usize>`
  plans once; `frame_show(frame) -> FrameShow::{States, Tween{from, to, progress}}`.
  an3's transitions return `Tween` too (time: between observed times).
- `animate/tween.rs`: rows matched by `(group, occurrence)` (tweenr's id rule),
  matched rows in `from` order; unmatched rows are not drawn while moving (no
  `enter`/`exit`, an4). Continuous columns and number styles linear by their ease,
  colours in Lab D65 + alpha (matches farver exactly on the fixtures), discrete
  columns and other styles switch at eased 0.5. Discrete positions are numbers
  after the stat, so a row changing category slides, as in gganimate.
- **Groups across states.** Per-state stats renumber groups densely per part, so the
  same number meant different groups in different states (a Gentoo bar flew into
  the Chinstrap one). `compute_layer` now records each output group's layer-wide rank
  (`ComputedLayer::group_ranks`) and `compute_states` renumbers each state's groups
  by it (`number_groups_by_rank`). Group numbers of animated layers are therefore
  ranks (not dense); every reader of `group_index` copes (they bucket by value).
- `animate/ease.rs`: tweenr's 31 easing formulas; `ease_aes(x=)`/`(y=)` fill the
  x-like/y-like names not given, as gganimate.
- Lines/polygons with uneven point counts: matched in order with a warning (transformr
  inserts points; deviation row). If a user asks, port `find_splits_c` and
  `insert_points_c` (transformr `align_rings.cpp`) per matched group.
