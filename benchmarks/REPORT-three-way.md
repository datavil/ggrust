# ggrust vs lets-plot vs R ggplot2: benchmark report

Measured on 2026-11-08. ggplot2 and lets-plot at commit `9db79d3`; ggrust re-run at
`a3e895a` (0.8.0 plus chunked line strokes in PNG, the fix this benchmark prompted).
Scripts: `three_way.py`, `three_way.R` and `three_way_charts.py`.

## Summary

11 common plot types, 3 sizes (10k, 100k, 1M rows), SVG and PNG, 66 cases per library.
Each number is the median warm render time, from building the plot to the file on disk.

| ggrust compared with | SVG speed-up, median (range) | PNG speed-up, median (range) | memory, median (range) |
|---|---:|---:|---:|
| lets-plot 4.11 | **60x** (24x to 180x) | **77x** (27x to 336x) | 4.5x lower SVG, 9.2x lower PNG |
| R ggplot2 4.0.1 | **18x** (5x to 84x) | **7.2x** (2.3x to 36x) | 3.3x lower SVG, 3.2x lower PNG |

- **ggrust is faster than both libraries in all 66 cases.**
- **The line PNG weakness is fixed.** The first run (ggrust `9db79d3`) found one: `geom_line`
  to PNG took 0.89 s at 100k rows (ggplot2 0.54 s) and 17.3 s at 1M rows (ggplot2 5.4 s),
  because ggrust rasterised 6 self-overlapping polylines of ~166k vertices each in one
  piece. Stroking long lines in chunks (same picture, `memory/png_chunked_line_strokes.md`)
  brought it to 0.23 s and 1.98 s, 2.3x and 2.7x faster than ggplot2. lets-plot did not
  finish the 1M-row line PNG in 300 s. The re-run moved no other ggrust case beyond
  sub-millisecond noise.
- **Point-heavy plots gain the most.** At 1M rows (scatter, smooth, facet) ggrust writes
  the PNG in 0.12 to 0.14 s, ggplot2 + ragg in 4.2 to 4.7 s (~35x), lets-plot in 40 to
  46 s (~330x).
- **Stat plots (histogram, bar, boxplot, violin, density, bin2d)** at 1M rows are 5x to
  16x faster than ggplot2 and 75x to 180x faster than lets-plot (all sizes: 4.6x to 74x and
  27x to 180x). This is where R comes closest: its stats are vectorised C and few marks
  are drawn.
- **Memory.** At 1M rows ggrust peaks 52 to 369 MB above the data, ggplot2 345 MB to
  1.9 GB, lets-plot 616 MB to 9 GB. ggrust uses more memory than ggplot2 only in two
  10k-row PNGs (bar and facet, a few MB).
- **Ranking of the other two.** ggplot2 is a median 6.6x faster than lets-plot on the
  same work. lets-plot pays to serialise every row to JSON, and its PNG export is
  especially slow and memory-hungry (a 1M-point scatter PNG peaks at 8.0 GB).
- **Startup** (new process, import, one tiny PNG): ggrust 0.18 s, lets-plot 0.31 s,
  R 0.44 s (median of 6; first ggrust run 0.57 s, cold disk cache).

![overview](../docs/images/benchmarks/overview.png)

![time PNG](../docs/images/benchmarks/time-png.png)

![time SVG](../docs/images/benchmarks/time-svg.png)

![memory PNG](../docs/images/benchmarks/memory-png.png)

![memory SVG](../docs/images/benchmarks/memory-svg.png)

The same charts with context are on the docs site's [benchmarks page](https://ggrust.datavil.org/benchmarks/).

## Scenarios

The data looks like everyday data: `x` and `y` are shifted, widening normals, and `group`
has 6 levels (`g0` to `g5`). One CSV per size, shared by all three libraries.

| scenario | spec | what it stresses |
|---|---|---|
| scatter | `geom_point(size=1)` | marks: one per row |
| color_discrete | `geom_point(size=1)`, `color="group"` | marks + discrete scale + legend |
| histogram | `geom_histogram(bins=30)` | binning stat, few marks |
| bar | `geom_bar()` on `group` | counting stat, discrete axis |
| boxplot | `geom_boxplot()` by group | quantiles + outliers |
| violin | `geom_violin()` by group | KDE per group |
| density | `geom_density(alpha=0.3)`, `fill="group"` | KDE + transparent overlapping fills |
| smooth | points + `geom_smooth(method="lm")` | marks + model fit with SE band |
| line | `geom_line()`, `color="group"` | sort by x, very long polylines |
| bin2d | `geom_bin2d(bins=30)` | 2D binning + continuous fill scale |
| facet | points + `facet_wrap(~group, ncol=3)` | 6 panels, per-panel layout |

