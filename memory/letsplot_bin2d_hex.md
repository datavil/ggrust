# 2D bins (bin2d, binhex) follow lets-plot (lp5.2)

Parity tests: `tests/test_bin2d_hex.py` (fills + texts in 14 cases). Kotlin sources:
`Bin2dStat.kt`, `BinHexStat.kt`, `HexGeom.kt` (no local lets-plot checkout; fetch with
`gh api repos/JetBrains/lets-plot/git/trees/master?recursive=1` + raw.githubusercontent).

- **Spec**: lets_plot emits `geom="bin2d"` / `geom="hex"` with no stat; the geoms
  default to stats `bin2d` / `binhex`. `bins`/`binwidth` are `[x, y]` lists
  (lets-plot errors on a scalar; ggrust accepts one, as ggplot2).
- **Grids are not BinStat's**: range from `overallX/YRange`, widened by half a
  preliminary bin (`bins` only: span/count; hex: span/(count-1)), then recounted.
  bin2d walks x-major, hex y-major; odd hex rows shift +bw/2.
- **Hex assignment** is the exact port (coarse rect index, 6 neighbours, ray-cast
  polygon test in a y-scaled regular space, border -> lowest row then rightmost).
  Integer data hits borders: keep the port literal, nearest-centre shortcuts differ.
- **"Resolution hack" rows matter for axes**: with `drop`, empty corner cells
  (0,0), (0,1), (1,0) become NaN-padded rows whose other coordinate still trains
  the axis. Leaving them out changed hex x axes (an extra -4 label in lets-plot).
- **Sizes**: ggrust ships `..binwidth..`/`..binheight..` and uses them as the
  `"res"` unit (fixes #1290; deviation). Hex y span is `h*res/sqrt3` (lets-plot),
  drawn tips `2h*res/3`: slightly past the span, covered by default expansion.
- **Border ties depend on summation order**: lets-plot `getHexagonCenter` adds the odd
  row shift to `i*w` first (`start + w/2 + (i*w + w/2)`), while `computeBins` reports
  centres left to right. Keep both orders: a point exactly on a border (the max-x point
  of a range, often) otherwise lands in the neighbour hexagon. This, not the limit
  filter, was the "hex + ylim" difference: stat rows past limits are dropped in
  lets-plot and ggplot2 too (tested in `test_stat_rows_past_limits_match_lets_plot`).
- 1M points: 16 ms (bin2d) / 28 ms (hex) vs lets-plot 0.8 s / 1.8 s; no rayon.

**How to apply:** read before contours/density2d (lp5.3) or any 2D stat.
