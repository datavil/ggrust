# Backlog

Ideas and gaps that are **not scheduled**. Scheduled work lives in [plan.md](../plan.md)
and [plans/](plans/). When an item is picked up, move it to `plan.md` and delete it
here. Every item still needs checking against lets-plot and ggplot2 before coding
(AGENTS.md, "Parity with lets-plot").

Source tags: `[gaps]` ggplot-rs comparison (2026-11-10, from the crate source of
[ggplot-rs](https://github.com/sipemu/ggplot-rs) 0.42.1, MIT OR Apache-2.0; ideas only,
copying code needs attribution), `[users]` issue-tracker survey (2026-11-04, small
reaction counts, read as direction not votes), `[later]` earlier "consider later" list,
`[accessors]` accessor proposals (2026-11-04), `[plotnine]` plotnine comparison
(2026-11-11, plotnine 0.15.9 exports and parameters read from its source, each ggrust
gap checked against the installed API or by rendering). `(plan x.y)` means already
scheduled.

Dropped as already in ggrust (checked in code 2026-11-10): `position_fill`,
`position_jitterdodge`, `geom_freqpoly`, `geom_errorbar` / `crossbar` / `linerange` /
`pointrange`, `geom_ribbon`, `stat_summary_bin`, `scale_*_discrete`, `scale_*_grey`,
`scale_linewidth_*`, `scale_*_identity`, `geom_curve`, custom fonts, interactive legend,
jittered boxplot outliers, NA keys in legends, significance brackets, patchwork-style
grids, grid captions and tags, gghighlight, docs gallery.

Plotnine features ggrust already covers under another name or option, so not listed as
gaps (2026-11-11): `geom_violin(style=)` is `show_half`, `draw_quantiles` is `quantiles`
and `quantile_lines`, `geom_text(adjust_text=)` is `geom_text_repel`, `format_string` is
`label_format`, `theme(figure_size=, dpi=)` is `ggsize` and `ggsave`, `aspect_ratio` is
`coord_fixed`, `axis_ticks_pad` is `axis_text_spacing`, `axis_ticks_minor` is
`axis_minor_ticks`, datetime values on color scales render today.

## 1. Coordinates and positions

- `coord_radial` / radar charts [gaps]
- `coord_map` / `coord_sf`: map projections [gaps]
- `position_stack(reverse=)` and `position_fill(reverse=)` [plotnine]

## 2. Geoms and stats

- `stat_quantile` / `geom_quantile` beyond linear `y ~ x`: other formulas (splines),
  `method="rqss"`, the `weight` aesthetic [gaps] [plotnine]
- `geom_smooth`: methods `rlm`, `glm`, `gls` [plotnine]; formula terms
  beyond `y ~ x` and `y ~ poly(x, k)`, `method_args` beyond loess `span`
- `stat_summary` and `stat_summary_bin`: Python callables for `fun` and `fun_data`
  (rejected today because the engine only receives data values) [plotnine]
- `geom_text(parse=)` is warned about and ignored (needs math text) [plotnine]
- GAM smoothing method for `geom_smooth` [gaps]
- `stat_contour_filled` as a standalone layer and its `level_low`, `level_high`,
  `level_mid`, `nlevel` variables (`geom_contour_filled` landed; breaks per panel
  when faceted, ggplot2 takes them over the whole layer) [gaps]
- `geom_density_ridges` with the full ggridges options (ggrust has `area_ridges`) [gaps]

### Specialised charts (lower priority for cellestial)

- Business and infographic: Sankey / alluvial (plan ex2.6), chord, treemap, circle
  packing, sunburst, waffle (lets-plot #1336), waterfall, funnel, Pareto, Marimekko,
  Venn, word cloud, bullet, variable-radius pie, timeline, tile map, wind barbs
  [gaps] [users]
- Spatial: `geom_sf` / `geom_sf_text` (GeoJSON), map tiles, polygon label placement
  (polylabel), `annotation_scale` [gaps]; `geom_map` (geopandas geometries) [plotnine]

- `geom_smooth(method="lm")` with a degree (`deg=` or `poly(x, k)`) of n or more
  draws nothing without a warning (R errors); with one less the band is NaN [bug hunt]

## 3. Scales and color

- Minor breaks: side panels and polar coords still draw halfway minor lines
  (`minor_breaks`, `oob` and `rescaler` landed with 3.11) [plotnine]
- `rescaler=` on binned color scales (`scale_*_steps`, ggplot2 rescales the values
  and breaks before taking bin midpoints) and on size and alpha scales (ggplot2 4.0
  rejects it there) [ggplot2]
- `scale_*_gradient2(midpoint=)` with a non-identity `trans`: the midpoint is compared
  with transformed values without being transformed (ggplot2 transforms it); found by a
  bug hunter on 2026-11-11, older than the new transforms
- Degenerate transformed bars: `geom_col` with `trans="exp"` labels the zero baseline
  `-733`, with `trans="reciprocal"` the axis has no labels (0 is outside the domain)
- `drop` and `na_translate` on discrete scales [plotnine]. Size L (priorities 3.12 has the plan): `DiscreteColumn`
  drops unused dictionary levels when it reads Arrow (`data/column.rs`), so `drop=False`
  needs the full level list carried to the scales; `na_translate=True` (ggplot2's
  default) needs an NA level with `na_value` on axes and legends, where ggrust and
  lets-plot drop the rows today (`na_translate=False` behavior).
- `breaks_with_step` (scale/breaks.rs) keeps a break that lies within `step / 10000` outside
  the domain and clamps it onto the end: a y range from 0.187 to 11,616 gets a break
  labelled "0.1873" (0 clamped up). Seen under `coord_cartesian(ylim=)` and
  `coord_trans()`; the tolerance should only absorb float error relative to the value
- `scale_*_gradient2(trans=..., midpoint=...)` (and `steps2`) use `midpoint` in
  transformed units as given: `trans="log10", midpoint=100` colors everything near the
  low end; ggplot2 transforms the midpoint first (found by the 3.18 bug hunter)
- Binned position scales `scale_x_binned`, `scale_y_binned` (priorities 3.18, L), and
  `guide_coloursteps()` as a guide name with `even_steps=False`. The color, fill, size
  and alpha binned scales and `guide_bins()` landed [gaps]
- `guide_bins()` on a continuous (not binned) scale: ggplot2 uses the breaks as bin
  edges; ggrust raises a SpecError. `scale_linewidth_binned` and `guide_bins(angle=)`
  are not built [ggplot2]
- Bivariate color scale with a bivariate legend [gaps]
  (lets_plot needs `palettable`; `geom_imshow` `cmap` already does) [later]; plotnine
  spells these `scale_*_cmap` and `scale_*_cmap_d` [plotnine]
- Locale-aware label formatting: decimal marks, currency, percent, month names [gaps]
- Set common scale options once, for example one palette for every color scale
  (ggplot2 #4269) [users]

- `scale_*_distiller` accepts discrete data and any
  `direction` without an error (ggplot2 errors on discrete data) [bug hunt]

## 4. Layout, guides and annotation

- `annotation_stripes` [plotnine]
- `facet_grid(scales="free") + coord_flip()`: panels of one grid column may get different
  horizontal ranges while only the bottom row draws the axis (bug hunt 2026-11-11, not yet
  checked against ggplot2, may predate the facet options)
- Facets: `facet_grid(drop=False)` (needs the full level list, see `drop` on discrete
  scales), `facet_wrap(space=)` and ggplot2 4.0's two-letter `dir` codes (`"lb"`, `"tr"`)
- Per-variable facet labellers, ggplot2 `labeller(cyl = label_both, .default = ...)`
  (the `labeller=` option and `label_*` / `as_labeller` exist)
- Per-guide `position=` on `guide_legend` and `guide_colorbar` (ggplot2): legends on
  several sides need one legend band per side in the layout (size L; the other
  per-guide options landed in 3.10). Guide `theme=` also rejects complete themes
  and block-level elements (`legend_position`, `legend_box*`, `legend_spacing*`) [plotnine]
- Built-in themes: `theme_538`, `theme_tufte` [plotnine]
- Theme options: `axis_ticks_direction` (and `_x`, `_y`), `legend_justification_top` (and
  bottom, left, right, inside), `legend_text_position`, `legend_title_position`,
  `legend_box_background`, `legend_box_margin`, `legend_frame` [plotnine]
- `watermark`: an image drawn over the plot [plotnine]
- `gggrid` `sharex` / `sharey` with discrete scales (lets-plot #1015) [users]
- Corner-aware legend and tag layout [gaps]
- Rich text in labels: bold, italic, color spans (ggtext style) [gaps]
- Complete themes with `ink=` and `paper=` (ggplot2 4.0: `theme_grey(ink="white",
  paper="black")` draws a whole plot dark from two colors); today a dark plot is a
  dark flavor (`flavor_darcula`, `flavor_high_contrast_dark`) and `theme_dark` darkens
  only the panel [gaps]

## 5. Python API

- `after_scale`, `stage`, and expressions in `after_stat` (`after_stat(count / max(count))`)
  in `aes` [plotnine]. `after_stat("name")` landed; the rest needs an expression language
  evaluated on stat output and on scaled values, which ggrust does not have
- `as_discrete()` on a computed variable (`aes(linetype=as_discrete("..quantile.."))`,
  `"..level.."`) is ignored: the scale stays continuous and the legend shows generated
  breaks (0.3, 0.4, ...) instead of the computed values
- `aes(color=pl.Series([...]))` (and pandas Series) is rejected ("cannot send a value of
  type Series"); lists and numpy arrays work
- Errors with required keyword-only `__init__` arguments (`GuideNotFoundError`,
  `DatasetNotFoundError`, ...) fail to pickle or copy (multiprocessing, joblib)
- Short facet panels in a `gggrid` cell get a degenerate y axis (one break such as
  `9.2`): `gggrid([ggplot(mpg, aes('displ', 'hwy')) + geom_point() + geom_smooth()
  + facet_wrap('drv')] * 6, ncol=3) + ggsize(1300, 700)`
- More example datasets beyond `ggrust.data` (`penguins`, `mpg`, `economics`, `iris`):
  `diamonds` (2.7 MB CSV, would need a compact format), `mtcars` (license unclear) [plotnine]
- `scale_color_datetime` (also fill, alpha, size) [plotnine]: left out of the 4.1
  aliases because a datetime column on a non-position scale shows epoch numbers in the
  legend; needs temporal legend labels in the engine first.
- Accessors, tier 2 [accessors]: `p.panels()` (pixel boxes and data-to-pixel mapping),
  `p.layers[i].defaults`, `ggrust.theme_defaults("bw")`, a readable `repr` such as
  `<PlotSpec: 2 layers (point, smooth), 3 scales, facet_wrap(~species)>`.
- Not worth it: `last_plot()`, plotnine's `theme_set` / `theme_update` / `theme_get` and
  `plotnine.options` (global state, ruled out by AGENTS.md), and setters on everything
  (`with_data` plus `+` cover it).
- Ruled out by the user on 2026-11-11 as too far from ggrust's identity (a
  lets_plot-compatible ggplot2 grammar, used from Python): the `frame >> ggplot()` pipe,
  `qplot()`, the plotnine-only `geom_ribbon` `where` aesthetic, `scale_*_desaturate` and
  `scale_*_timedelta`, method chaining as sugar for `+`, patchwork-style composition operators
  (`|`, `/`, `-`, `&`, `*`, `Beside`, `Stack`; `gggrid` is the way), string expressions
  in `aes` (`aes("np.log(x)")`), `geom_density` options kept only for plotnine code (`cut`,
  `clip`, `gridsize`), the plotnine-only themes `theme_matplotlib`, `theme_seaborn` and
  `theme_xkcd`, the `mavg` and `gpr` smoothing methods, a spec-rendering CLI, and Renko
  charts and technical indicators.
- Dropped by the user on 2026-11-11: a conda-forge package and Pyodide / WASM builds.
- Ruled out by the user on 2026-11-13 (plan 5.7): ggpubr's `ggarrange()` (`gggrid` covers it)
  and its quick-plot wrappers (`ggboxplot`, `ggviolin`, ...; `qplot()`-like). ggpubr's
  `theme_pubr` / `theme_pubclean` stay with ex2.1.
- Dropped by the user on 2026-11-12: notebook widget integrations (marimo, anywidget,
  Panel) and the row-selection tool, finance charts (candlestick / OHLC, calendar
  heatmap), networks and trees (`geom_network`, arc diagram, tree geom), `geom_cooks`,
  clipping per layer, and a vertical fill gradient.

## 6. Output, interactivity and integrations

- Export: multi-page PDF (`save_as_pdf_pages`) [plotnine]

## Sources

[lets-plot issues](https://github.com/JetBrains/lets-plot/issues),
[plotnine issues](https://github.com/has2k1/plotnine/issues),
[ggplot2 issues](https://github.com/tidyverse/ggplot2/issues)
