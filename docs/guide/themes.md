# Themes

The theme styles everything that is not data: background, grid lines, axis text, legend
placement and fonts.

## Built-in themes

```python
--8<-- "themes.py:code"
```

--8<-- "figures/themes.html"

ggrust has `theme_grey` (the default), `theme_classic`, `theme_minimal`,
`theme_minimal2`, `theme_bw`, `theme_light`, `theme_void` and `theme_none`. The
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
- `element_line(...)` styles lines: `color`, `size`, `linetype`.
- `element_rect(...)` styles boxes: `fill`, `color`, `size`.
- `element_blank()` removes the element.

Every theme option is listed in the [API reference](../reference/index.md#themes).
