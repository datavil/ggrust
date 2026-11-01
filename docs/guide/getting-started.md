# Getting started

## Install

```bash
pip install ggrust
```

ggrust works in Jupyter, VS Code notebooks and plain Python scripts. In a notebook, a
plot shows itself when it is the last expression of a cell. In a script, call
`plot.show()` to open it in the browser, or [export it](export.md) to a file.

## A first plot

A plot has three parts: **data**, a **mapping** from columns to visual properties
(aesthetics), and at least one **layer** that draws the mapped data.

```python
--8<-- "first_plot.py:code"
```

--8<-- "figures/first_plot.html"

- `ggplot(homes, ...)` sets the data: a polars or pandas DataFrame, a pyarrow table,
  or a dict of lists or numpy arrays.
- `aes(x="area_m2", y="price_k")` maps two columns to the x and y positions.
- `geom_point()` adds a layer that draws one point per row.

The parts are joined with `+`. Each `+` returns a new plot, so a base plot can be reused.

## Mapped and constant aesthetics

Inside `aes()`, an aesthetic follows a column, and a scale and a legend are made for
it. Outside `aes()`, as a layer argument, it is one constant value for the whole layer.

```python
gg.geom_point(gg.aes(color="neighbourhood"))  # one color per neighbourhood, with a legend
gg.geom_point(color="#b7410e")  # every point the same color
```

## Labels, a theme and a size

Add more pieces with `+`: `labs()` for titles and axis names, a theme, and `ggsize()`
for the size in pixels.

```python
--8<-- "first_plot_styled.py:code"
```

--8<-- "figures/first_plot_styled.html"

## The grammar in one picture

Every plot goes through the same steps, as in ggplot2 and lets-plot:

1. **Data** and the **mapping** pick columns for each aesthetic.
2. Each layer's **stat** summarises the data (`identity` keeps it as is; `bin`,
   `boxplot`, `ydensity` and others compute new values).
3. A **position** adjusts overlapping marks (`stack`, `dodge`, `jitter`).
4. **Scales** turn data values into pixels, colors and sizes, and make axes and legends.
5. **Facets** split the data into panels; a **coordinate system** places them.
6. The **theme** styles everything that is not data.

The next pages take these one at a time, starting with [layers](layers.md).