Visual check, 10k rows (columns: ggrust, lets-plot, ggplot2). The three libraries draw the
same data, bins, quantiles, densities, fits and panels. Style differences are each
library's defaults: ggplot2's grey theme, its larger points at `size=1`, smaller text at
this pixel size, and its alphabetical level order (lets-plot and ggrust use first
appearance).

![comparison 1](results/three-way-20261108-images/comparison-1.png)

![comparison 2](results/three-way-20261108-images/comparison-2.png)

## Methodology

**Machine.** Apple M4 Pro (12 cores), 24 GB RAM, macOS 27.0.1, Python 3.14.
**Versions.** ggrust 0.8.0 (release build), lets-plot 4.11.0, R 4.5.2 with ggplot2 4.0.1,
svglite 2.2.2, ragg 1.5.0, data.table for reading.

**Fair work.**
- Same data: one CSV per size, read by each library before timing starts (reading is
  not timed: polars in Python, `data.table::fread` in R).
- Every row drawn: lets-plot runs with `sampling="none"` on every layer (its default
  silently draws only 100k points; see `memory/letsplot_default_sampling.md`). ggrust and
  ggplot2 never sample.
- Same output: 600 x 400 px; PNG at scale 2 (1200 x 800 px). R uses the devices `ggsave`
  picks: `svglite` for SVG, `ragg::agg_png` (res 144) for PNG. Every library writes a file.
- Timed span: build the plot object, compute stats, lay out, render, write the file.
  For R that is `ggplot(...)` + `print(plot)` into the device + `dev.off()`.

**Fresh processes.** Each (library, scenario, size, format) runs in its own process, so
caches and heaps never carry over. Time: the first render is "cold", the reported number
is the median of the next three ("warm"); a configuration stops repeating after 30 s
in total. Timeout 300 s.

