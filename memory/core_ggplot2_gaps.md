# Core ggplot2 gaps: geom_col, geom_rug, stat_ellipse (exc.1-3)

- **geom_col is the bar geom, not a copy.** `BarGeom { name, stat, parameters }` with
  `BAR_GEOM` / `COL_GEOM`; `just` only on col. Any Python list of bar-like geom names
  (gghighlight `NO_COLOR_GEOMS`, `FILL_GEOMS`, `_label_target`) must include `"col"`.
- **Stacking and dodging stay lets-plot's** (first group at the bottom, slots split by the
  most groups at any x); ggplot2 differs, recorded in deviations.md.
- **geom_rug** reads `panel.flipped` and maps sides as ggplot2 `chartr("tblr","rlbt")`.
  Layers are clipped to the panel and there is no `clip="off"`, so `outside=True` only warns.
- **linewidth now has a default scale** (ggplot2 `scale_linewidth` range 1..6, NA width 0).
- **cov.trob parity:** start weights `1 + p/nu`, distances under the weighted scatter
  (a 2x2 inverse equals R's SVD route to 1e-12), weighted location updated before the
  stop test, final scatter from the last centred points over n. Iteration counts match.
  F(2, d) quantile closed form `d/2 * expm1(-2/d * ln1p(-p))`.
- **Sloped collinear points give a sliver in R too** (chol succeeds by rounding); only exact
  zero variance is degenerate.
- **Staging only your lines in a file others edited:** write a blob of `HEAD + your line`
  (`git hash-object -w`, `git update-index --cacheinfo`) instead of `git add`.
