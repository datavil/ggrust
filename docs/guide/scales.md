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

`rescaler=` takes a function `rescaler(x, to, from_)` (`x` is a polars Series) that
maps the data to 0..1 before the palette, as ggplot2's, for example to put the middle
of a gradient on a chosen value.

## Any aesthetic

Every scale also has a generic form that names its aesthetic, as in lets-plot:
`scale_manual`, `scale_continuous`, `scale_discrete`, `scale_gradient`,
`scale_gradient2`, `scale_gradientn`, `scale_hue`, `scale_grey`, `scale_brewer` and
`scale_viridis`. A list sets the same scale on several aesthetics at once:

```python
from ggrust import *
from ggrust.data import penguins

colors = {"Adelie": "#e07a5f", "Chinstrap": "#3d405b", "Gentoo": "#81b29a"}
(
    ggplot(penguins, aes("species", "body_mass_g", color="species", fill="species"))
    + geom_boxplot(alpha=0.4, na_rm=True)
    + scale_manual(["color", "fill"], values=colors)
)
```

`scale_identity(aesthetic)` uses the data values as they are, for a column that
already holds colors, sizes or shapes: `scale_identity("fill")`, or
`scale_identity(["color", "fill"])`.

## Position scales

`scale_x_continuous` and `scale_y_continuous` set `breaks`, `labels`, `limits` and a
`format`. `scale_x_log10`, `scale_x_log2` and `scale_x_reverse` (and their `y` twins) change
the axis itself. Here the x axis is logarithmic and `size` maps calories:

```python
--8<-- "text_repel.py:code"
```

--8<-- "figures/text_repel.html"

`guides(x=guide_axis(cap="both"))` caps an axis: its line stops at the outermost
breaks instead of running the length of the panel, as ggplot2's `guide_axis(cap=)`.
`guide_axis(title=...)` names the axis.

## Titles and legends

`labs()` names the plot, the axes and each legend by aesthetic
(`labs(color="Neighbourhood")`). `theme(legend_position=...)` moves legends to
`"right"`, `"left"`, `"top"`, `"bottom"` or hides them with `"none"`; `guides()` and
`guide_legend()` / `guide_colorbar()` tune a single legend.

## Legends on several sides

Each guide takes its own `position` (`"top"`, `"right"`, `"bottom"`, `"left"` or
`"inside"`), so one plot can have legends on several sides, as ggplot2 3.5.
`theme(legend_position_inside=[x, y])` places the inside ones, and
`theme(legend_title_position=...)` puts the title above, below or beside the keys
(also per guide, through `guide_legend(theme=theme(...))`).

`scale_size_binned` and `scale_alpha_binned` give every value the size or alpha of
its bin. Their legend is `guide_bins()`: touching keys labelled on their edges
(`show_limits=True` labels the outer edges too). It also works on `scale_*_steps`.

```python
--8<-- "legends.py:code"
```

--8<-- "figures/legends.html"

## Colorbars

`guide_colorbar` has options for a colorbar in the style of matplotlib:
`shape="triangle"` draws pointed caps past the ends (`show=(False, True)` for the
upper one only, `cap_size` for their length) and `minor_ticks=True` adds ticks
between the breaks. In the theme, `legend_frame=element_rect(...)` outlines the bar
and its caps, `legend_ticks` styles the ticks, and `legend_ticks_length` takes two
lengths, one per side of the bar, where a negative length points out of the bar.

```python
--8<-- "colorbar.py:code"
```

--8<-- "figures/colorbar.html"

All scales are listed in the [API reference](../reference/index.md#position-scales).
