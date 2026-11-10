# Plan for group lp5 (lets-plot gaps, score 3)

Detail for the `lp5.N` steps in [plan.md](../../plan.md) Phase 8. Source of each gap:
[plan-letsplot-gaps.md](plan-letsplot-gaps.md). One step per session (AGENTS.md
"Cost discipline"); each step follows the AGENTS.md workflow (measure lets-plot,
implement, render and look, bug-hunt, `make check`, commit, push, tick plan.md).

Notes that apply to every step:

- `../lets-plot` (Kotlin source) is **not checked out** on this machine. Measure
  behavior from `lets_plot` 4.11.0's offline `to_svg()` (see memory
  `letsplot_reference_svg.md`) and check ggplot2 when lets-plot looks wrong. Clone
  the source first if a step needs an exact port (sum, bin2d, hex, contour, qq).
- Python signatures are already known (lets_plot 4.11.0, `inspect.signature`); they
  are listed per step. `sampling`, `color_by`, `fill_by` stay unsupported (score 1).
- lp4 is not done yet. No lp5 step depends on lp4, except where noted.

## Suggested order

Dependencies first, then small before large, the two architectural steps last:

| Order | Step | Size | Depends on |
| --- | --- | --- | --- |
| 1 | lp5.1 `geom_count` / `stat_sum` | S | |
| 2 | lp5.2 `geom_bin2d`, `geom_hex` | M | |
| 3 | lp5.3 contour family | L | 2D KDE (density.rs kernels) |
| 4 | lp5.4 qq family | M | quantile functions |
| 5 | lp5.9 text layer options, `manual_key` | M | number_format.rs |
| 6 | lp5.11 mapped `hjust` / `vjust` | S | |
| 7 | lp5.10 positional scale options, linewidth / stroke scales | M | |
| 8 | lp5.7 facet label options | S | |
| 9 | lp5.8 minor ticks, `exponent_format`, `label_text` | M | |
| 10 | lp5.6 `ggmarginal` | L | layout change |
| 11 | lp5.12 bistro | M | lp5.4, lp5.6 |
| 12 | lp5.5 `coord_polar` | XL | coord/geom change (decision below) |
| 13 | lp5.13 vector PDF | L | dependency decision below |

## Steps

### lp5.1 `geom_count` / `stat_sum`

- API: `geom_count(mapping, data, stat, position, show_legend, inherit_aes, manual_key, tooltips, na_rm)`; `stat="sum"` on any geom.
- `stat/sum.rs`: count rows (or sum `weight`) per distinct (x, y) within each group;
  outputs `..n..` and `..prop..` (measure whether lets-plot's `prop` is per group, as
  ggplot2, and whether it also has `..proppct..`). Default mapping `size = ..n..`.
- `geom_count` = point geom with stat `sum`; size legend from the existing size scale.
- Tests: counts on discrete x/y and on continuous duplicates, weights, groups; SVG
  radius parity with lets-plot on a mpg-like frame.

### lp5.2 `geom_bin2d`, `geom_hex`

- API: `bins` (default `[30, 30]`), `binwidth`, `drop`; hex adds `width_unit`, `height_unit`.
- `stat/bin2d.rs`: rectangular bins sharing the 1D bin boundary rules of `stat/bin.rs`
  (one helper per axis); outputs `..count..`, `..density..`, tile `width`/`height`;
  draws through `geom/tile.rs`.
- `stat/binhex.rs` + `geom/hex.rs`: hexagon centres on the offset grid, polygons
  drawn as one path per fill. Do not copy #1290 (wrong tile size with `bins=[n, 1]`):
  check against ggplot2.
- `drop=False` keeps empty bins (count 0).
- Tests: bin counts sum to n, edges match lets-plot breaks, #1290 repro, 1M-point
  benchmark (hot path; rayon only if it pays).

### lp5.3 Contour family

Largest stat step; may split into 5.3a (contour, contourf) and 5.3b (density2d(f)).

