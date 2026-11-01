---
name: overflow-guards
description: Legends, axis labels and titles are fitted to the canvas (ggrust deviation from lets-plot and ggplot2); how and where
metadata:
  type: project
---

lets-plot and ggplot2 4.0.1 both let big legends, long axis labels and long titles run off the canvas
(survey script reproduced 7 cases on 2026-11-01). ggrust fits them, only when they would overflow, so
plots that fit stay pixel-identical to lets-plot.

- Legends: overflowing stacks first wrap into side-by-side stacks (`wrap_stacks`); then `layout/legend.rs::fit_legend` with `layout::legend_budget` (preferred 50% across for automatic
  grids, `most` for user ncol/nrow grids). Order: reflow grid, narrower label wrap, then first keys + "+K more".
- Axis labels (discrete, only past 1/2 of the content: 1/3 caught lets-plot's normal vertical labels): first wrapped at
  spaces (`wrap_labels`; x into each tick slot, laid out flat; y if lines fit the slot), else cut in the middle
  (`fit_labels`). Numbers are never cut (a cut number misleads).
- Titles: `wrap_headings` in `PreparedPlot::arrange` (two layout passes); `Arrangement.titles` is what is drawn.
- Geoms stay clipped to the panel, as in both references.

**Why:** the user chose robustness over copying shared bugs ([[deviate-from-lets-plot-via-ggplot2]]).
**How to apply:** parity tests with extreme labels may need a bigger `ggsize`; new layout code must keep
fitting plots byte-identical. Facets use the same `fit_x_labels`/`fit_y_labels`, budget split among axis rows/columns. gggrid collected legends get a budget over the grid (lp3.4); stacked legends split it by water-filling (`water_fill`), never equally (short legends left a long one cut). Colorbars are measured, not fitted.
