# Benchmarks

*November 2026. ggrust 0.8.0, ggplot2 4.0.1, lets-plot 4.11.0.*

ggrust does the heavy work of a plot (stats, scales, layout, rendering) in Rust. Here is
what that buys you compared with the two libraries people usually reach for: R's
**ggplot2**, the reference grammar of graphics, and **lets-plot**, whose Python API ggrust
follows. We ran 11 everyday plots at 10k, 100k and 1M rows, saved each as SVG and PNG,
and measured time and memory. Every chart on this page uses linear axes and was drawn by
ggrust.

![ggrust vs ggplot2 vs lets-plot at 1M rows](images/benchmarks/overview.png#only-light)
![ggrust vs ggplot2 vs lets-plot at 1M rows](images/benchmarks/overview-dark.png#only-dark)

At 1M rows the typical plot takes **62 ms** in ggrust, **334 ms** in ggplot2 and **5.9 s**
in lets-plot, and ggrust needs 4x less memory than ggplot2 and 8x less than lets-plot.
ggrust is faster in 64 of the 66 cases against ggplot2 and in all 66 against lets-plot.
The two cases it loses are [described below](#where-ggrust-is-slower).

## Summary

Total time for all 11 plots, and the median peak memory per plot. *Factor* is the time
divided by ggrust's.

=== "1M rows"

    | library | SVG total | factor | PNG total | factor | memory per plot |
    |---|---:|---:|---:|---:|---:|
    | **ggrust** 0.8.0 | **0.89 s** | **1.0** | **18.1 s** | **1.0** | **129 MB** |
    | ggplot2 4.0.1 | 9.67 s | 10.9 | 25.0 s | 1.4 | 560 MB |
    | lets-plot 4.11.0 | 71.6 s | 80.5 | > 495 s | > 27 | 1.0 GB |

    The PNG totals are dominated by one plot: the 1M-row line chart takes 17.3 s of
    ggrust's 18.1 s. Without it, ggrust renders the other 10 PNGs in 0.8 s and ggplot2 in
    19.5 s. lets-plot did not finish the line PNG within 300 s; it counts as 300 s here.

=== "100k rows"

    | library | SVG total | factor | PNG total | factor | memory per plot |
    |---|---:|---:|---:|---:|---:|
    | **ggrust** 0.8.0 | **0.10 s** | **1.0** | **1.06 s** | **1.0** | **28 MB** |
    | ggplot2 4.0.1 | 1.66 s | 17.1 | 3.23 s | 3.0 | 121 MB |
    | lets-plot 4.11.0 | 6.55 s | 67.3 | 46.3 s | 43.5 | 248 MB |

=== "10k rows"

    | library | SVG total | factor | PNG total | factor | memory per plot |
    |---|---:|---:|---:|---:|---:|
    | **ggrust** 0.8.0 | **0.02 s** | **1.0** | **0.15 s** | **1.0** | **18 MB** |
    | ggplot2 4.0.1 | 0.86 s | 35.2 | 1.17 s | 7.8 | 36 MB |
    | lets-plot 4.11.0 | 0.95 s | 38.8 | 8.41 s | 56.2 | 128 MB |

## Rules

A benchmark is only as good as its fairness. Every library does the same work:

- **Same data.** One CSV per size, read before the clock starts (polars in Python,
  `data.table::fread` in R). Reading is not timed.
- **Every row drawn.** lets-plot silently samples large layers by default (a scatter
  draws at most 100k points), so it runs with `sampling="none"`. ggrust and ggplot2
  never sample.
- **Same output.** 600 x 400 px; PNG at scale 2 (1200 x 800 px). R uses the devices
  `ggsave` picks: svglite for SVG and ragg for PNG.
- **The whole job is timed.** From building the plot object to the file on disk:
  stats, scales, layout, rendering and writing.
- **Fresh processes.** Each library, plot, size and format runs in its own process.
  Reported time is the median of three warm renders after a first, cold one.
- **Memory** is the peak resident memory of that process minus its memory after
  imports and data load, so it covers everything rendering allocates.

## The plots

The data looks like everyday data: two numeric columns (`x`, `y`) of shifted, widening
normals and a 6-level `group` column.

| plot | spec | what it stresses |
|---|---|---|
| scatter | `geom_point(size=1)` | one mark per row |
| color_discrete | `geom_point(size=1)`, `color="group"` | marks, discrete scale, legend |
| histogram | `geom_histogram(bins=30)` | binning stat |
| bar | `geom_bar()` on `group` | counting stat, discrete axis |
| boxplot | `geom_boxplot()` by group | quantiles and outliers |
| violin | `geom_violin()` by group | a kernel density per group |
| density | `geom_density(alpha=0.3)`, `fill="group"` | densities, overlapping transparent fills |
| smooth | points + `geom_smooth(method="lm")` | marks and a model fit with its band |
| line | `geom_line()`, `color="group"` | sorting, very long lines |
| bin2d | `geom_bin2d(bins=30)` | 2D binning, continuous fill scale |
| facet | points + `facet_wrap("group", ncol=3)` | 6 panels |

All three libraries draw the same picture: the same bins, quartiles, densities, fits
and panels. The visible differences are each library's default style (ggplot2's grey
theme, its larger points at `size=1` and its alphabetical group order).

??? note "Show the outputs side by side (10k rows: ggrust, lets-plot, ggplot2)"

    ![Scatter to violin, side by side](images/benchmarks/comparison-1.png)

    ![Density to facets, side by side](images/benchmarks/comparison-2.png)

## Results

### Time

Each panel has its own linear axis, so you can read every plot's gap.

=== "PNG"

    ![Render time to PNG per plot at 1M rows](images/benchmarks/time-png.png#only-light)
    ![Render time to PNG per plot at 1M rows](images/benchmarks/time-png-dark.png#only-dark)

=== "SVG"

    ![Render time to SVG per plot at 1M rows](images/benchmarks/time-svg.png#only-light)
    ![Render time to SVG per plot at 1M rows](images/benchmarks/time-svg-dark.png#only-dark)

Plots that draw a mark per row (scatter, smooth, facet) gain the most: at 1M rows ggrust
writes the PNG in 0.11 to 0.14 s, ggplot2 in 4.2 to 4.7 s and lets-plot in 40 to 46 s.
Stat plots (histogram, bar, boxplot, violin, density, bin2d) draw few marks, so the stat
dominates. R's stats are vectorised C, which makes this where ggplot2 comes closest:
ggrust is 5x to 16x faster at 1M rows.

### Memory

=== "PNG"

    ![Peak memory while rendering to PNG per plot at 1M rows](images/benchmarks/memory-png.png#only-light)
    ![Peak memory while rendering to PNG per plot at 1M rows](images/benchmarks/memory-png-dark.png#only-dark)

=== "SVG"

    ![Peak memory while rendering to SVG per plot at 1M rows](images/benchmarks/memory-svg.png#only-light)
    ![Peak memory while rendering to SVG per plot at 1M rows](images/benchmarks/memory-svg-dark.png#only-dark)

Data crosses into Rust once, as Arrow, and is not copied into a JSON spec. A 1M-point
scatter PNG peaks 52 MB above the data in ggrust, 761 MB in ggplot2 and 8.0 GB in
lets-plot.

### As data grows

![Scatter plot render time as data grows](images/benchmarks/scaling.png#only-light)
![Scatter plot render time as data grows](images/benchmarks/scaling-dark.png#only-dark)

## Where ggrust is slower

**Line charts saved as PNG with 100k rows or more.** The `line` plot draws 6 lines of
about 166k vertices each at 1M rows, zigzagging across the whole panel. ggrust takes
0.89 s at 100k rows (ggplot2 0.54 s) and 17.3 s at 1M rows (ggplot2 5.4 s, lets-plot more
than 300 s). The same plot as SVG is 5x faster in ggrust than in ggplot2, so the cost is
rasterising very long anti-aliased lines, not the stat or the layout. A fix is in
progress.

## Setup

- **Machine.** Apple M4 Pro (12 cores), 24 GB memory, macOS 27.
- **Python.** Python 3.14, ggrust 0.8.0 (release build), lets-plot 4.11.0, polars.
- **R.** R 4.5.2, ggplot2 4.0.1, svglite 2.2.2, ragg 1.5.0, data.table.
- **Not measured.** Interactive HTML (lets-plot renders it in the browser, and ggplot2
  has no equivalent), notebook display and PDF.

## Reproduce

The scripts live in the repository's
[`benchmarks/`](https://github.com/datavil/ggrust/tree/main/benchmarks) folder:
`three_way.py` drives the runs, `three_way.R` is the ggplot2 worker and
`three_way_charts.py` draws the charts on this page. Raw numbers and the full table are
in [`REPORT-three-way.md`](https://github.com/datavil/ggrust/blob/main/benchmarks/REPORT-three-way.md).

```bash
uv run python benchmarks/three_way.py --work /tmp/ggrust-bench
uv run python benchmarks/three_way_charts.py benchmarks/results/three-way-<stamp>.json
```

The full run takes about 45 minutes, most of it lets-plot at 1M rows. Pull requests that
make any of the three libraries faster or the comparison fairer are welcome.
