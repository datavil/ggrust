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
Delete `LetsPlot.setup_html()`: ggrust has no `LetsPlot` and needs no setup, since every
HTML output embeds its own small runtime. For static notebook output (lets-plot's
`no_js=True`), call `ggrust.display("svg")` or `ggrust.display("png")`.

## What is supported

| Area | Supported |
|---|---|
| Geoms | point, jitter, sina, line, path, segment, hline, vline, rect, polygon, tile, raster, imshow, bar, histogram, boxplot, violin, area, ribbon, area_ridges, errorbar, crossbar, linerange, pointrange, text, label, text_repel, label_repel, bracket, function, blank |
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
`NotImplementedError`. Use `gggrid` for a grid, or for plots placed freely (overlap
allowed) add `inset_element(plot, left, bottom, right, top, align_to="full")` to an
empty `ggplot() + theme_void()`. lets_plot's `ggbunch()` is not ported for the same reason.

Options that ggrust accepts for compatibility but does not act on raise a
`GgrustWarning` that names the option, rather than being silently ignored. For
`sampling=` this concerns lets_plot's stratified, group and vertex samplers
(`sampling_random_stratified`, `sampling_pick`, `sampling_group_random`,
`sampling_group_systematic`, `sampling_vertex_vw`, `sampling_vertex_dp`): they build
the lets_plot spec, then warn and draw every row. `sampling_random`,
`sampling_systematic` and `sampling="none"` work.

## Differences in behavior

ggrust aims for the same picture and the same stat results as lets-plot. Where
lets-plot does something that looks wrong, ggrust follows ggplot2 instead. The known
differences:

- **No default sampling.** lets-plot silently samples large layers (for example points
  above 100,000 rows). ggrust draws every row; dense point layers in HTML are drawn on
  a canvas, so they stay fast. To draw fewer rows, ask for it: `sampling=5000` (a
  shortcut for `sampling_random(5000)`), `sampling_systematic(n)`, or ggrust's
  `sampling_fraction(0.2)` (20% of the rows). As in lets-plot, sampling runs on each
  facet panel's stat output (a histogram samples its bins). `sampling_random` picks
  other rows than lets-plot for the same seed; without a seed it uses a fixed one, so
  renders repeat. `sampling_systematic(n)` keeps every `ceil(rows / n)`-th row, which
  can be a few rows off lets-plot's count. `sampling_none()` returns `"none"`
  (lets_plot has no such function; it writes `sampling="none"`).
- **Unordered pandas categoricals** keep their category order (as in ggplot2). lets-plot
  uses first appearance. Polars `Categorical` (first appearance) and `Enum` (category
  order) match lets-plot.
- **Unknown export units raise.** `to_png(..., unit="pt")` raises an error that lists the
  accepted units; lets-plot silently uses inches.
- **Continuous axis `labels` without `breaks` are used.** lets-plot ignores them; ggrust
  follows ggplot2 and puts them on the axis's generated breaks (about 5, the same at any
  plot size, kept on their values under zoom). A different number of labels raises
  `SpecError`, which lists the generated breaks: pass `breaks=` too.
- **Export sizes without a unit are inches**, as in lets-plot and ggplot2; a unitless `w` or `h` of
  20 or more raises (it was probably meant as pixels): pass `unit="px"`.
- **PDF export is vector.** lets-plot converts its PNG to a one-page raster PDF with
  Pillow; ggrust writes paths and real, selectable text in the embedded font, on a page
  of the same physical size. No Pillow needed.
- **HTML structure differs.** ggrust's HTML does not contain lets-plot's
  `plt-container` element or load `lets-plot.min.js`. Code that scrapes lets-plot's
  HTML (for example browser tests) has to target ggrust's output instead.
- **Fonts.** ggrust embeds one open font, so text measurement and PNGs are identical on
  every machine. Text widths, and therefore some label placements, differ slightly from
  lets-plot.
