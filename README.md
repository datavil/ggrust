# ggrust

[![GitHub](https://img.shields.io/badge/GitHub-100000?style=flat&logo=github&logoColor=white)](https://github.com/datavil/ggrust)
[![License: MIT](https://img.shields.io/badge/License-MIT-ff0000)](https://opensource.org/licenses/MIT)
[![Docs](https://img.shields.io/badge/Docs-datavil.github.io-377eb8)](https://datavil.github.io/ggrust/)

The __Grammar of Graphics__, written in Rust, used from Python.

__ggrust__ is a plotting library with a [Lets-Plot](https://lets-plot.org/) compatible Python API. Plots are built the familiar ggplot way, layer by layer with `+`, while the stats, scales, layout and rendering all run in Rust.

It was started for [Cellestial](https://github.com/datavil/cellestial), which needed a maintained backend for its Lets-Plot based API. For most code, switching is an import swap: `lets_plot` -> `ggrust`.

Data crosses into Rust once, as Arrow, so [Polars](https://pola.rs/) and pandas frames work directly.

## Installation

```bash
pip install ggrust
```

```bash
uv add ggrust
```

## Quickstart

```python
import polars as pl
from ggrust import *

frame = pl.DataFrame(
    {
        "height": [150, 162, 171, 180, 168, 175],
        "weight": [52, 60, 68, 82, 65, 74],
        "group": ["a", "a", "b", "b", "c", "c"],
    }
)

plot = (
    ggplot(frame, aes(x="height", y="weight", color="group"))
    + geom_point(size=4)
    + labs(title="Height vs weight")
    + ggsize(600, 400)
)

plot.to_png("plot.png")
```

<p align="center">
  <img src="https://raw.githubusercontent.com/datavil/ggrust/main/assets/quickstart.png" alt="Scatter plot coloured by group" width="600">
</p>

In a notebook the plot shows as interactive HTML (tooltips, zoom and pan with `ggtb()`).

## Examples

### Distributions

Violins with boxplots on top, using the same layering as ggplot.

<details>
<summary><b>Show code</b></summary>

```python
import numpy as np

rng = np.random.default_rng(0)
days = ["Mon", "Tue", "Wed", "Thu", "Fri"]
commute = pl.DataFrame(
    {
        "day": np.repeat(days, 200),
        "minutes": np.concatenate([rng.gamma(4 + i, 6, 200) for i in range(5)]),
    }
)

(
    ggplot(commute, aes(x="day", y="minutes", fill="day"))
    + geom_violin(alpha=0.6)
    + geom_boxplot(width=0.15, fill="white")
    + labs(title="Commute time by weekday")
    + theme_classic()
    + ggsize(600, 400)
)
```

</details>

<p align="center">
  <img src="https://raw.githubusercontent.com/datavil/ggrust/main/assets/violin.png" alt="Violin and boxplot of commute times" width="600">
</p>

### Facets

Split a plot into panels by a column.

<details>
<summary><b>Show code</b></summary>

```python
months = np.arange(1, 13)
temperature = pl.DataFrame(
    {
        "month": np.tile(months, 3),
        "temp": np.concatenate(
            [10 + 8 * np.sin((months - 3) / 12 * 2 * np.pi) + offset for offset in (0, 5, -4)]
        ),
        "city": np.repeat(["Lisbon", "Athens", "Oslo"], 12),
    }
)

(
    ggplot(temperature, aes(x="month", y="temp", color="city"))
    + geom_line(size=1.2)
    + geom_point()
    + facet_wrap("city", ncol=3)
    + labs(title="Monthly temperature", y="°C")
    + theme_bw()
    + ggsize(900, 350)
)
```

</details>

<p align="center">
  <img src="https://raw.githubusercontent.com/datavil/ggrust/main/assets/facets.png" alt="Monthly temperature faceted by city" width="800">
</p>

## What it covers

- __Geoms__: point, line, path, bar, histogram, boxplot, violin, area ridges, tile, raster, rect, text, label, text/label repel, segment, polygon, imshow, brackets and more.
- __Scales__: continuous, discrete, log, reversed, hue, manual, gradients, viridis, brewer.
- __Layout__: `facet_wrap`, `gggrid` with collected legends, `coord_fixed`, `coord_flip`, themes (`theme_classic`, `theme_bw`, `theme_void`).
- __Outputs__: SVG, PNG, PDF, and HTML through `ggsave`. Dense scatter layers are drawn on a canvas in HTML.

ggrust is a __subset__ of Lets-Plot, not all of it. Features follow what Cellestial and everyday plotting need. See the [migration note](https://datavil.github.io/ggrust/migration/) for the differences.

## Status

Early, but usable: the full Cellestial test suite passes on ggrust. In the [benchmark report](benchmarks/REPORT.md) it renders a median 40x faster than Lets-Plot for SVG and 22x for PNG, on equal work. Expect rough edges and API gaps; issues are welcome.

## Development

```bash
uv sync          # create the dev environment
make develop     # build the Rust extension into .venv
make check       # fmt, clippy, ruff, cargo test, pytest
```

Roadmap in [plan.md](plan.md), contributor rules in [AGENTS.md](AGENTS.md).
