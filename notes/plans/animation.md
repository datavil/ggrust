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
- [x] **an1 Frames without tweening (80 percent of the value).** `transition_manual(frames,
  cumulative=False)`, `animate()`, `Animation`, `anim_save`, `ggsave(".gif"/".png")`,
  HTML player, GIF + APNG writers, `{frame}`/`{nframes}`/`{progress}`/`{current_frame}`
  labels, `start_pause`/`end_pause`/`rewind`. Frames rendered in parallel.
  Goldens: frames 0 and last; GIF decoded and compared per frame (not bytes).
- [x] **an2 `transition_states` + tweening.** `transition_length`, `state_length`, `wrap`;
  `ease_aes()` with gganimate's easing names (linear, quadratic, cubic, quartic, quintic,
  sine, circular, exponential, elastic, back, bounce; `-in`, `-out`, `-in-out`), per
  aesthetic (`ease_aes(y="bounce-out")`). State label variables. Tween parity vs tweenr
  fixtures.
- [x] **an3 `transition_time` and `transition_reveal`.** Time: frames spaced in time, rows
  tweened between observed times, `{frame_time}`. Reveal: paths/lines/areas grow along
  `along`, with the interpolated end point (gganimate's `keep_last`), `{frame_along}`.
- [x] **an4 Enter/exit and shadows (trimmed by the user, 2026-11-14).** `enter_fade`/`exit_fade`,
  `enter_grow`/`exit_shrink`, `shadow_wake`, `shadow_mark`, composed with `+`, gganimate's
  arguments and defaults; with `transition_states` and `transition_time` at least. Parity
  vs tweenr `.complete_states` and gganimate's transmuters and `shadow_wake` in the R
  fixture script.
- [ ] **an6 (on request) Performance pass.** Time box agreed with the user first. Reference: R
  gganimate + gifski on the same spec and frame count (report honestly, AGENTS rule 8).
  Baseline measured 2026-11-15 (`benchmarks/animation.py`, see `benchmarks/README.md`):
  ggrust was 29x to 60x faster with 4x to 20x less memory, but its GIFs were 1.3x to 11x
  larger than gifski's. Fixed the same day (asked by the user): the GIF encoder merges
  pixel-identical frames and writes only the changed rectangle, so GIFs are now 0.5x to
  0.8x gifski's and rendering is 65x to 130x faster than gganimate.

Dropped from the animation plan by the user (2026-11-14), listed in
[backlog.md](../backlog.md) ("Animation"): the other enter/exit effects (fly, drift,
recolor, appear/disappear as explicit effects), `shadow_trail`, the views of an5
(`view_follow`, `view_step`, `view_zoom`), `transition_filter`/`layers`/`events`/
`components`/`null`, tooltips in the HTML player, MP4.

## Decisions (user, 2026-11-14)

1. Outputs for an1: HTML player, GIF and APNG. GIF adds the `gif` and `color_quant`
   crates. No MP4.
2. A plot with a transition animates when displayed in a notebook (default `animate()`
   settings), as gganimate's `print` does.
3. HTML frames are SVG by default, PNG when the scene uses the canvas path (dense points).
4. Scope: an0 to an3 are scheduled (plan.md Phase 13). an4, an5 and an6 move to
   "on request".
5. (2026-11-14, later) an4 is built trimmed (fade, grow/shrink, shadow_wake,
   shadow_mark); an5 and the rest of the optional features are dropped to the backlog;
   an6 stays on request.

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
  inserts points; deviation row). Ported after an3, see "Fixes after an3".

## Notes (an3)

Done 2026-11-14. Ported from gganimate 1.0 (`R/transition-time.R`, `R/transition_reveal.R`)
and tweenr (`tween_along`, `src/along.cpp`), MIT. gganimate and tweenr are still not
installed here: `get_times`, `get_row_along` and `numeric_along_interpolator` (as R)
are copied into `tests/fixtures/generate_animation_reference.R`.

