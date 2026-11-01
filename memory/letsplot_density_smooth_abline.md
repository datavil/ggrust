# geom_density, geom_smooth, geom_abline follow lets-plot (with three recorded deviations)

**Works:** port the stats line by line from lets-plot (`DensityStat`, `SmoothStat`,
`LoessRegression`, `PolynomialRegression`) and compare paths against lets-plot SVG:
curves agree to 0.005 px (smooth) and labels/vertex counts agree (density).

**Traps:**
- lets-plot repeats a path's first vertex (81 points for n=80): dedupe consecutive
  duplicates before comparing.
- Loess on more than `max_n` rows samples with Kotlin's `Random(seed)`:
  `KotlinRandom::next_int_until` (power-of-two top bits, else rejection on `next_int>>>1`)
  and the pick-vs-drop branch at size <= len/2 both matter for identical curves.
- lets-plot computes the smooth band with the straight-line standard error for every
  method. The user chose ggplot2's per-model SE (2026-11-01): polynomial via the
  orthogonal basis (`X'X` diagonal, matches R `predict(lm(y ~ poly(x, d)), se.fit=TRUE)`
  to 1e-12); loess as a linear smoother (the Apache fit and the spline are both linear
  in y), `s |l(x0)|`, `s^2 = RSS/delta1`, df `delta1^2/delta2` (R `predict.loess`),
  delta2 exact up to 500 points. Compare polynomial/loess curves with lets-plot using
  `se=False` (the band changes the y axis).
- Smooth band alpha 0.15 must not reach the legend key line, or the key is invisible.
- lets-plot draws density quantile bands as separate polygons, which leaves faint seams
  in PNG (lets-plot #985). ggrust splits bands only when the fill or alpha varies.
- `geom_abline()` without arguments draws y = x (ggplot2); lets-plot draws nothing.
- Mapped `intercept`/`slope` are used as they are (no scale), like lets-plot.
