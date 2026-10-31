# Zoom/pan (`ggtb`): JS ticks from a Rust payload, marks moved not re-rendered

Decided with the user (plan 4.3): the runtime computes zoomed breaks itself (a
port of `scale/breaks.rs` + `format.rs` default labels) from a payload Rust
writes; marks are repositioned at their size, never re-rendered.

- **Zoom = coord limits.** lets-plot zooms by `coord_cartesian(xlim, ylim)` in
  transformed units, so a zoomed axis must read exactly like a static render
  with those limits. `tests/test_zoom.py` checks the JS breaks against the
  engine's via node (y axis: its break count does not depend on label widths).
- **Templates, not styles.** Grid lines, ticks and labels are written by the
  SVG writer at pixel 0 (`layout/zoom.rs`, `svg::fragment`); the runtime stamps
  `translate()`d copies. Labels use `>@<` / `>^<` placeholders, replaced with a
  function (a `$` in a label is literal). Fixed and discrete breaks carry their
  labels as drawn, so rotated / two-row arrangements survive.
- **How marks move:** circles by `cx`/`cy` (and `r` * size factor, points only,
  as lets-plot `PointGeom`); `[data-anchor]` groups (non-circle markers, text and
  label boxes, bracket labels) by translate; everything else in `g.panel` by a
  matrix with `vector-effect: non-scaling-stroke`. A label anchored anywhere but
  its data point drifts by (scale - 1) x offset: anchor at the point.
- **Scene hooks:** each panel's pieces sit in `<g class="tile">` (document order
  = payload order); `Group.anchor` is flattened in static SVG and PNG (PNG must
  inline unclipped groups or text batching breaks). Rust test helpers find the
  panel with `Scene::groups_with_role` and read it with `Group::flattened`.
- **Traps:** the toolbar adds an `<svg>` icon before the plot in the root div, so
  `root.querySelector("svg")` finds an icon: use `root.ggrust.svg`. The browser
  pane, when hidden, never fires `requestAnimationFrame`: tests stub it with
  `setTimeout`. Synthetic `MouseEvent`s work for the runtime's own listeners.
- Not done: the layout is not redone while zoomed (lets-plot re-lays out), a
  user `format` on generated breaks falls back to the default format.
