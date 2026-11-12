# ggplot2 validation suite (`tests/test_ggplot2_build.py`)

R's `ggplot_build()` output for 35 fixed plots is checked in
(`tests/fixtures/ggplot2_build_reference.json`); the test compares ggrust's `build()`
against it without R. Every known difference is listed in `KNOWN` with a reason that
quotes its `deviations.md` row; a case fails on a new difference and on one that went
away (so the list stays exact).

**Works:**
- Read the same CSVs on both sides (`python/ggrust/data`). R must read them with
  `na.strings = c("", "NA")`: polars reads empty cells as null, R as `""`, and
  `complete.cases` then keeps rows polars drops (penguins `sex`: 4 Gentoo rows, a
  median off by 50 g looked like a quantile bug).
- Compare discrete positions by category name, not index: ggrust orders strings by
  first appearance, ggplot2 sorts them. Map ggrust's slot to ggplot2's level index and
  keep the dodge offset.
- Match rows by sorting, not by group number (group ids and order differ).
- JSON has no infinity: R's `-Inf`/`Inf` (ecdf padding) arrive as null; turn ggrust's
  infinities into None too.
- `stats::density()` is an FFT approximation (about 1e-3 of the peak); ggrust sums
  exactly. Density cases use a 2e-3 tolerance.
- Before calling a difference "inherited", check lets-plot itself: the bar parity
  helpers in `tests/test_bar_parity.py` (`_both`, `_lets_plot_bars`, `_ggrust_bars`)
  compare rectangles with lets-plot's SVG in a few lines.

**Traps:**
- Bars with `fill=` on mpg differed from lets-plot too, but only because lets-plot
  orders the categories by its stat output (deviations row "Discrete x order with
  groups"); stack order and dodge slots themselves match lets-plot. Compare
  rectangles as sets before suspecting the position code.
- Histograms bin `[low, high)` in ggrust and lets-plot (a value on a break goes to
  the bin above; checked against lets-plot's SVG on mpg). ggplot2 bins `(low, high]`.
- Polynomial and loess smooths average y over tied x first (lets-plot `averageByX`),
  so on iris/mpg they differ from `lm(y ~ poly(x, 2))` by up to 0.3 units.

Regenerate after adding a case on both sides:
`Rscript tests/fixtures/generate_ggplot2_build_reference.R` (from the repo root).
