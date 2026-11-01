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
| [`PlotSpec`](api/PlotSpec.md) | A plot: data, mapping, layers, scales and features. Build it with ``ggplot()``. |

## Geometries

Layers that draw data. Each one sets a geom, a default stat and a position.

| Name | Description |
|---|---|
| [`geom_point()`](api/geom_point.md) | Scatter points. |
| [`geom_jitter()`](api/geom_jitter.md) | Points with random noise added to their positions. Not drawn yet. |
| [`geom_sina()`](api/geom_sina.md) | Sina plot: points spread within the violin outline. |
| [`geom_line()`](api/geom_line.md) | Connect observations sorted by x. |
| [`geom_path()`](api/geom_path.md) | Connect observations in data order. |
| [`geom_segment()`](api/geom_segment.md) | Line segments from (x, y) to (xend, yend), optionally with arrows. |
| [`geom_hline()`](api/geom_hline.md) | Horizontal reference lines at ``yintercept``. |
| [`geom_vline()`](api/geom_vline.md) | Vertical reference lines at ``xintercept``. |
| [`geom_abline()`](api/geom_abline.md) | Straight reference lines ``y = intercept + slope * x`` across the panel. |
| [`geom_function()`](api/geom_function.md) | Evaluate ``fun`` at x values and draw the result. |
| [`geom_smooth()`](api/geom_smooth.md) | Draw a fitted curve with a confidence band (``stat="smooth"``), one per group. |
| [`geom_area()`](api/geom_area.md) | Fill the area under a line, stacked by default (``position="stack"``). |
| [`geom_ribbon()`](api/geom_ribbon.md) | Fill the band between ymin and ymax along x. |
| [`geom_area_ridges()`](api/geom_area_ridges.md) | Ridgeline plot: one density curve per y level. Not drawn yet. |
| [`geom_polygon()`](api/geom_polygon.md) | Draw filled polygons, one per group. |
| [`geom_rect()`](api/geom_rect.md) | Rectangles from ``xmin``, ``xmax``, ``ymin``, ``ymax``. Not drawn yet. |
| [`geom_bar()`](api/geom_bar.md) | Bars, by default the count of rows per x (``stat="count"``), stacked. |
| [`geom_histogram()`](api/geom_histogram.md) | Histogram of binned x values (``stat="bin"``), stacked. |
| [`geom_boxplot()`](api/geom_boxplot.md) | Box and whiskers plot, dodged by group. |
| [`geom_violin()`](api/geom_violin.md) | Violin plot: mirrored kernel density per x (``stat="ydensity"``). Not drawn yet. |
| [`geom_density()`](api/geom_density.md) | Kernel density estimate of ``x``, drawn as a filled area (``stat="density"``). |
| [`geom_errorbar()`](api/geom_errorbar.md) | Error bars from ymin to ymax at x, with fences ``width`` wide (default 0.45). |
| [`geom_crossbar()`](api/geom_crossbar.md) | Boxes from ymin to ymax at x with a midline at y, dodged by group. |
| [`geom_linerange()`](api/geom_linerange.md) | Draw a vertical line from ymin to ymax at x. |
| [`geom_pointrange()`](api/geom_pointrange.md) | Draw a vertical line from ymin to ymax at x with a point at y. |
| [`geom_tile()`](api/geom_tile.md) | Rectangles centred on (x, y), e.g. heatmap cells. Not drawn yet. |
| [`geom_raster()`](api/geom_raster.md) | Equal-sized tiles on a regular grid. Not drawn yet. |
| [`geom_imshow()`](api/geom_imshow.md) | Display an image array, as lets_plot ``geom_imshow``. |
| [`geom_text()`](api/geom_text.md) | Text labels at (x, y); the text comes from the ``label`` aesthetic. |
| [`geom_label()`](api/geom_label.md) | Text labels in a rounded box at (x, y). |
| [`geom_text_repel()`](api/geom_text_repel.md) | Text labels pushed apart so they do not overlap each other or the points. |
| [`geom_label_repel()`](api/geom_label_repel.md) | Boxed text labels pushed apart so they do not overlap each other or the points. |
| [`geom_bracket()`](api/geom_bracket.md) | Labelled brackets from ``xmin`` to ``xmax`` at height ``y`` (significance marks). |
| [`geom_blank()`](api/geom_blank.md) | Draw nothing; the mapped data still trains the scales. No tooltips (as lets_plot). |
| [`arrow()`](api/arrow.md) | Describe the arrow heads of a line (``geom_segment``, ``geom_path``). |

