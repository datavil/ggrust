# ggrust capabilities that replace cellestial workarounds

cellestial was built on lets-plot and works around what lets-plot lacks. ggrust now
does several of these things natively. This file maps each ggrust capability to the
cellestial code it can simplify, for the cellestial switch (plan.md `7.2`, done in the
cellestial repo, not here).

Started 2026-11-05. **Keep it current:** every ggrust step that makes a cellestial
workaround unnecessary adds a row here in the same commit. Paths are in
`../cellestial/cellestial/`.

The import swap (`lets_plot` -> `ggrust`) comes first and must keep cellestial's
output unchanged. The rewrites below come after, one at a time, each checked
against cellestial's tests and golden images.

## Workarounds ggrust can remove

| ggrust capability | ggrust step / commit | cellestial workaround today | Rewrite on ggrust |
|---|---|---|---|
| Arrow axes: `element_line(arrow=arrow(...), extent=0.25)` on `axis_line*` | 9.4 (`memory/arrow_axes.md`) | `layers/arrow.py`: UMAP corner arrows drawn as `geom_segment` / `geom_path` layers, with a `_TIP_FRACTION` hack because lets-plot caps arrowhead size to half the segment | A theme setting; the deferred arrow layer becomes a theme helper |
| Axis dendrograms from a precomputed scipy/scanpy linkage: `scale_*_dendro(linkage=Z)` + `guide_axis_dendro`, follows HTML zoom | ex1.9 `818c8ec`, `561285c` | `util/dendrogram.py`: reads scanpy `icoord`/`dcoord`, rescales by hand, draws `geom_path`/`geom_segment` beside the panel, sets x limits by hand to make room; used by `single/heatmap/heatmap.py`, `dotplot.py`, `stacked_violin.py`, `complex/annotated_heatmap.py` | Pass `adata.uns[key]["linkage"]` and labels; the tree orders the axis and aligns itself. Delete the path math |
| Side layers (ggside): annotation tracks in bands beside the panel, sharing its axis; `ggmarginal()` | ex2.2 `e39bc31` (`memory/side_panels.md`) | `complex/annotated_heatmap.py` (846 lines): every track is a separate `ggplot` with blanked themes, glued with `gggrid(align=True)`; tracks do not zoom with the heatmap | One plot: `ggplot(frame, aes(var, obs, fill=expr)) + geom_tile()`, then per track `+ new_scale_fill() + geom_xsidetile(aes(fill="cell_type")) + scale_fill_brewer(...)` (column tracks; `geom_ysidetile` for row tracks), `+ scale_y_dendro(ClusterTree.from_linkage(Z, labels))`, `+ ggside(x_pos="top", y_pos="left")`, `theme(ggside_panel_scale_x=..., ggside_panel_spacing=...)`, track order via `scale_xsidey_discrete(limits=[...])`. Keep only the AnnData ordering and binning |
| Several fill or color scales in one plot: `new_scale_fill()`, `new_scale_color()` | ex1.3 `0537c82` | `complex/annotated_heatmap.py`: each annotation needs its own fill scale, so each track must be its own plot | Together with side layers, all tracks live in one plot, each with its own legend |
| Axis on the right or top: `scale_*(position="right"\|"top"\|"both")` | 9.3 `5386867` | `complex/annotated_heatmap.py`: track titles sit on the left and widen the aligned grid's left axis (plan.md 9.3 repro) | `position="right"` on the track's y scale |
| Computed significance brackets: `stat_compare_means(method=, comparisons=, p_adjust_method=, label=, hide_ns=)` | ex1.7 `33c3713` | `layers/bracket.py`: runs `scipy.stats.mannwhitneyu` / `ttest_ind`, its own p-value correction and star thresholds, then draws `geom_bracket` | Use the stat. Check first that its p-values, adjustment and star thresholds match `_correct_pvalues` / `_significance_stars` (ggrust matches R) |
| Cluster outlines: `geom_mark_hull`, `geom_mark_ellipse` (with labels) | ex1.8 `2ddd496` | `layers/outline.py` `cluster_outlines`: per-cluster `scipy.stats.gaussian_kde` contours drawn as `geom_path` | Optional: hulls look different from density contours. Keep the KDE look if wanted (or use `geom_density2d` per group); offer hulls as a new style |
| Live figures of a grid: `SupPlotsSpec.figures` | spec accessors `794e33e` | `util/operations.py` `_grid_figures`: reads the name-mangled `vars(grid)["_SupPlotsSpec__figures"]` | `grid.figures` |
| Panel tags across a grid: `gggrid(tag_levels="A")`, grid captions | ex2.3 `4794478`, ua.7 `898773f` | Multi-panel figures label panels by hand or outside cellestial | `tag_levels=` |
| Side band axis theme elements: `theme(ggside_axis_text_y=element_blank())` hides only the x side band's track names (`_x` the y side band's), likewise `ggside_axis_ticks*`, `ggside_axis_line*`, `ggside_axis_ticks_length*` | 9.8 | `complex/annotated_heatmap.py`: `track_titles` split into row/column booleans and names moved by hand so the two bands' titles do not collide top-right | One `track_titles` theme switch per band: `ggside_axis_text_y` (column tracks) and `ggside_axis_text_x` (row tracks) |
| Side band panel look: `theme(ggside_panel_border=element_blank())` drops the band frames and keeps the main one; `ggside_panel_background=element_blank()` (or `element_rect(size=0)` to keep the fill) drops a frame inherited from `panel_background`; `ggside_xside_panel_*` / `ggside_yside_panel_*` for one axis's bands | 10.1 | Phase 5 (`claude/ggrust-phase5-f50370`): a bracket band on top of a dotplot drew the main panel's frame and background around itself (blocker) | Set the band elements in the dotplot theme; no per-band workaround needed |
| Leader-line labels for chosen rows or columns: `scale_y_discrete(breaks=keys, limits=order, guide=guide_axis_mark())` (any side, side band axes, italic via `axis_text_y`) | 9.9 | `complex/annotated_heatmap.py` (`claude/zamboni-fig3-d1c0fe`): `key_labels` widens the observation axis by `_LEADER_WIDTH`, spreads label slots by hand (`_spread_labels`), draws the leaders as a `geom_segment` layer in the panel margin (`_leader_layer`) and labels a continuous key axis at the spread positions | One scale with `guide=guide_axis_mark(spread="even")` (cellestial's current spacing) or the default `nearest`; drop `_spread_labels`, `_leader_layer`, `_LEADER_WIDTH` and the widened limits |
| A side band on each side of an axis, or several stacked on one side: `geom_ysidetile(aes(fill="module"), side_pos="right")` with another y band `side_pos="left"`; `side_band="name"` stacks bands outward, `side_size=` sizes one, `scale_ysidex_discrete(side_pos=, side_band=, ...)` sets one band's axis | 9.10 | `complex/annotated_heatmap.py`: gene labels and the module track share the right side (one y band per plot), while Zamboni et al. 2025 Fig. 3b has the labels on the left | Labels on the left (`scale_y_discrete(position="left", guide=guide_axis_mark())` or a left band) and the module track `side_pos="right"`, in one plot |

## New capabilities with no workaround yet

Things cellestial could not offer on lets-plot at all. Candidates for new cellestial
options, not part of the switch:

| ggrust capability | ggrust step / commit | Use in cellestial |
|---|---|---|
| Rasterized layers in SVG/PDF: `rasterize(layer, dpi=300)` | ex1.5 `ca52e9d` | Publication UMAPs with millions of cells as small vector PDFs |
| Point density coloring: `geom_pointdensity` | ex1.4 `50b5943` | Density-colored embeddings |
| Custom fonts: `register_font(path)` | 7.1b `08a0a59` | Journal fonts (Arial, Helvetica) in exported figures |
| Journal palettes: `scale_color_npg()`, `_lancet`, `_igv`, ... | ex1.1 `8320606` | Palette options for clusters (`igv` has 51 colors) |
| Beeswarm and quasirandom points | ex1.2 `99ff27b` | Alternative to jitter/sina in distribution plots |
| Raincloud plots: half violins, `stat_halfeye`, intervals | ex1.6 `a469251` | A distribution plot style |
| `gghighlight` | ex2.5 `0dd73e0` | Highlight one cluster or condition, grey out the rest |
| Secondary axes: `sec_axis`, `dup_axis` | exc.4 `5ee84c5` | Dual units on QC plots |
| `+` defers unknown operands: `PlotSpec.__add__`, `FeatureSpec.__add__` and `SupPlotsSpec.__add__` return `NotImplemented`, so a plain object's `__radd__(plot)` runs | 10.2 | `layers/_deferred.py`: `DeferredLayer` subclasses `PlotSpec` and skips its `__init__` only so Python tries its `__radd__` before `PlotSpec.__add__` raises | A plain class with `__radd__(self, plot)`; drop the `PlotSpec` base and the skipped `__init__` (the subclass form keeps working) |
