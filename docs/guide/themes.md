# Themes

The theme styles everything that is not data: background, grid lines, axis text, legend
placement and fonts.

## Built-in themes

```python
--8<-- "themes.py:code"
```

--8<-- "figures/themes.html"

ggrust has `theme_minimal2` (the default, as in lets-plot), `theme_grey`,
`theme_classic`, `theme_minimal`, `theme_bw`, `theme_light`, `theme_dark`, `theme_linedraw`,
`theme_void` and `theme_none`. The
`flavor_*` functions (`flavor_darcula`, `flavor_solarized_light`, ...) recolor any of
them. `theme_defaults("classic")` returns what a named theme sets, as a dict in
`theme()` vocabulary.

## ggplot2's look

`theme_ggplot2()` gives a plot ggplot2's default look: the `theme_grey` theme with
ggplot2's text sizes and colors and its ticks, and ggplot2's hue palette for discrete
colors and fills instead of lets-plot's ColorBrewer `Set1`:

```python
ggplot(data, aes("age", "weight", color="pet")) + geom_point() + theme_ggplot2()
```

The palette applies to discrete color and fill scales that do not pick one themselves
(no `scale_*()` call, or `scale_color_discrete()`); manual, brewer and viridis scales
keep their colors, and continuous scales already use ggplot2's blue gradient. Another
complete theme added afterwards replaces it, palette included.

Geom defaults stay lets-plot's (point and legend key sizes, boxplot whisker caps, bar
outlines, the dark grey ink of lines and outlines), and so do the axis breaks.

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

## Transparent backgrounds

For a plot on a slide, a web page or a dark notebook, `flavor_transparent()` makes
the theme's white backgrounds transparent: the plot, panel, legend, legend key and
strip backgrounds that the theme fills with its paper color. Backgrounds with a color
of their own stay, such as the grey panel and strips of `theme_grey()`, and so do
borders, grid lines, text and data:

```python
plot + theme_bw() + flavor_transparent()
```

Like the other flavors it stays when a `theme_*()` is added later, and it combines
with them: `flavor_darcula() + flavor_transparent()` keeps Darcula's light text for
a dark background. A `theme(...)` fill, such as
`theme(panel_background=element_rect(fill="ivory"))`, still wins.

For a single file, `ggsave(plot, "plot.png", bg="transparent")` gives the same
picture. `bg` also takes a color (`bg="white"`), which replaces the same white
backgrounds. Insets of the plot follow both. PNG, TIFF, SVG, PDF and HTML keep the
transparency; JPEG and GIF have none and show white.

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