## Positions

Arrange marks that would overlap.

| Name | Description |
|---|---|
| [`position_dodge()`](api/position_dodge.md) | Place overlapping objects side by side. |
| [`position_jitter()`](api/position_jitter.md) | Add random noise to positions. |
| [`position_jitterdodge()`](api/position_jitterdodge.md) | Dodge, then jitter (points over dodged boxplots). |
| [`position_nudge()`](api/position_nudge.md) | Shift positions by a fixed offset. |
| [`position_stack()`](api/position_stack.md) | Stack overlapping objects on top of each other. |
| [`position_fill()`](api/position_fill.md) | Stack overlapping objects and normalise each stack to height 1. |

## Position scales

Axes: breaks, labels, limits and transforms.

| Name | Description |
|---|---|
| [`scale_x_continuous()`](api/scale_x_continuous.md) | Continuous position scale x. |
| [`scale_y_continuous()`](api/scale_y_continuous.md) | Continuous position scale y. See ``scale_x_continuous``. |
| [`scale_x_discrete()`](api/scale_x_discrete.md) | Discrete position scale x. |
| [`scale_y_discrete()`](api/scale_y_discrete.md) | Discrete position scale y. See ``scale_x_discrete``. |
| [`scale_x_discrete_reversed()`](api/scale_x_discrete_reversed.md) | ``scale_x_discrete(reverse=True)``. |
| [`scale_y_discrete_reversed()`](api/scale_y_discrete_reversed.md) | ``scale_y_discrete(reverse=True)``. |
| [`scale_x_log10()`](api/scale_x_log10.md) | Continuous position scale x with the 'log10' transformation. See ``scale_x_continuous``. |
| [`scale_y_log10()`](api/scale_y_log10.md) | Continuous position scale y with the 'log10' transformation. See ``scale_y_continuous``. |
| [`scale_x_log2()`](api/scale_x_log2.md) | Continuous position scale x with the 'log2' transformation. See ``scale_x_continuous``. |
| [`scale_y_log2()`](api/scale_y_log2.md) | Continuous position scale y with the 'log2' transformation. See ``scale_y_continuous``. |
| [`scale_x_reverse()`](api/scale_x_reverse.md) | Continuous position scale x with the 'reverse' transformation. See ``scale_x_continuous``. |
| [`scale_y_reverse()`](api/scale_y_reverse.md) | Continuous position scale y with the 'reverse' transformation. See ``scale_y_continuous``. |
| [`scale_x_datetime()`](api/scale_x_datetime.md) | Date-time position scale x. |
| [`scale_y_datetime()`](api/scale_y_datetime.md) | Date-time position scale y. See ``scale_x_datetime``. |
| [`scale_x_time()`](api/scale_x_time.md) | Duration position scale x. |
| [`scale_y_time()`](api/scale_y_time.md) | Duration position scale y. See ``scale_x_time``. |
| [`lims()`](api/lims.md) | Limits for both axes: ``xlim(*x) + ylim(*y)``. |
| [`xlim()`](api/xlim.md) | Limits of the x axis, e.g. ``xlim(0, 10)``, ``xlim(None, 10)`` or ``xlim("B", "T")``. |
| [`ylim()`](api/ylim.md) | Limits of the y axis. See ``xlim``. |

## Color scales

Map data to `color` (lines, points, text) and `fill` (areas).

