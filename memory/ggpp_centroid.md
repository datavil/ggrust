# ggpp centroid stats (10.6b)

`stat="centroid"` / `"summary_xy"` live in `stat/centroid.rs`; Python in
`plot/stat_ggpp.py`. Parity fixture: `tests/fixtures/generate_ggpp_centroid_reference.R`
(ggpp 0.6.1; matches to 1e-12 per cluster and facet panel).

- **Scope (user, 2026-11-09)**: `fun` / `fun_x` / `fun_y` = "mean" | "median" by name,
  snake_case names, x and y only, no `mean_se` ranges, default geom "point" and
  `show_legend=False` as ggpp.
- **Grouping trap**: ggplot2 does not group by `label`; lets-plot and ggrust do. So the R
  fixture needs `group = cluster`, while in ggrust `aes(label="cluster")` is enough.
- Other columns follow `groups.rs` (mean / first value); ggpp sets a non-constant label to
  NA with a warning instead (deviation row).

**How to apply:** read before extending the centroid stats (more functions, ranges).
