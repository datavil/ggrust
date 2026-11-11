# Backlog priorities

Every item in [backlog.md](backlog.md), ranked. A lower number always comes first,
across and within tiers. The backlog keeps the detail and the
sources. When an item is picked up, move it to [plan.md](../plan.md) and delete it from
both files.

Ranked on 2026-11-11, following the order in AGENTS.md:

1. Wrong or silently ignored behavior comes before new features.
2. Then what cellestial needs, then lets-plot parity, because the port is meant to be an
   import swap. No backlog geom, stat or coord is used by cellestial today, so cellestial
   only lifts the accessors.
3. Then general plotting value per unit of work (the 20/80 rule). A feature that both
   ggplot2 and plotnine have ranks above one only a single library has.

"Found in" says which of ggplot2 (4.0.1), lets-plot (4.11) and plotnine (0.15.9) have the
item, checked against the installed packages on 2026-11-11. Per AGENTS.md, an item only
plotnine has needs the user's approval before it is built.

Size is a rough guess: **S** is about one file plus tests, **M** is a few files, **L** is
new engine work or a new subsystem. Items already scheduled keep their `plan.md` step.

## 2. cellestial and lets-plot parity

| # | Item | Why here | Size | Found in |
|---|---|---|---|---|

## 3. Core grammar gaps

| # | Item | Why here | Size | Found in |
|---|---|---|---|---|
| 3.8 | `position_dodge2`, `position_dodge(preserve=)` | ggplot2's boxplot default | M | ggplot2, plotnine |
| 3.10 | Per-guide options: `reverse`, `position`, `direction`, `order`, `theme`; `display`, `alpha`, `draw_ulim`, `draw_llim` on colorbars | Legend control is a frequent request | M | ggplot2, plotnine |
| 3.11 | `minor_breaks`, `oob` on position scales, `rescaler` | Standard scale controls | M | ggplot2, plotnine |
| 3.12 | `drop`, `na_translate` on discrete scales | Keeps empty levels in legends and axes. Larger than first thought: unused levels are dropped when the Arrow column is read, before any scale exists, and `na_translate` means a new NA level on axes and legends | M | ggplot2, plotnine |
| 3.14 | `geom_smooth` methods: `glm`, `rlm`, `gls`, GAM | Each method is real statistics work | L | ggplot2 (any R model function), plotnine (`glm`, `rlm`, `gls`; no GAM); lets-plot has `lm` and `loess` only |
| 3.15 | `after_stat`, `after_scale`, `stage` | ggplot2 and plotnine syntax, `..count..` covers the main case | M | ggplot2, plotnine |
| 3.17 | `coord_trans` | Transform after stats | M | ggplot2 (renamed `coord_transform` in 4.0), plotnine |
| 3.18 | Binned scales: `scale_*_binned`, `scale_color_steps`, `steps2`, `stepsn` | Useful for choropleths and heatmaps | M | ggplot2 only |
| 3.19 | `stat_quantile`, `geom_quantile` | Needs quantile regression | M | ggplot2, plotnine |
| 3.22 | `annotation_logticks`, `annotation_stripes` | Pairs with log scales and dodged categories | M | `annotation_logticks`: ggplot2, plotnine; `annotation_stripes`: plotnine only |
| 3.24 | `geom_text(parse=)` | Needs math text | L | ggplot2, plotnine |

## 4. Python API ergonomics

| # | Item | Why here | Size | Found in |
|---|---|---|---|---|
| 4.5 | Set common scale options once | Saves repetition | M | ggplot2 (`aesthetics=` on every scale, `palette.*` theme elements in 4.0), lets-plot (generic `scale_manual`, `scale_gradient`, ... with `aesthetic=`, which ggrust lacks) |
| 4.6 | Accessors tier 2: `p.panels()`, `p.layers[i].defaults`, `theme_defaults`, readable `repr` | Handy, nothing is blocked on it | M | none (ggrust's own API; ggplot2's `ggplot_build()` is the nearest) |

## 5. Output and integrations

