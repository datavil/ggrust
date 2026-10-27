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

## Scenarios: 2026-10-27, macOS arm64 (Apple M4 Pro, 12 cores), Python 3.13, lets-plot 4.11

`benchmarks/scenarios.py`: six common scatter variants (plain, discrete colour, continuous
colour, size + alpha, log scales, title + bottom legend), same data and `sampling="none"`.
Warm = median of 2 repeats after a cold call, fresh process each. Memory is the peak above
the data level, cumulative per process (the PNG row includes the SVG stage). Spec build is
lazy in both libraries (under 0.1 ms) and omitted below. Raw: `benchmarks/results/scenarios-*`.
Side-by-side figure: `python benchmarks/compare_images.py <images dir>`.

| scenario | points | stage | ggrust (s) | lets-plot (s) | speed-up | ggrust MB | lets-plot MB | memory ratio |
|---|---:|---|---:|---:|---:|---:|---:|---:|
| color_continuous | 10,000 | svg | 0.0032 | 0.0947 | 29x | 11 | 51 | 4.8x |
| color_continuous | 10,000 | png | 0.0438 | 0.7134 | 16x | 24 | 275 | 11.6x |
| color_discrete | 10,000 | svg | 0.0030 | 0.0939 | 32x | 11 | 49 | 4.6x |
| color_discrete | 10,000 | png | 0.0435 | 0.6794 | 16x | 24 | 255 | 10.6x |
| log_scales | 10,000 | svg | 0.0023 | 0.0817 | 35x | 11 | 49 | 4.7x |
| log_scales | 10,000 | png | 0.0416 | 0.6410 | 15x | 23 | 244 | 10.4x |
| scatter | 10,000 | svg | 0.0018 | 0.0816 | 45x | 11 | 45 | 4.3x |
| scatter | 10,000 | png | 0.0369 | 0.6457 | 18x | 23 | 247 | 10.5x |
| size_alpha | 10,000 | svg | 0.0038 | 0.0974 | 26x | 11 | 56 | 5.2x |
| size_alpha | 10,000 | png | 0.1198 | 1.3546 | 11x | 24 | 257 | 10.8x |
| titled_themed | 10,000 | svg | 0.0040 | 0.1039 | 26x | 11 | 55 | 5.2x |
| titled_themed | 10,000 | png | 0.0481 | 0.7827 | 16x | 25 | 269 | 11.0x |
| color_continuous | 100,000 | svg | 0.0305 | 1.1153 | 37x | 44 | 442 | 10.0x |
| color_continuous | 100,000 | png | 0.2410 | 4.8536 | 20x | 58 | 1299 | 22.4x |
| color_discrete | 100,000 | svg | 0.0298 | 0.9797 | 33x | 44 | 386 | 8.7x |
| color_discrete | 100,000 | png | 0.2463 | 4.4964 | 18x | 56 | 1252 | 22.2x |
| log_scales | 100,000 | svg | 0.0209 | 0.9090 | 43x | 22 | 410 | 18.7x |
| log_scales | 100,000 | png | 0.2502 | 4.2519 | 17x | 30 | 1210 | 40.7x |
| scatter | 100,000 | svg | 0.0161 | 0.9109 | 57x | 21 | 375 | 18.2x |
| scatter | 100,000 | png | 0.2326 | 4.2343 | 18x | 26 | 1293 | 50.0x |
| size_alpha | 100,000 | svg | 0.0373 | 1.0562 | 28x | 56 | 431 | 7.7x |
| size_alpha | 100,000 | png | 0.9072 | 11.6295 | 13x | 71 | 1314 | 18.6x |
| titled_themed | 100,000 | svg | 0.0373 | 1.1731 | 31x | 53 | 474 | 8.9x |
| titled_themed | 100,000 | png | 0.3049 | 5.2960 | 17x | 67 | 1423 | 21.1x |
