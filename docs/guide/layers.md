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

## Positions

Bars that share an x value are stacked by default. `position="dodge"` puts them side by
side instead; `"fill"` stacks them to 100 percent. `position_dodge(width=...)`,
`position_jitter(...)` and `position_nudge(...)` take parameters.

```python
--8<-- "dodged_bars.py:code"
```

--8<-- "figures/dodged_bars.html"

## Available geoms

| Kind | Geoms |
|---|---|
| Points | `geom_point`, `geom_jitter`, `geom_sina` |
| Lines | `geom_line`, `geom_path`, `geom_segment`, `geom_hline`, `geom_vline`, `geom_function` |
| Areas | `geom_area`, `geom_ribbon`, `geom_area_ridges`, `geom_polygon`, `geom_rect` |
| Bars | `geom_bar`, `geom_histogram` |
| Distributions | `geom_boxplot`, `geom_violin` |
| Ranges | `geom_errorbar`, `geom_crossbar`, `geom_linerange`, `geom_pointrange` |
| Tiles and images | `geom_tile`, `geom_raster`, `geom_imshow` |
| Text | `geom_text`, `geom_label`, `geom_text_repel`, `geom_label_repel`, `geom_bracket` |

Each one is documented, with every parameter, in the [API reference](../reference/geoms.md).
