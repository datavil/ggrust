# velocity_grid is a bit-exact scVelo port

`ggrust.velocity_grid()` (`crates/ggrust-core/src/velocity_grid.rs`) ports scVelo 0.3.4's
`compute_velocity_on_grid`. On float64 input it is bit-identical to scVelo on arm64 macOS
(fixture cases and pancreas: same grid, same NaN mask, same 40 streamlines). Fixture:
`tests/fixtures/velocity_grid_scvelo.json`, regenerate with
`tests/fixtures/generate_velocity_grid_reference.py` in cellestial's poetry venv (has scvelo).
The test allows 1e-12 relative so other platforms pass.

What it takes to stay exact:

- sklearn's kd-tree distance is `sqrt(fma(dy, dy, dx * dx))`: clang contracts
  `tmp += d * d` on arm64. A plain `dx*dx + dy*dy` left ~8% of distances 1 ulp off.
  Rank neighbours by the plain squared distance (fast), take the FMA distance for weights.
- `weight.sum(1)` (contiguous axis) is numpy's pairwise sum from 0, not first + pairwise.
  `(V[neighs] * w[:, :, None]).sum(1)` and `np.mean(..., axis=1)` over the strided middle
  axis are plain sequential sums.
- `np.percentile` linear: virtual index `(n - 1) * (q / 100)`, `_lerp` from the nearer end.
- `M` widens from the already widened `m` (`M + 0.01 * |M - m_new|`).
- scVelo hides stream points through `V_grid[0]` (u) only.

Not ported: scVelo computes in the input dtype; cellestial passes float32 UMAP coordinates
(pancreas `X_umap` is float32), so its grid differs ~1e-6 from ggrust's float64 (same
lines and vertex counts, vertices within 3e-4 UMAP units). Brute-force kNN with rayon is
5x faster than scVelo at 100k cells; no kd-tree needed so far.

Known differences kept on purpose (bug-hunt 2026-11-08): sklearn uses brute-force distances
(`|x|^2 - 2xy + |y|^2`) when `n_neighbors >= n_points // 2`; ggrust keeps exact ones (up to
2e-3 off with offset coordinates, never with the default `n_points / 50`). A flat axis is
accepted in quiver mode only (scVelo's stream branch fails on it too). Density is capped at
1e7 grid points (scVelo would just run out of memory).
