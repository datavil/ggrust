# lets-plot silently samples data

Source: `lets-plot/plot-builder/.../assemble/geom/DefaultSampling.kt`.

Default per-geom sampling (seed 37) unless the user passes `sampling="none"`
(lets_plot 4.11 has no `sampling_none()` function; ggrust's returns `"none"`):

| geom | default |
|---|---|
| point, tile, jitter | random 100_000 |
| line, area, density | systematic 50_000 |
| path / polygon | vertex simplification 50_000 |
| rect | random 20_000 |
| segment | random 10_000 |
| text | random 5_000 |
| bar | systematic 5_000 |
| safety (any) | random 200_000 |

**Why it matters:** a 1M-cell UMAP in lets-plot draws 100k points. Comparing that
against ggrust drawing 1M points is unfair in both directions.

**How to apply:** in benchmarks, run lets-plot with `sampling="none"` *and*
report the default-sampled numbers separately, with N drawn stated. Decided (user, 2026-11-09, step 11.1): ggrust never samples by default;
`sampling=` works on request (own picks, per panel on the stat output, as lets-plot).

**Exception (step 6.4):** lets_plot's `geom_boxplot` hard-codes `sampling=None` and raises
`TypeError` if you pass `sampling`. It samples the stat output (boxes + outliers), which stays
far below the limits: at 300k rows both libraries drew the same outliers (2,059 vs 2,058).
So benchmark boxplots without the kwarg.
