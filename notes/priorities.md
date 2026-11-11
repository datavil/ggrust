# Backlog priorities

Every item in [backlog.md](backlog.md), ranked. **1.1 is the highest priority**; a lower
number always comes first, across and within tiers. The backlog keeps the detail and the
sources. When an item is picked up, move it to [plan.md](../plan.md) and delete it from
both files.

Ranked on 2026-11-11, following the order in AGENTS.md:

1. Wrong or silently ignored behavior comes before new features.
2. Then what cellestial needs, then lets-plot parity, because the port is meant to be an
   import swap. No backlog geom, stat or coord is used by cellestial today, so cellestial
   only lifts the accessors.
3. Then general plotting value per unit of work (the 20/80 rule). A feature that both
   ggplot2 and plotnine have ranks above one only a single library has.

Size is a rough guess: **S** is about one file plus tests, **M** is a few files, **L** is
new engine work or a new subsystem. Items already scheduled keep their `plan.md` step.

## 1. Correctness

| # | Item | Why here | Size |
|---|---|---|---|
| 1.2 | Colorbar `lablim` is ignored with no warning. `format` now works on `scale_color_gradient`; re-check the other color scales | Silent loss of a user option | S |
| 1.3 | Check whether theme flavors change any palette colors | Open parity question, cheap to settle | S |

## 2. cellestial and lets-plot parity

