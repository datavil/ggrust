# Export

Every plot can be written as SVG, PNG, PDF or a standalone interactive HTML page. No
browser, Java or system library is needed.

## Files

```python
plot.to_svg("prices.svg")
plot.to_png("prices.png", scale=2)  # twice the pixels, for sharp slides
plot.to_pdf("prices.pdf")
plot.to_html("prices.html")  # interactive, opens in any browser

ggsave(plot, "prices.png", path="figures")  # the extension picks the format
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
both lets-plot and ggplot2.

## Fonts

ggrust embeds one open font (Liberation Sans, metrically compatible with Arial), so a
PNG is pixel-identical on every machine and in CI.

## Plot files

Save the plot itself, not a picture of it, and open it again in another Python
session, to restyle it, add layers or export it later:

```python
ggsave(plot, "prices.ggrs")

plot = ggload("prices.ggrs")
plot + theme_minimal()
```

A `.ggrs` file is a zip archive holding the plot (as JSON), its data (as Arrow, read
back as the polars, pandas or pyarrow frame it was) and the registered fonts it uses.
It is safe to open from anyone: loading never runs code from the file, and a damaged
file fails with a clear `PlotFileError` (every entry is checked against the SHA-256
hash recorded when it was saved). The hashes catch damage, not deliberate edits, so
they do not prove who made the file.

A plot holding Python functions (a `sec_axis` transform) cannot be saved: a plot file
stores no code. For worker processes, `pickle` works as usual (and can carry functions,
with pickle's usual rule: only unpickle data you trust).

## Notebooks and scripts

In Jupyter, a plot as the last expression of a cell renders inline. `plot.show()` does
the same from code, and outside a notebook it opens the plot in the default browser.
