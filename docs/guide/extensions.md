# ggplot2 extensions

ggplot2 has a large family of extension packages in R. ggrust builds many of the most
popular ones straight into the library, with the R package's function and parameter
names, so there is nothing extra to install. lets-plot has none of these, so this is
where ggrust goes beyond a lets-plot replacement.

Each feature below credits the R package it comes from. ggrust reimplements the
behavior in Rust; the R packages and their authors designed it.

| Feature | ggrust | From the R package |
|---|---|---|
| [Beeswarm and quasirandom points](#beeswarm-points) | [`geom_beeswarm`](../reference/api/geom_beeswarm.md), [`geom_quasirandom`](../reference/api/geom_quasirandom.md) | [ggbeeswarm](https://cran.r-project.org/package=ggbeeswarm) |
| [Journal color palettes](#journal-palettes) | `scale_color_npg`, `_aaas`, `_nejm`, `_lancet`, `_jama`, `_jco`, `_d3`, `_igv` (and `scale_fill_*`) | [ggsci](https://cran.r-project.org/package=ggsci) |
| [Significance tests and labels](#significance-tests) | [`stat_compare_means`](../reference/api/stat_compare_means.md), [`geom_pwc`](../reference/api/geom_pwc.md), [`stat_pvalue_manual`](../reference/api/stat_pvalue_manual.md), [`compare_means`](../reference/api/compare_means.md), [`stat_cor`](../reference/api/stat_cor.md), [`stat_poly_eq`](../reference/api/stat_poly_eq.md) | [ggpubr](https://cran.r-project.org/package=ggpubr), [ggpmisc](https://cran.r-project.org/package=ggpmisc) |
| [Cluster marks](#cluster-marks) | [`geom_mark_hull`](../reference/api/geom_mark_hull.md), [`geom_mark_ellipse`](../reference/api/geom_mark_ellipse.md), [`geom_mark_rect`](../reference/api/geom_mark_rect.md) | [ggforce](https://cran.r-project.org/package=ggforce) |
| [Highlighting](#highlighting) | [`gghighlight`](../reference/api/gghighlight.md) | [gghighlight](https://cran.r-project.org/package=gghighlight) |
| [Point density](#point-density) | [`geom_pointdensity`](../reference/api/geom_pointdensity.md) | [ggpointdensity](https://cran.r-project.org/package=ggpointdensity) |
| [Side panels](#side-panels) | `geom_xside*`, `geom_yside*` (tile, bar, col, density, histogram, boxplot, violin, point, line, text), [`ggmarginal`](../reference/api/ggmarginal.md) | [ggside](https://cran.r-project.org/package=ggside) |
| [Several color scales in one plot](#clustered-heatmaps) | [`new_scale_fill`](../reference/api/new_scale_fill.md), `new_scale_color`, `new_scale` | [ggnewscale](https://cran.r-project.org/package=ggnewscale) |
| [Dendrograms](#clustered-heatmaps) | `hclust`, [`scale_y_dendro`](../reference/api/scale_y_dendro.md), `scale_x_dendro`, `guide_axis_dendro`, [`geom_dendrogram`](../reference/api/geom_dendrogram.md) | [legendry](https://cran.r-project.org/package=legendry), [ggh4x](https://cran.r-project.org/package=ggh4x), [ggdendro](https://cran.r-project.org/package=ggdendro) |
| [Raincloud plots](#raincloud-plots) | [`stat_halfeye`](../reference/api/stat_halfeye.md), `stat_slab`, `stat_interval`, `stat_pointinterval`, [`geom_half_violin`](../reference/api/geom_half_violin.md), `geom_half_boxplot`, `geom_half_point` | [ggdist](https://cran.r-project.org/package=ggdist), [gghalves](https://cran.r-project.org/package=gghalves) |
| [Figure layouts](#figure-layouts) | `gggrid(design=, tag_levels=, axes=, axis_titles=)`, [`inset_element`](../reference/api/inset_element.md) | [patchwork](https://cran.r-project.org/package=patchwork) |
| [Rasterized layers](#rasterized-layers) | [`rasterize`](../reference/api/rasterize.md) | [ggrastr](https://cran.r-project.org/package=ggrastr), [scattermore](https://cran.r-project.org/package=scattermore) |

Two more, `geom_text_repel` / `geom_label_repel` ([ggrepel](https://cran.r-project.org/package=ggrepel))
and `geom_area_ridges` ([ggridges](https://cran.r-project.org/package=ggridges)), come
with the lets-plot API and are covered in [Layers](layers.md). ggrust also adds core
ggplot2 features lets-plot lacks: `geom_col`, `geom_rug`, `stat_ellipse`, and secondary
axes with `sec_axis` / `dup_axis`.

## Beeswarm points

`geom_beeswarm` places points side by side so that none overlap, which shows every
observation and the shape of the distribution at once. `geom_quasirandom` spreads them
by the local density instead. Both follow
[ggbeeswarm](https://cran.r-project.org/package=ggbeeswarm) by Erik Clarke, Scott
Sherrill-Mix and Charlotte Dawson.

```python
--8<-- "beeswarm.py:code"
```

--8<-- "figures/beeswarm.html"

## Journal palettes

The `scale_color_*` / `scale_fill_*` palettes of
[ggsci](https://cran.r-project.org/package=ggsci) by Nan Xiao take their colors from
scientific journals and data tools: `npg` (Nature Publishing Group), `aaas` (Science),
`nejm`, `lancet`, `jama`, `jco`, `d3` and `igv`. `igv` has 51 colors, enough for many
clusters. The examples on this page use several of them.

## Significance tests

`stat_compare_means` runs the test (`t.test`, `wilcox.test`, `anova`, `kruskal.test`),
adjusts the p-values if asked (`p_adjust_method`) and draws the brackets and labels, as
in [ggpubr](https://cran.r-project.org/package=ggpubr) by Alboukadel Kassambara. The
statistics are computed in Rust and match R's own tests. `stat_cor` labels a
correlation and `stat_poly_eq` a fitted polynomial, as in ggpubr and
[ggpmisc](https://cran.r-project.org/package=ggpmisc) by Pedro J. Aphalo.

```python
--8<-- "compare_means.py:code"
```

--8<-- "figures/compare_means.html"

`geom_pwc` compares every pair of groups by itself, also the colored groups within
each x, with Holm-adjusted p-values, as ggpubr's `geom_pwc`. To draw your own
comparisons, `compare_means()` returns them as a table (ggpubr's `compare_means`)
and `stat_pvalue_manual` draws a table's rows as brackets.

## Cluster marks

`geom_mark_hull`, `geom_mark_ellipse` and `geom_mark_rect` outline each group and can
label it outside the data, as in [ggforce](https://cran.r-project.org/package=ggforce)
by Thomas Lin Pedersen.

```python
--8<-- "cluster_marks.py:code"
```

--8<-- "figures/cluster_marks.html"

## Highlighting

`gghighlight` keeps the groups that match a condition in color and draws the rest in
grey behind them, with optional direct labels. The condition is a Polars expression,
evaluated per group by default. It follows
[gghighlight](https://cran.r-project.org/package=gghighlight) by Hiroaki Yutani.

```python
--8<-- "highlight.py:code"
```

--8<-- "figures/highlight.html"

## Point density

`geom_pointdensity` colors each point by the number of neighbours around it, so dense
regions stand out in a scatter plot that would otherwise overplot. It follows
[ggpointdensity](https://cran.r-project.org/package=ggpointdensity) by Lukas P. M.
Kremer, and stays fast on millions of points.

```python
--8<-- "point_density.py:code"
```

--8<-- "figures/point_density.html"

## Side panels

`geom_xside*` and `geom_yside*` layers draw into a panel beside the main one that
shares its x or y axis, after [ggside](https://cran.r-project.org/package=ggside) by
Justin Landis. They stay aligned with the main panel, also when you zoom.
[`ggmarginal`](../reference/api/ggmarginal.md) is lets-plot's shortcut for the same
idea.

Unlike ggside, an axis can have more than one band: `side_pos="left"` (or `"right"`,
`"top"`, `"bottom"`) puts a layer's band on that side, `side_band="name"` gives it a band
of its own (bands on one side stack outward in plot order) and `side_size=0.2` sets its
thickness. `scale_ysidex_discrete(side_pos="left", ...)` sets that band's own axis.

```python
--8<-- "side_panels.py:code"
```

--8<-- "figures/side_panels.html"

## Clustered heatmaps

A clustered heatmap combines three extensions:

- `hclust` clusters the rows (or columns) and `scale_y_dendro` orders the axis by the
  tree and draws it beside the labels, after
  [legendry](https://cran.r-project.org/package=legendry) and
  [ggh4x](https://cran.r-project.org/package=ggh4x) by Teun van den Brand.
  `geom_dendrogram` draws a tree as a layer, as
  [ggdendro](https://cran.r-project.org/package=ggdendro) by Andrie de Vries does. A
  tree computed elsewhere, for example by scipy or scanpy, can be passed in as its
  linkage matrix. With `leaf_levels={leaf: [levels]}` each leaf stands for a run of
  axis levels, so a tree over clusters can sit beside a heatmap of single cells: the
  runs follow the tree, and each leaf is drawn at the centre of its run.
- `new_scale_fill` starts a second fill scale, so each annotation track gets its own
  colors and legend, as in [ggnewscale](https://cran.r-project.org/package=ggnewscale)
  by Elio Campitelli.
- `geom_ysidetile` draws the annotation track beside the heatmap (see
  [Side panels](#side-panels)).

```python
--8<-- "clustered_heatmap.py:code"
```

--8<-- "figures/clustered_heatmap.html"

## Raincloud plots

`stat_halfeye` draws a density slab with the median and quantile intervals, after
[ggdist](https://cran.r-project.org/package=ggdist) by Matthew Kay.
`geom_half_violin`, `geom_half_boxplot` and `geom_half_point` draw half of each shape,
after [gghalves](https://cran.r-project.org/package=gghalves) by Frederik Tiedemann.
Together they make raincloud plots; panel B of the next example is a `stat_halfeye`.

## Figure layouts

`gggrid` accepts a `design` string that names a cell per letter, and `tag_levels` tags
the panels A, B, C (or a, 1, i). `axes="collect"` shows axes that repeat once per run
of plots (with `sharex` / `sharey`), and `axis_titles="collect"` turns repeated axis
titles into one title centred on the run. `inset_element` places a plot inside
another. These follow [patchwork](https://cran.r-project.org/package=patchwork) by Thomas Lin
Pedersen.

```python
--8<-- "figure_layout.py:code"
```

--8<-- "figures/figure_layout.html"

## Rasterized layers

`rasterize(layer, dpi=300)` draws one layer as an image inside SVG and PDF output while
axes and text stay vector, after [ggrastr](https://cran.r-project.org/package=ggrastr)
by Viktor Petukhov, Teun van den Brand and Evan Biederstedt, and
[scattermore](https://cran.r-project.org/package=scattermore) by Tomas Sieger and
Mirek Kratochvil. A scatter plot of a million points then saves as a small PDF that
still has sharp, selectable text.

```python
plot = ggplot(cells, aes("umap_1", "umap_2")) + rasterize(geom_point(size=0.5), dpi=300)
ggsave(plot, "umap.pdf")
```
