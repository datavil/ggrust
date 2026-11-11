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
- `coord_trans`: transform the coordinates after stats [gaps] [plotnine]
- `coord_map` / `coord_sf`: map projections [gaps]
- `position_dodge2`: variable-width dodging, the boxplot default in ggplot2 [gaps] [plotnine]
- `expand=` on `coord_cartesian` and `coord_fixed` [plotnine]
- `position_dodge(preserve=)`; `position_stack(reverse=)` and `position_fill(reverse=)`
  [plotnine]

## 2. Geoms and stats

- `stat_function`, `stat_quantile`, `stat_unique`, `stat_summary_2d` [gaps]; plotnine has
  all but `stat_summary_2d`, plus `geom_quantile` [plotnine]
- `stat_hull`: a convex hull as a stat (`geom_mark_hull` only draws an annotation)
  [plotnine]
- `geom_smooth`: methods `rlm`, `glm`, `gls`, `mavg`, `gpr` [plotnine]; formula terms
  beyond `y ~ x` and `y ~ poly(x, k)`, `method_args` beyond loess `span`
- `stat_summary` and `stat_summary_bin`: Python callables for `fun` and `fun_data`
  (rejected today because the engine only receives data values) [plotnine]
- Layer options that are warned about and ignored today [plotnine]:
  - `geom_boxplot`: `notch`, `notchwidth`
  - `geom_histogram`: `closed`, `pad`
  - `geom_area`, `geom_ribbon`: `outline_type`
  - `geom_bar`: `just` (`geom_col` has it)
  - `geom_path`: `lineend`, `linejoin`; `geom_segment`: `lineend`
  - `geom_text`: `parse`
- `geom_ribbon` `where` aesthetic: fill only where a condition holds [plotnine]
- GAM smoothing method for `geom_smooth` [gaps]
- `geom_contour_filled` / `stat_contour_filled` [gaps]
- `geom_cooks` (regression diagnostics) [gaps]
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
  (polylabel), `annotation_scale` [gaps]; `geom_map` (geopandas geometries) [plotnine]

## 3. Scales and color

- `minor_breaks` and `oob` on position scales (color scales have `oob`), `rescaler` on
  continuous scales [plotnine]
- `scale_*_gradient2(midpoint=)` with a non-identity `trans`: the midpoint is compared
  with transformed values without being transformed (ggplot2 transforms it); found by a
  bug hunter on 2026-11-11, older than the new transforms
- Degenerate transformed bars: `geom_col` with `trans="exp"` labels the zero baseline
  `-733`, with `trans="reciprocal"` the axis has no labels (0 is outside the domain)
- `drop` and `na_translate` on discrete scales [plotnine]. Not S: `DiscreteColumn`
  drops unused dictionary levels when it reads Arrow (`data/column.rs`), so `drop=False`
  needs the full level list carried to the scales; `na_translate=True` (ggplot2's
  default) needs an NA level with `na_value` on axes and legends, where ggrust and
  lets-plot drop the rows today (`na_translate=False` behavior).
- `scale_*_distiller`, `scale_*_desaturate`, `scale_*_ordinal` (color, fill, alpha, size),
  `scale_size_radius` [plotnine]
- Binned scales: `scale_*_binned`, `scale_color_steps`, `steps2`, `stepsn` [gaps]
- Bivariate color scale with a bivariate legend [gaps]
  (lets_plot needs `palettable`; `geom_imshow` `cmap` already does) [later]; plotnine
  spells these `scale_*_cmap` and `scale_*_cmap_d` [plotnine]
