---
name: axis-nested
description: How guide_axis_nested (legendry brackets, plan 10.3) is built, and the traps found
metadata:
  type: project
---

# Nested axis guide (`guide_axis_nested`)

Step 10.3 (2026-11-06). API agreed with the user: `key=` a mapping `{group: [levels]}`
or `key_range_manual(start, end, name)`, `type` bracket/box, one level, legendry theme
names in snake_case, brackets follow the HTML zoom. `side="opposite"` was added
(scanpy dotplot layout; mirrors `guide_axis_dendro(side=)`).

- **`side="axis"` rides on `AxisBreaks.nested`** (resolved in `choose_breaks`), so main
  axes, facets, gggrid and side band axes measure and draw it with no extra plumbing.
  `x_labels_height` / `y_axis_width` include its extent; drawing offsets use the
  `plain_*` versions (labels only). Keep both in step: measure = draw.
- **`side="opposite"` is layout-level** like axis dendrograms (`nested_band` in
  `layout_with`, drawn in `draw_axes`); facets drop it with a warning.
- **Squish the ends to the panel** (legendry `oob="squish"`): lets-plot's discrete
  expansion is 0.2, smaller than the 0.4 pad, so unclamped end brackets lost their tips
  to the clip.
- **legendry geometry**: shapes are (along, across) with across 1 nearest the axis;
  `bracket.size` 2 mm, `guide.spacing` 0 inside this guide (it overrides the theme's
  2.25 pt), `legendry.box` inherits `strip.background` (blank in lets-plot, so ggrust
  falls back to `rect`). Source fetched with `gh api repos/teunbrand/legendry/contents/R/...`.
- **Zoom**: group `nested-x`/`nested-y` outside `axis-*` (whose innerHTML the runtime
  rewrites); shapes sit in an inner group inside the clipped one so the one-axis matrix
  does not move the clip; names are anchored groups, translated, hidden outside the panel.
  Gap: squished ends are drawn squished, so panning past them shows the old end.
- User `\n` in names becomes `LINE_BREAK` at parse (a plain `\n` draws as a space).

See [[deviations-md]] row "Nested axis guide"; related: [[axis-marks]], [[dendrograms]],
[[side-panels]].
