# API reference

ggrust follows lets-plot's Python API. Every public name is listed here by
topic; open one for its parameters and examples. The usual import is

```python
from ggrust import *
```

Missing something you use from lets-plot?
[Open a feature request](https://github.com/datavil/ggrust/issues/new){ .md-button }

## Plotting

Start a plot, map columns to aesthetics, and work with the plot object.

| Name | Description |
|---|---|
| [`ggplot()`](api/ggplot.md) | Create a new plot. |
| [`aes()`](api/aes.md) | Define aesthetic mappings. |
| [`layer()`](api/layer.md) | Create a layer from a geom name, stat, data, mapping and position. |
| [`ggsize()`](api/ggsize.md) | Set the plot size in pixels. |
| [`as_discrete()`](api/as_discrete.md) | Use a discrete scale for a mapped column, optionally ordering its values. |
| [`after_stat()`](api/after_stat.md) | Map a variable the stat computes (ggplot2 ``after_stat()``). |
| [`PlotSpec`](api/PlotSpec.md) | A plot: data, mapping, layers, scales and features. Build it with ``ggplot()``. |

## Geometries

Layers that draw data. Each one sets a geom, a default stat and a position.

| Name | Description |
|---|---|
| [`geom_point()`](api/geom_point.md) | Scatter points. |
| [`geom_count()`](api/geom_count.md) | Points at each distinct (x, y), sized by the number of rows there (``stat="sum"``). |
| [`geom_jitter()`](api/geom_jitter.md) | Points with random noise added to their positions. |
| [`geom_sina()`](api/geom_sina.md) | Sina plot: points spread within the violin outline. |
| [`geom_beeswarm()`](api/geom_beeswarm.md) | Beeswarm plot: points of each x moved sideways so that none overlap. |
| [`geom_quasirandom()`](api/geom_quasirandom.md) | Quasirandom plot: points spread sideways within the density of their x. |
| [`geom_mark_hull()`](api/geom_mark_hull.md) | Outline each group of points with a concave hull, optionally with a callout. |
| [`geom_mark_ellipse()`](api/geom_mark_ellipse.md) | Outline each group of points with its smallest enclosing ellipse. |
| [`geom_mark_rect()`](api/geom_mark_rect.md) | Outline each group of points with its bounding rectangle. |
| [`geom_pointdensity()`](api/geom_pointdensity.md) | Scatter plot with each point colored by the density of points around it. |
| [`geom_half_violin()`](api/geom_half_violin.md) | Half of a violin: the density of y on one side of each x. |
| [`geom_half_point()`](api/geom_half_point.md) | Points jittered within one half of their x slot. |
| [`geom_half_boxplot()`](api/geom_half_boxplot.md) | Half of a box-and-whisker plot, on one side of each x. |
| [`geom_line()`](api/geom_line.md) | Connect observations sorted by x. |
| [`geom_step()`](api/geom_step.md) | Connect observations sorted by x with stairs. |
| [`geom_lollipop()`](api/geom_lollipop.md) | Draw a stick from a baseline to a marker at each (x, y). |
| [`geom_spoke()`](api/geom_spoke.md) | Draw a segment of length ``radius`` at ``angle`` radians through each point. |
| [`geom_dotplot()`](api/geom_dotplot.md) | Draw a dot plot: each observation a dot, stacked within its bin. |
| [`geom_ydotplot()`](api/geom_ydotplot.md) | Draw a dot plot along y, one column of stacks at each x. |
| [`stat_bindot()`](api/stat_bindot.md) | Draw a layer from the stacks of a dot plot. |
| [`geom_qq()`](api/geom_qq.md) | Plot a sample against theoretical quantiles, one point per value. |
| [`geom_qq_line()`](api/geom_qq_line.md) | Draw the line through two quantiles of a sample, to read a Q-Q plot against. |
| [`geom_qq2()`](api/geom_qq2.md) | Plot one sample against another, one point per quantile. |
| [`geom_qq2_line()`](api/geom_qq2_line.md) | Draw the line through two quantiles of two samples, to read ``geom_qq2`` against. |
| [`geom_path()`](api/geom_path.md) | Connect observations in data order. |
| [`geom_segment()`](api/geom_segment.md) | Line segments from (x, y) to (xend, yend), optionally with arrows. |
| [`geom_curve()`](api/geom_curve.md) | Curved lines from (x, y) to (xend, yend), optionally with arrows. |
| [`geom_streamline()`](api/geom_streamline.md) | Streamlines of a vector field, with an arrowhead along each line. |
| [`geom_hline()`](api/geom_hline.md) | Horizontal reference lines at ``yintercept``. |
| [`geom_vline()`](api/geom_vline.md) | Vertical reference lines at ``xintercept``. |
| [`geom_abline()`](api/geom_abline.md) | Straight reference lines ``y = intercept + slope * x`` across the panel. |
| [`geom_function()`](api/geom_function.md) | Evaluate ``fun`` at x values and draw the result. |
| [`geom_smooth()`](api/geom_smooth.md) | Draw a fitted curve with a confidence band (``stat="smooth"``), one per group. |
| [`geom_area()`](api/geom_area.md) | Fill the area under a line, stacked by default (``position="stack"``). |
| [`geom_ribbon()`](api/geom_ribbon.md) | Fill the band between ymin and ymax along x. |
| [`geom_area_ridges()`](api/geom_area_ridges.md) | Ridgeline plot: one density curve per y level. |
| [`geom_polygon()`](api/geom_polygon.md) | Draw filled polygons, one per group. |
| [`geom_surface()`](api/geom_surface.md) | Draw a filled surface over an x-y grid, under ``coord_3d()`` (ggrust only). |
| [`geom_rect()`](api/geom_rect.md) | Rectangles from ``xmin``, ``xmax``, ``ymin``, ``ymax``. |
| [`geom_bar()`](api/geom_bar.md) | Bars, by default the count of rows per x (``stat="count"``), stacked. |
| [`geom_col()`](api/geom_col.md) | Bars with heights taken from the data (ggplot2 ``geom_col``), stacked. |
| [`geom_histogram()`](api/geom_histogram.md) | Histogram of binned x values (``stat="bin"``), stacked. |
| [`geom_freqpoly()`](api/geom_freqpoly.md) | Frequency polygon: a line through the counts of binned x (``stat="bin"``). |
| [`geom_boxplot()`](api/geom_boxplot.md) | Box and whiskers plot, dodged by group. |
| [`geom_violin()`](api/geom_violin.md) | Violin plot: mirrored kernel density per x (``stat="ydensity"``). |
| [`geom_density()`](api/geom_density.md) | Kernel density estimate of ``x``, drawn as a filled area (``stat="density"``). |
| [`geom_rug()`](api/geom_rug.md) | Rug ticks along the panel edges, one per x and/or y value (ggplot2 ``geom_rug``). |
| [`geom_errorbar()`](api/geom_errorbar.md) | Error bars from ymin to ymax at x, with fences ``width`` wide (default 0.45). |
| [`geom_crossbar()`](api/geom_crossbar.md) | Boxes from ymin to ymax at x with a midline at y, dodged by group. |
| [`geom_linerange()`](api/geom_linerange.md) | Draw a vertical line from ymin to ymax at x. |
| [`geom_pointrange()`](api/geom_pointrange.md) | Draw a vertical line from ymin to ymax at x with a point at y. |
| [`geom_tile()`](api/geom_tile.md) | Rectangles centred on (x, y), e.g. heatmap cells. |
| [`geom_bin2d()`](api/geom_bin2d.md) | Tiles counting the rows in a grid of rectangles (``stat="bin2d"``). |
| [`geom_hex()`](api/geom_hex.md) | Hexagons counting the rows in a hexagonal grid (``stat="binhex"``). |
| [`geom_contour()`](api/geom_contour.md) | Draw contour lines of z over a grid of x and y (``stat="contour"``). |
| [`geom_contourf()`](api/geom_contourf.md) | Fill the bands between the contour lines of z (``stat="contourf"``). |
| [`geom_contour_filled()`](api/geom_contour_filled.md) | Fill the bands between the contour lines of z by band (ggplot2 ``geom_contour_filled``). |
| [`geom_density2d()`](api/geom_density2d.md) | Draw contour lines of a 2D kernel density estimate (``stat="density2d"``). |
| [`geom_quantile()`](api/geom_quantile.md) | Draw quantile regression lines (``stat="quantile"``, ggplot2). |
| [`geom_density2df()`](api/geom_density2df.md) | Fill the bands of a 2D kernel density estimate (``stat="density2df"``). |
| [`geom_raster()`](api/geom_raster.md) | Equal-sized tiles on a regular grid. |
| [`geom_imshow()`](api/geom_imshow.md) | Display an image array, as lets_plot ``geom_imshow``. |
| [`annotate()`](api/annotate.md) | Add a one-off layer from constants, without a data frame. |
| [`annotation_logticks()`](api/annotation_logticks.md) | Add log tick marks along the panel edges (ggplot2 ``annotation_logticks``). |
| [`geom_text()`](api/geom_text.md) | Text labels at (x, y); the text comes from the ``label`` aesthetic. |
| [`geom_label()`](api/geom_label.md) | Text labels in a rounded box at (x, y). |
| [`geom_text_repel()`](api/geom_text_repel.md) | Text labels pushed apart so they do not overlap each other or the points. |
| [`geom_label_repel()`](api/geom_label_repel.md) | Boxed text labels pushed apart so they do not overlap each other or the points. |
| [`geom_bracket()`](api/geom_bracket.md) | Labelled brackets from ``xmin`` to ``xmax`` at height ``y`` (significance marks). |
| [`geom_blank()`](api/geom_blank.md) | Draw nothing; the mapped data still trains the scales. No tooltips (as lets_plot). |
| [`arrow()`](api/arrow.md) | Describe the arrow heads of a line (``geom_segment``, ``geom_path``). |

## Statistics

Layers named after their stat: each summarises the data, then draws it with a geom.

| Name | Description |
|---|---|
| [`stat_bin()`](api/stat_bin.md) | Count rows in bins of x (``stat="bin"``); drawn as a histogram by default. |
| [`stat_bin2d()`](api/stat_bin2d.md) | Count rows in rectangular bins of x and y (``stat="bin2d"``); drawn as tiles by default. |
| [`stat_count()`](api/stat_count.md) | Count rows per x (``stat="count"``); drawn as bars by default. |
| [`stat_density()`](api/stat_density.md) | Kernel density estimate of x (``stat="density"``); drawn as a density curve by default. |
| [`stat_density2d()`](api/stat_density2d.md) | Contours of a 2D kernel density estimate of x and y (``stat="density2d"``). |
| [`stat_quantile()`](api/stat_quantile.md) | Quantile regression lines of y on x (``stat="quantile"``, ggplot2). |
| [`stat_boxplot()`](api/stat_boxplot.md) | Quartiles, whiskers and outliers of y per x (``stat="boxplot"``), drawn as boxplots. |
| [`stat_ydensity()`](api/stat_ydensity.md) | Kernel density of y per x (``stat="ydensity"``); drawn as violins by default. |
| [`stat_sina()`](api/stat_sina.md) | Points spread within the density of y per x (``stat="sina"``), as a sina plot. |
| [`stat_smooth()`](api/stat_smooth.md) | Fitted curve with a confidence band (``stat="smooth"``); drawn as a smooth by default. |
| [`stat_identity()`](api/stat_identity.md) | Leave the data as it is (``stat="identity"``); drawn as points by default. |
| [`stat_summary()`](api/stat_summary.md) | Summarise y at every distinct x (``stat="summary"``), drawn as point ranges. |
| [`stat_summary_bin()`](api/stat_summary_bin.md) | Summarise y in bins of x (``stat="summarybin"``), drawn as point ranges. |
| [`stat_summary_2d()`](api/stat_summary_2d.md) | Summarise ``z`` over a grid of rectangles (``stat="summary2d"``), as ggplot2. |
| [`stat_unique()`](api/stat_unique.md) | Drop duplicate rows before drawing (``stat="unique"``), as ggplot2. |
| [`stat_hull()`](api/stat_hull.md) | Draw the convex hull of each group (``stat="hull"``), as plotnine. |
| [`stat_ecdf()`](api/stat_ecdf.md) | Empirical cumulative distribution of x (``stat="ecdf"``), drawn as steps. |
| [`stat_qq()`](api/stat_qq.md) | Plot theoretical quantiles of a sample (``stat="qq"``), ggplot2's ``stat_qq``. |
| [`stat_qq_line()`](api/stat_qq_line.md) | Draw the fitting line of a Q-Q plot (``stat="qq_line"``), ggplot2's ``stat_qq_line``. |
| [`stat_sum()`](api/stat_sum.md) | Rows at each distinct (x, y) (``stat="sum"``), drawn as points sized by the count. |
| [`stat_centroid()`](api/stat_centroid.md) | One row per group at the centroid of its x and y (``stat="centroid"``), as ggpp. |
| [`stat_summary_xy()`](api/stat_summary_xy.md) | One row per group at a summary of x and of y (``stat="summary_xy"``), as ggpp. |
| [`stat_pointdensity()`](api/stat_pointdensity.md) | Density of points around each point (``stat="pointdensity"``), drawn as points. |
| [`stat_streamline()`](api/stat_streamline.md) | Streamlines of a vector field (``stat="streamline"``), drawn by ``geom``. |
| [`stat_slab()`](api/stat_slab.md) | Density slab of y per x: a filled density on one side of each x. |
| [`stat_pointinterval()`](api/stat_pointinterval.md) | Point summary of y with quantile intervals around it, per x. |
| [`stat_interval()`](api/stat_interval.md) | Stacked quantile intervals of y per x, colored by level. |
| [`stat_halfeye()`](api/stat_halfeye.md) | Half-eye plot: a density slab with a point interval at its base, per x. |
| [`stat_compare_means()`](api/stat_compare_means.md) | Test the groups of y along a discrete x and label the p-values. |
| [`stat_cor()`](api/stat_cor.md) | Label the correlation of x and y and its p-value, per group. |
| [`stat_ellipse()`](api/stat_ellipse.md) | Confidence ellipse of x and y per group (ggplot2 ``stat_ellipse``). |
| [`stat_poly_eq()`](api/stat_poly_eq.md) | Label a least squares polynomial fit of y on x (equation, R²), per group. |

## Positions

Arrange marks that would overlap.

| Name | Description |
|---|---|
| [`position_dodge()`](api/position_dodge.md) | Place overlapping objects side by side. |
| [`position_dodge2()`](api/position_dodge2.md) | Place overlapping boxes or bars side by side, centred on their span (ggplot2). |
| [`position_jitter()`](api/position_jitter.md) | Add random noise to positions. |
| [`position_jitterdodge()`](api/position_jitterdodge.md) | Dodge, then jitter (points over dodged boxplots). |
| [`position_nudge()`](api/position_nudge.md) | Shift positions by a fixed offset. |
| [`position_stack()`](api/position_stack.md) | Stack overlapping objects on top of each other. |
| [`position_fill()`](api/position_fill.md) | Stack overlapping objects and normalise each stack to height 1. |
| [`position_identity()`](api/position_identity.md) | Leave objects where they are (no position adjustment). |

## Sampling

Draw fewer rows of a large layer (ggrust draws every row by default).

| Name | Description |
|---|---|
| [`sampling_random()`](api/sampling_random.md) | Keep ``n`` rows chosen at random. |
| [`sampling_systematic()`](api/sampling_systematic.md) | Keep every k-th row, the smallest k that leaves at most ``n`` rows. |
| [`sampling_fraction()`](api/sampling_fraction.md) | Keep a random share of the rows (ggrust extension, not in lets_plot). |
| [`sampling_none()`](api/sampling_none.md) | Draw every row (no sampling). |
| [`sampling_random_stratified()`](api/sampling_random_stratified.md) | lets_plot's stratified random sampling. Not applied by ggrust. |
| [`sampling_pick()`](api/sampling_pick.md) | lets_plot's first-n sampling. Not applied by ggrust. |
| [`sampling_group_random()`](api/sampling_group_random.md) | lets_plot's random sampling of whole groups. Not applied by ggrust. |
| [`sampling_group_systematic()`](api/sampling_group_systematic.md) | lets_plot's systematic sampling of whole groups. Not applied by ggrust. |
| [`sampling_vertex_vw()`](api/sampling_vertex_vw.md) | lets_plot's Visvalingam-Whyatt path simplification. Not applied by ggrust. |
| [`sampling_vertex_dp()`](api/sampling_vertex_dp.md) | lets_plot's Douglas-Peucker path simplification. Not applied by ggrust. |

## Position scales

Axes: breaks, labels, limits and transforms.

| Name | Description |
|---|---|
| [`scale_x_continuous()`](api/scale_x_continuous.md) | Continuous position scale x. |
| [`scale_y_continuous()`](api/scale_y_continuous.md) | Continuous position scale y. See ``scale_x_continuous``. |
| [`scale_z_continuous()`](api/scale_z_continuous.md) | Continuous position scale z: the vertical axis of ``coord_3d()``. |
| [`sec_axis()`](api/sec_axis.md) | Add a secondary axis on the other side of a continuous position scale (ggplot2). |
| [`dup_axis()`](api/dup_axis.md) | Copy the primary axis to the other side (ggplot2 ``dup_axis()``). |
| [`derive()`](api/derive.md) | Take a secondary axis option from the primary axis (ggplot2 ``derive()``). |
| [`scale_x_discrete()`](api/scale_x_discrete.md) | Discrete position scale x. |
| [`scale_y_discrete()`](api/scale_y_discrete.md) | Discrete position scale y. See ``scale_x_discrete``. |
| [`scale_x_discrete_reversed()`](api/scale_x_discrete_reversed.md) | ``scale_x_discrete(reverse=True)``. |
| [`scale_y_discrete_reversed()`](api/scale_y_discrete_reversed.md) | ``scale_y_discrete(reverse=True)``. |
| [`scale_x_log10()`](api/scale_x_log10.md) | Continuous position scale x with the 'log10' transformation. See ``scale_x_continuous``. |
| [`scale_y_log10()`](api/scale_y_log10.md) | Continuous position scale y with the 'log10' transformation. See ``scale_y_continuous``. |
| [`scale_x_symlog()`](api/scale_x_symlog.md) | Continuous position scale x with the 'symlog' transformation. See ``scale_x_continuous``. |
| [`scale_y_symlog()`](api/scale_y_symlog.md) | Continuous position scale y with the 'symlog' transformation. See ``scale_y_continuous``. |
| [`scale_x_sqrt()`](api/scale_x_sqrt.md) | Continuous position scale x with the 'sqrt' transformation. See ``scale_x_continuous``. |
| [`scale_y_sqrt()`](api/scale_y_sqrt.md) | Continuous position scale y with the 'sqrt' transformation. See ``scale_y_continuous``. |
| [`boxcox_trans()`](api/boxcox_trans.md) | Box-Cox power transform, as a value for ``trans``. |
| [`scale_x_log2()`](api/scale_x_log2.md) | Continuous position scale x with the 'log2' transformation. See ``scale_x_continuous``. |
| [`scale_y_log2()`](api/scale_y_log2.md) | Continuous position scale y with the 'log2' transformation. See ``scale_y_continuous``. |
| [`scale_x_reverse()`](api/scale_x_reverse.md) | Continuous position scale x with the 'reverse' transformation. See ``scale_x_continuous``. |
| [`scale_y_reverse()`](api/scale_y_reverse.md) | Continuous position scale y with the 'reverse' transformation. See ``scale_y_continuous``. |
| [`scale_x_datetime()`](api/scale_x_datetime.md) | Date-time position scale x. |
| [`scale_y_datetime()`](api/scale_y_datetime.md) | Date-time position scale y. See ``scale_x_datetime``. |
| [`scale_x_time()`](api/scale_x_time.md) | Duration position scale x. |
| [`scale_y_time()`](api/scale_y_time.md) | Duration position scale y. See ``scale_x_time``. |
| [`expand_limits()`](api/expand_limits.md) | Widen the scales of the plot so they include the given values. |
| [`lims()`](api/lims.md) | Limits for both axes: ``xlim(*x) + ylim(*y)``. |
| [`xlim()`](api/xlim.md) | Limits of the x axis, e.g. ``xlim(0, 10)``, ``xlim(None, 10)`` or ``xlim("B", "T")``. |
| [`ylim()`](api/ylim.md) | Limits of the y axis. See ``xlim``. |

## Color scales

Map data to `color` (lines, points, text) and `fill` (areas).

| Name | Description |
|---|---|
| [`scale_color_continuous()`](api/scale_color_continuous.md) | Map ``color`` continuously: a gradient unless ``scale_mapper_kind`` says otherwise. |
| [`scale_color_discrete()`](api/scale_color_discrete.md) | Map ``color`` as categories with the default discrete palette. |
| [`scale_color_grey()`](api/scale_color_grey.md) | Map ``color`` to greys, from lightness ``start`` to ``end``. |
| [`scale_color_distiller()`](api/scale_color_distiller.md) | Map continuous ``color`` with a ColorBrewer palette as a smooth gradient. |
| [`scale_color_ordinal()`](api/scale_color_ordinal.md) | Map ordered categories of ``color`` to evenly spaced viridis colors. |
| [`scale_color_gradient()`](api/scale_color_gradient.md) | Map ``color`` to a two-color gradient (in LAB space). |
| [`scale_color_gradient2()`](api/scale_color_gradient2.md) | Map ``color`` to a diverging gradient around ``midpoint`` (continuous data only). |
| [`scale_color_gradientn()`](api/scale_color_gradientn.md) | Map ``color`` to a gradient through several colors, evenly spaced over the data. |
| [`scale_color_steps()`](api/scale_color_steps.md) | Map ``color`` to binned colors of a two-color gradient (ggplot2). |
| [`scale_color_steps2()`](api/scale_color_steps2.md) | Map ``color`` to binned colors of a diverging gradient (ggplot2). |
| [`scale_color_stepsn()`](api/scale_color_stepsn.md) | Map ``color`` to binned colors of an n-color gradient (ggplot2). |
| [`scale_color_binned()`](api/scale_color_binned.md) | Map ``color`` to binned colors: ``scale_color_steps`` (ggplot2). |
| [`scale_color_cmapmpl()`](api/scale_color_cmapmpl.md) | Map ``color`` through a matplotlib color map. |
| [`scale_color_viridis()`](api/scale_color_viridis.md) | Map ``color`` with a viridis-family color map. |
| [`scale_color_brewer()`](api/scale_color_brewer.md) | Map ``color`` with a ColorBrewer palette. |
| [`scale_color_hue()`](api/scale_color_hue.md) | Qualitative colors evenly spaced around the HCL color wheel, for ``color``. |
| [`scale_color_manual()`](api/scale_color_manual.md) | Your own colors for ``color``. |
| [`scale_color_identity()`](api/scale_color_identity.md) | Use the data as colors (names or hex codes); no legend by default. |
| [`scale_fill_continuous()`](api/scale_fill_continuous.md) | Map ``fill`` continuously. See ``scale_color_continuous``. |
| [`scale_fill_discrete()`](api/scale_fill_discrete.md) | Map ``fill`` as categories with the default discrete palette. |
| [`scale_fill_grey()`](api/scale_fill_grey.md) | Map ``fill`` to greys, from lightness ``start`` to ``end``. |
| [`scale_fill_distiller()`](api/scale_fill_distiller.md) | Map continuous ``fill`` with a ColorBrewer palette as a smooth gradient. |
| [`scale_fill_ordinal()`](api/scale_fill_ordinal.md) | Map ordered categories of ``fill`` to evenly spaced viridis colors. |
| [`scale_fill_gradient()`](api/scale_fill_gradient.md) | Map ``fill`` to a two-color gradient. See ``scale_color_gradient``. |
| [`scale_fill_gradient2()`](api/scale_fill_gradient2.md) | Map ``fill`` to a diverging gradient. See ``scale_color_gradient2``. |
| [`scale_fill_gradientn()`](api/scale_fill_gradientn.md) | Map ``fill`` to a gradient through several colors. See ``scale_color_gradientn``. |
| [`scale_fill_steps()`](api/scale_fill_steps.md) | Map ``fill`` to binned colors of a two-color gradient (ggplot2). |
| [`scale_fill_steps2()`](api/scale_fill_steps2.md) | Map ``fill`` to binned colors of a diverging gradient (ggplot2). |
| [`scale_fill_stepsn()`](api/scale_fill_stepsn.md) | Map ``fill`` to binned colors of an n-color gradient (ggplot2). |
| [`scale_fill_binned()`](api/scale_fill_binned.md) | Map ``fill`` to binned colors: ``scale_fill_steps`` (ggplot2). |
| [`scale_fill_cmapmpl()`](api/scale_fill_cmapmpl.md) | Map ``fill`` through a matplotlib color map. |
| [`scale_fill_viridis()`](api/scale_fill_viridis.md) | Map ``fill`` with a viridis-family color map. See ``scale_color_viridis``. |
| [`scale_fill_brewer()`](api/scale_fill_brewer.md) | Map ``fill`` with a ColorBrewer palette. See ``scale_color_brewer``. |
| [`scale_fill_hue()`](api/scale_fill_hue.md) | Qualitative colors evenly spaced around the HCL color wheel, for ``fill``. |
| [`scale_fill_manual()`](api/scale_fill_manual.md) | Your own colors for ``fill``. |
| [`scale_fill_identity()`](api/scale_fill_identity.md) | Use the data as fill colors; no legend by default. |

## Scales for any aesthetic

Name the aesthetic, or a list of them (`['color', 'fill']`) to set one scale on all.

| Name | Description |
|---|---|
| [`scale_manual()`](api/scale_manual.md) | Your own values for any aesthetic, or for several at once. |
| [`scale_continuous()`](api/scale_continuous.md) | Map any aesthetic, or several at once, continuously. |
| [`scale_discrete()`](api/scale_discrete.md) | Map any aesthetic, or several at once, as categories. |
| [`scale_gradient()`](api/scale_gradient.md) | Map color aesthetics to a two-color gradient (in LAB space). |
| [`scale_gradient2()`](api/scale_gradient2.md) | Map color aesthetics to a diverging three-color gradient. |
| [`scale_gradientn()`](api/scale_gradientn.md) | Map color aesthetics to an n-color gradient. |
| [`scale_hue()`](api/scale_hue.md) | Qualitative colors around the HCL color wheel, for color aesthetics. |
| [`scale_grey()`](api/scale_grey.md) | Greys from lightness ``start`` to ``end``, for color aesthetics. |
| [`scale_brewer()`](api/scale_brewer.md) | Map color aesthetics with a ColorBrewer palette. |
| [`scale_viridis()`](api/scale_viridis.md) | Map color aesthetics with a viridis-family color map. |

## Journal palettes (ggsci)

Discrete palettes from the R package ggsci (not in lets-plot).

| Name | Description |
|---|---|
| [`scale_color_npg()`](api/scale_color_npg.md) | Map ``color`` with the ggsci ``npg`` palette, from Nature Reviews Cancer (NPG). |
| [`scale_fill_npg()`](api/scale_fill_npg.md) | Map ``fill`` with the ggsci ``npg`` palette. See ``scale_color_npg``. |
| [`scale_color_aaas()`](api/scale_color_aaas.md) | Map ``color`` with the ggsci ``aaas`` palette, from Science (AAAS). |
| [`scale_fill_aaas()`](api/scale_fill_aaas.md) | Map ``fill`` with the ggsci ``aaas`` palette. See ``scale_color_aaas``. |
| [`scale_color_nejm()`](api/scale_color_nejm.md) | Map ``color`` with the ggsci ``nejm`` palette, from NEJM. |
| [`scale_fill_nejm()`](api/scale_fill_nejm.md) | Map ``fill`` with the ggsci ``nejm`` palette. See ``scale_color_nejm``. |
| [`scale_color_lancet()`](api/scale_color_lancet.md) | Map ``color`` with the ggsci ``lancet`` palette, from Lancet Oncology. |
| [`scale_fill_lancet()`](api/scale_fill_lancet.md) | Map ``fill`` with the ggsci ``lancet`` palette. See ``scale_color_lancet``. |
| [`scale_color_jama()`](api/scale_color_jama.md) | Map ``color`` with the ggsci ``jama`` palette, from JAMA. |
| [`scale_fill_jama()`](api/scale_fill_jama.md) | Map ``fill`` with the ggsci ``jama`` palette. See ``scale_color_jama``. |
| [`scale_color_jco()`](api/scale_color_jco.md) | Map ``color`` with the ggsci ``jco`` palette, from the Journal of Clinical Oncology (JCO). |
| [`scale_fill_jco()`](api/scale_fill_jco.md) | Map ``fill`` with the ggsci ``jco`` palette. See ``scale_color_jco``. |
| [`scale_color_d3()`](api/scale_color_d3.md) | Map ``color`` with the ggsci ``d3`` palette, from D3.js. |
| [`scale_fill_d3()`](api/scale_fill_d3.md) | Map ``fill`` with the ggsci ``d3`` palette. See ``scale_color_d3``. |
| [`scale_color_igv()`](api/scale_color_igv.md) | Map ``color`` with the ggsci ``igv`` palette, from the IGV genome browser. |
| [`scale_fill_igv()`](api/scale_fill_igv.md) | Map ``fill`` with the ggsci ``igv`` palette. See ``scale_color_igv``. |

## More scales per aesthetic (ggnewscale)

Start a new scale with its own legend (R package ggnewscale, not in lets-plot).

| Name | Description |
|---|---|
| [`new_scale_color()`](api/new_scale_color.md) | Start a new color scale (ggnewscale): ``new_scale("color")``. |
| [`new_scale_colour()`](api/new_scale_colour.md) | Start a new color scale (ggnewscale): alias of ``new_scale_color()``. |
| [`new_scale_fill()`](api/new_scale_fill.md) | Start a new fill scale (ggnewscale): ``new_scale("fill")``. |
| [`new_scale()`](api/new_scale.md) | Start a new scale for an aesthetic (R package ggnewscale). |

## Highlighting (gghighlight)

Grey out the rows or series that do not match a predicate, with direct labels (R package gghighlight, not in lets-plot).

| Name | Description |
|---|---|
| [`gghighlight()`](api/gghighlight.md) | Highlight the rows or series that match predicates (R package gghighlight). |

## Insets (patchwork)

Draw a plot or grid inside another plot's area (R package patchwork, not in lets-plot). `gggrid(design=, tag_levels=, tag_level=)` add patchwork's layouts and automatic tags to grids.

| Name | Description |
|---|---|
| [`inset_element()`](api/inset_element.md) | Draw a plot (or grid) inside the area of the plot it is added to. |

## Rasterized layers (ggrastr)

Draw a dense layer as a bitmap inside SVG and PDF output (R package ggrastr, not in lets-plot).

| Name | Description |
|---|---|
| [`rasterize()`](api/rasterize.md) | Draw a layer as a bitmap in SVG and PDF output (R package ggrastr). |

## Clustering and dendrograms (hclust, ggdendro, legendry)

Cluster rows or columns as R's hclust, draw the tree, and order a discrete axis by it with the tree beside the axis (not in lets-plot).

| Name | Description |
|---|---|
| [`hclust()`](api/hclust.md) | Cluster the rows (or columns) of a table hierarchically, as R's ``hclust``. |
| [`ClusterTree`](api/ClusterTree.md) | A clustering tree in R's ``hclust`` layout. |
| [`geom_dendrogram()`](api/geom_dendrogram.md) | Draw a clustering tree as line segments (R ggdendro). |
| [`scale_x_dendro()`](api/scale_x_dendro.md) | Discrete x scale ordered by a clustering tree, drawn below the axis (legendry). |
| [`scale_y_dendro()`](api/scale_y_dendro.md) | Discrete y scale ordered by a clustering tree, drawn left of the axis (legendry). |
| [`guide_axis_dendro()`](api/guide_axis_dendro.md) | Axis guide that draws the scale's tree beside the axis (R legendry). |

## Streamlines (matplotlib streamplot)

Integrate the streamlines and arrows of a gridded vector field as matplotlib's ``streamplot``, to draw with ``geom_path`` and ``geom_segment`` (not in lets-plot; ``geom_streamline`` draws them as one layer).

| Name | Description |
|---|---|
| [`streamplot()`](api/streamplot.md) | Streamlines of the vector field ``(u, v)``, as ``matplotlib.pyplot.streamplot``. |
| [`StreamplotSet`](api/StreamplotSet.md) | Streamlines and their arrows, as matplotlib's ``StreamplotSet``. |

## Velocity grids (scVelo compute_velocity_on_grid)

Average cell velocities onto a regular grid as scVelo's ``compute_velocity_on_grid``, ready for ``streamplot`` (not in lets-plot).

| Name | Description |
|---|---|
| [`velocity_grid()`](api/velocity_grid.md) | Average cell velocities onto a regular grid, as scVelo's ``compute_velocity_on_grid``. |
| [`VelocityGrid`](api/VelocityGrid.md) | Velocities on a regular grid, laid out for ``streamplot(x, y, u, v)``. |

## Density outlines (scipy gaussian_kde + skimage find_contours)

Outline a 2D point cloud where its kernel density falls to a fraction of the peak, as scipy's ``gaussian_kde`` with scikit-image's ``find_contours``, to draw with ``geom_path`` (not in lets-plot).

| Name | Description |
|---|---|
| [`density_outline()`](api/density_outline.md) | Outline a 2D point cloud where its kernel density falls to ``level`` of the peak. |

## Kernel densities (scipy gaussian_kde)

Evaluate a 1D Gaussian kernel density at chosen points, as scipy's ``gaussian_kde``, to build density shapes outside a layer (not in lets-plot).

| Name | Description |
|---|---|
| [`kernel_density()`](api/kernel_density.md) | Evaluate the Gaussian kernel density of ``values`` at the points ``at``. |

## Side panels and marginal plots (ggside, ggmarginal)

Layers in bands beside the panel that share one of its axes, such as heatmap annotation tracks and marginal densities (R package ggside; ``ggmarginal()`` as lets-plot).

| Name | Description |
|---|---|
| [`geom_xsidetile()`](api/geom_xsidetile.md) | Draw tiles in the x side panel (R package ggside). |
| [`geom_ysidetile()`](api/geom_ysidetile.md) | Draw tiles in the y side panel (R package ggside). |
| [`geom_xsidebar()`](api/geom_xsidebar.md) | Draw bars of counts in the x side panel (R package ggside). |
| [`geom_ysidebar()`](api/geom_ysidebar.md) | Draw bars of counts in the y side panel (R package ggside). |
| [`geom_xsidecol()`](api/geom_xsidecol.md) | Draw bars of given heights in the x side panel (R package ggside). |
| [`geom_ysidecol()`](api/geom_ysidecol.md) | Draw bars of given heights in the y side panel (R package ggside). |
| [`geom_xsidehistogram()`](api/geom_xsidehistogram.md) | Draw a histogram in the x side panel (R package ggside). |
| [`geom_ysidehistogram()`](api/geom_ysidehistogram.md) | Draw a histogram in the y side panel (R package ggside). |
| [`geom_xsidedensity()`](api/geom_xsidedensity.md) | Draw a density curve in the x side panel (R package ggside). |
| [`geom_ysidedensity()`](api/geom_ysidedensity.md) | Draw a density curve in the y side panel (R package ggside). |
| [`geom_xsidepoint()`](api/geom_xsidepoint.md) | Draw points in the x side panel (R package ggside). |
| [`geom_ysidepoint()`](api/geom_ysidepoint.md) | Draw points in the y side panel (R package ggside). |
| [`geom_xsideboxplot()`](api/geom_xsideboxplot.md) | Draw boxplots in the x side panel (R package ggside). |
| [`geom_ysideboxplot()`](api/geom_ysideboxplot.md) | Draw boxplots in the y side panel (R package ggside). |
| [`geom_xsideviolin()`](api/geom_xsideviolin.md) | Draw violins in the x side panel (R package ggside). |
| [`geom_ysideviolin()`](api/geom_ysideviolin.md) | Draw violins in the y side panel (R package ggside). |
| [`geom_xsideline()`](api/geom_xsideline.md) | Draw a line in the x side panel (R package ggside). |
| [`geom_ysideline()`](api/geom_ysideline.md) | Draw a line in the y side panel (R package ggside). |
| [`geom_xsidetext()`](api/geom_xsidetext.md) | Draw text labels in the x side panel (R package ggside). |
| [`geom_ysidetext()`](api/geom_ysidetext.md) | Draw text labels in the y side panel (R package ggside). |
| [`scale_xsidey_continuous()`](api/scale_xsidey_continuous.md) | Set the x side panel's own continuous y axis (R package ggside). |
| [`scale_xsidey_discrete()`](api/scale_xsidey_discrete.md) | Set the x side panel's own discrete y axis, e.g. the track order (R package ggside). |
| [`scale_ysidex_continuous()`](api/scale_ysidex_continuous.md) | Set the y side panel's own continuous x axis (R package ggside). |
| [`scale_ysidex_discrete()`](api/scale_ysidex_discrete.md) | Set the y side panel's own discrete x axis, e.g. the track order (R package ggside). |
| [`ggside()`](api/ggside.md) | Place the side panels (R package ggside). |
| [`ggmarginal()`](api/ggmarginal.md) | Add copies of a layer in margins of the panel: a marginal plot (as lets-plot). |

## Size, shape and alpha scales

Map data to point size, point shape, transparency and line widths.

| Name | Description |
|---|---|
| [`scale_size()`](api/scale_size.md) | Sizes for ``size``: data mapped linearly onto ``range``. |
| [`scale_size_binned()`](api/scale_size_binned.md) | Map ``size`` to binned sizes (ggplot2). |
| [`scale_size_area()`](api/scale_size_area.md) | Sizes whose circle area is proportional to the data. |
| [`scale_size_radius()`](api/scale_size_radius.md) | Map continuous ``size`` linearly onto the point radius. |
| [`scale_size_ordinal()`](api/scale_size_ordinal.md) | Map ordered categories of ``size`` to evenly spaced sizes. |
| [`scale_alpha_ordinal()`](api/scale_alpha_ordinal.md) | Map ordered categories of ``alpha`` to evenly spaced opacities. |
| [`scale_size_manual()`](api/scale_size_manual.md) | Your own sizes for ``size``, matched in order with the categories (or a dict). |
| [`scale_size_identity()`](api/scale_size_identity.md) | Use the data as sizes; no legend by default. |
| [`scale_shape()`](api/scale_shape.md) | Point shapes for categories. |
| [`scale_shape_manual()`](api/scale_shape_manual.md) | Your own point shapes (lets-plot codes 0..25) for ``shape``. |
| [`scale_shape_identity()`](api/scale_shape_identity.md) | Use the data as shape codes (0..25); no legend by default. |
| [`scale_linetype()`](api/scale_linetype.md) | Default line types for categories mapped to ``linetype``. |
| [`scale_linetype_manual()`](api/scale_linetype_manual.md) | Your own line types for ``linetype``. |
| [`scale_linetype_identity()`](api/scale_linetype_identity.md) | Use the data as line types (codes 0..6 or names such as ``"dashed"``); no legend by default. |
| [`scale_alpha()`](api/scale_alpha.md) | Transparency for ``alpha``: data mapped linearly onto ``range``. |
| [`scale_alpha_binned()`](api/scale_alpha_binned.md) | Map ``alpha`` to binned transparency (ggplot2). |
| [`scale_alpha_manual()`](api/scale_alpha_manual.md) | Your own alphas for ``alpha``, matched in order with the categories (or a dict). |
| [`scale_alpha_identity()`](api/scale_alpha_identity.md) | Use the data as alphas; no legend by default. |
| [`scale_linewidth()`](api/scale_linewidth.md) | Line widths for ``linewidth``: data mapped linearly onto ``range``. |
| [`scale_linewidth_identity()`](api/scale_linewidth_identity.md) | Use the data as line widths; no legend by default. |
| [`scale_stroke()`](api/scale_stroke.md) | Outline widths for ``stroke``: data mapped linearly onto ``range``. |
| [`scale_stroke_identity()`](api/scale_stroke_identity.md) | Use the data as point outline widths; no legend by default. |

## Coordinates and facets

Coordinate systems, and small multiples.

| Name | Description |
|---|---|
| [`coord_cartesian()`](api/coord_cartesian.md) | Cartesian coordinates; limits zoom without dropping data. |
| [`coord_fixed()`](api/coord_fixed.md) | Cartesian coordinates with a fixed aspect ratio. |
| [`coord_flip()`](api/coord_flip.md) | Swap the x and y axes. |
| [`coord_polar()`](api/coord_polar.md) | Bend the panel into a circle, for pie, donut, rose and radar charts. |
| [`coord_trans()`](api/coord_trans.md) | Cartesian coordinates with transformed axes: ``coord_transform`` (ggplot2's older name). |
| [`coord_transform()`](api/coord_transform.md) | Cartesian coordinates with transformed axes (ggplot2). |
| [`coord_3d()`](api/coord_3d.md) | Show x, y and z in a static 3D view (ggrust only: lets-plot and ggplot2 have no 3D). |
| [`facet_wrap()`](api/facet_wrap.md) | Wrap one panel per level of ``facets`` into rows. |
| [`facet_grid()`](api/facet_grid.md) | Lay panels out in a grid: columns by ``x``, rows by ``y``. |
| [`facet_null()`](api/facet_null.md) | Remove faceting: one panel (ggplot2's default facet). |
| [`label_value`](api/label_value.md) | Label facet strips with the value alone (the default). |
| [`label_both`](api/label_both.md) | Label facet strips with ``variable: value``. |
| [`label_context`](api/label_context.md) | Label facet strips with the value, or ``variable: value`` for several variables. |
| [`as_labeller()`](api/as_labeller.md) | Build a labeller from a lookup table or a function (ggplot2 ``as_labeller``). |
| [`Labeller`](api/Labeller.md) | A facet strip labeller. |

## Labels and guides

Titles, axis labels and legends.

| Name | Description |
|---|---|
| [`labs()`](api/labs.md) | Set the title, subtitle, caption, tag and axis or legend titles. |
| [`ggtitle()`](api/ggtitle.md) | Set the plot title and subtitle. |
| [`xlab()`](api/xlab.md) | Set the x axis title. |
| [`ylab()`](api/ylab.md) | Set the y axis title. |
| [`guides()`](api/guides.md) | Set guides (legends, colorbars) per aesthetic. |
| [`guide_legend()`](api/guide_legend.md) | Show a key legend for an aesthetic. |
| [`guide_colorbar()`](api/guide_colorbar.md) | Show a colorbar for a continuous color or fill scale. |
| [`guide_bins()`](api/guide_bins.md) | Show a binned scale's keys touching, labelled on their edges (ggplot2). |
| [`guide_axis()`](api/guide_axis.md) | Axis guide for a position scale, with axis capping (ggplot2 ``guide_axis``). |
| [`guide_axis_mark()`](api/guide_axis_mark.md) | Axis guide that labels the scale's breaks with leader lines (ComplexHeatmap ``anno_mark``). |
| [`guide_axis_nested()`](api/guide_axis_nested.md) | Axis guide with brackets naming groups of breaks (R legendry ``guide_axis_nested``). |
| [`key_range_manual()`](api/key_range_manual.md) | Give the groups of ``guide_axis_nested()`` by their first and last break (legendry). |
| [`layer_key()`](api/layer_key.md) | Describe a layer's entry in a manual legend (``manual_key=`` of any geom). |

## Themes

Complete themes, and `theme()` to change single elements.

| Name | Description |
|---|---|
| [`theme()`](api/theme.md) | Set theme options: element_*() values, numbers or names per theme element. |
| [`theme_grey()`](api/theme_grey.md) | Use the grey background theme. |
| [`theme_gray()`](api/theme_gray.md) | Use the grey background theme (alias of ``theme_grey``). |
| [`theme_classic()`](api/theme_classic.md) | Use the classic theme: axis lines, no grid. |
| [`theme_minimal()`](api/theme_minimal.md) | Use the minimal theme. |
| [`theme_minimal2()`](api/theme_minimal2.md) | Use lets-plot's default theme. |
| [`theme_bw()`](api/theme_bw.md) | Use the black-and-white theme. |
| [`theme_light()`](api/theme_light.md) | Use the light theme. |
| [`theme_dark()`](api/theme_dark.md) | Use the dark theme: a mid-grey panel with dark grid lines. |
| [`theme_linedraw()`](api/theme_linedraw.md) | Use the line-drawing theme: a black frame and thin black grid lines. |
| [`theme_void()`](api/theme_void.md) | Use the classic theme without lines and axes. |
| [`theme_none()`](api/theme_none.md) | Use the unstyled theme. |
| [`theme_defaults()`](api/theme_defaults.md) | Return what a named theme sets, in ``theme()`` vocabulary. |

## Theme elements and flavors

Building blocks for `theme()`, and color flavors for any theme.

| Name | Description |
|---|---|
| [`element_text()`](api/element_text.md) | Style text. |
| [`element_line()`](api/element_line.md) | Style a line (axis lines, ticks, grid). |
| [`element_rect()`](api/element_rect.md) | Style a rectangle (backgrounds, borders). |
| [`element_blank()`](api/element_blank.md) | Hide a theme element. |
| [`element_geom()`](api/element_geom.md) | Set default geom colors. |
| [`element_markdown()`](api/element_markdown.md) | Style text written in Markdown (drawn as plain text by ggrust). |
| [`margin()`](api/margin.md) | Return margins ``[top, right, bottom, left]`` (deprecated in lets_plot). |
| [`flavor_standard()`](api/flavor_standard.md) | Use the standard color flavor of the theme. |
| [`flavor_darcula()`](api/flavor_darcula.md) | Use the Darcula color flavor. |
| [`flavor_solarized_light()`](api/flavor_solarized_light.md) | Use the Solarized light color flavor. |
| [`flavor_solarized_dark()`](api/flavor_solarized_dark.md) | Use the Solarized dark color flavor. |
| [`flavor_high_contrast_light()`](api/flavor_high_contrast_light.md) | Use the high-contrast light color flavor. |
| [`flavor_high_contrast_dark()`](api/flavor_high_contrast_dark.md) | Use the high-contrast dark color flavor. |

## Grids and interactivity

Combine plots into one figure, and configure tooltips and the zoom/pan toolbar.

| Name | Description |
|---|---|
| [`gggrid()`](api/gggrid.md) | Combine plots into a grid. |
| [`SupPlotsSpec`](api/SupPlotsSpec.md) | Several plots in one figure. Build it with ``gggrid()``. |
| [`GGBunch`](api/GGBunch.md) | A free-layout bunch of plots. Not supported: use insets or ``gggrid`` instead. |
| [`layer_tooltips()`](api/layer_tooltips.md) | Configure hover tooltips. |
| [`ggtb()`](api/ggtb.md) | Add the zoom/pan toolbar to an HTML plot. |

## Export and setup

Write files, and choose how plots show in notebooks.

| Name | Description |
|---|---|
| [`ggsave()`](api/ggsave.md) | Save a plot to ``filename`` (inside ``path`` if given); the extension picks the format. |
| [`ggload()`](api/ggload.md) | Open a plot saved with ``ggsave(plot, "name.ggrs")``. |
| [`display()`](api/display.md) | Choose how plots show in notebooks, for the rest of the session. |
| [`engine_version`](api/engine_version.md) | Return the version of the Rust engine. |

## Fonts

Register font files (Arial, a brand font, CJK fonts) for `element_text(family=...)`.

| Name | Description |
|---|---|
| [`register_font()`](api/register_font.md) | Register a font file so plots can draw with it. |
| [`register_font_dir()`](api/register_font_dir.md) | Register every font file under a directory, searched recursively. |
| [`registered_fonts()`](api/registered_fonts.md) | Return the registered font families and their faces. |

## Example datasets

Small everyday datasets as polars DataFrames: `from ggrust.data import penguins`.

| Name | Description |
|---|---|
| [`data`](api/data.md) | Small example datasets, as polars DataFrames: ``from ggrust.data import penguins``. |

## Aliases

Other spellings of the functions above, as in ggplot2 and plotnine (not in lets-plot). Each is the same function as its target.

| Name | Description |
|---|---|
| [`coord_equal()`](api/coord_equal.md) | Alias of [`coord_fixed()`](coord_fixed.md). |
| [`geom_bin_2d()`](api/geom_bin_2d.md) | Alias of [`geom_bin2d()`](geom_bin2d.md). |
| [`geom_density_2d()`](api/geom_density_2d.md) | Alias of [`geom_density2d()`](geom_density2d.md). |
| [`geom_density_2d_filled()`](api/geom_density_2d_filled.md) | Alias of [`geom_density2df()`](geom_density2df.md). |
| [`guide_colourbar()`](api/guide_colourbar.md) | Alias of [`guide_colorbar()`](guide_colorbar.md). |
| [`scale_alpha_continuous()`](api/scale_alpha_continuous.md) | Alias of [`scale_alpha()`](scale_alpha.md). |
| [`scale_alpha_discrete()`](api/scale_alpha_discrete.md) | Alias of [`scale_alpha_ordinal()`](scale_alpha_ordinal.md). |
| [`scale_color_gray()`](api/scale_color_gray.md) | Alias of [`scale_color_grey()`](scale_color_grey.md). |
| [`scale_colour_aaas()`](api/scale_colour_aaas.md) | Alias of [`scale_color_aaas()`](scale_color_aaas.md). |
| [`scale_colour_binned()`](api/scale_colour_binned.md) | Alias of [`scale_color_binned()`](scale_color_binned.md). |
| [`scale_colour_brewer()`](api/scale_colour_brewer.md) | Alias of [`scale_color_brewer()`](scale_color_brewer.md). |
| [`scale_colour_cmapmpl()`](api/scale_colour_cmapmpl.md) | Alias of [`scale_color_cmapmpl()`](scale_color_cmapmpl.md). |
| [`scale_colour_continuous()`](api/scale_colour_continuous.md) | Alias of [`scale_color_continuous()`](scale_color_continuous.md). |
| [`scale_colour_d3()`](api/scale_colour_d3.md) | Alias of [`scale_color_d3()`](scale_color_d3.md). |
| [`scale_colour_discrete()`](api/scale_colour_discrete.md) | Alias of [`scale_color_discrete()`](scale_color_discrete.md). |
| [`scale_colour_distiller()`](api/scale_colour_distiller.md) | Alias of [`scale_color_distiller()`](scale_color_distiller.md). |
| [`scale_colour_gradient()`](api/scale_colour_gradient.md) | Alias of [`scale_color_gradient()`](scale_color_gradient.md). |
| [`scale_colour_gradient2()`](api/scale_colour_gradient2.md) | Alias of [`scale_color_gradient2()`](scale_color_gradient2.md). |
| [`scale_colour_gradientn()`](api/scale_colour_gradientn.md) | Alias of [`scale_color_gradientn()`](scale_color_gradientn.md). |
| [`scale_colour_gray()`](api/scale_colour_gray.md) | Alias of [`scale_color_grey()`](scale_color_grey.md). |
| [`scale_colour_grey()`](api/scale_colour_grey.md) | Alias of [`scale_color_grey()`](scale_color_grey.md). |
| [`scale_colour_hue()`](api/scale_colour_hue.md) | Alias of [`scale_color_hue()`](scale_color_hue.md). |
| [`scale_colour_identity()`](api/scale_colour_identity.md) | Alias of [`scale_color_identity()`](scale_color_identity.md). |
| [`scale_colour_igv()`](api/scale_colour_igv.md) | Alias of [`scale_color_igv()`](scale_color_igv.md). |
| [`scale_colour_jama()`](api/scale_colour_jama.md) | Alias of [`scale_color_jama()`](scale_color_jama.md). |
| [`scale_colour_jco()`](api/scale_colour_jco.md) | Alias of [`scale_color_jco()`](scale_color_jco.md). |
| [`scale_colour_lancet()`](api/scale_colour_lancet.md) | Alias of [`scale_color_lancet()`](scale_color_lancet.md). |
| [`scale_colour_manual()`](api/scale_colour_manual.md) | Alias of [`scale_color_manual()`](scale_color_manual.md). |
| [`scale_colour_nejm()`](api/scale_colour_nejm.md) | Alias of [`scale_color_nejm()`](scale_color_nejm.md). |
| [`scale_colour_npg()`](api/scale_colour_npg.md) | Alias of [`scale_color_npg()`](scale_color_npg.md). |
| [`scale_colour_ordinal()`](api/scale_colour_ordinal.md) | Alias of [`scale_color_ordinal()`](scale_color_ordinal.md). |
| [`scale_colour_steps()`](api/scale_colour_steps.md) | Alias of [`scale_color_steps()`](scale_color_steps.md). |
| [`scale_colour_steps2()`](api/scale_colour_steps2.md) | Alias of [`scale_color_steps2()`](scale_color_steps2.md). |
| [`scale_colour_stepsn()`](api/scale_colour_stepsn.md) | Alias of [`scale_color_stepsn()`](scale_color_stepsn.md). |
| [`scale_colour_viridis()`](api/scale_colour_viridis.md) | Alias of [`scale_color_viridis()`](scale_color_viridis.md). |
| [`scale_fill_gray()`](api/scale_fill_gray.md) | Alias of [`scale_fill_grey()`](scale_fill_grey.md). |
| [`scale_linetype_discrete()`](api/scale_linetype_discrete.md) | Alias of [`scale_linetype()`](scale_linetype.md). |
| [`scale_linewidth_continuous()`](api/scale_linewidth_continuous.md) | Alias of [`scale_linewidth()`](scale_linewidth.md). |
| [`scale_shape_discrete()`](api/scale_shape_discrete.md) | Alias of [`scale_shape()`](scale_shape.md). |
| [`scale_size_continuous()`](api/scale_size_continuous.md) | Alias of [`scale_size()`](scale_size.md). |
| [`scale_size_discrete()`](api/scale_size_discrete.md) | Alias of [`scale_size_ordinal()`](scale_size_ordinal.md). |
| [`scale_stroke_continuous()`](api/scale_stroke_continuous.md) | Alias of [`scale_stroke()`](scale_stroke.md). |
| [`scale_x_date()`](api/scale_x_date.md) | Alias of [`scale_x_datetime()`](scale_x_datetime.md). |
| [`scale_y_date()`](api/scale_y_date.md) | Alias of [`scale_y_datetime()`](scale_y_datetime.md). |
| [`stat_bin_2d()`](api/stat_bin_2d.md) | Alias of [`stat_bin2d()`](stat_bin2d.md). |
| [`stat_density_2d()`](api/stat_density_2d.md) | Alias of [`stat_density2d()`](stat_density2d.md). |
| [`stat_function()`](api/stat_function.md) | Alias of [`geom_function()`](geom_function.md). |

## Errors and warnings

Every error ggrust raises is a `GgrustError` and the builtin that matches the problem,
so `except ValueError` (as with lets_plot) keeps working. Errors from the engine carry
where they happened: `SpecError.path` is the spec path of the bad value
(`"layers[0].size"`), and `ColumnNotFoundError` lists the available columns and
suggests a close name.

```python
from ggrust.errors import ColumnNotFoundError

try:
    plot.to_svg()
except ColumnNotFoundError as error:
    print(error.column, error.path, error.available)
```

| Name | Description |
|---|---|
| [`GgrustWarning`](api/GgrustWarning.md) | Something in the plot was ignored or adjusted; the message says what and where. |
| [`GgrustError`](api/GgrustError.md) | Base class of every ggrust error. |
| [`SpecError`](api/SpecError.md) | Raised when the plot specification holds an invalid value. |
| [`UnsupportedValueError`](api/UnsupportedValueError.md) | Raised when a spec holds a value of a type the engine cannot receive. |
| [`DataError`](api/DataError.md) | Raised when the plot data cannot be read (malformed or inconsistent columns). |
| [`ColumnNotFoundError`](api/ColumnNotFoundError.md) | Raised when a mapping refers to a column that is not in the data. |
| [`UnsupportedColumnTypeError`](api/UnsupportedColumnTypeError.md) | Raised when a mapped column has a type ggrust cannot plot. |
| [`UnsupportedDataError`](api/UnsupportedDataError.md) | Raised when the data object is not a kind ggrust reads (e.g. a Series). |
| [`RenderError`](api/RenderError.md) | Raised when drawing the output fails, e.g. for an image too large to allocate. |
| [`ExportError`](api/ExportError.md) | Raised when export options are invalid (file name, unit, size, dpi, scale). |
| [`PlotFileError`](api/PlotFileError.md) | Raised when a ``.ggrs`` plot file cannot be opened. |
| [`PlotIndexError`](api/PlotIndexError.md) | Raised when a layer or grid cell index is out of range or malformed. |
| [`GuideNotFoundError`](api/GuideNotFoundError.md) | Raised when a plot shows no guide for the asked aesthetic. |
| [`FontNotFoundError`](api/FontNotFoundError.md) | Raised when a font file or directory to register does not exist. |
| [`FontError`](api/FontError.md) | Raised when a font cannot be registered or embedded. |
| [`DatasetNotFoundError`](api/DatasetNotFoundError.md) | Raised when ``ggrust.data`` has no dataset of the asked name. |
