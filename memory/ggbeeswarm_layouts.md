# ggbeeswarm layouts: stats with the plot ranges, R density, exact fixtures

Step ex1.2 (2026-11-04). What works and what to reuse:

- `geom_beeswarm` / `geom_quasirandom` are point geoms (`geom/beeswarm.rs`) over stats
  `beeswarm` / `quasirandom`. Not positions: ggbeeswarm sizes points by the trained scale
  ranges (1/100 of each range, a discrete axis counts levels), and only stats see them
  (`StatContext`). Dodging is the stat's `dodge_width` (lets-plot `dodge()` reused) because
  each group needs its own swarm; Python turns `position_dodge(w)` into it.
- Stats need `LayerData::discrete_axes()`; build.rs now sets the flags before
  `stat.compute` as well as after (no other stat reads them).
- `stat/r_density.rs` is R's `density()` (linear binning, kernel sum = R's FFT, R >= 4.4
  coordinates: kernel spacing = bin spacing; the pre-4.4 `2(up-lo)/(2n-1)` is wrong now).
  Matches R to 1e-9. Use it whenever a port needs R's density, not the lets-plot one.
- The swarm (beeswarm `swarmx`, `fast=TRUE` C and R code agree) and compact swarm are exact
  from the R algorithm; ties: candidate order 0, then +, then -, first minimum wins.
- Reference fixtures from R: round data (e.g. 6 decimals) before `write_json`, whose
  `digits = NA` keeps 15 significant digits. Unrounded data moved a point across a
  `center`/`square` row break and looked like an algorithm bug.
- Quasirandom width after dodging is 0.4 times the *dodged* x resolution (ggplot2 loses
  `mapped_discrete` in `collide`), so groups fit their dodge slots.
- ggbeeswarm 0.7.2 `varwidth` is a no-op bug; ggrust follows vipor's documentation.
- Swarm speed: try candidates sorted by (|offset|, R candidate order) and stop at the first
  free one; test each only against neighbours within one diameter of its offset. Exact,
  and 40k points in one swarm went from 11.7 s to 1.9 s. One swarm of 100k is still
  about 12 s (swarm) / 19 s (compactswarm): a neighbour is 1% of the range, so the work is
  about n x n / 100.