- API: `geom_contour(bins, binwidth)`, `geom_contourf`, `geom_density2d(kernel, adjust, bw, n, bins, binwidth)`, `geom_density2df`.
- `stat/contour/` core: build the grid from (x, y, z) rows in **any row order**
  (#1357): sort unique x and y, place z by index; missing nodes leave their cells
  empty, `z = 0` is a value, not a hole (#1356). Marching squares for isolines
  (saddle cells resolved by the cell mean), joined into paths; isobands for the
  filled variant, closed polygons with holes (even-odd fill).
- Levels: `bins` default 10 (measure lets-plot's pretty-break rule) or `binwidth`;
  outputs `..level..` (contourf: band level, measure lets-plot's choice of band value).
- `stat/density2d.rs`: 2D product-kernel KDE on an `n x n` grid (default n = 100),
  bandwidth per axis from the existing `density.rs` helpers, then the contour core.
- Geoms: contour = path, contourf = polygon; `color = ..level..` by default.
- Tests: known analytic surfaces (cone, saddle), unsorted and incomplete grids,
  isoband area sums, lets-plot vertex counts on the faithful-like demo data.

### lp5.4 qq family

- API: `geom_qq(distribution, dparams)`, `geom_qq_line(distribution, dparams, quantiles)`, `geom_qq2`, `geom_qq2_line(quantiles)`.
- `stat/qq.rs`: sorted sample vs theoretical quantiles at `(i - 0.5) / n` (measure
  lets-plot's plotting positions). Distributions as lets-plot: `norm` (default),
  `uniform`, `t`, `gamma`, `exp`, `chi2`, with `dparams`. Inverse CDFs in one small
  `stat/distribution.rs` (reuse the t quantile from `stat/smooth.rs`; gamma / chi2 by
  Newton on the regularised incomplete gamma). No new dependency unless the user agrees
  (`statrs` would cover it).
- `qq_line`: line through the `quantiles` pair (default `[0.25, 0.75]`) over the x range.
- `qq2` / `qq2_line`: sample x vs sample y, quantiles matched by sorting (unequal
  lengths interpolated, as lets-plot).
- Tests: quantiles against R / scipy values to 1e-9, per-group lines, SVG parity.

### lp5.5 `coord_polar`

**Needs a design decision before starting** (ask the user). `Coord::map` maps one
point; polar needs every straight data-space edge bent: lines, rects (bars become
wedges), area and ribbon edges, grid lines (circles) and a circular theta axis.

- Proposed design (from ggplot2's `coord_munch`): add `Panel::path(&[(x, y)]) -> Vec<(px, py)>`
  that interpolates in data space for non-linear coords and is the identity for
  cartesian; geoms that draw rects and lines call it instead of mapping corners. Audit
  which geoms map corners directly first; if many files must change, stop and report
  (AGENTS.md "Extension points").
- API: `xlim`, `ylim`, `theta` ('x' | 'y'), `start`, `direction`, `transform_bkgr`.
- Axes: theta labels around the circle, r axis on the left; grid as circles and spokes.
- Do not copy lets-plot's open polar bugs (#1215 abline, #1213 `size_unit`, #1095,
  #1097, #1036, #1039, #1061, #1182): follow ggplot2.
- Tests: pie chart (bar + `theta='y'`), radar line, wind-rose stacked bars, abline;
  visual golden images.

### lp5.6 `ggmarginal`

- API: `ggmarginal(sides, size=0.1, layer)`, added with `+`; lets-plot puts marginal
  layers into thin panels on the given sides (`'tblr'`) sharing the main panel's x or
  y scale. Measure the layout (gap, size as fraction, no axes on margins) from lets-plot SVG.
- Layout change in `layout/`: a panel can carry side panels with one shared position
  scale; marginal layers train only the shared axis. Check facets + margins (lets-plot
  behavior) and say what is unsupported.
- Do not copy #972 (no bar margins): bars on margins work, as the shared scale allows.
- Tests: histogram / density / boxplot margins on each side, with `coord_flip`, with facets (error or support).

### lp5.7 Facet label options

- `facet_grid`: `x_order`, `y_order` (1 ascending, -1 descending, 0 as in data; measure),
  `x_format`, `y_format` (reuse `scale/label_pattern.rs` / `number_format.rs`),
  `x_labwidth`, `y_labwidth`; `facet_wrap(labwidth=)`: wrap strip text at n characters
  (measure lets-plot's word-break rule). Strips grow to the wrapped line count.
- Also add `facet_grid` column validation (7.0 left it open) if it is cheap; ask first.
- Tests: order and format on numeric and string facets, wrapped strip heights vs lets-plot.

### lp5.8 Minor ticks, `exponent_format`, `label_text`

- Theme: `axis_minor_ticks`, `axis_minor_ticks_x/_y`, `axis_minor_ticks_length*`
  (#1183): minor ticks at the minor grid positions the scale already computes.
- `exponent_format` ('e', 'pow', 'pow_full', or a tuple with limits): how big and small
  numbers print in default labels. Superscripts in SVG (`tspan`), PNG and HTML text
  measurement; measure lets-plot's limits.
- `label_text`: theme element for `geom_label` text; apply `angle` (lets-plot #1474
  ignores it; follow ggplot2).
- Tests: tick positions, label strings for each format mode, superscript rendering in PNG.

### lp5.9 Text layer options, `manual_key`

- `label_format` on `geom_text` / `geom_label` (lets-plot format strings, already
  parsed for scales and tooltips), `na_text` (default `'n/a'`, measure),
  `check_overlap` (drop labels that overlap an earlier one, in draw order, as ggplot2; #270).
- `manual_key`: `layer_key(label, group, index, **aes)` legend entries for layers with no
  mapped aesthetic; measure merging with existing legends.
- Undo the periodic table mass rounding (gaps file section 12) and rebuild the docs examples.
- Tests: formatted label strings, NA text, overlap culling count, manual legend keys vs lets-plot.

### lp5.10 Positional scale options, linewidth and stroke scales

- `break_width` on continuous positional scales (datetime already has it): breaks every
  `break_width` from a nice origin.
- `lablim`: truncate tick labels to n characters with an ellipsis (measure the marker).
- `na_value` on positional scales: where missing positions are drawn (measure; ggplot2
  keeps NA rows out by default).
- `scale_linewidth`, `scale_linewidth_identity`, `scale_stroke`, `scale_stroke_identity`
  (range, breaks, legend). Do not copy #1158 (stroke does not grow shape 19): check ggplot2.
- Do not copy #769 (limits outside the data crash).
- Tests: break positions, truncated labels, legends for linewidth / stroke vs lets-plot.

### lp5.11 Mapped `hjust` / `vjust`

- Mapped `hjust` / `vjust` on `geom_text` / `geom_label` (#1143): numbers per row and
  the named values (`left`, `middle`, `right`, `top`, `center`, `bottom`, `inward`,
  `outward`), identity-scaled, no legend.
- Tests: per-row anchors in SVG, `inward` / `outward` against ggplot2.

### lp5.12 Bistro: `corr_plot`, `joint_plot`, `qq_plot`

Pure Python on the spec API (`python/ggrust/bistro/`), after lp5.4 and lp5.6.

- `corr_plot(data, show_legend, flip, threshold)` with lets-plot's builder methods
  (`tiles`, `points`, `labels`, `palette_*`, `build`); polars correlation. Do not copy
  #866 (`theme_void` breaks axes).
- `joint_plot(data, x, y, geom, bins, binwidth, color, size, alpha, color_by, show_legend, reg_line, se, marginal)`.
- `qq_plot(data, sample, x, y, distribution, dparams, quantiles, group, show_legend, marginal, color, fill, alpha, size, shape, line_color, line_size, linetype)`.
- Tests: `as_dict()` equal to lets_plot's for the same call (where the builder output
  is a plain spec), rendered pictures side by side.

### lp5.13 Vector PDF

**Needs a dependency decision before starting** (ask the user):

1. `svg2pdf` (+ `usvg`, already in the resvg stack): render the SVG we already write
   to PDF. Least code; fonts embedded as glyph outlines or subset fonts.
2. Own writer from `Scene`: PDF path / text operators, embed the bundled font subset.
   More code, no new dependency, fastest.

Recommendation: option 1 first (20/80), switch only if benchmarks show a problem.
Keep the raster path behind an option if lets-plot parity on pixels is wanted.
Avoid lets-plot #1509 (`linetype=0` drops titles) and check #1440 (CJK text).
Tests: PDF parses, text is selectable (extract with `pdftotext` if present), page size
from `w`, `h`, `unit`, `dpi`; rasterised PDF matches the PNG.

## Open questions for the user

1. `coord_polar` design: accept the `Panel::path` munching approach (touches every geom
   that maps rect corners or line ends)?
2. Vector PDF: `svg2pdf` dependency or own writer?
3. qq distributions: hand-written inverse CDFs or the `statrs` crate?
4. Execution order: follow the suggested order above, or plan.md's numbering?
