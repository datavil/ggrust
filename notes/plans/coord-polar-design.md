# coord_polar design proposal (backlog 2.5)

Decided by the user on 2026-11-11: option B, static HTML with a warning.

`Coord::map` maps one point. Polar needs every straight data-space edge bent: lines,
bar rectangles (wedges), area edges, grid circles.

## Option A: `Panel::path` munching in every geom (plan-lp5's proposal, ggplot2's and lets-plot's way)

About 25 geom files map corners or line ends directly (`panel.map`, `x_pixel`,
`y_pixel`) and each would change. AGENTS.md says to stop and report when a feature needs
many unrelated files edited.

## Option B: warp the drawn scene (recommended)

Draw each layer unchanged on an identity panel where pixels stand for (theta, r), as
`coord_3d` already reuses the 2D geoms, then transform its scene nodes once:

- `Path`: resampled along each edge, then each vertex mapped (ggplot2's munching, done once).
- `Rects`: become closed paths, then the same, so bars become wedges.
- `Points`, `Text`: anchor mapped, marker and glyph shape kept.
- `Group`: recursed. `Image`, `Raster` layers: a typed error naming the layer.

Cost: one new coord file, one warp file, one draw branch in `build.rs`, plus the polar
axis guides of 2.6. No geom file changes. An arrowhead drawn in pixels bends slightly
with its line, the one visible difference from option A.

## HTML

`coord_3d` is static in HTML (no tooltips, zoom, interactive legend), and hover targets
use the flat panel. Proposed: the same for polar at first, with a `GgrustWarning`, and
tooltips added later as a backlog item. PNG, SVG, PDF unaffected.

## Parity

API as lets-plot: `xlim`, `ylim`, `theta`, `start`, `direction`, `transform_bkgr`, with
its radius padding (`R_EXPAND` 0.15, `R_PADDING` 0.06) and the extra discrete theta
segment. Its open polar bugs (#1215 abline, #1213 `size_unit`, #1095, #1097, #1036,
#1039, #1061, #1182) follow ggplot2, as plan-lp5 already says.
