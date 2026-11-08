# density_outline is a scipy gaussian_kde + skimage find_contours port

`ggrust.density_outline()` (`crates/ggrust-core/src/density_outline.rs`) reproduces the
recipe "gaussian_kde on a padded linspace grid, find_contours at level * max, np.interp
back to data". `kde.rs` (`GaussianKde<D>`, any dimension: reuse it for 1D violins) and
`marching_squares.rs` are general; `numeric.rs` holds numpy-order helpers (pairwise sum,
linspace, interp) shared with `velocity_grid.rs`. Fixture:
`tests/fixtures/density_outline_scipy.json`, regenerate with
`tests/fixtures/generate_density_outline_reference.py` in cellestial's poetry venv.

Kept standalone (not a `stat_density2d` option): that stat follows lets-plot (R product
kernel bandwidths, layer-range grid, triangle isolines); a second kernel and contour engine
behind options would have risked its parity for no shared code.

What it takes to match scipy 1.17 on arm64 macOS (measured with probes):

- **Memory layout changes numpy's sums.** `np.average` over `points.T` is pairwise when
  `points.T` is C-contiguous (polars `to_numpy()` is F-ordered, so cellestial hits this)
  and sequential when `points` is C-ordered. ggrust follows the contiguous order; the
  fixture generator uses `np.asfortranarray`. A C-ordered reference differs by ~1e-15.
- `np.cov`'s `dot(X, (X * w).T)` is a sequential FMA dot per entry; `c *= 1 / fact`.
- `linalg.cholesky`: `l10 = c10 * (1 / l00)`, `l11 = sqrt(fma(-l10, l10, c11))` (lower
  triangle read). `solve_triangular`: multiply by `1 / L_kk`, subtraction NOT fused.
- Kernel (Cython): `arg = fma(r, r, arg)`, `estimate = fma(w, exp(-arg / 2) * norm,
  estimate)`, points in order per query; `norm = pow(2 pi, -d/2) / L_00 / L_11`.
- `np.interp` is `fma(slope, x - xp[j], fp[j])` (clang contraction), exact `fp[j]` on hits.
- Not reproduced: `neff = 1 / np.vecdot(w, w)` is an Accelerate `ddot` whose order no
  lane/tree variant matched (71/84 n at best). Scott/Silverman factors can be 1 ulp off,
  vertices then differ ~1e-14 of the span; a numeric `bandwidth` is bit-identical.
- float32 input (pbmc3k `X_umap`): numpy keeps `linspace` in float32, so cellestial's
  current outlines differ from float64 ones by ~1e-5 UMAP units (same contours and
  vertex counts). Known gap, as in velocity_grid.

Bug-hunt (2026-11-08): marching squares bit-identical to skimage on 6000 random small grids
(saddles, exact ties, NaN). Known and kept: a `level` that lands exactly on a grid value can
flip `value > level` through the 1-ulp `neff` difference (10/1334 constructed ties);
near-singular C-ordered input can pass scipy's Cholesky and fail ggrust's (layout rounding).
Overflowing covariance, padded axes and underflowing bandwidths now raise `SpecError`.