**Memory.** Each worker runs under `/usr/bin/time -l`. Memory = peak RSS of the process
minus its RSS after imports and data load (measured with `ps` the same way in Python
and R). So it covers what rendering allocates, including any one-time renderer setup
(lets-plot's PNG path costs ~200 MB even for a 10k-row histogram).

**Not measured.** Interactive HTML (lets-plot renders it in the browser, R has no
equivalent in ggplot2), notebook display, PDF.

**Reproduce.**

```bash
uv run python benchmarks/three_way.py --work /tmp/ggrust-bench     # ~45 min on an M4 Pro
uv run python benchmarks/three_way_charts.py benchmarks/results/three-way-<stamp>.json ...  # writes docs/images/benchmarks/
```

Raw results: `results/three-way-20261108-112228.json` (10k) and
`results/three-way-20261108-112328.json` (100k, 1M) for all three libraries, and
`results/three-way-20261108-123445.json`, the ggrust re-run at `a3e895a` that replaces
ggrust's earlier rows. Pass all three to `three_way_charts.py` in that order.

## Full results

Time in seconds (warm median), memory in MB above the data. Speed-ups are lets-plot or
ggplot2 time divided by ggrust time. ggrust rows are from the `a3e895a` re-run.

| scenario | rows | format | ggrust s | lets-plot s | ggplot2 s | vs lets-plot | vs ggplot2 | ggrust MB | lets-plot MB | ggplot2 MB |
|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|
| scatter | 10,000 | png | 0.008 | 0.624 | 0.096 | 80.3x | 12.3x | 23 | 250 | 30 |
| scatter | 10,000 | svg | 0.002 | 0.082 | 0.050 | 38.2x | 23.0x | 12 | 57 | 33 |
| color_discrete | 10,000 | png | 0.009 | 0.689 | 0.146 | 75.7x | 16.0x | 24 | 277 | 40 |
| color_discrete | 10,000 | svg | 0.003 | 0.088 | 0.099 | 26.9x | 30.0x | 14 | 57 | 44 |
| histogram | 10,000 | png | 0.012 | 0.313 | 0.054 | 26.8x | 4.7x | 22 | 200 | 26 |
| histogram | 10,000 | svg | 0.001 | 0.027 | 0.043 | 29.7x | 47.8x | 11 | 27 | 21 |
| bar | 10,000 | png | 0.009 | 0.336 | 0.056 | 35.4x | 5.9x | 24 | 199 | 22 |
| bar | 10,000 | svg | 0.001 | 0.027 | 0.045 | 28.6x | 47.6x | 12 | 27 | 22 |
| boxplot | 10,000 | png | 0.007 | 0.307 | 0.079 | 42.3x | 10.9x | 23 | 201 | 37 |
| boxplot | 10,000 | svg | 0.001 | 0.034 | 0.065 | 27.3x | 51.4x | 13 | 26 | 34 |
| violin | 10,000 | png | 0.011 | 0.641 | 0.080 | 56.4x | 7.0x | 23 | 218 | 62 |
| violin | 10,000 | svg | 0.004 | 0.202 | 0.066 | 48.8x | 16.0x | 15 | 32 | 49 |
| density | 10,000 | png | 0.016 | 0.907 | 0.114 | 57.6x | 7.2x | 24 | 213 | 38 |
| density | 10,000 | svg | 0.005 | 0.217 | 0.098 | 45.3x | 20.4x | 13 | 30 | 31 |
| smooth | 10,000 | png | 0.008 | 0.663 | 0.118 | 78.2x | 14.0x | 22 | 273 | 162 |
| smooth | 10,000 | svg | 0.002 | 0.088 | 0.073 | 46.0x | 38.0x | 13 | 54 | 163 |
| line | 10,000 | png | 0.034 | 2.167 | 0.147 | 64.2x | 4.4x | 25 | 216 | 71 |
| line | 10,000 | svg | 0.003 | 0.064 | 0.097 | 23.9x | 35.9x | 14 | 35 | 54 |
| bin2d | 10,000 | png | 0.008 | 0.311 | 0.093 | 39.4x | 11.8x | 22 | 202 | 34 |
| bin2d | 10,000 | svg | 0.001 | 0.031 | 0.081 | 28.8x | 74.1x | 12 | 26 | 25 |
| facet | 10,000 | png | 0.021 | 1.453 | 0.191 | 70.5x | 9.2x | 44 | 567 | 35 |
| facet | 10,000 | svg | 0.002 | 0.089 | 0.146 | 51.3x | 84.0x | 15 | 51 | 39 |
| scatter | 100,000 | png | 0.019 | 4.210 | 0.528 | 224.0x | 28.1x | 29 | 1300 | 163 |
| scatter | 100,000 | svg | 0.009 | 0.889 | 0.206 | 95.1x | 22.0x | 24 | 376 | 173 |
| color_discrete | 100,000 | png | 0.029 | 4.362 | 0.530 | 150.4x | 18.3x | 42 | 1326 | 104 |
| color_discrete | 100,000 | svg | 0.025 | 0.977 | 0.270 | 39.6x | 10.9x | 50 | 382 | 163 |
| histogram | 100,000 | png | 0.013 | 0.543 | 0.060 | 41.7x | 4.6x | 29 | 247 | 88 |
| histogram | 100,000 | svg | 0.003 | 0.253 | 0.048 | 92.2x | 17.5x | 17 | 77 | 70 |
| bar | 100,000 | png | 0.012 | 0.578 | 0.074 | 48.9x | 6.3x | 24 | 246 | 79 |
| bar | 100,000 | svg | 0.003 | 0.260 | 0.060 | 82.3x | 19.0x | 19 | 77 | 86 |
| boxplot | 100,000 | png | 0.015 | 0.632 | 0.093 | 42.2x | 6.2x | 37 | 248 | 110 |
| boxplot | 100,000 | svg | 0.006 | 0.338 | 0.077 | 56.2x | 12.8x | 30 | 104 | 65 |
| violin | 100,000 | png | 0.016 | 1.037 | 0.097 | 63.1x | 5.9x | 37 | 255 | 134 |
| violin | 100,000 | svg | 0.010 | 0.585 | 0.081 | 60.0x | 8.3x | 30 | 113 | 79 |
| density | 100,000 | png | 0.017 | 1.189 | 0.093 | 71.5x | 5.6x | 37 | 244 | 77 |
| density | 100,000 | svg | 0.006 | 0.505 | 0.076 | 83.1x | 12.5x | 27 | 94 | 92 |
| smooth | 100,000 | png | 0.020 | 4.064 | 0.558 | 200.5x | 27.5x | 33 | 1308 | 238 |
| smooth | 100,000 | svg | 0.011 | 0.890 | 0.272 | 81.1x | 24.8x | 29 | 378 | 251 |
| line | 100,000 | png | 0.232 | 24.128 | 0.539 | 104.1x | 2.3x | 40 | 380 | 372 |
| line | 100,000 | svg | 0.017 | 0.650 | 0.197 | 37.7x | 11.4x | 31 | 166 | 141 |
| bin2d | 100,000 | png | 0.011 | 0.552 | 0.092 | 52.3x | 8.7x | 28 | 248 | 133 |
| bin2d | 100,000 | svg | 0.003 | 0.267 | 0.081 | 104.3x | 31.6x | 18 | 81 | 70 |
| facet | 100,000 | png | 0.034 | 5.036 | 0.570 | 149.0x | 16.9x | 53 | 1640 | 209 |
| facet | 100,000 | svg | 0.008 | 0.931 | 0.291 | 120.5x | 37.7x | 38 | 343 | 141 |
| scatter | 1,000,000 | png | 0.119 | 39.790 | 4.247 | 335.8x | 35.8x | 52 | 8161 | 761 |
| scatter | 1,000,000 | svg | 0.086 | 9.071 | 1.571 | 104.9x | 18.2x | 122 | 3490 | 740 |
| color_discrete | 1,000,000 | png | 0.209 | 42.579 | 4.301 | 203.6x | 20.6x | 238 | 8857 | 770 |
| color_discrete | 1,000,000 | svg | 0.237 | 9.480 | 1.835 | 40.1x | 7.8x | 369 | 3357 | 818 |
| histogram | 1,000,000 | png | 0.032 | 3.054 | 0.203 | 95.1x | 6.3x | 122 | 733 | 414 |
| histogram | 1,000,000 | svg | 0.022 | 2.769 | 0.193 | 127.3x | 8.9x | 114 | 616 | 416 |
| bar | 1,000,000 | png | 0.035 | 3.162 | 0.216 | 90.2x | 6.2x | 98 | 754 | 415 |
| bar | 1,000,000 | svg | 0.027 | 2.799 | 0.207 | 102.4x | 7.6x | 87 | 622 | 399 |
| boxplot | 1,000,000 | png | 0.062 | 4.755 | 0.334 | 76.2x | 5.4x | 220 | 1040 | 469 |
| boxplot | 1,000,000 | svg | 0.054 | 4.065 | 0.334 | 74.6x | 6.1x | 214 | 903 | 543 |
| violin | 1,000,000 | png | 0.067 | 6.021 | 0.319 | 89.9x | 4.8x | 162 | 1096 | 498 |
| violin | 1,000,000 | svg | 0.060 | 5.602 | 0.312 | 93.5x | 5.2x | 154 | 882 | 515 |
| density | 1,000,000 | png | 0.044 | 5.708 | 0.232 | 130.0x | 5.3x | 118 | 864 | 403 |
| density | 1,000,000 | svg | 0.033 | 5.003 | 0.220 | 150.0x | 6.6x | 114 | 764 | 345 |
| smooth | 1,000,000 | png | 0.132 | 40.869 | 4.715 | 308.8x | 35.6x | 142 | 8457 | 1116 |
| smooth | 1,000,000 | svg | 0.100 | 9.688 | 1.787 | 96.8x | 17.9x | 208 | 3562 | 986 |
| line | 1,000,000 | png | 1.982 | >300 | 5.445 | - | 2.7x | 276 | timeout | 1854 |
| line | 1,000,000 | svg | 0.197 | 8.728 | 1.039 | 44.2x | 5.3x | 323 | 1489 | 1089 |
| bin2d | 1,000,000 | png | 0.026 | 3.523 | 0.283 | 135.9x | 10.9x | 72 | 847 | 467 |
| bin2d | 1,000,000 | svg | 0.018 | 3.197 | 0.283 | 180.3x | 16.0x | 61 | 689 | 577 |
| facet | 1,000,000 | png | 0.140 | 45.489 | 4.698 | 326.0x | 33.7x | 133 | 9023 | 962 |
| facet | 1,000,000 | svg | 0.073 | 11.229 | 1.887 | 153.7x | 25.8x | 248 | 3170 | 801 |
