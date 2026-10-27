# Bars, stats and positions follow lets-plot (Phase 2.1)

Measured against `to_svg()`; parity tests in `tests/test_bar_parity.py`.

- **Stats run per group**, groups in first-appearance order; other columns are carried
  as the group mean (numbers) or first value (categories). Bins are laid out once over
  the whole layer's x range (0.7 bin of slack each side, anchor shift <= half a bin).
- **Histogram breaks bin as `(low, high]`**, the first bin closed (`bracketingIndicesOrNull`):
  a value on an inner break goes to the LEFT bin.
- **Stack**: groups sorted by group index, positive and negative stacks apart;
  `mode="groups"` (default) means rows of one group at one x take their max, not a sum.
  `vjust` moves the whole bar, so bars can dip below 0 and get clipped (lets-plot too).
- **Dodge**: slot = group index, `(v + (slot - (n-1)/2) * resolution * width - x) / n + x`;
  width from the option, else the row's `width`, else 1 (points).
- **Axis ranges are lets-plot's dry-run ranges**: position-adjusted x/y plus the
  pre-position width span (`Geom::setup_aesthetics` columns never train after the
  position). Training on the moved rectangles widens dodged axes with narrow bars.
- **Zero-based y** (`rangeIncludesZero`): 0 joins the range and the zero end is not padded.
- Draw rows in reverse; zero-size rects become 0.1 px; alpha fades the fill only.
  Bar: fill #118ED8, outline white, size 0.75. Histogram: fill and outline #474747, size 0.5.
  Key: cell 23, rect inset 1 px + half the outline.
- **Discrete axis order with a stat: deliberate difference.** lets-plot orders levels by
  first appearance in the STAT OUTPUT (groups concatenated), so adding `fill` reorders x.
  ggrust keeps the data's level order, as ggplot2 does (levels trained before the stat).

**How to apply:** read before touching boxplot/violin/jitter positions or any stat.
