---
hide: [navigation, toc]
---

<div class="hero" markdown>

<div markdown>

<h1>ggrust</h1>

<p class="tagline">The grammar of graphics, with a Rust engine and a lets-plot API.</p>

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

</div>

## A taste

```python
--8<-- "hero.py:code"
```

ggrust was built for [cellestial](https://github.com/datavil/cellestial), a single-cell
plotting library, after lets-plot's Kotlin notebook support ended. It is a general
plotting library: the [gallery](gallery/index.md) uses everyday data.
