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
