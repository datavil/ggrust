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

Each issue's repro (from the issue body; #1443 with `geom_histogram` since `geom_density`
is missing) rendered on ggrust and on lets-plot 4.11.0, 2026-10-31, and inspected side by
side. Script: `issue_repros.py` (scratch, not committed). Verdicts:

- **Avoided:** ggrust already does better than lets-plot.
- **Copied:** ggrust shows the same bug. Score = significance of fixing it in ggrust.
- **Gap:** ggrust does not support the feature the repro uses (reported in a `GgrustWarning`).

| Score | lets-plot issue | Area | Verdict on ggrust | Expected (ggplot2) |
| --- | --- | --- | --- | --- |
| 5 | #1132, #997 | `as_discrete` | **Gap.** Only `levels=` works; plain `as_discrete('c')`, `label=`, `order=`, `order_by=` are ignored with "'as_discrete' is not supported yet", so a numeric column stays continuous (colorbar instead of a discrete legend). | Discrete scale, label and ordering applied. |
| 4 | #1398 | `geom_bar(stat='identity')`, one group per x | **Copied.** Bars overlap (80 drawn over 90) instead of stacking to 170 and 50. | Stacks. |
| 4 | #1443 | Free facet scales with a binned stat (histogram) | **Copied.** Every panel spans the full data range. | Per-panel range. |
| 4 | #339, #906 | Text at the plot edge, wide last axis label | **Copied.** Both clipped at the panel/plot edge. | Not clipped (ggplot2 clips geoms to the panel but never cuts axis labels). |
| 4 | #1506 | Many legends in a small plot | **Copied.** Legends run off the bottom of the canvas. | Legends fit (ggplot2 lays them out in a box; grows or wraps). |
| 4 | #1504 | `element_text(angle=, vjust=)` on axis titles | **Gap.** Reported "not supported yet; ignored". (lets-plot ignores it silently.) | Applied. |
| 4 | #1015 | `gggrid(sharex=, sharey=)` | **Gap.** `layout.sharex` is reported unsupported and ignored. (lets-plot accepts it but it does not work with discrete scales.) | Shared axes. |
| 3 | #1143 | Mapped `hjust` (and `vjust`) on text and label | **Gap.** "mapping data to 'hjust' is not supported yet"; all labels centred. | Any number per row. |
| 2 | #1129 | Duplicate legend labels in `scale_*_manual(labels=)` | **Copied.** Two levels labelled "Label A" merge; the red A key disappears. | Two keys with the same label. |
| 2 | #1147 | Legend `override_aes` with `linetype` / `linewidth` on a rect key | **Copied, differently.** No dashes; key drawn as an odd small notched shape. | Dashed outline at the given width. |
| 2 | #1299 | `coord_fixed` + free facet scales | **Copied, with a warning.** Non-square grid, but ggrust warns that it is not supported. | ggplot2 errors. Decide: keep the warning or raise. |
| 1 | #704 | Stacked bars on a log10 y scale | **Copied.** Stacking happens in log space, so totals look far too large. | ggplot2 does the same (stacking on transformed scales is ill-defined). Keep. |
| 1 | #701 | `label` aesthetic creates groups | **Same as lets-plot.** | ggplot2 also groups by discrete `label`. Not a bug. Keep. |
| 1 | #1495 | Many discrete axis labels | **Avoided (as ggplot2).** ggrust draws every label (they overlap); lets-plot silently drops 3 of 4. | ggplot2 draws all. Keep. |
| - | #1270 | Boxplot with a group of only NA | **Avoided.** Draws a and b; lets-plot fails to render. | |
| - | #769 | Color limits outside the data | **Avoided.** All four limit cases render with out-of-range points grey; lets-plot crashes on two. | |
| - | #1512 | Line legend keys ignore `alpha` | **Avoided.** Key alpha matches the lines. | |
| - | #1267 | Text size with a sqrt size scale | **Avoided.** Text size is right. ggrust-only oddity: the size legend for a single value lists five near-identical breaks (9.99992e9 ... 1.000008e10). Score 2. | One key. |
| n/a | #1474, #985, #1511 | `label_text` angle (needs `geom_crossbar`), seams in density fills (needs `geom_density2df`/`geom_density`), removing boxplot outliers (no `outliers=` option in either library) | Not testable until the geoms exist; add `outliers=` with the boxplot work. | |

## 12. Docs workarounds to undo

When one of these gaps closes, put the original spec back in the docs example that works
around it, then rebuild with `uv run python docs/build_examples.py`.

| Gap | Example | Workaround to undo |
| --- | --- | --- |
| `geom_curve` (section 1) | `docs/examples/periodic_table.py` | Two key arrows drawn straight with `geom_segment`. lets-plot uses `geom_curve(curvature=.4, ncp=1)` and `geom_curve(curvature=-.4, ncp=1)` for the first and third arrows. |
| `label_format` (section 7) | `docs/examples/periodic_table.py` | Atomic masses rounded to 3 decimals in `docs/data/make_data.py` (`elements()`). lets-plot uses `label_format=".3~f"` on the mass `geom_text`. |
| `plot_caption` (section 6) | `docs/examples/periodic_table.py` | Caption dropped. lets-plot: `labs(caption="© 1869, Dmitri Mendeleev")` and `plot_caption=element_text(size=18)`. |
| `element_text(margin=...)` (section 6) | `docs/examples/periodic_table.py` | Title margin dropped. lets-plot: `plot_title=element_text(..., margin=[30, 0, 5, 0])` (a list, not only the dict form of #1265). |

## Proposed order

1. **Score 5, exposed but erroring or ignored:** `as_discrete`, `geom_sina`,
   `position_fill`, `position_nudge`, `PlotSpec.show()`.
2. **Score 5, everyday grammar:** interval family, ribbon and area, abline, density,
   smooth, datetime scales.
3. **Section 11 score-4 fixes:** single-group identity stacking (#1398), free facet ranges
   for binned stats (#1443), edge clipping of text and axis labels (#339, #906), legend
   overflow (#1506), axis title angle, `gggrid` sharex/sharey.
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