| # | Item | Why here | Size |
|---|---|---|---|
| 2.1 | `p.guide_data("color")` | cellestial can reuse a cluster's color in annotations | S |
| 2.2 | `p.columns`, `p.validate()` | The internals already exist | S |
| 2.3 | `p.scale_info("x")` | Completes accessors tier 1 | M |
| 2.4 | `geom_qq`, `geom_qq_line`, `stat_qq` (plan lp5.4) | In lets-plot, already scheduled | M |
| 2.5 | `coord_polar` (plan lp5.5) | In lets-plot, unlocks pie, donut and rose charts | L |
| 2.6 | Polar axis guides | Needed as soon as 2.5 lands | M |
| 2.7 | `scale_color_cmapmpl`, `scale_fill_cmapmpl` (plotnine's `scale_*_cmap`, `scale_*_cmap_d`) | In lets-plot, `geom_imshow` already resolves colormaps | S |
| 2.8 | `geom_lollipop` | In lets-plot, built from existing parts | S |
| 2.9 | `geom_spoke` | In lets-plot and plotnine | S |
| 2.10 | `geom_dotplot`, `stat_bindot` | In lets-plot and plotnine, stacking layout is the hard part | M |

## 3. Core grammar gaps

| # | Item | Why here | Size |
|---|---|---|---|
| 3.1 | More `trans` names: `log`, `log1p`, `exp`, `logit`, `probit`, `reciprocal`, `pseudo_log`, `symlog`, `boxcox`, plus `scale_*_symlog` | Common in science, contained in one file | S |
| 3.2 | `stat_summary` shortcuts: `mean_cl_normal`, `mean_cl_boot`, `median_hilow` | Standard error bars for grouped data | S |
| 3.3 | `annotate()` | Very common ggplot idiom | S |
| 3.4 | Facet labellers: `labeller=`, `label_value`, `label_both`, `label_context`, `as_labeller` | Common, and facets are central to cellestial's plot types | M |
| 3.5 | `facet_grid(space=, margins=, drop=)`, `as_table`, `shrink`, `facet_null` | Finishes the facet API | M |
| 3.6 | Layer options warned about and ignored: `notch`, `notchwidth`, `closed`, `pad`, `just` on `geom_bar`, `lineend`, `linejoin`, `outline_type` | Each is small, and users see the warning | M |
| 3.7 | Density and raster options: `cut`, `clip`, `bounds`, `gridsize`, `interpolation`, `hjust`, `vjust` | Same kind as 3.6, used less | S |
| 3.8 | `position_dodge2`, `position_dodge(preserve=)` | ggplot2's boxplot default | M |
| 3.9 | `position_stack(reverse=)`, `position_fill(reverse=)` | Small and common | S |
| 3.10 | Per-guide options: `reverse`, `position`, `direction`, `order`, `theme`; `display`, `alpha`, `draw_ulim`, `draw_llim` on colorbars | Legend control is a frequent request | M |
| 3.11 | `minor_breaks`, `oob` on position scales, `rescaler` | Standard scale controls | M |
| 3.12 | `drop`, `na_translate` on discrete scales | Keeps empty levels in legends and axes | S |
| 3.13 | `geom_smooth`: `fullrange`, `formula`, `method_args` | Most-used smoothing options | S |
| 3.14 | `geom_smooth` methods: `glm`, `rlm`, `gls`, `mavg`, `gpr`, GAM | Each method is real statistics work | L |
| 3.15 | `after_stat`, `after_scale`, `stage` | ggplot2 and plotnine syntax, `..count..` covers the main case | M |
| 3.16 | `expand=` on `coord_cartesian`, `coord_fixed` | Small | S |
| 3.17 | `coord_trans` | Transform after stats | M |
| 3.18 | Binned scales: `scale_*_binned`, `scale_color_steps`, `steps2`, `stepsn` | Useful for choropleths and heatmaps | M |
| 3.19 | `stat_quantile`, `geom_quantile` | Needs quantile regression | M |
| 3.20 | `stat_unique`, `stat_hull`, `stat_summary_2d` | Small stats, seldom needed | S |
| 3.21 | `scale_*_distiller`, `scale_*_desaturate`, `scale_*_ordinal`, `scale_size_radius` | Mostly thin wrappers | S |
| 3.22 | `annotation_logticks`, `annotation_stripes` | Pairs with log scales and dodged categories | M |
| 3.23 | `geom_ribbon` `where` aesthetic | Niche | S |
| 3.24 | `geom_text(parse=)` | Needs math text | L |

## 4. Python API ergonomics

| # | Item | Why here | Size |
|---|---|---|---|
| 4.1 | Spelling aliases (`scale_colour_*`, `geom_density_2d`, `coord_equal`, ...), plus the ggplot2 names `stat_function` and `geom_contour_filled`, already covered by `geom_function` and `geom_contourf` | Cheap, eases moving from plotnine or ggplot2 | S |
| 4.2 | Standalone `stat_*` functions that exist only as `stat=` names | Same | S |
| 4.3 | Method chaining as sugar for `+` | Additive, already designed | M |
| 4.4 | Composition operators `\|`, `/`, `-`, `&`, `*`, `plot_spacer`, `Beside`, `Stack` | Nicer than `gggrid` for quick layouts | M |
| 4.5 | Set common scale options once | Saves repetition | M |
| 4.6 | Accessors tier 2: `p.panels()`, `p.layers[i].defaults`, `theme_defaults`, readable `repr` | Handy, nothing is blocked on it | M |
| 4.7 | Example datasets | Docs and tutorials | S |
| 4.8 | Expressions in `aes` (`np.log(x)`, `factor(x)`, `reorder(x, y)`) | Needs evaluation in Python; polars and `as_discrete` cover it | M |
| 4.9 | `qplot`, `frame >> ggplot(...)` | Little value | S |

## 5. Output and integrations

| # | Item | Why here | Size |
|---|---|---|---|
| 5.1 | Validation suite against R `ggplot_build()` output | Guards every stat, so it pays back on later work | M |
| 5.2 | Notebook integrations: marimo, anywidget, Panel | Two-way selection and zoom | L |
| 5.3 | Selection tool that returns the selected rows | Depends on 5.2 | M |
| 5.4 | conda-forge package | Common install route for scientists | M |
| 5.5 | Multi-page PDF (`save_as_pdf_pages`) | Batch reports | S |
| 5.6 | JPEG, TIFF and other raster formats in `ggsave` | Some journals ask for TIFF | S |
| 5.7 | ggpubr helpers beyond `compare_means` | Publication plots | M |
| 5.8 | Pyodide and WASM builds | Reach, but a large build effort | L |
| 5.9 | CLI that renders a spec file | Not a Python-first need | S |

## 6. Theming and polish

| # | Item | Why here | Size |
|---|---|---|---|
| 6.1 | Locale-aware label formatting | Correct labels outside English | M |
| 6.2 | Theme options: `axis_ticks_direction`, `legend_justification_*`, `legend_text_position`, `legend_title_position`, `legend_box_background`, `legend_box_margin`, `legend_frame` | Fine layout control | M |
| 6.3 | Dark theme | User request | S |
| 6.4 | Built-in themes: `theme_538`, `theme_dark`, `theme_linedraw`, `theme_matplotlib`, `theme_seaborn`, `theme_tufte`, `theme_xkcd` | Data only, except the hand-drawn look of xkcd | M |
| 6.5 | Rich text in labels (ggtext style) | Wanted, but touches text layout | L |
| 6.6 | `gggrid` `sharex` / `sharey` with discrete scales | Narrow case | M |
| 6.7 | Corner-aware legend and tag layout | Narrow case | M |
| 6.8 | Clipping per layer | Narrow case | M |
| 6.9 | Axis capping | Cosmetic | S |
| 6.10 | Vertical fill gradient | Cosmetic | M |
| 6.11 | `watermark` | Rarely needed | S |
| 6.12 | Bivariate color scale with a bivariate legend | Niche, needs a new guide | L |

## 7. Specialised charts

| # | Item | Why here | Size |
|---|---|---|---|
| 7.1 | `geom_contour_filled` options beyond `geom_contourf`, if any remain after 4.1 | Probably closes with the alias | S |
| 7.2 | `geom_density_ridges` with the full ggridges options | `geom_area_ridges` covers the main use | M |
| 7.3 | `geom_cooks` (regression diagnostics) | Niche | S |
| 7.4 | `coord_radial`, radar charts | Follows 2.5 | M |
| 7.5 | Networks and trees: `geom_network`, arc diagram, tree geom | Trees are partly served by the dendrogram work | L |
| 7.6 | Business and infographic: Sankey / alluvial (plan ex2.6), treemap, waffle, waterfall, chord, circle packing, sunburst, funnel, Pareto, Marimekko, Venn, word cloud, bullet, variable-radius pie, timeline, tile map, wind barbs | Outside the scientific core; Sankey first, as it is scheduled | L |
| 7.7 | Finance and time series: candlestick / OHLC, Renko, technical indicators, calendar heatmap | Outside the scientific core | L |
| 7.8 | Spatial: `coord_map`, `coord_sf`, `geom_sf`, `geom_sf_text`, `geom_map`, map tiles, polylabel placement, `annotation_scale` | A subsystem of its own | L |