| Name | Description |
|---|---|
| [`scale_color_continuous()`](api/scale_color_continuous.md) | Map ``color`` continuously: a gradient unless ``scale_mapper_kind`` says otherwise. |
| [`scale_color_gradient()`](api/scale_color_gradient.md) | Map ``color`` to a two-color gradient (in LAB space). |
| [`scale_color_gradient2()`](api/scale_color_gradient2.md) | Map ``color`` to a diverging gradient around ``midpoint`` (continuous data only). |
| [`scale_color_gradientn()`](api/scale_color_gradientn.md) | Map ``color`` to a gradient through several colors, evenly spaced over the data. |
| [`scale_color_viridis()`](api/scale_color_viridis.md) | Map ``color`` with a viridis-family color map. |
| [`scale_color_brewer()`](api/scale_color_brewer.md) | Map ``color`` with a ColorBrewer palette. |
| [`scale_color_hue()`](api/scale_color_hue.md) | Qualitative colors evenly spaced around the HCL color wheel, for ``color``. |
| [`scale_color_manual()`](api/scale_color_manual.md) | Your own colors for ``color``. |
| [`scale_color_identity()`](api/scale_color_identity.md) | Use the data as colors (names or hex codes); no legend by default. |
| [`scale_fill_continuous()`](api/scale_fill_continuous.md) | Map ``fill`` continuously. See ``scale_color_continuous``. |
| [`scale_fill_gradient()`](api/scale_fill_gradient.md) | Map ``fill`` to a two-color gradient. See ``scale_color_gradient``. |
| [`scale_fill_gradient2()`](api/scale_fill_gradient2.md) | Map ``fill`` to a diverging gradient. See ``scale_color_gradient2``. |
| [`scale_fill_gradientn()`](api/scale_fill_gradientn.md) | Map ``fill`` to a gradient through several colors. See ``scale_color_gradientn``. |
| [`scale_fill_viridis()`](api/scale_fill_viridis.md) | Map ``fill`` with a viridis-family color map. See ``scale_color_viridis``. |
| [`scale_fill_brewer()`](api/scale_fill_brewer.md) | Map ``fill`` with a ColorBrewer palette. See ``scale_color_brewer``. |
| [`scale_fill_hue()`](api/scale_fill_hue.md) | Qualitative colors evenly spaced around the HCL color wheel, for ``fill``. |
| [`scale_fill_manual()`](api/scale_fill_manual.md) | Your own colors for ``fill``. |
| [`scale_fill_identity()`](api/scale_fill_identity.md) | Use the data as fill colors; no legend by default. |

## Size, shape and alpha scales

Map data to point size, point shape and transparency.

| Name | Description |
|---|---|
| [`scale_size()`](api/scale_size.md) | Sizes for ``size``: data mapped linearly onto ``range``. |
| [`scale_size_area()`](api/scale_size_area.md) | Sizes whose circle area is proportional to the data. |
| [`scale_size_manual()`](api/scale_size_manual.md) | Your own sizes for ``size``, matched in order with the categories (or a dict). |
| [`scale_size_identity()`](api/scale_size_identity.md) | Use the data as sizes; no legend by default. |
| [`scale_shape()`](api/scale_shape.md) | Point shapes for categories. |
| [`scale_shape_manual()`](api/scale_shape_manual.md) | Your own point shapes (lets-plot codes 0..25) for ``shape``. |
| [`scale_shape_identity()`](api/scale_shape_identity.md) | Use the data as shape codes (0..25); no legend by default. |
| [`scale_alpha()`](api/scale_alpha.md) | Transparency for ``alpha``: data mapped linearly onto ``range``. |
| [`scale_alpha_manual()`](api/scale_alpha_manual.md) | Your own alphas for ``alpha``, matched in order with the categories (or a dict). |
| [`scale_alpha_identity()`](api/scale_alpha_identity.md) | Use the data as alphas; no legend by default. |

## Coordinates and facets

Coordinate systems, and small multiples.

| Name | Description |
|---|---|
| [`coord_cartesian()`](api/coord_cartesian.md) | Cartesian coordinates; limits zoom without dropping data. |
| [`coord_fixed()`](api/coord_fixed.md) | Cartesian coordinates with a fixed aspect ratio. |
| [`coord_flip()`](api/coord_flip.md) | Swap the x and y axes (not drawn yet; ggrust warns and keeps them). |
| [`facet_wrap()`](api/facet_wrap.md) | Wrap one panel per level of ``facets`` into rows. |
| [`facet_grid()`](api/facet_grid.md) | Lay panels out in a grid: columns by ``x``, rows by ``y``. |

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
| [`theme_void()`](api/theme_void.md) | Use the classic theme without lines and axes. |
| [`theme_none()`](api/theme_none.md) | Use the unstyled theme. |

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
| [`GGBunch`](api/GGBunch.md) | lets_plot's deprecated free-layout bunch of plots. Not supported; use ``gggrid``. |
| [`layer_tooltips()`](api/layer_tooltips.md) | Configure hover tooltips. |
| [`ggtb()`](api/ggtb.md) | Add the zoom/pan toolbar to an HTML plot. |

## Export and setup

Write files, and lets-plot compatibility helpers.

| Name | Description |
|---|---|
| [`ggsave()`](api/ggsave.md) | Save a plot to ``path/filename``; the extension picks the format. |
| [`LetsPlot`](api/LetsPlot.md) | lets_plot's global settings object. ggrust needs no setup; calls are accepted. |
| [`engine_version`](api/engine_version.md) | Return the version of the Rust engine. |

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
