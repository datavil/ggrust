# gganimate animation (an1)

How animation is built, so an2/an3 extend it instead of re-deriving it.

- **Spec**: one `animation` key: `transition` (`{name, ...}`), `animate` (gganimate
  `animate()` options), `frame` (draw one output frame as a still; used by
  `Animation.frame(i)`). Parsed by hand in `animate/spec.rs` so errors carry
  `animation.*` paths. Python `+` merges the key field by field (`plot/core.py`).
- **States like facets**: the transition's column is carried through mapping like a
  facet column (`build.rs` `StateVariable`, `assign_states`). Scales train once over all
  rows; stats and positions run per state (`compute_states` splits, computes, concats
  with `LayerData::concat`) and every computed row keeps its state index
  (`PreparedLayer.states`). Layers without the column are static (drawn every frame).
- **Frames**: `Animation` (`animate/mod.rs`) owns the `PreparedPlot`; a scene frame is
  `with_layer_frames` over the rows of the frame's states plus filled title templates
  (`build/frame.rs`). Writers only see `FrameSource` (`render/animation.rs`).
- **Planning** (`animate/timeline.rs`) is a port of `animate.gganim`: pauses, rewind,
  `detail`, `unique(round(seq(...)))` with R's half-even rounding. Manual transitions
  have one scene frame per state, so `nframes` shrinks to #states and fps scales with
  it (`animate(p, fps=1)` with 4 states gives fps 0.04, by design).
- **Writers**: consecutive equal output frames merge into one longer GIF/APNG frame;
  delays come from cumulative times so they sum exactly. Frames render in parallel in
  chunks (memory bounded). HTML ships distinct frames once plus a timeline; canvas
  point batches switch every frame to PNG.
- **Traps**: label templates take names only (no expressions); the state column must
  be listed in `PlotSpec::collect_columns` or it never crosses to Rust; Python checks
  the column early (`_check_state_column`) for a did-you-mean over all columns.
- For tweening (an2): extend the `Transition` trait (`frame_states` now returns whole
  states per scene frame) rather than adding a parallel path.
