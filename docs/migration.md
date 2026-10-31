# Migrating from lets-plot

ggrust keeps lets-plot's Python API so that most code only needs a new import.
It was built for [cellestial](https://github.com/datavil/cellestial), whose port is
an import swap.

## The import swap

```python
# before
from lets_plot import *
import lets_plot as lp

# after
from ggrust import *
import ggrust as lp
```

Spec objects behave like lets-plot's: `PlotSpec`, `LayerSpec`, `FeatureSpec`,
`FeatureSpecArray` and `SupPlotsSpec` compose with `+`, and `as_dict()` returns the
same layout, so code that inspects or edits specs keeps working.
`LetsPlot.setup_html()` is accepted and does nothing: every HTML output embeds its own
small runtime, so there is nothing to load.

## What is supported

| Area | Supported |
|---|---|
| Geoms | point, jitter, sina, line, path, segment, hline, vline, rect, polygon, tile, raster, imshow, bar, histogram, boxplot, violin, area_ridges, text, label, text_repel, label_repel, bracket, function, blank |
| Positions | identity, dodge, jitter, jitterdodge, nudge, stack, fill |
| Scales | x/y continuous and discrete, log2, log10, reverse; color/fill continuous, gradient, gradient2, gradientn, viridis, brewer, hue, manual, identity; size, shape, alpha |
| Coordinates and facets | `coord_cartesian`, `coord_fixed`, `coord_flip`, `facet_wrap`, `facet_grid` |
| Layout | `theme` and its elements, theme presets, flavors, `labs`, `guides`, `ggsize`, `gggrid` |
| Interactivity | `layer_tooltips` (lines, format, title), `ggtb` zoom/pan toolbar |
| Output | `_repr_html_`, `to_svg`, `to_png`, `to_html`, `to_pdf`, `ggsave` |

## What is not supported

These lets-plot features are out of scope: maps (livemap, GIS, geocoding), `bistro`
(`corr_plot`, `qq_plot`, ...), smoothing and regression, contour and 2D density, polar
coordinates, pie and waterfall charts, and frontends other than Jupyter and standalone
HTML. `GGBunch` is exported so code that names it still imports, but creating one raises
`NotImplementedError`; use `gggrid` instead.

Options that ggrust accepts for compatibility but does not act on (for example a
layer's `sampling`) raise a `GgrustWarning` that names the option, rather than being
silently ignored.

## Differences in behavior

ggrust aims for the same picture and the same stat results as lets-plot. Where
lets-plot does something that looks wrong, ggrust follows ggplot2 instead. The known
differences:

- **No default sampling.** lets-plot silently samples large layers (for example points
  above 100,000 rows). ggrust draws every row; dense point layers in HTML are drawn on
  a canvas, so they stay fast.
- **Unordered pandas categoricals** keep their category order (as in ggplot2). lets-plot
  uses first appearance. Polars `Categorical` (first appearance) and `Enum` (category
  order) match lets-plot.
- **Unknown export units raise.** `to_png(..., unit="pt")` raises an error that lists the
  accepted units; lets-plot silently uses inches.
- **Export sizes without a unit are inches**, as in lets-plot; a unitless `w` or `h` of
  20 or more raises (it was probably meant as pixels): pass `unit="px"`.
- **PDF export** writes the PNG's pixels on one page at its dpi, like lets-plot, but needs
  no Pillow.
- **HTML structure differs.** ggrust's HTML does not contain lets-plot's
  `plt-container` element or load `lets-plot.min.js`. Code that scrapes lets-plot's
  HTML (for example browser tests) has to target ggrust's output instead.
- **Fonts.** ggrust embeds one open font, so text measurement and PNGs are identical on
  every machine. Text widths, and therefore some label placements, differ slightly from
  lets-plot.