- **Frame positions are rounded.** Both transitions map each value to a frame
  position (`round(1 + (n-1)(v-low)/(high-low))` for time, `round((n-1)(v-low)/(high-low)) + 1`
  for reveal, R's half-even rounding) and the frames stand for `seq(low, high, length.out = n)`.
  So reveal's "interpolated end point" is interpolated between rounded positions, as in
  gganimate. `animate/along.rs` holds this plus `{frame_time}`/`{frame_along}` formats
  by column type (series annotation `int`, `date`, `datetime`, `time`).
- **States carry numbers.** `assign_states` returns the sorted numeric value of every
  state (`PreparedPlot::state_values`); categorical columns are a `SpecError` for these
  transitions. Stats run per value (gganimate's pre-stat transition pastes the value
  into the group, which also splits stats).
- **Trait additions.** `scene_frames(&StateSet, requested)` (count, numbers, class);
  `state_positions()`; `FrameShow::Time { position }` and `FrameShow::Reveal { position,
  keep_last }`. `Animation::new` plans per tile and layer once (`AlongLayer`): time keeps
  `(position, state)` anchors (a layer's times that round to an earlier time's frame are
  dropped, with a warning); reveal keeps the rows sorted by `(group, position)`.
- **Time frames.** At an anchor the state's rows as they are; between two anchors
  `tween::tween` at `(f - a)/(b - a)`; outside a layer's anchors nothing. gganimate's
  first move is off by one when a layer's first time is not frame 1 (deviation row).
- **Reveal frames.** `RevealRows::frame` returns `(from, to, progress)` per drawn row;
  `tween::mix_rows` mixes rows of one layer with a progress per row (raw rows are
  `(row, row, 0)`). Connected geoms (`CONNECTED_GEOMS`) keep the rows reached plus the
  moving end; others show the moving row, the last row when `keep_last`. A raw row equal
  to the moving row is drawn once (gganimate's repeat removal).
- Python: `transition_time(time, range=None)`, `transition_reveal(along, range=None,
  keep_last=True)`; `range` takes numbers, dates, date-times and times of day (sent as
  epoch or day milliseconds). The state column export knows `time` and `along`.

Follow-up (user request, after an3): `range` carries `range_kind` (`number`, `date`,
`datetime`, `time`, set by the Python API) and `along::plan` rejects a kind that does not
fit the column's `TimeClass`; date, date-time and time-of-day states print with the
`{frame_time}` formats in every label variable (`Animation::state_labels`).

## Fixes after an3 (user request, 2026-11-14)

- **transformr alignment** (`animate/align.rs`): `tween::tween` takes a `Shape` per layer
  (`Shape::of(geom)`, gganimate's `layer_type`). For paths and polygons every group in
  both states gets `max(50, n_from, n_to)` points (transformr `min_n`), even when the counts
  match, via `add_points`/`find_splits_c`/`insert_points_c`; inserted rows copy the vertex
  before them; polygons are padded to 4 points, made clockwise and the second ring rotated
  (`rotate_c`). Line geoms that sort by x are sorted before inserting. The R translation in
  the fixture script ("align") proves the coordinates. Not ported: NA-separated
  multi-paths/holes (those groups match row by row) and `match = FALSE` cutting.
- **Labels**: years padded to 4 digits (negatives as ISO `-0001`); `TimeClass::DateTime
  { with_time }` is settled once from the state values (`TimeClass::settled`), so every
  label of a date-time column uses one format (user's choice, deviation row).
- Bug hunt: the old uneven-points warning stays for joined geoms that are not aligned
  (area, ribbon, density, violin, smooth, ...; gganimate's `layer_type` for them was not
  checked). `range=` ends take part in choosing the date-time format. `geom_step` is
  aligned as a path, so a tweened step shows small stairs along its diagonal, as gganimate
  (`GeomStep` is a `GeomPath`).

## Notes (an4, trimmed)

Done 2026-11-14. Ported from gganimate 1.0 (`R/transmuters.R`, `R/shadow-wake.R`,
`R/shadow-mark.R`, `R/scene.R`) and tweenr (`.complete_states`, `tween_at`), MIT. The R
fixture script copies `.complete_states`, `transmute_fade`, `transmute_grow` and
`shadow_wake`'s frame and step logic (`enter_exit`, `wake` sections).

- **Enter/exit** (`animate/transmute.rs`, `tween::tween_rows`): tweenr's rule. Rows
  match by `(group, occurrence)`; exiting rows stay in `from` order and move to their
  exit state, entering rows follow in `to` order and move from their enter state. The
  first and last frame of a move are the states as they are (exit rows are not drawn
  on the arrival frame). Effects added with `+` apply in order (gganimate's factory
  list; the Python `+` concatenates the `enter`/`exit` lists).
- **Per geom** as `transmuters.R`: fade sets `alpha` where it is set, otherwise the
  color/fill alpha (`geom_smooth`: color always); grow scales `size`/`stroke` only for
  geoms whose ggrust `size` is ggplot2's `size` (`MARK_SIZE_GEOMS`); bars scale `y` and
  `ymax`; paths/lines/steps/polygons/violins shrink to their group mean; boxplots close
  on `middle`. gganimate's per-geom overrides (`...`) are not offered.
- **Identity and phase** (`tween::Drawn`): each drawn row carries a `RowKey`
  `(group, occurrence)` (gganimate's `.id`) and a `Phase` (raw, static, transition,
  enter, exit). A states frame is `raw` on the last frame of its run (the frame before
  moving on), `static` before; a time frame at a layer's own time is `raw`.
- **Shadows** (`animate/shadow.rs`): `Animation::drawn` builds any frame's rows (no
  shadow); `shadowed` adds them. Wake: frames and `at` as `get_frames`/`setup_params`,
  values `end + ease(at) * (value - end)`, rows kept for keys drawn now outside
  `exclude_phase`, reordered by first appearance of the key. Mark: raw rows precomputed
  once per layer (`Animation::marks`), constants applied. Every shadow part gets its own
  group numbers (`shadow::stack`), so a line's wake is separate lines. `shadow_mark`
  with `transition_reveal` is a `SpecError` (no raw frames).
- `None` values are dropped from `as_dict()`: `shadow_wake(size=False)` is sent as
  `false`, an absent `size`/`alpha` means gganimate's default 0.
