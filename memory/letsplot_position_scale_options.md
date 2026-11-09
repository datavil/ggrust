# Positional scale options, linewidth and stroke scales (lp5.10)

Measured with lets_plot 4.11 `to_svg()` and R ggplot2 (2026-11-09).

- **`break_width` (numbers) = fixed breaks.** lets-plot draws `break_width=w` exactly
  as `breaks=[every multiple of w in the expanded domain]`, in transformed units
  (log10 + 2 = every other power of ten), listed in ascending data order. What looks
  like widening (0..100, w=1 shows -5, -2, 1, ...) is only label thinning of crowded
  fixed breaks. So: generate the multiples in `ContinuousPositionScale::breaks`, then
  reuse `fixed()`; `has_fixed_breaks()` must say true. Compare against lets-plot's
  own `breaks=[...]` output when a case is crowded: ggrust turns crowded fixed labels
  vertical where lets-plot thins them (pre-existing gap).
- **`lablim` is discrete only** in lets-plot, positional and style scales alike:
  first n characters + `...`, applied after `format`/`labels`, fractions truncate,
  0 = no limit. Continuous scales ignore it silently. Shared helper:
  `scale::format::cut_label`.
- **Positional `na_value`:** lets-plot documents it and ignores it (rows dropped;
  a discrete level name crashes it). ggplot2 draws missing and limit-censored
  positions at `na.value` and trains the axis on it. ggrust follows ggplot2 for
  continuous axes in `PositionScale::positions` (before stats).
- **linewidth / stroke:** lets-plot range 1..7 units (x 2.2 px), missing 1 unit,
  discrete levels evenly over the range. `na_value` on a default (no `range`)
  numeric scale is ignored with a warning, as for size.
- **Shape 19 and stroke (#1158):** in R, 19 is 21 filled in its color (same outer
  size for any stroke); 16 and 15..18 only grow a little (font size). ggrust grows 19
  and 20 by the outline; 16 stays lets-plot so default points do not change.
- **#769:** lets-plot crashes when style limits exclude all data; ggrust gives
  `na_value`.
