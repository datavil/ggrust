# Benchmarks

`benchmarks/run.py` measures ggrust against lets-plot on **identical work**:

- same polars frame, same scatter plot (`geom_point(size=1)`), same size (600 x 400)
- lets-plot runs with `sampling="none"`: by default it silently draws at most 100k
  points (see `memory/letsplot_default_sampling.md`); ggrust always draws every point
- PNG at scale 2 for both (lets-plot's default)
- each measurement in a fresh process: **cold** = first call (includes start-up work),
  **warm** = median of the following calls
- **memory** = peak resident set above the level reached after building the data

Run: `uv run python benchmarks/run.py` (raw results land in `benchmarks/results/`).

## Results: 2026-10-25, Linux x86-64, 4 cores, Python 3.13, lets-plot 4.11

| output | points | ggrust warm | lets-plot warm | speed-up | ggrust memory | lets-plot memory |
|---|---:|---:|---:|---:|---:|---:|
| SVG | 10,000 | 0.003 s | 0.125 s | 42x | 4.7 MB | 49 MB |
| SVG | 100,000 | 0.026 s | 1.357 s | 52x | 13 MB | 335 MB |
| SVG | 1,000,000 | 0.279 s | 16.877 s | 60x | 84 MB | 3,005 MB |
| PNG | 10,000 | 0.053 s | 1.267 s | 24x | 16 MB | 186 MB |
| PNG | 100,000 | 0.279 s | 9.320 s | 33x | 17 MB | 984 MB |
| PNG | 1,000,000 | 2.448 s | 101.086 s | 41x | 31 MB | 8,822 MB |

Cold times are within a few percent of warm for both libraries (see the raw files).
Known ggrust hot spots are listed in `plan.md` (Phase 6): marker rasterisation
(~1.3 µs per marker), integer-to-float column casts, string level hashing.
