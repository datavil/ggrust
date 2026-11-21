# Facets and grids

## Small multiples with `facet_wrap`

`facet_wrap` draws one panel per value of a column. The panels share their scales, so
they are easy to compare.

```python
--8<-- "facets.py:code"
```

--8<-- "figures/facets.html"

`ncol` and `nrow` set the layout. Each panel can have its own axis range with
`scales="free"`, `"free_x"` or `"free_y"`.

Facets combine with any layer. Here each city gets a ribbon from the daily low to the
daily high:

```python
--8<-- "ribbon.py:code"
```

--8<-- "figures/ribbon.html"

## Several plots with `gggrid`

`gggrid` arranges different plots in one figure. Pass a list of plots and `ncol`;
`widths` and `heights` set relative sizes, and `align=True` lines up their panels.

```python
--8<-- "grid.py:code"
```

--8<-- "figures/grid.html"

## Changing plots of a grid

A grid is a container of figures. `grid[i] += feature` adds a feature to one cell, and a
slice adds it to several: every cell, a row, a column or a range of figures.

```python
grid = gggrid([points, bars, line, boxes], ncol=2)
grid[:] += scale_color_viridis()             # every plot
grid[0, :] += geom_smooth()                  # the first row
grid[:, 1] += theme(legend_position="none")  # the second column
grid[1:3] += labs(x="weight")                # figures 1 and 2
```

The feature reaches every plot of the selected cells, inside nested grids too; empty
cells stay empty, and a `design=` area spanning several selected cells gets it once.
This changes the grid in place, like `grid[i] += feature`; grids copied earlier are not
changed. The slice forms are ggrust's own: lets_plot has no grid indexing, so keep the
list form (`gggrid([plot + feature for plot in plots])`) where code must run on both.

Two meanings of `+ theme()`: `grid + theme(...)` themes the grid figure (its background,
titles and collected legends), while `grid[:] += theme(...)` themes each plot, as
`plot + theme(...)` does. The same goes for `ggsize`, `ggtitle` and tags.
