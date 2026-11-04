# ggdist slabs and gghalves half geoms

Step ex1.6 (2026-11-04). gghalves 0.1.4 is MIT; ggdist 3.3.3 is GPL (read for behaviour,
reimplemented from its documented algorithms). R reference: `tests/fixtures/generate_ggdist_reference.R`.

- **Bounded density** (ggdist default): bounds from `bounder_cdf` (p = 0.01, R type 7
  quantiles); a side is bounded when `min - 3 bw <= bound` (else it stays open at 3 bw).
  R `density()` on the bounds widened by their width per bounded side (`3n - 2` points),
  fold the outer parts back by index, then `approx` onto `seq(min, max, n)` when trimmed.
  `RDensity::on_grid` (R's `from`/`to`/`bw`) gives it; densities match R to 1e-9.
- **Sheather-Jones dpi** (`bw.SJ(method="dpi")`): pair distances counted in 1000 bins of
  `1.01 x range / 1000`, bin = `trunc(x / width)` (not `x - min`); n > 500 bins the values
  first. Two-stage plug-in with constants 1.23 / 2.394 / 1.349. Matches to 1e-9 incl.
  n = 1200. Falls back to nrd0 when TD is not positive (zero IQR).
- **Mode**: integer data = most frequent value (first in data order); else the peak of
  a bounded dpi density on 2001 points.
- **Slab geometry**: region `[x, x + width x res]` for `topright`; thickness / layer max
  x `scale` (0.9) x the region, so dodging (xmin/xmax shrink by n) gives R's 0.45 per
  slot. `normalize="all"` runs in geom setup, so per facet panel (R: whole layer).
- **Stat variables reach the geom**: computed columns stay in `DrawLayer.data`, so
  `geom_pointinterval` reads `...width..` to size lines. Do not output a style aesthetic
  (`linewidth`) from a stat: it would go through a style scale.
- **Sizes**: ggdist's defaults are ggplot2 units; the user chose converted defaults
  (0.78-1.81, fatten 2.5, interval size 5.2) so plots look as in R. Recorded in plan.md.
- **gghalves**: spans are 0.75 (ggplot2's boxplot stat width) for all three geoms;
  `DefaultPosition::named("dodge")` with the span `width` column reproduces R's dodged
  xmin/x/xmax exactly. Half box: box from the outer edge to `x -/+ nudge`, whiskers and
  outliers at `x -/+ nudge` (stat `outlier_nudge`, applied before dodge, survives it).
  Half points: `x +/- span/4`, jitter `range_scale x span/4` and 0.4 x the group's y
  resolution, ggrust's seeded generator (not R's numbers).
- **Reuse pattern**: half geoms build a modified `LayerData` and call the full geom's
  drawing with `DrawLayer { data: &modified, ..*layer }`; violin/boxplot/pointrange
  drawing was split into `draw_violins`, `draw_boxes`, `draw_point_ranges`.
- **Test trap**: lets-plot's ydensity repeats a vertex at each quantile band edge, so a
  violin outline has more vertices than the stat grid; drop repeated y before comparing.
- Open: `stat_interval` colors use the default discrete scale (R: ordinal viridis).
