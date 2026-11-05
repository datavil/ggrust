# ggrust extensions plan

Features from popular ggplot2 extension packages, and a few core ggplot2 features,
that lets-plot does not have. Chosen with the user on 2026-11-04 from a survey of the
ggplot2 extension ecosystem. Everything else from that survey (Tier 3: upset, Venn,
treemaps, waffle, streamgraphs, bump charts, pattern fills, networks, animation, maps,
plus ggbreak, ggmagnify, geomtextpath and ggtext geoms) is **only added if users ask
for it**.

Rules of engagement: [AGENTS.md](AGENTS.md). Main roadmap: [plan.md](plan.md).

## Rules for this plan

- **There is no lets-plot reference here.** So the reference is the R extension package,
  checked against ggplot2's own grammar. Use the R package's function and parameter
  names, in snake_case Python form (`new_scale_color`, `geom_quasirandom`), so R users
  recognise them. Ask the user when a name or behaviour is not obvious.
- **Every step is a deviation.** Each step adds a row to [deviations.md](deviations.md)
  ("ggrust extension, lets-plot has none, ggplot2 needs package X"), and the steps that
  change direction also get a row in the `plan.md` decisions table.
- **Licenses.** ggrust is MIT. Many ggplot2 extensions are GPL. Read a GPL package to
  learn what it does, then implement from the algorithm or paper. Do not port GPL code
  line by line. Check each package's license when its step starts. Palette hex values
  and published algorithms are fine to use.
- **One file + one registry line** still holds: a new geom, stat or position is one file
  in `crates/ggrust-core/src/{geom,stat,position}/`, plus its Python wrapper. If a step
  needs edits across many unrelated files (likely for `ex1.3` and `ex2.2`), stop and
  show the user the design before writing code.
- **Verify like any step:** Rust unit tests, pytest, rendered PNGs compared against the
  R package's output for the same data (render the R version with `Rscript` when R is
  available, else describe the reference in the step notes).
- Steps are numbered `exT.N`: tier T, item N. The order within a tier is the proposed
  order (value for cellestial first, then the cheapest).

## Tier 1: high value for cellestial, widely used

- [x] ex1.1 **ggsci journal palettes.** Discrete `scale_color_*` / `scale_fill_*` for
  `npg`, `aaas`, `nejm`, `lancet`, `jama`, `jco`, `d3` (`category10`, `category20`,
  `category20b`, `category20c`), `igv` (51 colours, good for many clusters), with the
  `palette=` and `alpha=` parameters as ggsci. A palette table next to the brewer ones in
  `scale/style`, no new scale machinery. Done when each palette's colours match ggsci's
  hex values and a too-short palette warns like the brewer `overflow` path.
