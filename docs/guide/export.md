# Export

Every plot can be written as SVG, PNG, PDF or a standalone interactive HTML page. No
browser, Java or system library is needed.

## Files

```python
plot.to_svg("prices.svg")
plot.to_png("prices.png", scale=2)  # twice the pixels, for sharp slides
plot.to_pdf("prices.pdf")
plot.to_html("prices.html")  # interactive, opens in any browser

gg.ggsave(plot, "prices.png", path="figures")  # the extension picks the format
```

Without a path, `to_svg()` and `to_html()` return the text instead of writing a file.

## Size and resolution

The plot's size comes from `ggsize(width, height)` in pixels (600 by 400 by default).
Exports can override it with `w`, `h` and a `unit`:

```python
plot.to_png("poster.png", w=8, h=5, unit="in", dpi=300)
plot.to_pdf("figure.pdf", w=180, h=120, unit="mm", dpi=300)
plot.to_svg("web.svg", w=800, h=500, unit="px")
```

Units are `"in"`, `"cm"`, `"mm"` and `"px"`. A size without a unit is in inches, as in
lets-plot.

## Fonts

ggrust embeds one open font (Liberation Sans, metrically compatible with Arial), so a
PNG is pixel-identical on every machine and in CI.

## Notebooks and scripts

In Jupyter, a plot as the last expression of a cell renders inline. `plot.show()` does
the same from code, and outside a notebook it opens the plot in the default browser.
