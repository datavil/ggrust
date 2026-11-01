# Scales and legends

A scale turns data values into something visible: a position on an axis, a color, a
size. Every mapped aesthetic has a scale. ggrust picks a default one from the column's
type; add a `scale_*` to change it.

## Discrete colors

`scale_color_brewer` and `scale_fill_brewer` use the ColorBrewer palettes
(`"Set1"`, `"Set2"`, `"Dark2"`, ...). `scale_*_manual(values=[...])` takes your own.

```python
--8<-- "scatter.py:code"
```

--8<-- "figures/scatter.html"

## Continuous colors

A numeric column gets a gradient and a colorbar. `scale_fill_gradient2` diverges around
a `midpoint`; `scale_*_gradientn` takes any list of colors; `scale_*_viridis` uses the
viridis palettes.

```python
--8<-- "heatmap.py:code"
```

--8<-- "figures/heatmap.html"

## Position scales

`scale_x_continuous` and `scale_y_continuous` set `breaks`, `labels`, `limits` and a
`format`. `scale_x_log10`, `scale_x_log2` and `scale_x_reverse` (and their `y` twins) change
the axis itself. Here the x axis is logarithmic and `size` maps calories:

```python
--8<-- "text_repel.py:code"
```

--8<-- "figures/text_repel.html"

## Titles and legends

`labs()` names the plot, the axes and each legend by aesthetic
(`labs(color="Neighbourhood")`). `theme(legend_position=...)` moves legends to
`"right"`, `"left"`, `"top"`, `"bottom"` or hides them with `"none"`; `guides()` and
`guide_legend()` / `guide_colorbar()` tune a single legend.

All scales are listed in the [API reference](../reference/scales.md).
