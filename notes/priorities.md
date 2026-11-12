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
| 3.10 | Per-guide `position` (`guide_legend(position=)`, `guide_colorbar(position=)`); the other per-guide options landed | Legends on several sides need a legend band per side in the layout | L | ggplot2 |
| 3.11 | `rescaler` on continuous color scales (`minor_breaks` and `oob` on position scales landed) | Needs a numeric Python callback in style-scale mapping and guides, and a call signature (ggplot2: `function(x, to, from)`) | M | ggplot2, plotnine |
| 3.12 | `drop`, `na_translate` on discrete scales | Keeps empty levels in legends and axes. Larger than first thought: unused levels are dropped when the Arrow column is read, before any scale exists. Plan: an optional declared level list on `DiscreteColumn` from Arrow dictionaries (polars Enum, pandas categoricals), carried through take/reorder/merge and trained first when `drop=False`; the `drop`/`na_translate` parameters on about 40 discrete scale functions; an NA key in legends and an NA category on axes, and `na_translate` means a new NA level on axes and legends | L | ggplot2, plotnine |
| 3.14 | `geom_smooth` methods: `glm`, `rlm`, `gls`, GAM | Each method is real statistics work | L | ggplot2 (any R model function), plotnine (`glm`, `rlm`, `gls`; no GAM); lets-plot has `lm` and `loess` only |
| 3.15 | `after_scale`, `stage`, expressions in `after_stat` (`after_stat("name")` landed) | Need an expression language over stat output and scaled values; `..count..` and `after_stat("count")` cover the main case | L | ggplot2, plotnine |
| 3.18 | Binned position, size and alpha scales (`scale_x_binned`, `scale_size_binned`, `guide_bins()`); the color and fill steps scales landed | Binned axes and size legends need their own guide | M | ggplot2 only |
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
| 5.6 | JPEG, TIFF and other raster formats in `ggsave` | Some journals ask for TIFF | S | ggplot2, plotnine; lets-plot saves SVG, HTML, PNG, PDF only |
| 5.7 | ggpubr helpers beyond `compare_means` | Publication plots | M | ggplot2 extension (ggpubr) |

## 6. Theming and polish

| # | Item | Why here | Size | Found in |
|---|---|---|---|---|
| 6.1 | Locale-aware label formatting | Correct labels outside English | M | ggplot2 (through scales' `label_number()` and friends) |
| 6.2 | Theme options: `axis_ticks_direction`, `legend_justification_*`, `legend_text_position`, `legend_title_position`, `legend_box_background`, `legend_box_margin`, `legend_frame` | Fine layout control | M | ggplot2 3.5+, plotnine; `axis_ticks_direction` is in neither |
| 6.5 | Rich text in labels (ggtext style) | Wanted, but touches text layout | L | lets-plot `element_markdown` (ggrust already has it), ggplot2 extension (ggtext); check what is still missing |
| 6.6 | `gggrid` `sharex` / `sharey` with discrete scales | Narrow case | M | lets-plot |
| 6.7 | Corner-aware legend and tag layout | Narrow case | M | n/a (layout quality) |
| 6.9 | Axis capping | Cosmetic | S | ggplot2 (`guide_axis(cap=)`) |
| 6.12 | Bivariate color scale with a bivariate legend | Niche, needs a new guide | L | ggplot2 extension (biscale) |

## 7. Specialised charts

| # | Item | Why here | Size | Found in |
|---|---|---|---|---|
| 7.1 | `geom_contour_filled` fills bands by an ordered discrete level, as ggplot2 (today an alias of `geom_contourf`, continuous fill) | Discrete legend of bands | S | ggplot2 |
| 7.2 | `geom_density_ridges` with the full ggridges options | `geom_area_ridges` covers the main use | M | ggplot2 extension (ggridges); lets-plot `geom_area_ridges` covers part |
| 7.4 | `coord_radial`, radar charts | Follows 2.5 | M | `coord_radial`: ggplot2; radar charts: lets-plot through `coord_polar` |
| 7.6 | Business and infographic: Sankey / alluvial (plan ex2.6), treemap, waffle, waterfall, chord, circle packing, sunburst, funnel, Pareto, Marimekko, Venn, word cloud, bullet, variable-radius pie, timeline, tile map, wind barbs | Outside the scientific core; Sankey first, as it is scheduled | L | ggplot2 extensions (ggalluvial, treemapify, waffle, ...) |
| 7.8 | Spatial: `coord_map`, `coord_sf`, `geom_sf`, `geom_sf_text`, `geom_map`, map tiles, polylabel placement, `annotation_scale` | A subsystem of its own | L | ggplot2 (`coord_sf`, `geom_sf`), lets-plot (`geom_map`, `coord_map`, `geom_livemap`, map tiles), plotnine (`geom_map`) |

## Non-essential (plotnine only)

Only plotnine has these, so they fall outside ggrust's identity (AGENTS.md). They stay
here for the record and are built only if the user asks for them.

| # | Item | Why here | Size | Found in |
|---|---|---|---|---|
| N.1 | `annotation_stripes` (was part of 3.22) | Shades alternate dodged categories | S | plotnine only |
| N.2 | Multi-page PDF, `save_as_pdf_pages` (was 5.5) | Batch reports | S | plotnine only |
| N.3 | `theme_538`, `theme_tufte` (were part of 6.4) | Data only | S | plotnine only (ggthemes for ggplot2) |
| N.4 | `watermark` (was 6.11) | Rarely needed | S | plotnine only |
