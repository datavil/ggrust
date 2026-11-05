---
name: axis-positions
description: How position= axis sides are built (plan 9.3) and how a secondary axis should reuse them
metadata:
  type: project
---

# Axis sides (`position=`)

Plan step 9.3 (2026-11-05).

- **Sides live on the scale** (`AxisSides { primary, secondary }` on both position scale
  structs). Layout reads the *screen* scales, so `coord_flip` moves x "top" to the right
  with no extra code (lets-plot and ggplot2 agree).
- **One side = one `AxisEdge`** (`layout/side.rs`): side + base pixel (panel edge past the
  strips on that side). `axis_side_nodes(edge, layout, breaks, theme, pixel, span)` draws a
  side for *any* breaks: a secondary axis (exc.4) only needs its own `AxisBreaks` and band.
- **Bottom/left keep the exact old float order**, so every existing golden stayed
  byte-identical; top/right mirror them. Rotated top/right labels mirror the label *box*
  (box centre from alignment + angle), which matches lets-plot's output.
- **Bands**: `PlotLayout.axis_bands` (Margins) holds ticks+labels+tree per side;
  `top_band`/`right_band` now mean facet strips only. Titles and legends go past both.
- **Facets**: lets-plot draws top/right axes on every panel over strips (broken); ggrust
  follows ggplot2 (outer panels, outside strips). Tiles carry `has_top_axis`,
  `has_right_axis`, `is_row_end`; an unfinished row's last panel draws its right axis in
  the empty slot (as the bottom axis of an unfinished column).
- **Zoom templates of a two-sided axis are groups**; the JS runtime must replace `@`/`^`
  globally (it replaced the first only).
- lets-plot errors on x `position="left"` (`Illegal X-axis position`), as ggrust does.

See [[deviations-md]] rows "Facet axes with position" and "Right y axis title angle";
related: [[dendrograms]], [[arrow-axes]].
