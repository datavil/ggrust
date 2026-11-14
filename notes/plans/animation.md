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

p                                         # notebook: animated HTML player
anim = animate(p, nframes=100, fps=10)    # Animation object, _repr_html_
anim_save("life.gif", anim)               # gif | png (APNG) | html
ggsave(p, "life.gif")                     # same, default animate() settings
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

- [ ] **an0 Spike: find the seam.** Read `prepare_plot` and the draw path, prove a frame
  can be drawn from modified computed layer data with fixed scales (throwaway test).
  Output: a short design note in this file (data model of a "state", where tweening
  plugs in). Stop and show the user if the seam is not clean.
- [ ] **an1 Frames without tweening (80 percent of the value).** `transition_manual(frames,
  cumulative=False)`, `animate()`, `Animation`, `anim_save`, `ggsave(".gif"/".png")`,
  HTML player, GIF + APNG writers, `{frame}`/`{nframes}`/`{progress}`/`{current_frame}`
  labels, `start_pause`/`end_pause`/`rewind`. Frames rendered in parallel.
  Goldens: frames 0 and last; GIF decoded and compared per frame (not bytes).
- [ ] **an2 `transition_states` + tweening.** `transition_length`, `state_length`, `wrap`;
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
