# Dendrograms: R hclust results, trees beside discrete axes

Step ex1.9 (2026-11-04). Design agreed with the user: legendry names, precomputed tree,
axis band or opposite band.

- **R parity needs R's exact choices.** Nearest-neighbour list scanned from item 0 with
  strict `<` (ties go to the lowest pair), the merged cluster keeps the lower index,
  ward.D2 squares the distances first and reports `sqrt`. Merge rows: singleton first,
  else lower step first. Order = depth-first, first side before second.
- **Correlation ties need R's `cor()` arithmetic.** Integer data give exactly tied
  correlations only when computed the same way: mean corrected by the mean residual,
  sums / (n - 1), cov / (sd_x * sd_y). A unit-vector dot product broke R parity on the
  `ties` fixture. (On arm64 R has no long double; x86 R may differ in the last bits.)
- **Primary input is scanpy's tree** (`ClusterTree.from_linkage(uns[key]["linkage"], obs[group].cat.categories)`): order = depth first, `Z[i, 0]` before `Z[i, 1]`, which is scipy `dendrogram()`'s `leaves` / scanpy's `categories_ordered`; icoord/dcoord match exactly (fixture via `uv run --with scipy`, scipy is not a dependency). `hclust()` is the fallback.
- **The tree orders the axis by setting the discrete limits** in `scale/spec.rs`; explicit
  limits conflict and raise. Data levels missing from the tree raise in `build::axis_scale`.
- **The drawing maps leaves through `effective_levels()`** (not the tree order), so
  `reverse` and `coord_flip` (screen scales) just work.
- **Bands are part of the axis area** (`x_axis_band`, `y_axis_width`, `top_band`,
  `right_band` in `layout_with`), so gggrid alignment needed no change. Facets drop the
  tree with a warning (facet layout builds its own `PlotLayout`).
- **HTML zoom:** trees live in their own `dendrogram-x` / `dendrogram-y` groups (the
  zoom rewrites `axis-*` innerHTML), clipped by the engine to the panel span; the runtime
  wraps the paths in an inner `<g>` and sets a one-axis matrix (outer clip stays put),
  strokes non-scaling. Browser test: `tests/test_dendrogram_zoom.py`.
- **Leaves over runs of levels (10.5, `leaf_levels={leaf: [levels]}`):** the tree
  sets the limits to the runs end to end (`AxisDendrogram::ordered_levels`), and
  `draw` places each leaf at the midpoint of its run's first and last axis positions
  (a level -> position HashMap, so 50k cells stay linear). `leaf_levels: None` is the
  one-level-per-leaf case, so every path goes through `levels_of`. Checks live twice
  on purpose: Python fails early, Rust `leaf_levels_from_value` guards raw specs.
  Tests: `tests/test_dendrogram_leaf_levels.py`, goldens `dendro_runs_*`.
- Fixtures: `tests/fixtures/generate_hclust_reference.R` (jsonlite `auto_unbox` +
  `I()` so length-1 vectors stay lists). legendry and ggh4x installed into a scratch
  `R_LIBS` for the reference render.

See [[deviations-md]] rows "Hierarchical clustering", "geom_dendrogram", "Axis
dendrograms".
