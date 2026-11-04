# gghighlight: a Python spec rewrite at `+` time

`python/ggrust/plot/gghighlight.py`, hooked into `PlotSpec.__add__` (kind `"gghighlight"`).
No geom or stat: each target layer becomes a grey copy (in place) + a highlighted copy
(appended) + optional `geom_label_repel` / `geom_blank` layers, as the R package (MIT).

- **lets-plot merges plot columns into a layer with the same row count**
  (`LayerFrames::column`). A grey layer with "only the mapped columns" still got the
  faceting column from the plot data and was faceted. Fix: layer flag
  `merge_plot_data: false` (spec/layer.rs, build.rs). Any rewrite that relies on a
  column being *absent* from a layer needs it.
- Rewritten layers carry `inherit_aes=False` + the merged mapping, so the plot's
  `as_discrete` mapping annotations must be copied into the layer `data_meta`
  (`discrete_annotation` reads the layer's meta for non-inherited aesthetics).
  Series annotations fall back to the plot's by column name, but are copied too.
- Grouped predicates: `group_by(keys, maintain_order=True).agg(expr)`. A per-row
  expression gives `List` columns; lists of length 1 count as one value (R
  `summarise()`), longer ones fall back to per row with a `GgrustWarning` (parity).
- Series keys = `group`, else every discrete non-position mapped column. R also
  takes a discrete `x` (ggplot2 grouping), but ggrust lines group like lets-plot
  (no position aesthetics), and taking `x` broke every line into single points.
- Numeric predicates: drop null *and* NaN before ranking (polars sorts NaN last =
  "largest").
- Plot data becomes the first layer's highlighted rows (R), so later layers see
  them; inheriting layers excluded by `n=` are pinned to the full data.
