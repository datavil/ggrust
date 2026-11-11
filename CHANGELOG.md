# Changelog

All notable changes to ggrust. Each version is also a [GitHub Release](https://github.com/datavil/ggrust/releases).

## [Unreleased]

### Added

- `stat_unique`, `stat_hull` (plotnine) and `stat_summary_2d`.
- `coord_cartesian(expand=)` and `coord_fixed(expand=)`, as ggplot2.
- `position_stack(reverse=)` and `position_fill(reverse=)`.
- `geom_smooth(fullrange=, formula=, method_args=)`, as ggplot2 (`formula` is `y ~ x` or `y ~ poly(x, k)`).
- `geom_density(bounds=)` (boundary reflection, as ggplot2); `cut`, `clip` and `gridsize` are accepted for plotnine code.
- `geom_raster(hjust=, vjust=, interpolate=)`, as ggplot2.
- `annotate()`: a one-off layer from constants, as ggplot2.
- `stat_summary` and `stat_summary_bin`: `fun_data` shortcuts `mean_cl_normal`, `mean_cl_boot` and `median_hilow`, with `fun_args` `conf_int`, `B` and `seed`.
- More `trans` names: `log`, `log1p`, `exp`, `logit`, `probit`, `reciprocal`, `pseudo_log`, `symlog` and `boxcox_trans(p, offset)`, plus `scale_x_symlog` and `scale_y_symlog`.
- `geom_spoke`, as lets-plot, with `arrow` and `pivot`.
- `geom_lollipop`, as lets-plot, with `dir`, `fatten`, `slope` and `intercept`.
- `geom_dotplot`, `geom_ydotplot` and `stat_bindot`, as lets-plot, with `stackdir`, `stackratio`,
  `dotsize`, `stackgroups` and the `dotdensity` and `histodot` methods.
- `scale_color_cmapmpl` and `scale_fill_cmapmpl`: any matplotlib color map, by name or object (needs matplotlib).
- Polar axes: theta labels and ticks around the ring, the radius axis on the panel edge.
- `coord_polar` for pie, donut, rose and radar charts, with lets-plot's `theta`, `start`, `direction` and `transform_bkgr`. HTML output is static for now.
- `geom_qq`, `geom_qq_line`, `geom_qq2`, `geom_qq2_line`, as lets-plot, with `distribution` (`norm`, `uniform`, `t`, `gamma`, `exp`, `chi2`), `dparams` and `quantiles`; `stat_qq` and `stat_qq_line`, as ggplot2's names.
- `PlotSpec.scale_info("x")`: a position scale after the build, one entry per panel, with limits, levels, domain, breaks and labels.
- `PlotSpec.columns`, every data column the plot reads, and `PlotSpec.validate()`, which builds without drawing and returns the warnings.
- `PlotSpec.guide_data(aesthetic)`: a polars frame with the keys of a legend or colorbar (mapped value and label), as ggplot2's `get_guide_data()`. `build()` also returns them under `guides`.
- Horizontal `geom_errorbar`, `geom_linerange`, `geom_pointrange` and `geom_crossbar`: map `y`, `xmin` and `xmax`, or pass `orientation="y"`, as in lets-plot. `height` on `geom_errorbar` is a deprecated alias of `width`.
- `ggrust.bistro.funky_heatmap` and `position_arguments`, a port of the R package funkyheatmap: a table of bars, circles, rounded squares, rectangles, pies, text and PNG images, with column groups, row groups and legends, from funkyheatmap's `data` / `column_info` / `row_info` / `column_groups` / `row_groups` / `palettes` / `legends` arguments. The result is an ordinary plot, so `+` adds layers in table coordinates. ggrust additions: `col_annot_position="bottom"` (column names under the table), `col_group_radius` (pill-shaped group headers), `legend_position="right"|"none"`, and a canvas that grows to fit long labels. A new gallery example rebuilds the scIB benchmark summary figure (panel a of Luecken et al. 2022).

### Changed

- `lablim` on a continuous color, fill, size or alpha scale now warns. It still has no effect, as in lets-plot; use `format=` to shorten numeric labels.

## [0.8.1] - 2026-11-09

### Added

- Working `sampling=` on layers: `sampling_random(n, seed=None)` and `sampling_systematic(n)` (lets_plot specs, ggrust's own row picks), `sampling_fraction(fraction, seed=None)` (ggrust extension: `0.2` keeps 20% of the rows), `sampling_none()` (returns `"none"`), chains joined with `+`, and `sampling=5000` as a shortcut for `sampling_random(5000)`. Sampling runs on each facet panel's stat output, as lets-plot. ggrust still never samples by default.
- `sampling_random_stratified`, `sampling_pick`, `sampling_group_random`, `sampling_group_systematic`, `sampling_vertex_vw` and `sampling_vertex_dp` are exported so lets_plot code imports; rendering warns and skips them.
- `geom_text` and `geom_label` take `label_format` (a number format such as `".3~f"` or a template such as `"{.1f} kg"` or `"{} items"`, also for text labels and constant `label=`), `na_text` (what a missing label shows, default `n/a`) and `check_overlap=True` (a label that touches one drawn before it in the same layer is skipped), as lets_plot. The periodic table example formats its atomic masses with `label_format` instead of rounding the data.
- `stat_centroid(fun="mean"|"median")` and `stat_summary_xy(fun_x=, fun_y=)`, as the R package ggpp (`stat="centroid"` / `"summary_xy"` in any geom): one row per group and facet panel at the centre of its x and y. `geom_text(aes(label="cluster"), stat="centroid")` names each cluster at its centre instead of in a legend.
- `geom_density2d(contour_var="density"|"ndensity"|"count")` and `geom_density2df(...)`, as ggplot2: contour the density, the density divided by its maximum (comparable across samples of different sizes) or the density times the number of points. lets_plot has no `contour_var`.
- `breaks=[...]` on `geom_density2d`, `geom_density2df`, `geom_contour` and `geom_contourf`: explicit contour levels (in `contour_var` units), winning over `bins` and `binwidth`; bands are drawn only between two breaks, as ggplot2.
- `geom_segment` and `geom_curve` take `size_start`/`size_end` and `stroke_start`/`stroke_end`: the line stops at the outline of a point of that size and stroke at each end (as lets-plot, same pixels), so arrows between points no longer hide under them. Mapping a column to them warns and is not supported yet (see deviations.md).
- `geom_segment(spacer=)`: pixels cut off both ends, as `geom_curve(spacer=)` and lets_plot.
- `scale_linewidth()`, `scale_linewidth_identity()`, `scale_stroke()` and `scale_stroke_identity()`, as lets_plot: `range` (default 1..7), breaks, labels, limits and legends. Mapping `stroke` now uses a 1..7 scale (it was not scaled). Limits that exclude the data draw `na_value` instead of failing (lets_plot issue 769).
- `scale_x_continuous(break_width=5)` (and y): a break at every multiple of the width, in transformed units (`break_width=1` on a log10 axis is every power of ten). A width that is 0, negative or not a number raises `SpecError`.
- `na_value=` on continuous position scales, as ggplot2: missing positions, and positions outside `limits`, are drawn at that value instead of being dropped. lets_plot documents the option but drops the rows.
- `lablim=` on discrete position scales cuts long tick labels to that many characters plus `...`, as lets_plot. Continuous axes ignore it, as lets_plot does.

### Changed

- `sampling=` is applied instead of being ignored with a warning. `sampling={"name": "random", ...}` and `sampling_systematic(...)` now drop rows; invalid samplings raise `SpecError` (a float `sampling=0.2` raises `UnsupportedValueError` and suggests `sampling_fraction(0.2)`).
- The default `linewidth` scale follows lets_plot 4.11: range 1..7 (was ggplot2's 1..6) and missing values drawn 1 unit wide (were not drawn). Plots that map `linewidth` draw slightly thicker lines.
- Point shapes 19 and 20 grow with `stroke` like shape 21, as in ggplot2 (lets_plot ignores it, issue 1158). Shape 16, the default, is unchanged.
- `GGBunch()` raises with a recipe for free layouts (`inset_element(..., align_to="full")` on an empty `ggplot()`); lets_plot's `ggbunch()` will not be ported (see deviations.md).
- Continuous axis `labels=[...]` without `breaks=` are used, as in ggplot2: they name the axis's generated breaks (about 5, the same at any plot size, kept on their values under HTML zoom). A different number of labels raises `SpecError`, which lists the generated breaks. lets-plot ignores such labels, and ggrust used to warn and ignore them (see deviations.md).
- The `GGBunch` stub's error no longer refers to lets_plot: "GGBunch is not supported by ggrust yet; use gggrid() to lay out plots in a grid".
- `ggsave(plot, filename)` without `path` writes to `filename` as given (relative to the working directory, or absolute), as ggplot2. It no longer writes into a `lets-plot-images/` folder (lets_plot's default; see deviations.md).

## [0.8.0] - 2026-11-08

### Added

- `geom_streamline()` / `stat_streamline()`, as R metR: `ggplot(data) + geom_streamline(aes(x=, y=, u=, v=))` draws streamlines with one arrowhead half way along each line, per group and facet panel. Rows that form a complete regular grid are used as the field; scattered rows are first averaged onto a grid with `velocity_grid()` (`gridded=` forces either). Lines are integrated with `streamplot()` (`density`, `minlength`, `maxlength`, `integration_direction`, `broken_streamlines`). New stat-only aesthetics `u` and `v`. Not in lets-plot (see deviations.md).
- `velocity_grid(points, vectors, ...)`: average vectors at scattered points onto a regular grid (Gaussian-weighted nearest neighbours), as scVelo's `compute_velocity_on_grid`, without scVelo. Returns a `VelocityGrid(x, y, u, v)` that feeds `streamplot()` directly. Same values as scVelo 0.3.4 on the reference cases; about 7x faster.
- `density_outline(points, level=...)`: outline a 2D point cloud where its Gaussian kernel density falls to `level` times its peak, as scipy's `gaussian_kde` plus scikit-image's `find_contours`, without either. Returns an `(x, y, path)` frame for `geom_path()`. Same contours as scipy 1.17 / scikit-image 0.26 on the reference cases; about 10x faster.
- `kernel_density(values, at, bandwidth=...)`: the Gaussian kernel density of a 1D sample at given points, as scipy's 1D `gaussian_kde` (Scott, Silverman or a factor), returned as an `(x, density)` frame. Within 1e-13 of scipy's values on the reference cases.

## [0.7.1] - 2026-11-08

### Added

- `streamplot(x, y, u, v, density=...)`: matplotlib's streamlines computed in Rust, no matplotlib needed. Returns a `StreamplotSet` with `lines` (x, y, group) and `arrows` (x, y, xend, yend, group) polars frames, ready for `geom_path()` + `geom_segment(arrow=)`. Lines and arrows match matplotlib 3.10 exactly on the reference cases. Masked arrays and NaN/inf are treated as missing; a density that would need more than 1e8 mask cells raises an error instead of aborting.

## [0.7.0] - 2026-11-07

### Changed

- HTML output draws point layers with more than 5,000 markers on a canvas (was 50,000), so mid-sized scatters make much smaller pages. SVG and PNG output are unchanged.
- Smaller interactive HTML: the interactive legend indexes keys per marker instead of wrapping runs of points, and point tooltips on canvas layers reuse the canvas coordinates. Rendering is unchanged.
- `PlotSpec`, `FeatureSpec` and `SupPlotsSpec` `+` returns `NotImplemented` for operands it does not handle, so another object's `__radd__(plot)` runs: a plain class can extend `+` without subclassing `PlotSpec`. Unhandled operands still raise `TypeError` (Python's standard message). lets_plot raises in `__add__` itself.

### Added

- `grid[i] = figure` on `gggrid` results replaces a cell in place (int or `(row, column)`, negative from the end, `None` empties it), so `grid[i] += layer` adds a layer to one plot of a grid, nested grids included.
- `scale_x_dendro(leaf_levels=)` / `scale_y_dendro(leaf_levels=)`: dendrogram leaves that span a run of axis levels, `{leaf: [levels]}` (for example a tree over clusters beside a heatmap of their cells). The runs are placed end to end in the tree's order, each in its given order, and each leaf is drawn at the centre of its run, on either side, reversed, under `coord_flip()` and in `gggrid(align=True)`. Every leaf needs a run, and a level belongs to one run; mistakes raise `SpecError` naming the leaf.
- `gggrid(axes=, axis_titles=)`, as patchwork's `plot_layout(axes=, axis_titles=)`: `"collect"` (or `"collect_x"` / `"collect_y"`) draws an axis that repeats in neighbouring plots once, on the outer plot of the run, and frees the room of the others; repeated axis titles become one, centred on the run's panels. Axes count as equal when their scales and axis theme match (use `sharex` / `sharey`) and, for x, they span the same columns (y: rows). Empty cells, nested grids and faceted plots break runs. A collecting grid keeps its panels equal (relative to `widths` / `heights`), as patchwork: the dropped room is shared by all panels. Default `"keep"`.
- `guide_axis_nested()` and `key_range_manual()`, as R legendry: brackets (or boxes) naming groups of an axis's breaks, past the axis labels or across the panel (`side="opposite"`). Groups as a mapping `{group: [levels]}` or `key_range_manual(start, end, name)`; `bracket="line" | "square" | "chevron"`, `type="box"`, `pad_discrete`, `angle`; multi-line names with `\n`. Theme elements `legendry_bracket`, `legendry_bracket_size`, `legendry_box`, `legendry_guide_spacing`. Works on every side, `coord_flip()`, facets, `gggrid(align=True)`, side band axes and the HTML zoom.
- Side band panel theme elements, as R ggside's `ggside.panel.*`: `theme(ggside_panel_border=, ggside_panel_background=)` and their `ggside_xside_panel_*` / `ggside_yside_panel_*` forms, falling back to the main `panel_border` / `panel_background`. `ggside_panel_border=element_blank()` drops the side bands' frames and keeps the main panel's.

## [0.6.2] - 2026-11-06

### Fixed

- Interactive legend: clicking a key on a dense (canvas) point layer no longer blocks the page. The key dims at once and the canvas is redrawn once on the next frame; a double-click redraws one to two times instead of three.

## [0.6.1] - 2026-11-06

### Added

- Side band axis theme elements, as R ggside's `ggside.axis.*`: `theme(ggside_axis_text, ggside_axis_ticks, ggside_axis_ticks_length, ggside_axis_minor_ticks, ggside_axis_minor_ticks_length, ggside_axis_line)`, each with `_x`, `_x_top`, `_x_bottom`, `_y`, `_y_left`, `_y_right` variants. Anything unset falls back to the main `axis_*` theme.
- `guide_axis_mark()`: label a subset of an axis's breaks with leader lines (ComplexHeatmap's `anno_mark` as an axis guide). `spread="nearest"` or `"even"`, `link_width`, `padding`. Works on discrete and continuous axes, every side, side bands, facets and `gggrid(align=True)`.
- Several side bands per axis: `side_pos=` on a ggside layer overrides `ggside()` for that layer, `side_band=` names a band (bands on one side stack outward in plot order), and `side_size=` sets a band's share of the panel. `scale_xsidey_*` / `scale_ysidex_*` take `side_pos=` and `side_band=` to set one band's axis.
- Docs: the ggrust logo, and a showcase of plots on the README and docs home page.

### Fixed

- A y side band's own x axis now reserves room for its ticks and labels (it could overflow the canvas).
- Blank side band axis labels no longer take up room.

## [0.6.0] - 2026-11-06

### Highlights

- Side panels and marginal plots: ggside-style side layers and lets-plot's `ggmarginal()`.
- Plot files: `ggsave(plot, "x.ggrs")` writes a self-contained plot file, `ggload("x.ggrs")` opens it.
- Notebook output mode is now chosen with `ggrust.display(mode)`.

### Added

- Side panels (ggside): `geom_xside*` / `geom_yside*` layers for tile, bar, col, histogram, density, point, boxplot, violin, line and text; `scale_xsidey_*` / `scale_ysidex_*`; `ggside(x_pos=, y_pos=)`; `theme(ggside_panel_scale*, ggside_panel_spacing*)`. Bands follow `coord_flip`, axis `position=` and HTML zoom.
- `ggmarginal(sides, size, layer)`, with the same spec as lets_plot.
- `.ggrs` plot files: `ggsave(plot, "x.ggrs")` stores the spec, data (Arrow IPC) and registered fonts in one zip; `ggload("x.ggrs")` restores it (polars, pandas, pyarrow or dict data). Loading runs no code and verifies sizes and hashes.
- `ggrust.display("html" | "svg" | "png")` picks what notebooks show: interactive HTML (default), static SVG without JavaScript, or PNG.
- Boxplot tooltips show each statistic as a lets-plot style side tooltip.
- Docs: a guide page and gallery section for the built-in ggplot2 extensions, with credits to the R packages.

### Changed

- `ggtb()` shows the layers menu by default; pass `layers=False` to remove it.

### Breaking

- The `LetsPlot` shim is removed. Delete `LetsPlot.setup_html()` calls; use `ggrust.display(mode)` to switch output. `display` is not in `__all__`, so `from ggrust import *` does not shadow IPython's `display()`.

### Fixed

- Side band axes: discrete labels are no longer blanked and get room to fit; thin continuous bands keep at least one label.

## [0.5.0] - 2026-11-05

### Highlights

- Built-in ggplot2 extensions: ggsci palettes, ggbeeswarm, ggnewscale, ggpointdensity, ggrastr, gghalves/ggdist, ggpubr/ggpmisc statistics, ggforce marks, dendrograms, gghighlight and patchwork extras.
- Secondary axes, axes on top/right, custom fonts, and an interactive legend in HTML output.
- Read-only spec accessors: `layer_data()`, `build()` and plot properties.

### Added

- ggsci journal palettes: `scale_color_*` / `scale_fill_*` for npg, aaas, nejm, lancet, jama, jco, d3 and igv.
- ggbeeswarm: `geom_beeswarm`, `geom_quasirandom`.
- ggnewscale: `new_scale_color()`, `new_scale_fill()`, `new_scale()`.
- ggpointdensity: `geom_pointdensity`, `stat_pointdensity`.
- ggrastr: `rasterize(layer, dpi=300)` draws a layer as an image in SVG and PDF.
- gghalves and ggdist: half geoms, slabs and intervals for raincloud plots.
- ggpubr / ggpmisc: `stat_compare_means`, `stat_cor`, `stat_poly_eq`.
- ggforce: `geom_mark_hull`, `geom_mark_ellipse`, `geom_mark_rect`.
- Dendrograms: hierarchical clustering, `geom_dendrogram`, `scale_*_dendro` and `guide_axis_dendro` (follow HTML zoom and pan).
- gghighlight: grey out non-matching rows, with direct repel labels.
- patchwork extras: tags, design layouts, insets and nested alignment.
- `geom_col`, `geom_rug` and `stat_ellipse` (ggplot2 semantics).
- Secondary axes: `sec_axis`, `dup_axis`, `derive`.
- `position=` on x and y scales draws the axis on the top/right side, or both.
- Custom fonts: `register_font`, `register_font_dir`, `registered_fonts`.
- Spec accessors: `PlotSpec.data`, `.mapping`, `.layers`, `.scales`, `.theme` and more; `with_data()`; `layer_data(i)` and `build()` return the engine's computed data as polars frames. `SupPlotsSpec` gets `figures`, `grid[i]`, `with_figure()`, `with_layout()` and more. New `PlotIndexError`.
- Theme geom colors as lets-plot: defaults follow the theme flavor, the color names `"pen"`, `"brush"`, `"paper"` work everywhere, and `theme(geom=element_geom(...))` is drawn.
- Manual legends: `manual_key=` on every geom and `layer_key(...)`.
- `gggrid` captions via `labs(caption=)`.
- `geom_boxplot(outlier_jitter=, outlier_seed=)`.
- A style scale with an explicit `na_value` gets an "NA" legend key or colorbar swatch.
- Interactive legend in HTML output: click legend keys to hide and show groups.

### Fixed

- Notebook HTML works in Google Colab (tooltips, toolbar and canvas layers were static there).
- `ipykernel` is no longer a runtime dependency.

## [0.4.0] - 2026-11-03

### Highlights

- Arrow axis lines, `oob=` on color scales, and colorbar end labels.
- HTML plots shrink to fit their container.

### Added

- `element_line(arrow=, extent=)` on axis lines: arrow heads and partial axis lines (for UMAP-style corner axes).
- `oob="censor" | "squish"` on continuous color and fill scales.
- `guide_colorbar(label_position="ends")`: min and max labels beside the ends of the bar.
- `lineheight` on `geom_text` and `geom_label`.

### Changed

- A later style scale that sets a mapper (`scale_color_manual` after a default brewer scale, for example) now replaces the earlier one, as in ggplot2. Names, breaks, labels, limits, guide and trans still merge.
- HTML plots wider than their notebook cell or docs column shrink to fit, keeping their aspect ratio. Tooltips and zoom keep working.

### Fixed

- `gggrid(guides="collect")`: each collected legend keeps its own figure's theme.
- Horizontal colorbars reserve room for end labels so they no longer run into the title.

## [0.3.0] - 2026-11-03

### Highlights

- `facet_grid`, vector PDF output, and `geom_curve`.
- `corr_plot` with a Rust correlation matrix.

### Added

- `facet_grid`: panels by columns and rows, row strips, free scales per grid column or row.
- Facet label options: `x_order`/`y_order`, `x_format`/`y_format`, `x_labwidth`/`y_labwidth` on `facet_grid`, `labwidth` on `facet_wrap`, multi-line strips.
- `geom_curve` (curvature, angle, ncp, spacer, arrow).
- Vector PDF output from `ggsave`: paths, embedded subset fonts and images.
- `corr_plot` (bistro).
- Minor ticks and `exponent_format`.
- `halo_width` / `halo_color` on `geom_text` and `geom_text_repel`.
- Mapped `hjust` / `vjust` on `geom_text` and `geom_label`.

### Changed

- Faster zoom in `ggtb` plots: points are drawn on a canvas from 2,000 points, using a sprite atlas.

### Fixed

- Box zoom keeps the aspect ratio of `coord_fixed` panels.
- `exponent_format` applies to HTML zoom labels.

## [0.2.1] - 2026-11-02

### Added

- `ggtb(layers=...)`: a toolbar menu that hides and shows layers.

### Fixed

- Zoom keeps mirrored images (for example `geom_imshow` with a flipped axis) correct.

## [0.2.0] - 2026-11-02

### Highlights

- Many new geoms and stats: density, smooth, step, contours, hexbins, summaries and more.
- Datetime scales.
- Documentation site at https://ggrust.datavil.org with live plots, guides and a gallery.

### Added

- `geom_abline`, `geom_density`, `geom_smooth` (with standard errors per model), `geom_step`, `geom_freqpoly`, `geom_count` / `stat_sum`.
- `geom_bin2d`, `geom_hex`, `geom_contour`, `geom_contourf`, `geom_density2d`, `geom_density2df`.
- `stat_summary`, `stat_summary_bin`, `stat_ecdf`.
- Datetime scales.
- Grey and discrete color scales; `scale_linetype_manual`, `scale_linetype_identity`; `expand_limits`.
- `orientation="y"` on every layer (automatic on a discrete y).
- `gggrid(sharex=, sharey=)`.
- Theme options: caption, title positions, text margins, line types.

### Changed

- Faster PNG for large point layers, lower memory for large SVG/HTML outputs, faster repel labels.
- `ggtb` toolbar restyled.

### Fixed

- Identity bars stack rows of one group; free facet scales compute stats per panel.
- Axis title `hjust`/`vjust` follow the title angle.
- Collected legends fit the grid; end x labels fit the canvas.
- Stat positions that the geom does not draw no longer train the axes.
- `gggrid` cells can shrink below 50 px.

## [0.1.0] - 2026-11-01

First public release of ggrust: a grammar-of-graphics plotting library written in Rust and used from Python, with a lets_plot-compatible API (`aes`, `ggplot`, `geom_*`, `scale_*`, `theme`, `gggrid`, `ggtb`, `+` composition). Porting lets_plot code is mostly an import swap.

### Highlights

- Rust does the stats, scales, layout and rendering; data crosses from Python once, as Arrow.
- Outputs: SVG, PNG, PDF and interactive HTML for notebooks (tooltips, zoom and pan, canvas for dense scatters).
- Faster than lets-plot on equal work in the included benchmarks.

### Added

- Geoms: point, jitter, path, line, segment (with arrows), polygon, vline, hline, text, label, text_repel, label_repel, blank, bar, histogram, boxplot, violin, sina, area_ridges, bracket, function, tile, rect, raster, imshow, errorbar, linerange, pointrange, crossbar, area, ribbon.
- Stats: count, bin, boxplot, ydensity, densityridges, sina.
- Positions: stack, dodge, fill, nudge, jitter, jitterdodge.
- Scales: continuous and discrete position scales with transforms, breaks, labels and formats; hue, manual, gradient, viridis and ColorBrewer color scales; size, alpha, shape and identity scales; `scale_size_area`.
- Coords: `coord_cartesian`, `coord_fixed`, `coord_flip`.
- Facets: `facet_wrap`. Layout: `gggrid`.
- Themes: built-in themes, `theme()` and `element_*`; legend placement, horizontal legends, `guide_legend(override_aes)`, colorbars.
- `as_discrete` with `order`, `order_by` and `label`.
- Interactive HTML: hover tooltips (general and axis), `ggtb()` zoom and pan, canvas point layers.
- `ggsave` to SVG, PNG and PDF with units and dpi; `PlotSpec.show()`.
- Typed errors in `ggrust.errors` with the spec path and close-match suggestions.
- Prebuilt abi3 wheels for Linux, macOS and Windows (including ARM64).

### Fixed (relative to lets-plot)

- Legends, axis labels and titles are fitted to the canvas instead of overflowing it.
- lets-plot issues 1504, 1452 and 1506 do not occur in ggrust.

[0.6.0]: https://github.com/datavil/ggrust/releases/tag/v0.6.0
[0.5.0]: https://github.com/datavil/ggrust/releases/tag/v0.5.0
[0.4.0]: https://github.com/datavil/ggrust/releases/tag/v0.4.0
[0.3.0]: https://github.com/datavil/ggrust/releases/tag/v0.3.0
[0.2.1]: https://github.com/datavil/ggrust/releases/tag/v0.2.1
[0.2.0]: https://github.com/datavil/ggrust/releases/tag/v0.2.0
[0.1.0]: https://github.com/datavil/ggrust/releases/tag/v0.1.0
