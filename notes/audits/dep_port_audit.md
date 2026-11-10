# Dependency port audit (cellestial + ggrust)

Read-only audit, 2026-11-08. Nothing in either repo was changed. cellestial was read
from its working tree (`dev`, with the user's WIP in `layers/stream.py`); reference
sources were read from the poetry venv `cellestial-zhk8JeAQ-py3.14` (scvelo 0.3.4,
scikit-image 0.26.0, scanpy 1.12.1, scipy, scikit-learn 1.8.0).

## 1. Declared dependencies

cellestial `pyproject.toml`:

| Group | Packages |
|---|---|
| core | anndata, ggrust, numpy, pandas, polars[pyarrow], **scikit-image**, scipy, **tqdm**, mudata |
| `spatial` extra | spatialdata (pulls shapely, geopandas, dask, xarray) |
| `extras` extra | matplotlib, scanpy[leiden], **scvelo** |
| dev | scanpy, scvelo, pycairo, cairosvg, pooch, spatialdata, ... (not shipped) |

ggrust `pyproject.toml`: runtime deps are only `polars`. numpy, pandas, pyarrow are
lazy, optional imports (`_data.py:64`, `plot_file.py:259,506`, `plot/series_meta.py`,
`plot/geom_imshow.py`). Nothing to port on the ggrust side.

## 2. Third-party use in cellestial (every import checked)

| Package | Used for (file:line) | Class |
|---|---|---|
| anndata, mudata | container types everywhere (`frames/_container.py`, `util/utilities.py`, `datasets`) | Keep |
| numpy | array glue everywhere | Keep |
| pandas | `frames/_container.py:8..103` isinstance checks and `pd.Index` on anndata `.obs`/`.var` | Keep (anndata needs it anyway) |
| scipy.sparse `issparse` | `frames/_container.py:224,398`, `frames/operations.py:27..52` | Keep (anndata needs scipy) |
| scipy.stats `gaussian_kde` | 2D: `layers/outline.py:61`; 1D (violins): `single/heatmap/utilities.py:848` | Port candidate (see B) |
| scikit-image `measure.find_contours` | `layers/outline.py:34,77` (only use of skimage in the repo) | **Port candidate (A2)** |
| scvelo `compute_velocity_on_grid` | `layers/stream.py:228,240` (only scvelo use) | **Port candidate (A1)** |
| tqdm | `datasets/datasets.py:248,273` download progress bar | Port candidate, trivial (C) |
| shapely | `spatial/utilities.py:205..210` `get_coordinates`, `count_coordinates` | Keep (comes with spatialdata) |
| xarray | `spatial/utilities.py:177` isinstance check | Keep (comes with spatialdata) |
| spatialdata | `spatial/utilities.py:41`, `frames/build.py:19` | Keep (optional extra, data source) |
| scanpy `tl.dendrogram` | `util/dendrogram.py:49` (runtime) | Unsure (D) |
| scanpy `tl.rank_genes_groups` | `single/differential/utilities.py:115` (runtime fallback); the other 4 hits are docstring examples | Keep (statistics, large) |
| scanpy `tl.pca` | `single/quick/elbow.py:120` | Keep (SVD solver details, not bit-exact portable) |
| scanpy `datasets.*`, squidpy `datasets.*` | docstring examples only (`util/markers.py:150`, `spatial/spatial.py:253`, ...) | Keep (docs only) |
| matplotlib | no `import matplotlib` left in `cellestial/`; only a cache dir in `tests/conftest.py:12`. Reaches the install only via the `extras` group (scvelo, scanpy) | Falls away with A1 + removing scanpy from extras |

Almost all `import scanpy as sc` hits are in docstring `jupyter-execute` examples; the
runtime scanpy calls are exactly `dendrogram`, `rank_genes_groups`, `pca`.

## 3. Ranked port candidates

### A1. scvelo `compute_velocity_on_grid` (rank 1: highest value)

- **Used at** `cellestial/layers/stream.py:240`, with `autoscale=False,
  adjust_for_stream=True`. That path never calls `quiver_autoscale` (the one function
  that opens a matplotlib figure), so no matplotlib is needed.
- **Reference**: `scvelo/plotting/velocity_embedding_grid.py:28..~110`, about 80
  lines of numpy. Pieces:
  1. drop non-finite rows (trivial);
  2. per-dimension `linspace` over `[min - 1% range, max + 1%]`, `int(50*density)` points,
     `meshgrid`, flatten (trivial; reproduce `np.linspace` formula exactly);
  3. `NearestNeighbors(n_neighbors=int(n_obs/50)).kneighbors(X_grid)`: exact kNN of
     2500 grid points (default density 1) against the cells. Needs a kd-tree or brute
     force in Rust, about 60 to 100 lines. Exact results; only distance ties could
     order differently from sklearn, and ties are essentially absent for float
     embeddings;
  4. Gaussian weights `norm.pdf(dists, scale=0.5*mean grid step)` (one formula),
     weighted mean of neighbour velocities, divide by `max(1, mass)`;
  5. stream branch: mass cutoff `10**(min_mass-6)` clipped to `0.9*max`, plus the
     `np.percentile(length, cutoff_perc)` cutoff (linear-interpolation percentile,
     ~10 lines), set masked `V_grid[0]` to NaN, reshape to `(2, ns, ns)`.
- **Size**: about 150 lines of Rust plus a ~60 line Python wrapper (mirrors
  `streamplot.rs` at 803 lines / `streamplot.py` at 263 lines, so this is a fraction
  of that session). Natural home: `streamplot.rs` neighbour module, exposed as
  `ggrust.velocity_grid(...)` or folded into a `ggrust.stream_from_points(...)`.
- **Parity**: near-exact. Float sums differ in last bits depending on accumulation
  order (numpy sums the neighbour axis sequentially, which Rust can match with a plain
  loop; the pdf uses `exp`, so allow ~1e-12 relative error). The streamplot port
  was already bit-exact vs matplotlib, so the only drift is this ~1e-12 on grid
  inputs. A parity test against scvelo on the pancreas data settles it.
- **Weight shed**: large. `scvelo` is an `extras` dependency purely for this
  function, and it pulls in scanpy, umap-learn (numba, pynndescent), scikit-learn,
  loompy, matplotlib, plus whatever scanpy needs (seaborn, statsmodels, patsy,
  h5py, networkx, joblib, session-info2, ...). Dropping scvelo from `extras` removes
  the whole tree for users who only want cellestial plots. scanpy stays only if the
  user still wants dendrograms / rank_genes_groups / pca through it (they are
  runtime-optional calls; see D). matplotlib then leaves `extras` too.
- **Value / effort**: very high / low. **Do this first.**

### A2. scikit-image `find_contours` + scipy 2D `gaussian_kde` for cluster outlines (rank 2)

- **Used at** `cellestial/layers/outline.py:61` (kde), `:77` (find_contours) in
  `cluster_outlines`: evaluate a 2D KDE on a 200x200 grid, contour at
  `level * max`, map indices back to data coordinates.
- **scikit-image is a core (non-optional) dependency used only here.** Its tree:
  numpy, scipy, networkx, pillow, imageio, tifffile, packaging, lazy-loader
  (networkx, imageio, tifffile, lazy-loader would leave the install; pillow and
  packaging may stay through other packages).
- **Reference**: `skimage/measure/_find_contours.py` (218 lines, mostly docstring)
  plus `_find_contours_cy.pyx` (about 150 lines: marching squares with the
  `fully_connected='low'`, `positive_orientation='low'` defaults, and segment
  stitching). gaussian_kde: `scipy/stats/_kde.py` (733 lines, but the used path is
  about 40: Scott factor `n**(-1/(d+4))`, data covariance times factor squared,
  Cholesky, evaluate `mean of exp(-0.5 * mahalanobis)`).
- **Size**: about 250 to 350 lines of Rust (marching squares ~200, KDE ~60, wrapper).
- **Parity**: feasible exactly for the contour geometry (deterministic, same
  interpolation `fraction` formulas, same path ordering if the stitching is copied
  line by line). The KDE agrees to ~1e-13 (different summation order; harmless since the
  contour is then thresholded at a linear interpolation).
- **Do not reuse the existing `stat/contour.rs`** as-is: it splits cells into four
  triangles around the centre (lets-plot's algorithm), so its saddle handling and
  vertex order differ from skimage's. Either port skimage's algorithm as its own
  function, or accept a visible-but-minor difference and record it in
  `deviations.md`. `plan.md` step 10.6a (`contour_var`, `density2d`) already
  mentions "lets cellestial drop scipy KDE + skimage marching squares"; that route
  gives a different outline (triangle method, product kernel with per-axis nrd0
  bandwidth rather than a full-covariance Scott KDE), so it would need a decision from
  the user. A dedicated `ggrust.density_outline(x, y, ...)` exact port is simpler for
  cellestial.
- **Weight shed**: removes scikit-image from the mandatory install (the only
  non-scipy heavy compiled dependency in core). scipy stays (anndata).
- **Value / effort**: high / medium.

### B. scipy 1D `gaussian_kde` for heatmap violins (rank 4)

- `single/heatmap/utilities.py:848`, Scott bandwidth `n**(-1/5)` times the sample
  std (ddof=1), grid evaluation. About 30 lines. Exact up to summation order.
- Sheds nothing (scipy stays through anndata), so value is only speed and a tidier
  dependency story. ggrust already has `stat/density.rs` (R `bw.nrd0`), which is a
  different bandwidth rule: do not substitute it without a deviation. Low priority;
  could be bundled with A2 since the KDE code is shared.

### C. tqdm download progress bar (rank 3, tiny)

- `datasets/datasets.py:248,273,..`. A ~15 line stderr progress printer
  replaces it. tqdm is small (pure Python), so the install saving is small, but it
  removes a mandatory dependency for almost no work. Notebook widgets are lost
  (tqdm.auto is not used; plain `tqdm`). This belongs in cellestial's Python, not
  ggrust: no Rust port needed.

### D. scanpy `tl.dendrogram` (unsure)

- `util/dendrogram.py:49`. scanpy computes the group means (optionally in PCA
  space), a correlation distance, `scipy.cluster.hierarchy.linkage`
  (average), then orders leaves. Port is about 150 to 250 lines (hierarchical
  clustering with the exact scipy tie-break/ordering rules plus
  `optimal_leaf_ordering` if enabled; reading scanpy's `_dendrogram` would settle that).
  ggrust already has the `ClusterTree.from_linkage` consumer side.
- Does not shed scanpy as long as `rank_genes_groups` / `pca` are still delegated to
  it, and those are not good port candidates. Leave it unless scanpy is to be
  removed from the install entirely.

## 4. Not worth porting

- `rank_genes_groups` (t-test, wilcoxon, logreg; large statistics), `pca` (ARPACK /
  randomized SVD, not reproducible bit-for-bit across solvers).
- pandas, anndata, mudata, scipy.sparse, polars, numpy: data structures, required by anndata.
- shapely / xarray / spatialdata: optional `spatial` extra, data source.
- squidpy, scanpy datasets: documentation only.

## 5. Recommended order

| # | Item | Effort | Sheds |
|---|---|---|---|
| 1 | A1 `compute_velocity_on_grid` -> Rust (`ggrust.velocity_grid` or similar) | ~150 Rust + 60 Python lines, 1 session | scvelo, thereby umap-learn, numba, scikit-learn, loompy, matplotlib out of `extras`; scanpy no longer needed for the stream layer |
| 2 | A2 KDE + marching squares exact port for `cluster_outlines` | ~300 Rust lines, 1 session | scikit-image (mandatory dep) and networkx, imageio, tifffile, lazy-loader |
| 3 | C tqdm removal | trivial, cellestial only | tqdm |
| 4 | B 1D gaussian_kde (bundle with A2) | ~30 lines | nothing in the install |
| 5 | D dendrogram | medium | nothing unless scanpy is dropped completely |

After 1 to 3, cellestial's mandatory install is anndata, mudata, ggrust, numpy,
pandas, polars, scipy; scanpy stays an optional convenience (dendrogram,
rank_genes_groups, pca); `extras` could shrink to scanpy alone. This audit treated
anything not imported in `cellestial/` or `python/ggrust/` as out of scope; the
cellestial `dev` working tree was not modified.
