# Boxplots follow lets-plot, with three ggplot2 corrections (Phase 2.2)

Parity tests: `tests/test_boxplot_parity.py` (shapes as panel fractions, drawing order).

- `geom_boxplot` is TWO layers in the spec: the box (`stat="boxplot"`) and a point layer
  (`stat="boxplot_outlier"`, `na_rm=True`, `show_legend=False`). The outlier stat emits a
  NaN placeholder row per x without outliers, so groups keep their dodge slots.
- Default position of both: `dodge(width=0.95)` (`DefaultPosition`; the outlier stat's
  default beats the point geom's identity, as lets-plot `GeomProto`).
- Box: width 0.9 x resolution (or `varwidth`: sqrt(n)/sqrt(max n), no 0.9), fill white,
  outline #474747 size 0.75; midline outline x `fatten` (2); fences `whisker_width` (0.5)
  of the box; draw order boxes, midlines, whiskers; alpha fades the fill only.
- Quantiles are taken in axis units (after a log transform), as lets-plot.

Deliberate differences (ggplot2 wins, per the user's rule):
- **Quartiles are R type 7.** lets-plot averages the two neighbours (Q1 of 1..6 = 2.5, not
  2.25). Parity data must use odd group sizes, where both agree.
- **`coef` reaches the outlier layer** (Python builder). lets_plot drops it, so with
  `coef=0.5` points between the fences vanish.
- **A mapping overridden by a constant still groups** (`build.rs` map_layer). lets-plot
  drops it: `outlier_fill="white"` over `aes(fill=g)` pooled the outliers of all g.

**How to apply:** violins (2.3) and jitterdodge (2.4) reuse the dodge slots and the
overridden-mapping grouping; check both before porting them.
