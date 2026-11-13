# ggpubr / ggpmisc statistics

Step ex1.7 (2026-11-04). ggpubr 1.0, ggsignif 0.6.4 and ggpmisc 1.0 are GPL: read for
behaviour, implemented from the statistics. R reference: `tests/fixtures/generate_pubr_reference.R`
(base R for the numbers; ggpubr/ggpmisc for `layers`, from a scratch `R_LIBS`).

- **Tests** (`stat/hypothesis.rs`, `stat/distribution.rs`): tails computed directly
  (normal via `Q(1/2, z^2/2)`, t/F via the incomplete beta on the small side), exact
  null distributions by additive DP in f64 (no cancellation). R's switches: rank sum
  exact when both n < 50 and no ties; signed rank exact when n < 50, no ties, no
  zeros; Kendall exact when n < 50 and no ties; Spearman AS 89 (enumeration to n = 9,
  Edgeworth above, `round(q)` upper / `round(q) + 2` lower) up to n = 1290, t approx
  above or with ties. All match R to 1e-6 on the fixture first time.
- **Formatting**: R's `format(x, digits)` decides digits by scaling to `digits`
  digits and rounding half to even (`0.995` -> `"1"`), then prints with sprintf; exact
  decimal rounding alone gives `"0.99"`. `format.pval` below eps is `"<2e-16"`.
- **Layouts** (ggsignif): bracket i at `ymax + range (0.05 + 0.12 (i - 1))`, tips 0.03
  range (data units, so `lenstart`/`lenend` columns + `tiplength_unit="identity"`);
  global label at the x range's left with hjust 0.2, vjust 0; per-x labels hjust 0.5.
  The y range is the plot's raw data range over all layers (`StatContext.y_range`).
- **Level names**: `StatContext.x_levels` carries the discrete x levels in axis order
  (filled in `compute_layer`), so `comparisons`/`ref_group` resolve names in Rust.
- **Text labels from a stat**: output an `Aesthetic::Label` discrete column; style
  resolution reads it as text. With `per_group` (f64 only), emit the label's slot as a
  number and swap in the discrete column after (`pubr_labels::labels_per_group`).
- **vjust units**: lets-plot/ggrust `vjust` spans 0.7 em, ggplot2's a text height, so
  ggpubr's 1.4 stacking step becomes 2 `vjust` units (labels 1.4 em apart).
- ggpubr 1.0's text-mode `stat_cor` prints raw p-values (a regression); ggrust prints
  2 significant digits. Group label stacking follows ggrust's group order (first
  appearance), not R's sorted factor levels.

Step 5.7 (2026-11-13, ggpubr 0.6.3 + rstatix 0.7.3, reference
`tests/fixtures/generate_pubr_helpers_reference.R`):

- **`compare_means()`** pairs follow R's `pairwise.*.test` matrix read row by row
  (`(1,2), (1,3), (2,3), (1,4)`), `group.by` partitions keep first appearance (not
  sorted), `p.adj` is `signif(p.adjust(p), 2)` over every row, `p.signif` from raw p.
- **`geom_pwc` p-values are rstatix's**, not ggpubr's: every p is `signif(p, 3)`;
  a family with more than 2 levels goes through `pairwise_*` (combn order
  `(1,2), (1,3), (1,4), (2,3)`), adjusts the rounded p and then `p_round`s p and
  p.adj (3 decimals, `signif 3` below 0.001); when no family has more than 2 levels,
  ggpubr adjusts over the panel without rounding. `p.format` is
  `format.pval(eps = 1e-4)` with `"<0.0001"`. Labels go through
  `gsub("=+(\\s+)?<", "<\\1")`.
- Heights use the y range at stat time (the data, not the brackets); `"all"` puts
  every label at one height; `legend.var` keeps the stacking order legend-major.
- Dodge: ggpubr's 0.8 fits `ggboxplot`, not ggrust's boxes (0.95): default 0.95.
- R's `layer_data` of `geom_pwc` stacks left tips, bars, right tips; ggpubr's
  `geom_bracket` (stat_pvalue_manual) gives each bracket its three rows in turn.

2026-11-13 (r / slope labels, ggpmisc 1.0 + ggpubr 0.6.3 checked in a scratch `R_LIBS`):

- ggpmisc's `stat_poly_eq` has **no** `r.label` (`use_label("R")` errors there; only
  `stat_correlation` computes it) and no slope label: `"R"` and `"slope"` parts are
  ggrust extensions, linear formula only, decimals (not significant digits).
- Text computed variables are `Computed` variants holding a discrete column;
  `labels_per_group` takes the variable list and the closure returns
  `[label, one text per variable]`. A `..name..` mapping on `label` then just
  replaces the stat's `label` column (build.rs `stat_mappings`).
- ggpubr text-mode `rr.label` is `"R2 = 0.99"`; ggrust writes `R²` (the plotmath look).
