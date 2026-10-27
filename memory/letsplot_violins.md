# Violins follow lets-plot, bandwidth follows R (Phase 2.3)

Parity tests: `tests/test_violin_parity.py` (path points as panel fractions; cases pass a
numeric `bw`). R + ggplot2 4.0.1 are installed: `Rscript` gives ground truth.

- Density per group AND per distinct x, on `n` (512, max 1024) points; trimmed by default;
  untrimmed grids reach `tails_cutoff` (3) bandwidths past the data, or the overall y
  range of every layer (`StatContext.y_range`) when `tails_cutoff=None`.
- Above `fs_max` (5000) values only kernels within 5 bandwidths are summed.
- `..quantile..` bands come from cumulative kernel sums on the grid; at a band change
  the previous point is repeated under the new band, so bands share their edge.
- `violinwidth` is normalised over the WHOLE layer (area: density / max density).
- Drawing: per violin a filled band (no outline) + two open side lines; split by band
  only when fill/colour vary along the violin; quantile lines at the top point of each
  band; `show_half` puts the other side on the centre line.
- lets-plot writes paths as `M p L p ...` (first point twice): ignore it in parity.
- **Deliberate difference:** bandwidth is R's `bw.nrd0`/`bw.nrd` (sample sd, type 7 IQR,
  fallbacks sd -> |x[1]| -> 1). lets-plot uses the population sd.
- Stats run groups (and violin x bins) in parallel with rayon, merged in order:
  1M rows, 10 violins: ~60 ms vs lets-plot ~5.4 s.

**How to apply:** `densityridges` (2.5) reuses `stat/density.rs`; check this first.