- Locale-aware label formatting: decimal marks, currency, percent, month names [gaps]
- Set common scale options once, for example one palette for every color scale
  (ggplot2 #4269) [users]
- Axis capping (lets-plot #1184), vertical fill gradient (lets-plot #994) [users]

## 4. Layout, guides and annotation

- `annotation_logticks`, `annotation_stripes` [plotnine]
- Facets: `facet_grid(space=, margins=, drop=)`, `as_table` and `shrink` on both facets,
  `facet_null` [plotnine]
- Facet labellers: `labeller=`, `label_value`, `label_both`, `label_context`,
  `as_labeller` (`format` and `labwidth` exist) [plotnine]
- Composition operators `|`, `/`, `-`, `&`, `*` and `plot_spacer`, `Beside`, `Stack`, next
  to `gggrid` [plotnine]
- Per-guide options: `reverse`, `position`, `direction`, `order`, `theme` on `guide_legend`
  and `guide_colorbar`; `display`, `alpha`, `draw_ulim`, `draw_llim` on `guide_colorbar`
  [plotnine]
- Built-in themes: `theme_538`, `theme_dark`, `theme_linedraw`, `theme_matplotlib`,
  `theme_seaborn`, `theme_tufte`, `theme_xkcd` [plotnine]
- Theme options: `axis_ticks_direction` (and `_x`, `_y`), `legend_justification_top` (and
  bottom, left, right, inside), `legend_text_position`, `legend_title_position`,
  `legend_box_background`, `legend_box_margin`, `legend_frame` [plotnine]
- `watermark`: an image drawn over the plot [plotnine]
- Clipping per layer (ggplot2 #6773) [users]
- `gggrid` `sharex` / `sharey` with discrete scales (lets-plot #1015) [users]
- Corner-aware legend and tag layout [gaps]
- Rich text in labels: bold, italic, color spans (ggtext style) [gaps]
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
- `after_stat`, `after_scale`, `stage` in `aes` (ggrust has `..count..` for computed
  variables; nothing reaches values after scaling) [plotnine]
- Expressions in `aes`: `aes("np.log(x)")`, `factor(x)`, `reorder(x, y)`. Today a string
  must name a column (`as_discrete` covers `factor` and ordering) [plotnine]
- `qplot` and `frame >> ggplot(...)` [plotnine]
- Example datasets like `plotnine.data` (`mpg`, `mtcars`, `diamonds`, `penguins`, ...)
  [plotnine]
- Spelling aliases, cheap if the plotnine crowd matters [plotnine]: `scale_colour_*`,
  `scale_*_gray`, `guide_colourbar`, `geom_bin_2d`, `geom_density_2d`, `coord_equal`,
  `scale_*_date`, `scale_*_timedelta`, `scale_*_sqrt`, `scale_alpha_continuous`,
  `scale_alpha_discrete`, `scale_size_continuous`, `scale_size_discrete`,
  `scale_stroke_continuous`, `scale_shape_discrete`, `scale_linetype`,
  `scale_linetype_discrete`, `scale_color_datetime` (also fill, alpha, size),
  `position_identity`
- Standalone `stat_*` functions that exist only as `stat=` names: `stat_bin`,
  `stat_bin_2d`, `stat_boxplot`, `stat_count`, `stat_density`, `stat_density_2d`,
  `stat_identity`, `stat_sina`, `stat_smooth`, `stat_ydensity` [plotnine]
- Accessors, tier 2 [accessors]: `p.panels()` (pixel boxes and data-to-pixel mapping),
  `p.layers[i].defaults`, `ggrust.theme_defaults("bw")`, a readable `repr` such as
  `<PlotSpec: 2 layers (point, smooth), 3 scales, facet_wrap(~species)>`.
- Not worth it: `last_plot()`, plotnine's `theme_set` / `theme_update` / `theme_get` and
  `plotnine.options` (global state, ruled out by AGENTS.md), and setters on everything
  (`with_data` plus `+` cover it).

## 6. Output, interactivity and integrations

- Selection tool that returns the selected rows (lets-plot #998) [users]
- Notebook integrations: Panel (lets-plot #870), marimo, anywidget. `_repr_html_` is
  one-way today; anywidget would let selection and zoom flow back to Python [users]
- Distribution: conda-forge (lets-plot #785, #982), Pyodide / WASM wheel
  (lets-plot #1196; also a standalone WASM build) [users] [gaps]
- Export: multi-page PDF (`save_as_pdf_pages`); JPEG, TIFF and other raster formats in
  `ggsave` (today `svg`, `png`, `pdf`, `html`, `ggrs`) [plotnine]
- CLI that renders a spec file (TOML or JSON) to SVG, PNG or PDF [gaps]
- ggpubr helpers beyond `compare_means` (themes, `ggarrange` style) [gaps]
- Validation suite against R `ggplot_build()` output, reused as a stat-correctness
  check [gaps]

## Sources

[lets-plot issues](https://github.com/JetBrains/lets-plot/issues),
[plotnine issues](https://github.com/has2k1/plotnine/issues),
[ggplot2 issues](https://github.com/tidyverse/ggplot2/issues)
