# Themes

The theme styles everything that is not data: background, grid lines, axis text, legend
placement and fonts.

## Built-in themes

```python
--8<-- "themes.py:code"
```

--8<-- "figures/themes.html"

ggrust has `theme_grey` (the default), `theme_classic`, `theme_minimal`,
`theme_minimal2`, `theme_bw`, `theme_light`, `theme_dark`, `theme_linedraw`,
`theme_void` and `theme_none`. The
`flavor_*` functions (`flavor_darcula`, `flavor_solarized_light`, ...) recolor any of
them.

## Changing details with `theme()`

`theme()` sets single elements. Add it after a built-in theme to adjust it:

```python
(
    plot
    + theme_minimal()
    + theme(
        legend_position="bottom",
        axis_title=element_text(size=14, face="bold"),
        panel_grid_minor=element_blank(),
        plot_background=element_rect(fill="#fbf8f4"),
    )
)
```

- `element_text(...)` styles text: `size`, `color`, `face`, `angle`, `hjust`, `vjust`.
- `element_line(...)` styles lines: `color`, `size`, `linetype`. Axis lines also
  take `arrow` and `extent` (see below).
- `element_rect(...)` styles boxes: `fill`, `color`, `size`.
- `element_blank()` removes the element.

## Arrow axes

Axis lines can end in arrow heads and cover only part of the axis, as the corner
arrows of UMAP plots. `arrow()` sets the heads (sized in pixels), `extent` the
fraction of the panel each line covers from the bottom-left corner. With axis text
and ticks removed and `hjust=0`, the titles sit right next to the arrows:

```python
(
    plot
    + theme_classic()
    + theme(
        axis_line=element_line(arrow=arrow(length=8, type="closed"), extent=0.2),
        axis_text=element_blank(),
        axis_ticks=element_blank(),
        axis_title_x=element_text(hjust=0),
        axis_title_y=element_text(hjust=0),
    )
)
```

The default theme (`theme_minimal2`) hides the y axis line with its own
`axis_line_y`, which wins over `axis_line` (lets-plot inheritance): there, set
`axis_line_x` and `axis_line_y`, or start from a theme with both lines such as
`theme_classic()`.

The arrows belong to the axes, not the data: they keep their size whatever the data
range and stay put while zooming an interactive plot. lets-plot has no such option;
ggplot2 has `element_line(arrow=)` and the ggh4x package truncated axes.

Every theme option is listed in the [API reference](../reference/index.md#themes).
