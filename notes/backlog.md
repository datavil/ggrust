# Backlog

Ideas and gaps that are **not scheduled**. Scheduled work lives in [plan.md](../plan.md)
and [plans/](plans/). When an item is picked up, move it to `plan.md` and delete it
here. Every item still needs checking against lets-plot and ggplot2 before coding
(AGENTS.md, "Parity with lets-plot").

Source tags: `[gaps]` ggplot-rs comparison (2026-11-10, from the crate source of
[ggplot-rs](https://github.com/sipemu/ggplot-rs) 0.42.1, MIT OR Apache-2.0; ideas only,
copying code needs attribution), `[users]` issue-tracker survey (2026-11-04, small
reaction counts, read as direction not votes), `[later]` earlier "consider later" list,
`[accessors]` accessor proposals (2026-11-04). `(plan x.y)` means already scheduled.

Dropped as already in ggrust (checked in code 2026-11-10): `position_fill`,
`position_jitterdodge`, `geom_freqpoly`, `geom_errorbar` / `crossbar` / `linerange` /
`pointrange`, `geom_ribbon`, `stat_summary_bin`, `scale_*_discrete`, `scale_*_grey`,
`scale_linewidth_*`, `scale_*_identity`, `geom_curve`, custom fonts, interactive legend,
jittered boxplot outliers, NA keys in legends, significance brackets, patchwork-style
grids, grid captions and tags, gghighlight, docs gallery.

## 1. Coordinates and positions

- `coord_polar` for pie, donut and rose charts [gaps] [users] (plan lp5.5)
- `coord_radial` / radar charts [gaps]
- `coord_trans`: transform the coordinates after stats [gaps]
- `coord_map` / `coord_sf`: map projections [gaps]
- `position_dodge2`: variable-width dodging, the boxplot default in ggplot2 [gaps]

## 2. Geoms and stats

- `geom_dotplot` / `stat_bindot` [gaps]
- `geom_qq`, `geom_qq_line` / `stat_qq` [gaps] (check plan lp5)
- `stat_function`, `stat_quantile`, `stat_unique`, `stat_summary_2d` [gaps]
- GAM smoothing method for `geom_smooth` [gaps]
- `geom_contour_filled` / `stat_contour_filled` [gaps]
- `geom_spoke` (vector field glyphs), `geom_lollipop`, `geom_cooks` (regression
  diagnostics) [gaps]
- `geom_density_ridges` with the full ggridges options (ggrust has `area_ridges`) [gaps]

### Specialised charts (lower priority for cellestial)

- Business and infographic: Sankey / alluvial (plan ex2.6), chord, treemap, circle
  packing, sunburst, waffle (lets-plot #1336), waterfall, funnel, Pareto, Marimekko,
  Venn, word cloud, bullet, variable-radius pie, timeline, tile map, wind barbs
  [gaps] [users]
- Finance and time series: candlestick / OHLC, Renko, technical indicators, calendar
  heatmap [gaps]
- Networks and trees: `geom_network`, arc diagram, tree geom [gaps]
- Spatial: `geom_sf` / `geom_sf_text` (GeoJSON), map tiles, polygon label placement
  (polylabel), `annotation_scale` [gaps]

## 3. Scales and color

- Binned scales: `scale_*_binned`, `scale_color_steps`, `steps2`, `stepsn` [gaps]
- Bivariate color scale with a bivariate legend [gaps]
- `scale_color_cmapmpl` / `scale_fill_cmapmpl`: any matplotlib colormap by name
  (lets_plot needs `palettable`; `geom_imshow` `cmap` already does) [later]
- Locale-aware label formatting: decimal marks, currency, percent, month names [gaps]
- Colorbar `format` and `lablim` are ignored for every color scale (plan 5.2) [later]
- Check whether theme flavors (`flavor_darcula`, ...) change any palette colors [later]
- Set common scale options once, for example one palette for every color scale
  (ggplot2 #4269) [users]
- Axis capping (lets-plot #1184), vertical fill gradient (lets-plot #994) [users]

## 4. Layout, guides and annotation

- Clipping per layer (ggplot2 #6773) [users]
- `gggrid` `sharex` / `sharey` with discrete scales (lets-plot #1015) [users]
- Corner-aware legend and tag layout [gaps]
- Rich text in labels: bold, italic, color spans (ggtext style) [gaps]
- Polar axis guides, once `coord_polar` lands [gaps]
- Dark theme (plotnine #164) [users]

## 5. Python API

- **Method chaining as sugar for `+`** [later]. `ggplot(frame).add(geom_point(), ...)`
  and `ggplot(frame).geom_point().theme_classic()` next to `ggplot(frame) + geom_point()`.
  Additive only: `+` stays canonical, so the cellestial port stays an import swap.
  `PlotSpec.__add__` (`python/ggrust/plot/core.py`) already duplicates the plot, so each
  chained call returns `self + spec` and stays immutable.
  1. Generic `.add(*specs)` on `PlotSpec`: a few lines, typed, no name clashes.
  2. One method per function through `__getattr__` over an allowlist of public `geom_*`,
     `scale_*`, `theme*`, `coord_*`, `facet_*`, `labs`, `ggsize`, plus a generated `.pyi`
     stub (so `ty` and IDEs see signatures) and a test that fails when the stub drifts.
     No hand-written wrappers: they double the API surface.
  Only on `PlotSpec`, not layers or `gggrid`. Docs show `+` first. When done, record it
  in the `plan.md` decisions table as an extension (plotnine #1047 asks for it too).
- Accessors, tier 1 [accessors] (the built part is in [plans/accessors.md](plans/accessors.md)):
  - `p.guide_data("color")`: polars frame of legend keys with color and label (ggplot2
    `get_guide_data()`). Lets cellestial reuse a cluster's UMAP color in annotations.
  - `p.scale_info("x")`: trained limits, breaks and labels per panel after the build.
  - `p.columns`: every data column the spec uses (`referenced_columns` exists already).
  - `p.validate()`: the warnings without rendering (`validate_spec` exists already).
- Accessors, tier 2 [accessors]: `p.panels()` (pixel boxes and data-to-pixel mapping),
  `p.layers[i].defaults`, `ggrust.theme_defaults("bw")`, a readable `repr` such as
  `<PlotSpec: 2 layers (point, smooth), 3 scales, facet_wrap(~species)>`.
- Not worth it: `last_plot()` (global state, ruled out by AGENTS.md) and setters on
  everything (`with_data` plus `+` cover it).

## 6. Output, interactivity and integrations

- Selection tool that returns the selected rows (lets-plot #998) [users]
- Notebook integrations: Panel (lets-plot #870), marimo, anywidget. `_repr_html_` is
  one-way today; anywidget would let selection and zoom flow back to Python [users]
- Distribution: conda-forge (lets-plot #785, #982), Pyodide / WASM wheel
  (lets-plot #1196; also a standalone WASM build) [users] [gaps]
- CLI that renders a spec file (TOML or JSON) to SVG, PNG or PDF [gaps]
- ggpubr helpers beyond `compare_means` (themes, `ggarrange` style) [gaps]
- Validation suite against R `ggplot_build()` output, reused as a stat-correctness
  check [gaps]

## Sources

[lets-plot issues](https://github.com/JetBrains/lets-plot/issues),
[plotnine issues](https://github.com/has2k1/plotnine/issues),
[ggplot2 issues](https://github.com/tidyverse/ggplot2/issues)
