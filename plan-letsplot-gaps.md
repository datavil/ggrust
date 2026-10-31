# lets-plot features missing from ggrust

Gap analysis of ggrust (`main` at `1719dfd`) against lets-plot 4.11.0, done 2026-10-31.
This is a findings list and a proposed order, not a commitment. Scope still follows
AGENTS.md: cellestial first, then general plotting needs, not all of lets-plot.

## How this was measured

- **Names:** every public name in `lets_plot` vs `ggrust` (`dir()`). lets-plot has 223,
  ggrust 128; 99 lets-plot names are missing.
- **Signatures:** parameter names of every function both expose (`inspect.signature`).
  ggrust mirrors them almost exactly; only geo/map parameters are missing.
- **Options probe:** each lets-plot parameter passed to the ggrust function and the plot
  rendered; a parameter counts as a gap when ggrust reports it in its unsupported-options
  `GgrustWarning`. A parameter that raised a type error counts as consumed (supported).
  Not covered: options ggrust consumes but renders differently.
- **Registries:** every `stat=` and `position=` name lets-plot accepts, passed to a layer.
- **lets-plot issues:** all 144 open issues in `JetBrains/lets-plot` (2026-10-31) read by
  title, bodies of the relevant ones. Linked as `#n` in the "lets-plot issues" column
  (https://github.com/JetBrains/lets-plot/issues/n). Per AGENTS.md, do not copy these
  behaviors: check ggplot2 and ask.
- cellestial's full suite (807 tests) passes on ggrust, so **none of these gaps blocks
  cellestial today**.

## Significance scale

| Score | Meaning |
| --- | --- |
| **5** | Must do. ggrust already exposes the name but errors, or the feature is everyday grammar that ported code will hit. |
| **4** | Should do. Common in general plotting; its absence is noticeable. |
| **3** | Useful. Regular but not daily use, or a convenience over existing features. |
| **2** | Nice to have. Niche, recent, or unstable in lets-plot itself. |
| **1** | Can be skipped. Geo, deprecated, runtime plumbing that ggrust does not need, or a semantics we deliberately do not copy. |

## 1. Geoms

| Score | Missing | Needs stat | lets-plot issues | Note |
| --- | --- | --- | --- | --- |
| 5 | `geom_sina` (exposed, engine errors) | `sina` | | Python API exists; engine says "not supported by ggrust yet" (`geom::NOT_DRAWN_YET`, `stat::NOT_COMPUTED_YET`). |
| 5 | `geom_errorbar`, `geom_linerange`, `geom_pointrange`, `geom_crossbar` | identity | | Interval family; one drawing path. |
| 5 | `geom_ribbon`, `geom_area` | identity / stack | #1199, #994 (gradient fill requests) | |
| 5 | `geom_smooth` | `smooth` (lm, loess, CI band) | | loess is the costly part. |
| 5 | `geom_density` | `density` | #1443 (free facet scales show the full data range, unlike ggplot2) | `stat/density.rs` already holds the shared KDE. Follow ggplot2 on #1443. |
| 5 | `geom_abline` | identity | #1215 (not in `coord_polar`) | |
| 4 | `geom_step`, `geom_freqpoly` | identity / bin | #838 (no `direction='mid'`), #854 (confusing x tooltip with groups) | Support `mid` as ggplot2 does. |
| 3 | `geom_count` | `sum` | | |
| 3 | `geom_bin2d`, `geom_hex` | `bin2d`, `binhex` | #1290 (wrong tile size when `bins=[n, 1]`) | |
| 3 | `geom_contour`, `geom_contourf`, `geom_density2d`, `geom_density2df` | `contour(f)`, `density2d(f)` | #1357 (fails on unsorted grids), #1356 (wrong isolines on incomplete grids or `z = 0` nodes) | Needs a marching-squares core; accept any row order. |
| 3 | `geom_qq`, `geom_qq_line`, `geom_qq2`, `geom_qq2_line` | `qq`, `qq_line`, `qq2`, `qq2_line` | | |
| 2 | `geom_dotplot`, `geom_ydotplot` | `dotplot`, `ydotplot` | #1283 | |
| 2 | `geom_lollipop` | identity | #1512 (line alpha not in legend) | |
| 2 | `geom_spoke` | identity | #1282, #1070, #989 (no radius unit or scale, absent from legend and tooltips) | Unfinished in lets-plot. |
| 2 | `geom_curve`, `geom_pointdensity` | identity / `pointdensity` | | |
| 2 | `geom_band`, `geom_bracket_dodge` | identity | | Recent lets-plot additions. |
| 2 | `geom_pie` | `count2d` | #661 (labels do not rotate to fit) | Needs `layer_labels`. |
| 1 | `geom_map`, `geom_livemap` | | many (#1448, #795, #771, #695, #685, ...) | Geo, see section 8. |

## 2. Stats (as `stat=` values and `stat_*` layers)

ggrust computes `identity`, `count`, `bin`, `boxplot` (+ outliers), `ydensity`, `densityridges`.

| Score | Missing | lets-plot issues | Note |
| --- | --- | --- | --- |
| 5 | `density`, `smooth`, `sina` | #1443 | With their geoms above. |
| 4 | `summary`, `summary_bin` (+ `stat_summary`, `stat_summary_bin`) | #1507 (no `mean_se` / `mean_sdl` shortcuts) | Mean and CI overlays. Offer the ggplot2 shortcuts. |
| 4 | `ecdf` (+ `stat_ecdf`) | | |
| 3 | `sum` (+ `stat_sum`), `bin2d`, `binhex`, `contour`, `contourf`, `density2d`, `density2df`, `qq`, `qq_line`, `qq2`, `qq2_line` | #1290, #1356, #1357 | With their geoms. |
| 2 | `dotplot`, `ydotplot`, `count2d`, `pointdensity` | | With their geoms. |

## 3. Positions

| Score | Gap | lets-plot issues | Note |
| --- | --- | --- | --- |
| 5 | `position_fill` (exposed, engine errors) | #1398 (`stat='identity'` does not stack with one group) | `position::NOT_APPLIED_YET`. Close to `stack` plus normalisation; check #1398 against stack too. |
| 5 | `position_nudge` (exposed, engine errors) | | `position::NOT_APPLIED_YET`. |
| 2 | `position_dodgev` | | Vertical dodge. |

## 4. Scales

| Score | Missing | lets-plot issues | Note |
| --- | --- | --- | --- |
| 5 | `scale_x_datetime`, `scale_y_datetime`, `scale_x_time`, `scale_y_time` | #1378 (no `date_breaks`), #995 (no minor tick labels on dates) | Dates are sent as epoch milliseconds today, so axes show raw numbers. Offer `date_breaks` as ggplot2 does. |
| 4 | `scale_color_grey`, `scale_fill_grey`, `scale_color_discrete`, `scale_fill_discrete` | | Small. |
| 4 | `scale_linetype_manual`, `scale_linetype_identity` | #1147 (dotted legend key lost at large size) | |
| 3 | `scale_linewidth`, `scale_linewidth_identity`, `scale_stroke`, `scale_stroke_identity` | #1158 (stroke does not grow shape 19) | |
| 3 | `scale_x_continuous`: `break_width`, `lablim`, `na_value` (all positional scales) | #769 (limits outside the data crash) | Reported unsupported. Do not copy #769. |
| 2 | Multi-aesthetic forms: `scale_hue`, `scale_gradient`, `scale_gradient2`, `scale_gradientn`, `scale_manual`, `scale_brewer`, `scale_viridis`, `scale_grey`, `scale_identity`, `scale_continuous`, `scale_discrete` | | Thin wrappers with an `aesthetic=` list. |
| 1 | `scale_color_cmapmpl`, `scale_fill_cmapmpl`, `scale_cmapmpl` | | Needs matplotlib colormaps. |

## 5. Coordinates and layout

| Score | Missing | lets-plot issues | Note |
| --- | --- | --- | --- |
| 4 | `expand_limits` | | Small (a blank layer). |
| 3 | `coord_polar` | #1215, #1213, #1095, #1097, #1036, #1039, #1061, #1182 | Many open bugs and limits in lets-plot (no abline, `size_unit` gives emptiness, no raster, no rotated labels, slow tooltips). Design from ggplot2. |
| 3 | `ggmarginal` | #972 (no bar margins) | |
| 3 | `facet_grid`: `x_order`, `y_order`, `x_format`, `y_format`, `x_labwidth`, `y_labwidth`; `facet_wrap`: `labwidth` | | Reported unsupported. |
| 2 | `labs(tag=...)` and theme `plot_tag*` | | |
| 2 | `ggbunch` (new free layout), `ggdeck` | #424 (patchwork-like layout requested) | `GGBunch` (deprecated) raises on purpose. |
| 1 | `coord_map` | | Geo. |

## 6. Theme

| Score | Unsupported theme parameters | lets-plot issues |
| --- | --- | --- |
| 4 | `legend_key_width`, `legend_key_height`, `axis_ticks_length_x/_y`, `plot_caption`, `plot_title_position`, `plot_caption_position`, `strip_text_y`, `strip_background_y`, `strip_spacing_x`, `axis_text_spacing_x/_y`, `element_line(linetype=)`, `element_rect(linetype=)` | |
| 4 | `element_text(margin=...)` | #1265 (dict margins not accepted) |
| 3 | Minor ticks: `axis_minor_ticks*`, `axis_minor_ticks_length*` | #1183 |
| 3 | `exponent_format`, `label_text` | #1474 (`angle` ignored for `label_text`) |
| 2 | On-top layering: `axis_ontop*`, `panel_grid_ontop*`, `panel_border_ontop`; `panel_inset`; `geom` | #1043 (tooltip anchor ignores `panel_inset`) |
| 2 | Tooltip styling: `tooltip`, `tooltip_text`, `tooltip_title_text`, `tooltip_merge`, `tooltip_max_count`; axis tooltips `axis_tooltip*` | #1178, #859 |
| 1 | `plot_message` | |

## 7. Layer options (reported unsupported on every geom)

| Score | Option | lets-plot issues | Note |
| --- | --- | --- | --- |
| 4 | `orientation` (bar, histogram, boxplot, violin) | | Horizontal geoms without `coord_flip`. |
| 3 | Text and label `label_format`, `check_overlap`, `na_text` | #270 | |
| 3 | `manual_key` (custom legend entries) | | |
| 2 | `size_unit` (point, text, label) | #1213, #270 | Buggy in lets-plot. |
| 2 | `nudge_unit`, `alpha_stroke`, `halo_width`, `halo_color` | | |
| 2 | `layer_labels` (pie labels), `layer_key` | #661 | |
| 1 | `color_by`, `fill_by` (paint_a/b/c aesthetics) | | |
| 1 | `sampling` and every `sampling_*` function | #141 (random sampling drops boxplot boxes and keeps outliers) | lets-plot samples large data silently; ggrust draws everything. Sampling breaks stats (#141). Recommend recording "no default sampling" as a deliberate deviation. |
| 1 | `flat`, `geodesic`, `spacer` (path, segment) | #795, #771 | Geo / segment spacing. |

## 8. Geo and maps (score 1)

`geom_map`, `geom_livemap`, `coord_map`, `map` / `map_join` / `use_crs` on point, path,
polygon, text and label, `geo_data`, `maptiles_*`, `tilesets`. They need geopandas input,
projections and tile fetching, and carry many open lets-plot bugs (#1448, #795, #771, #695,
#685, #381, #352, #350, #125). Recommend recording them as out of scope.

## 9. Bistro (prebuilt plots, `lets_plot.bistro`)

| Score | Missing | lets-plot issues | Note |
| --- | --- | --- | --- |
| 3 | `corr_plot`, `joint_plot`, `qq_plot` | #866 (`corr_plot` + `theme_void` broken axes), #295 | After tiles, smooth, density and qq exist. Pure Python on the spec API. |
| 2 | `residual_plot`, `waterfall_plot`, `image_matrix` | | |

## 10. Output and runtime

| Score | Gap | lets-plot issues | Note |
| --- | --- | --- | --- |
| 5 | `PlotSpec.show()` | | Missing method; notebooks and scripts call it. |
| 3 | Vector PDF | #1509 (`to_png` drops titles and axes when a layer has `linetype=0`), #1440 (Japanese fonts in `ggsave`) | ggrust writes a raster PDF; lets-plot's is vector. |
| 1 | `LetsPlot.setup_html` options, `NO_JS`, `OFFLINE`, `frontend_context`, `cfg`, settings utilities, `is_production`, `get_global_bool` | | ggrust inlines its runtime; keep names only if users hit import errors. |
| 1 | `font_family_info`, `font_metrics_adjustment` | | |

## 11. Open lets-plot bugs in features ggrust already has

These are not gaps, but places where ggrust follows lets-plot by default. **Not verified
against ggrust yet**: each needs a quick repro to confirm ggrust does not copy the bug.

| lets-plot issue | Area | ggplot2 / expected behavior |
| --- | --- | --- |
| #1398 | `geom_bar(stat='identity')` does not stack with one group | Stacks. |
| #1270 | `geom_boxplot` not drawn when a group has a single category value | Draws. |
| #1511 | Boxplot outliers cannot be removed | `outlier_shape=NA` / `outliers=FALSE`. |
| #1443 | Histogram with free facet scales shows the full data range | Per-panel range. |
| #704 | `geom_bar` + `fill` + log10 y gives wrong axis labels | Correct labels. |
| #769 | Continuous scale limits outside the data crash | Draws an empty or clipped range. |
| #1512 | Line, path legend keys ignore `alpha` | Legend shows alpha. |
| #1147 | Dotted line legend key lost at larger size | Visible. |
| #1504, #1474 | `angle` ignored for axis titles and `label_text` | Applied. |
| #1143 | `geom_label` `hjust` only accepts 0, 0.5, 1 | Any number. |
| #1129, #997, #1132 | Duplicate legend labels rejected; `labels` breaks custom order; `as_discrete(label=)` ignored with `levels` | Allowed / kept / applied. |
| #701 | `label` aesthetic creates groups in text geoms | Does not group. |
| #1015 | `gggrid` `sharex`/`sharey` broken with discrete scales | Shared. |
| #1299 | `coord_fixed` + free facet scales gives a non-square grid | ggplot2 errors ("doesn't support free scales"). |
| #906, #339, #1495, #1506 | Wide axis labels cropped, text outside plot bounds, axis labels dropped, legends wrapped badly | Not clipped, laid out correctly. |
| #1267 | `geom_text` label size too small with log10 transform | Unaffected by transform. |
| #985 | Visible gaps between filled geometries | No seams. |

## Proposed order

1. **Score 5, exposed but erroring:** `geom_sina`, `position_fill`, `position_nudge`,
   `PlotSpec.show()`.
2. **Score 5, everyday grammar:** interval family, ribbon and area, abline, density,
   smooth, datetime scales.
3. **Section 11 repros:** confirm ggrust avoids the listed lets-plot bugs (cheap, protects
   parity work).
4. **Score 4:** step and freqpoly, summary and ecdf, grey, discrete and linetype scales,
   `expand_limits`, `orientation`, score-4 theme parameters.
5. **Score 3:** 2D and contour families, qq, `coord_polar`, `ggmarginal`, facet label
   options, minor ticks, bistro, vector PDF.
6. **Score 2 and 1:** only on demand. Record the score-1 items (geo, sampling, cmapmpl,
   runtime plumbing) as decisions in `plan.md`.

## Open questions for the user

1. Should geo and maps be recorded as out of scope?
2. Sampling: keep "draw everything" as a deliberate deviation (recommended, see #141)?
3. Should these become a new phase in `plan.md` (for example Phase 8) once 7.x is done?
