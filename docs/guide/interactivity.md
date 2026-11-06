# Interactivity

In a notebook, and in HTML export, ggrust plots are interactive. The picture is drawn by
Rust; a small embedded script (no download, no CDN) adds the interaction.

## Tooltips

Hovering a mark shows a tooltip with the values mapped to it. `layer_tooltips()`
chooses the lines: `@column` inserts a value, text before `|` is the line's label.
`tooltips="none"` turns them off for a layer.

```python
--8<-- "tooltips.py:code"
```

--8<-- "figures/tooltips.html"

`.format("price_k", ".0f")` formats a value, `.anchor("top_right")` pins the tooltip
to a corner and `.min_width(...)` sets its width.

## Zoom and pan

Add `ggtb()` for the toolbar above the plot (try it on the plot above). It has pan,
rubber-band zoom, centerpoint zoom and reset. Axes are recomputed at every step, so
breaks and labels stay readable.

## Large data

Point layers with more than 5,000 points (2,000 when the plot has a toolbar, `ggtb()`) are drawn on a
canvas from binary buffers instead of thousands of SVG elements, so zooming stays fast. ggrust does not sample: every row is drawn (lets-plot
samples layers above 100,000 points by default).

```python
--8<-- "many_points.py:code"
```

--8<-- "figures/many_points.html"
