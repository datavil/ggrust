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
