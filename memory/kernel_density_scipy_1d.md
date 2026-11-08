# kernel_density is scipy's 1D gaussian_kde over GaussianKde<1>

`ggrust.kernel_density(values, at, bandwidth=)` (`crates/ggrust-core/src/kernel_density.rs`)
is `scipy.stats.gaussian_kde(values, bw_method=bandwidth)(at)` as an `(x, density)` frame,
a thin wrapper over the shared `kde.rs` (`GaussianKde<1>`, no second KDE). Fixture:
`tests/fixtures/kernel_density_scipy.json`, regenerate with
`tests/fixtures/generate_kernel_density_reference.py` in cellestial's poetry venv.

Kept standalone (not a `stat_ydensity` / `stat_density` option): those stats follow
lets-plot's R bandwidths (`bw.nrd0`, ...) and kernels; a scipy bandwidth mode would add a
second rule set behind one option for a caller (hand-placed violins) that needs the raw
density at chosen points, not a layer.

Parity (scipy 1.17, arm64 macOS):

- In 1D, `np.cov`'s `dot(X, (X * w).T)` is a (1, n) x (n, 1) product that numpy sends to
  Accelerate's `ddot`: `np.dot(a, b)` matched 40/40 probes, sequential FMA 3/40, no lane/tree
  variant better than 25/40 (as for `neff`, see [density_outline](density_outline_scipy_skimage_port.md)).
  Not reproduced: densities are bit-identical for tiny samples, otherwise ~1e-15 of the peak;
  relative error grows in the tails with the exp argument (5e-13 at 1e-6 of the peak on
  zero-heavy expression data). Tests use 1e-12 of the peak and 1e-12 relative above 1e-6 peak.
- **Offset data amplifies that ulp.** Points and queries are whitened (`x / L`) before
  subtracting, so a 1-ulp bandwidth difference moves `x / L` by about `|x| / sd` ulps:
  `normal(1e6, 1)` differs from scipy by 7e-11 of the peak (2e-9 relative), `1e9` by 7e-8.
  Both are equally far from an exact reference; documented in the docstring, not fixed
  (scipy parity forbids centering first). Another lane/tree/tail probe of the ddot: best 14/60.
- float32 samples: scipy promotes them to float64 exactly before any arithmetic (np.cov
  dtype, `_get_output_dtype`), so `tolist()` input matches; unlike density_outline there is
  no float32 linspace in the recipe (the caller passes `at`).
- Constant values: scipy raises `LinAlgError`; ggrust raises `SpecError` at `values`
  ("all equal"), so callers catching `LinAlgError` must catch `SpecError` (a `ValueError`).
