---
hide: [navigation, toc]
---

<div class="hero" markdown>

<div markdown>

<div class="hero-head">

<div class="hero-logo">
  <img src="images/logo/ggrust-logo-light.svg#only-light" alt="ggrust logo" width="112" height="112">
  <img src="images/logo/ggrust-logo-dark.svg#only-dark" alt="ggrust logo" width="112" height="112">
</div>

<h1>ggrust</h1>

<p class="tagline">The grammar of graphics, with a Rust engine and a lets-plot API.</p>

</div>

<div class="badges">
  <span>python</span><span>rust</span><span>grammar of graphics</span>
  <span>interactive</span><span>arrow</span><span>polars</span>
</div>

[:lucide-rocket: Get started](guide/getting-started.md){ .md-button .md-button--primary }
[:lucide-images: Gallery](gallery/index.md){ .md-button }

</div>

<div class="ggrust-figure"><iframe src="plots/hero.html" title="Monthly temperatures" width="480" height="320" scrolling="no"></iframe></div>

</div>

ggrust is a plotting library for Python. You describe a plot with `ggplot`, `aes`,
`geom_*`, `scale_*` and `theme`, joined with `+`, exactly as in
[lets-plot](https://lets-plot.org). Rust computes the stats, scales and layout and
draws the picture. Hover the plot above for tooltips: every plot on this site is a live
ggrust plot.

## Showcase

<div class="gallery showcase" markdown>

[![Clustered heatmap](images/gallery/clustered_heatmap.png)<span>Clustered heatmap</span>](gallery/clustered_heatmap.md)
[![Cluster marks](images/gallery/cluster_marks.png)<span>Cluster marks</span>](gallery/cluster_marks.md)
[![Raincloud plot](images/gallery/raincloud.png)<span>Raincloud plot</span>](gallery/raincloud.md)
[![Side panels](images/gallery/side_panels.png)<span>Side panels</span>](gallery/side_panels.md)
[![Significance brackets](images/gallery/compare_means.png)<span>Significance brackets</span>](gallery/compare_means.md)
[![Pulsar ridges](images/gallery/unknown_pleasures.png)<span>Pulsar ridges</span>](gallery/unknown_pleasures.md)
[![Periodic table](images/gallery/periodic_table.png)<span>Periodic table</span>](gallery/periodic_table.md)
[![Figure layout](images/gallery/figure_layout.png)<span>Figure layout</span>](gallery/figure_layout.md)
[![Beeswarm](images/gallery/beeswarm.png)<span>Beeswarm</span>](gallery/beeswarm.md)
[![Point density](images/gallery/point_density.png)<span>Point density</span>](gallery/point_density.md)
[![Netflix movie durations](images/gallery/netflix_ridges.png)<span>Netflix movie durations</span>](gallery/netflix_ridges.md)
[![One hundred thousand points](images/gallery/many_points.png)<span>One hundred thousand points</span>](gallery/many_points.md)

</div>

[:octicons-arrow-right-24: See the full gallery](gallery/index.md)

## Fast and lean

On identical work (same data, every row drawn, same output size), ggrust renders SVG a
median **40x** faster and PNG **22x** faster than lets-plot, with **14x** lower peak
memory. A one-million-point scatter PNG peaks at 44 MB in ggrust and 9.6 GB in lets-plot.

[:octicons-arrow-right-24: Benchmark report](https://github.com/datavil/ggrust/blob/main/benchmarks/REPORT.md)

## Why ggrust

<div class="grid cards why" markdown>

-   :lucide-layers:{ .lg } **The lets-plot API**

    ---

    `from ggrust import *` instead of `from lets_plot import *`. Specs compose with
    `+` and `as_dict()` gives the same layout, so most code ports as an import swap.

    [:octicons-arrow-right-24: Migrating from lets-plot](migration.md)

-   :lucide-zap:{ .lg } **Fast, in Rust**

    ---

    Stats, scales, layout and rendering run in Rust. Data crosses from Python once, as
    Arrow: polars, pandas, pyarrow and dicts of numpy arrays all work.

    [:octicons-arrow-right-24: Layers](guide/layers.md)

-   :lucide-mouse-pointer-click:{ .lg } **Interactive by default**

    ---

    Notebook and HTML output has tooltips, and zoom and pan with `ggtb()`. Dense point
    layers are drawn on a canvas, so a hundred thousand points stay smooth.

    [:octicons-arrow-right-24: Interactivity](guide/interactivity.md)

-   :lucide-image:{ .lg } **SVG, PNG, PDF, HTML**

    ---

    One embedded font makes text measurement, and every PNG, identical on every
    machine. Export without a browser or any system library.

    [:octicons-arrow-right-24: Export](guide/export.md)

-   :lucide-puzzle:{ .lg } **ggplot2 extensions, built in**

    ---

    Popular R extensions work out of the box, under their R names: ggbeeswarm, ggsci,
    ggpubr, ggforce, gghighlight, ggpointdensity, ggside, ggnewscale, patchwork and more.

    [:octicons-arrow-right-24: ggplot2 extensions](guide/extensions.md)

-   :lucide-file-check:{ .lg } **Ready for publication**

    ---

    Your own fonts with `register_font`, rasterized point layers in vector PDFs,
    panel tags across a figure, secondary axes and axes on any side.

    [:octicons-arrow-right-24: Export](guide/export.md)

</div>

## A taste

```python
--8<-- "hero.py:code"
```

## Install

=== "pip"

    ```bash
    pip install ggrust
    ```

=== "uv"

    ```bash
    uv add ggrust
    ```

=== "poetry"

    ```bash
    poetry add ggrust
    ```

Wheels are published for Linux, macOS and Windows, for Python 3.11 and newer.
Nothing else is needed: no Java, no browser, no JavaScript build.

ggrust was built for [cellestial](https://github.com/datavil/cellestial), a single-cell
plotting library, after lets-plot's Kotlin notebook support ended. It is a general
plotting library: the [gallery](gallery/index.md) uses everyday data.