| # | Item | Why here | Size | Found in |
|---|---|---|---|---|
| 5.1 | Validation suite against R `ggplot_build()` output | Guards every stat, so it pays back on later work | M | n/a (ggrust tooling) |
| 5.2 | Notebook integrations: marimo, anywidget, Panel | Two-way selection and zoom | L | none |
| 5.3 | Selection tool that returns the selected rows | Depends on 5.2 | M | none |
| 5.4 | conda-forge package | Common install route for scientists | M | n/a (lets-plot and plotnine are on conda-forge) |
| 5.5 | Multi-page PDF (`save_as_pdf_pages`) | Batch reports | S | plotnine only |
| 5.6 | JPEG, TIFF and other raster formats in `ggsave` | Some journals ask for TIFF | S | ggplot2, plotnine; lets-plot saves SVG, HTML, PNG, PDF only |
| 5.7 | ggpubr helpers beyond `compare_means` | Publication plots | M | ggplot2 extension (ggpubr) |
| 5.8 | Pyodide and WASM builds | Reach, but a large build effort | L | n/a (build target) |

## 6. Theming and polish

| # | Item | Why here | Size | Found in |
|---|---|---|---|---|
| 6.1 | Locale-aware label formatting | Correct labels outside English | M | ggplot2 (through scales' `label_number()` and friends) |
| 6.2 | Theme options: `axis_ticks_direction`, `legend_justification_*`, `legend_text_position`, `legend_title_position`, `legend_box_background`, `legend_box_margin`, `legend_frame` | Fine layout control | M | ggplot2 3.5+, plotnine; `axis_ticks_direction` is in neither |
| 6.3 | Dark theme | User request | S | lets-plot `flavor_darcula` and `flavor_high_contrast_dark` (ggrust already has both), ggplot2 and plotnine `theme_dark`; may be done, check what the request still needs |
| 6.4 | Built-in themes: `theme_538`, `theme_dark`, `theme_linedraw`, `theme_tufte` | Data only | M | `theme_dark`, `theme_linedraw`: ggplot2, plotnine; `theme_538`, `theme_tufte`: plotnine only (ggthemes for ggplot2) |
| 6.5 | Rich text in labels (ggtext style) | Wanted, but touches text layout | L | lets-plot `element_markdown` (ggrust already has it), ggplot2 extension (ggtext); check what is still missing |
| 6.6 | `gggrid` `sharex` / `sharey` with discrete scales | Narrow case | M | lets-plot |
| 6.7 | Corner-aware legend and tag layout | Narrow case | M | n/a (layout quality) |
| 6.8 | Clipping per layer | Narrow case | M | none (requested in ggplot2 #6773) |
| 6.9 | Axis capping | Cosmetic | S | ggplot2 (`guide_axis(cap=)`) |
| 6.10 | Vertical fill gradient | Cosmetic | M | none (requested in lets-plot #994) |
| 6.11 | `watermark` | Rarely needed | S | plotnine only |
| 6.12 | Bivariate color scale with a bivariate legend | Niche, needs a new guide | L | ggplot2 extension (biscale) |

## 7. Specialised charts

| # | Item | Why here | Size | Found in |
|---|---|---|---|---|
| 7.1 | `geom_contour_filled` fills bands by an ordered discrete level, as ggplot2 (today an alias of `geom_contourf`, continuous fill) | Discrete legend of bands | S | ggplot2 |
| 7.2 | `geom_density_ridges` with the full ggridges options | `geom_area_ridges` covers the main use | M | ggplot2 extension (ggridges); lets-plot `geom_area_ridges` covers part |
| 7.3 | `geom_cooks` (regression diagnostics) | Niche | S | none |
| 7.4 | `coord_radial`, radar charts | Follows 2.5 | M | `coord_radial`: ggplot2; radar charts: lets-plot through `coord_polar` |
| 7.5 | Networks and trees: `geom_network`, arc diagram, tree geom | Trees are partly served by the dendrogram work | L | ggplot2 extensions (ggraph, ggtree) |
| 7.6 | Business and infographic: Sankey / alluvial (plan ex2.6), treemap, waffle, waterfall, chord, circle packing, sunburst, funnel, Pareto, Marimekko, Venn, word cloud, bullet, variable-radius pie, timeline, tile map, wind barbs | Outside the scientific core; Sankey first, as it is scheduled | L | ggplot2 extensions (ggalluvial, treemapify, waffle, ...) |
| 7.7 | Finance and time series: candlestick / OHLC, calendar heatmap | Outside the scientific core | L | ggplot2 extension (tidyquant) |
| 7.8 | Spatial: `coord_map`, `coord_sf`, `geom_sf`, `geom_sf_text`, `geom_map`, map tiles, polylabel placement, `annotation_scale` | A subsystem of its own | L | ggplot2 (`coord_sf`, `geom_sf`), lets-plot (`geom_map`, `coord_map`, `geom_livemap`, map tiles), plotnine (`geom_map`) |
