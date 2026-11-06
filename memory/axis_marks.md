---
name: axis-marks
description: How guide_axis_mark (leader-line labels, plan 9.9) is built, and the traps found
metadata:
  type: project
---

# Leader-line axis labels (`guide_axis_mark`)

Step 9.9 (2026-11-06). API chosen by the user: the scale's `breaks=` picks the subset,
the guide only lays it out; `spread="nearest"` (ComplexHeatmap `anno_mark`) default.

- **The guide rides on `AxisBreaks.arrangement`** (`LabelArrangement::Mark`), set by
  `choose_breaks` from `PositionScale::mark()`. Every consumer of breaks (main axes,
  side band axes, facets, gggrid, zoom) then sees it without new plumbing. Measure
  (`x_labels_height`, `y_axis_width`) and draw (`axis_side_nodes` ->
  `axis_mark::mark_nodes`, `LabelPlacer` Mark arm) must agree: room = link_width + label.
- **Never wrap, blank or rotate marked labels**: `fit_*_labels` skip wrapping,
  `keep_labels_inside` (bands) skips them, `LabelPlacer` ignores the theme angle.
- **Discrete `breaks=` without `limits=` reorders the axis** (lets-plot: breaks lead).
  Heatmap marks need `limits=` too; the first render looked like a link bug.
- **Zoom**: the axis has fixed breaks, so the runtime stamps `placer.node` at the true
  pixel (labels at breaks, no links); reset restores the saved static axis. Documented gap.
- **`ggsave(plot, "a/b.png")` writes under `lets-plot-images/`** (lets-plot's default
  `path`). Render inspection PNGs with `plot.to_png(path)`; the stray untracked
  `lets-plot-images/` folder came from that.

See [[deviations-md]] row "Leader-line axis labels"; related: [[side-panels]],
[[axis-positions]], [[dendrograms]].
