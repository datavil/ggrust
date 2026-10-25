# lets-plot silently samples data

Source: `lets-plot/plot-builder/.../assemble/geom/DefaultSampling.kt`.

Default per-geom sampling (seed 37) unless the user passes `sampling=sampling_none()`:

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

**How to apply:** in benchmarks, run lets-plot with `sampling=sampling_none()` *and*
report the default-sampled numbers separately, with N drawn stated. Decide (ask the user)
whether ggrust copies these defaults; cellestial users may rely on them without knowing.
