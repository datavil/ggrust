---
name: overflow-guards
description: Legends, axis labels and titles are fitted to the canvas (ggrust deviation from lets-plot and ggplot2); how and where
metadata:
  type: project
---

lets-plot and ggplot2 4.0.1 both let big legends, long axis labels and long titles run off the canvas
(survey script reproduced 7 cases on 2026-11-01). ggrust fits them, only when they would overflow, so
plots that fit stay pixel-identical to lets-plot.

- Legends: `layout/legend.rs::fit_legend` with `layout::legend_budget` (preferred 50% across for automatic
  grids, `most` for user ncol/nrow grids). Order: reflow grid, narrower label wrap, then first keys + "+K more".
- Axis labels: `layout::fit_labels` cuts discrete labels in the middle to 1/3 of the content; numbers are never cut (a cut number misleads).
- Titles: `wrap_headings` in `PreparedPlot::arrange` (two layout passes); `Arrangement.titles` is what is drawn.
- Geoms stay clipped to the panel, as in both references.

**Why:** the user chose robustness over copying shared bugs ([[deviate-from-lets-plot-via-ggplot2]]).
**How to apply:** parity tests with extreme labels may need a bigger `ggsize`; new layout code must keep
fitting plots byte-identical. Facets, gggrid and colorbars are not guarded yet.
