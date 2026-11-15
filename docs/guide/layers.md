# Layers

A layer is a **geom** (the shape that is drawn), a **stat** (what is computed from the
data first) and a **position** (how overlapping shapes are arranged). The `geom_*`
functions set all three with sensible defaults.

## Stacking layers

Layers are drawn in the order they are added. Here a boxplot summarises each transport
mode, and a jitter layer draws every commuter on top of it.

```python
--8<-- "boxplot.py:code"
```

--8<-- "figures/boxplot.html"

A mapping given to `ggplot()` is shared by every layer. A mapping given to a layer, like
`aes(color="mode")` above, applies to that layer only.

## Stats

`geom_histogram` uses the `bin` stat: it counts rows in bins and draws one bar per bin.

```python
--8<-- "histogram.py:code"
```

--8<-- "figures/histogram.html"

When the data already holds the values to draw, use `stat="identity"`. Here each row
is a number of cups:

```python
--8<-- "stacked_bars.py:code"
```

--8<-- "figures/stacked_bars.html"

A stat can also be the layer itself, as in ggplot2: `stat_bin`, `stat_count`,
`stat_density`, `stat_smooth`, `stat_boxplot` and the others draw their usual geom,
or any other through `geom=`:

```python
from ggrust import *
from ggrust.data import mpg

ggplot(mpg, aes("hwy")) + stat_bin(geom="step", bins=15)
```

## Positions

Bars that share an x value are stacked by default. `position="dodge"` puts them side by
side instead; `"fill"` stacks them to 100 percent. `position_dodge(width=...)`,
`position_jitter(...)` and `position_nudge(...)` take parameters.

```python
--8<-- "dodged_bars.py:code"
```

--8<-- "figures/dodged_bars.html"

## Annotations

A few layers mark up a plot instead of showing data:

- `geom_band` draws a band across the whole panel between `xmin` and `xmax`
  (vertical) or `ymin` and `ymax` (horizontal), mapped or constant
  (`geom_band(xmin=1, xmax=2)`), as lets-plot's.
- `annotate(geom, x, y, ...)` adds a one-off layer from constants, as ggplot2's.
- An infinite position (`float("inf")`, `float("-inf")`) is drawn at the panel's
  edge and does not stretch the axis, as in ggplot2. This works for every geom.
- `nudge_x` and `nudge_y` on text, label and bracket layers move by data units by
  default; `nudge_unit="px"` (or `"size"`) moves by pixels, whatever the data range.
  `geom_label(alpha_stroke=True)` fades the text and border with `alpha`, not only
  the fill.

```python
--8<-- "annotations.py:code"
```

--8<-- "figures/annotations.html"

## Prebuilt plots

`ggrust.bistro` holds whole plots that would otherwise take several layers, with
lets-plot's names and arguments. Each returns an ordinary plot, so scales, labels and
themes are added with `+`.

| Function | Draws |
|---|---|
| [`joint_plot`](../reference/api/joint_plot.md) | Two variables as points, 2D bins or a 2D density, with a regression line and their distributions on the margins. |
| [`residual_plot`](../reference/api/residual_plot.md) | The residuals of a polynomial (`method="lm"`, `deg=`) or local (`"loess"`) fit of y on x. |
| [`qq_plot`](../reference/api/qq_plot.md) | A sample against a theoretical distribution, or two samples against each other. |
| [`corr_plot`](../reference/api/corr_plot.md) | A correlation matrix as tiles, points or labels. |
| [`image_matrix`](../reference/api/image_matrix.md) | A grid of images from a 2D array of arrays. |
| [`funky_heatmap`](../reference/api/funky_heatmap.md) | A table of bars, circles, pies and text, after the R package funkyheatmap. |

```python
--8<-- "joint_plot.py:code"
```

--8<-- "figures/joint_plot.html"

## Looking inside a plot

A plot can report what it computed, without drawing:

```python
from ggrust import *
from ggrust.data import mpg

plot = ggplot(mpg, aes("displ", "hwy")) + geom_point() + facet_wrap("drv")

plot  # in a script, repr(plot) is one line: <PlotSpec: 1 layer (point), facet_wrap(~drv)>
plot.layer_data(0)  # the layer's data after its stat and position, as a polars frame
plot.scale_info("x")  # limits, breaks and labels of the x scale, per panel
plot.panels()  # each panel's pixel box and data ranges
plot.columns  # every data column the plot reads
plot.validate()  # builds without drawing and returns the warnings

geom_point().defaults  # a geom's default stat, position and aesthetics
theme_defaults("classic")  # what a named theme sets, in theme() vocabulary
```

`plot.guide_data("color")` returns the keys of a legend, and `plot.build()`
everything at once.

## Available geoms

| Kind | Geoms |
|---|---|
| Points | `geom_point`, `geom_jitter`, `geom_sina`, `geom_count`, `geom_dotplot`, `geom_qq` |
| Lines | `geom_line`, `geom_path`, `geom_step`, `geom_segment`, `geom_spoke`, `geom_lollipop`, `geom_hline`, `geom_vline`, `geom_abline`, `geom_function`, `geom_smooth` |
| Areas | `geom_area`, `geom_ribbon`, `geom_area_ridges`, `geom_polygon`, `geom_shape`, `geom_rect`, `geom_band` |
| Bars | `geom_bar`, `geom_col`, `geom_histogram` |
| Distributions | `geom_boxplot`, `geom_violin`, `geom_density`, `geom_density2d`, `geom_contour`, `geom_contour_filled` |
| Ranges | `geom_errorbar`, `geom_crossbar`, `geom_linerange`, `geom_pointrange` |
| Tiles and images | `geom_tile`, `geom_raster`, `geom_imshow` |
| Text | `geom_text`, `geom_label`, `geom_text_repel`, `geom_label_repel`, `geom_bracket`, `geom_bracket_dodge` |

Each one is documented, with every parameter, in the [API reference](../reference/index.md#geometries).
ggplot2 and plotnine spellings work too (`scale_colour_*`, `geom_bin_2d`,
`stat_density_2d`, `coord_equal`, ...); they are listed under
[Aliases](../reference/index.md#aliases).