- [x] ex1.2 **ggbeeswarm.** `geom_beeswarm` (`method="swarm"`, `cex`, `priority`,
  `side`) and `geom_quasirandom` (`method="quasirandom"`, van der Corput offsets scaled by
  the group's density; `width`, `varwidth`, `bandwidth`). As positions or stats over the
  existing violin density code (`stat/ydensity.rs`), seeded and deterministic. Works with
  `orientation` and `position_dodge`. Done when point offsets match the R package on a
  test set, within tolerance.
- [x] ex1.3 **ggnewscale.** `new_scale_color()`, `new_scale_fill()`, `new_scale("size")`
  added with `+`: layers after it get their own scale and their own legend for that
  aesthetic. Needs a scale key that is "aesthetic + scale group" in the spec and in
  training. **Design review with the user first**: this touches the spec, scale training
  and guides. Done when two color scales (e.g. a continuous one for points and a
  discrete one for labels) draw with two legends, and `guides="collect"` in `gggrid`
  still merges equal ones.
- [x] ex1.4 **ggpointdensity.** `geom_pointdensity` / `stat_pointdensity`: each point
  gets `..density..` (neighbours within `adjust`-scaled radius, or 2D KDE with
  `method="kde2d"`), mapped to color by default. Neighbour counting on a grid in Rust,
  parallel, so 1M points stays fast. Done when density values match the R package and
  a 1M-point UMAP benchmark is reported.
- [x] ex1.5 **ggrastr / scattermore.** `rasterize(layer, dpi=300)` (ggrastr name) or a
  layer parameter: that layer is drawn into a PNG and embedded as an image in SVG and PDF
  output, the rest stays vector. Reuse the PNG backend and the `geom_imshow` image path.
  Done when a 1M-point UMAP saves as a small vector PDF with crisp axes and text, and
  the raster lines up with the axes to the pixel.
- [x] ex1.6 **ggdist / gghalves: half geoms and raincloud plots.** `geom_half_violin`,
  `geom_half_boxplot`, `geom_half_point` (`side="l"|"r"`, `nudge`), and ggdist's
  `stat_halfeye` / `stat_slab` / `stat_interval` / `stat_pointinterval` (density slab
  plus median and quantile intervals). Built on the violin, boxplot and interval geoms.
  Done when a raincloud plot (half violin + boxplot + jittered points) renders as in
  the R packages.
- [x] ex1.7 **ggpubr / ggpmisc computed statistics.** `stat_compare_means` (`method`
  t.test, wilcox.test, anova, kruskal.test; `comparisons=`, `label="p.signif"|"p.format"`,
  `ref_group`, `hide_ns`) drawing through `geom_bracket`; `stat_cor` (Pearson, Spearman,
  Kendall, R and p label); `stat_poly_eq` (equation, R², per group). Tests in Rust
  (no SciPy at runtime). Done when p-values and coefficients match R to 1e-6 on test
  data and labels place without overlap as ggpubr.
- [x] ex1.8 **ggforce cluster marks.** `geom_mark_hull` (concave hull, `concavity`,
  `expand`, `radius`), `geom_mark_ellipse`, `geom_mark_rect`, with optional `label` /
  `description` callouts placed outside the data. For outlining clusters on UMAPs.
  Done when hulls enclose every point of their group with the given expansion and the
  callouts do not overlap the marks.
- [x] ex1.9 **Dendrograms (ggdendro / ggtree / ggh4x `guide_dendro`).** Hierarchical
  clustering in Rust (`complete`, `average`, `single`, `ward.D2`; euclidean and
  correlation distance), `geom_dendrogram`, and a dendrogram drawn beside a discrete
  axis that also orders that axis's levels. For cellestial's clustered heatmaps and
  dotplots (scanpy `dendrogram=True`). Done when leaf order and merge heights match R's
  `hclust` on test data, and a heatmap with row and column dendrograms renders aligned.

## Tier 2: general plotting

- [ ] ex2.1 **Extra themes.** `theme_cowplot` (also `theme_half_open`,
  `theme_minimal_grid`), `theme_pubr` / `theme_pubclean` (ggpubr), as `theme()` presets
  in `python/ggrust`. Done when each matches the R theme's look on a reference plot.
- [ ] ex2.2 **ggside / ggExtra marginal plots.** Builds on plan.md `lp5.6 ggmarginal`
  (lets-plot parity first: `ggmarginal(sides, size, layer)`), then ggside's
  `geom_xside*` / `geom_yside*` layers for any geom in the margin, including bar and
  boxplot margins (lets-plot#972) and facets. **Design review with the user first.**
- [x] ex2.3 **patchwork extras for `gggrid`.** Insets (`inset_element(plot, left,
  bottom, right, top)`), nested grids that keep alignment, a `design=` layout string
  (`"AAB\nCCB"`), and automatic panel tags across a grid (`tag_levels="A"|"a"|"1"|"i"`,
  prefix and suffix) using the existing `plot_tag` theme element.
- [ ] ex2.4 **ggh4x.** `facet_nested` / `facet_nested_wrap` (nested strips that merge
  equal parents), `force_panelsizes` (fixed panel widths and heights), and
  `facetted_pos_scales` (per-panel position scales). Nested axis guides
  (`guide_axis_nested`) for interaction labels like `group.condition`.
- [x] ex2.5 **gghighlight.** `gghighlight(predicate, label_key=, use_direct_label=,
  unhighlighted_params=)`: layers are redrawn with non-matching rows greyed out behind
  the matching ones, optionally labelled with the repel layout. Pure spec rewrite in
  Python where possible.
- [ ] ex2.6 **ggalluvial / ggsankey.** `geom_alluvium`, `geom_stratum`, `stat_stratum`
  (ggalluvial) and `geom_sankey` (ggsankey): flows between discrete axes, for example
  cell-type composition across conditions. Ribbons as x-splines (`knot.pos`, `curve_type`)
  drawn as polygons.
- [ ] ex2.7 **ggcorrplot.** Finish plan.md `lp5.12` `corr_plot` (lets-plot parity, partly
  done) first, then ggcorrplot's options where lets-plot has none: `hc.order`
  (reuses ex1.9 clustering), `p.mat` with `insig="blank"|"pch"`, `lab` and `outline.color`.

## Core ggplot2 gaps (not in lets-plot)

- [x] exc.1 `geom_col` (= `geom_bar(stat="identity")`, ggplot2 name).
- [x] exc.2 `geom_rug` (`sides`, `length`, `outside`).
- [x] exc.3 `stat_ellipse` (`type="t"|"norm"|"euclid"`, `level`, `segments`), drawn as
  path or polygon. Shares nothing with ex1.8's mark ellipse: this one is a statistical
  confidence ellipse.
- [x] exc.4 Secondary axes: `sec_axis(transform, name, breaks, labels)` and `dup_axis()`
  on `scale_x/y_continuous(sec_axis=...)`. Depends on plan.md `9.3` (position scale
  `position=`), do that first. Done 2026-11-05: the transform is a Python callable the
  engine calls back on 1000 samples of the final range (user's choice), breaks from their
  own scale placed through the table; facets, flip, log/reverse, grid sharing and
  `align=True`; HTML zoom moves fixed secondary breaks. Tests: `tests/test_sec_axis.py`
  (R positions via `tests/fixtures/generate_sec_axis_reference.R`), `scale::secondary`.
- [ ] exc.5 `geom_spoke` is a lets-plot geom (gaps file, score 2), so it follows
  lets-plot parity in plan.md `lp6.2`, not this plan. Listed here so it is not done twice.

## User asks (users_asks.md)

- [x] ua.7 Grid captions: `gggrid(...) + labs(caption=...)` is drawn (no longer warned), as
  lets-plot 4.11: a bottom band below the figures and collected legends, justified along
  the figures with the `plot_caption` theme element (right by default). Grid titles and
  `tag_levels=` tags already rendered (ex2.3). Tests: `tests/test_grid_caption.py` (lets-plot
  baseline and band), grid unit `grid_caption_is_drawn_at_the_bottom_without_a_warning`.

## Added on user request (not scheduled)

Tier 3 and the rest of Tier 2 from the 2026-11-04 survey. Move an item into a tier
above when a user asks for it, with the date and who asked:

- Tier 2 not chosen: ggbreak (broken axes), ggmagnify (zoom insets), geomtextpath (text
  along curves), ggtext geoms (`geom_richtext`, `geom_textbox`).
- Tier 3: ggupset, ggVennDiagram, treemapify, waffle, ggstream, ggbump, ggpattern,
  ggraph, gganimate, maps (sf / ggspatial, already out of scope in plan.md).
