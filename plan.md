# ggrust plan

Rust plotting engine with a lets_plot-compatible Python API, built for cellestial.
Rules of engagement: [AGENTS.md](AGENTS.md). Lessons: [memory/](memory/README.md).

## Decisions (agreed with the user)

| Topic | Decision |
|---|---|
| Python API | lets_plot-compatible subset. cellestial port = import swap |
| Interactivity | Rust renders everything; tiny embedded vanilla-JS runtime for tooltips + zoom/pan; canvas for dense point layers |
| Build | maturin (PyO3 wheel) + uv |
| Repo | `datavil/ggrust`, import name `ggrust` |
| Data transfer | Arrow PyCapsule interface (polars, pandas, pyarrow, dict of numpy), zero-copy where possible. No JSON for data |
| Spec transfer | Spec (without data) crosses as a JSON string parsed by serde_json: tiny, one parser for fixtures and live calls, no pythonize/pyo3 version lock |
| Fonts | One embedded open font so text measurement and PNGs are identical on every machine |

## Scope: what cellestial actually uses

Counted from `cellestial/` source (calls per symbol). This drives priority.

- **Spec objects:** `aes` (130), `PlotSpec`/`LayerSpec`/`FeatureSpec`/`FeatureSpecArray`/`SupPlotsSpec`, `as_dict()`, `+`, `gggrid` (19), `ggsize` (11), `labs`/`xlab`/`ylab`/`ggtitle`
- **Theme:** `theme` (31), `element_blank` (46), `element_text` (36), `element_rect`, `element_line`, `theme_classic`, `theme_bw`, `theme_void`; params `legend_position`, `axis_text_*`, `axis_title_*`, `axis_line`, `panel_background`
- **Interaction:** `layer_tooltips` (26), `ggtb` (25)
- **Geoms:** point, path (9), segment (7), text (6), tile, boxplot, violin, raster, rect, polygon, vline, hline, line, bar, histogram, label, text_repel, label_repel, bracket, area_ridges, imshow, function, blank
- **Positions:** dodge, jitterdodge (+ identity, stack, jitter implied)
- **Scales:** x/y continuous (21), x/y discrete, `scale_y_log2`, `scale_y_reverse`, color/fill: brewer (11), viridis (13), hue (6), gradient, gradient2, gradientn, manual, continuous; `scale_size`; `guides`/`guide_legend`
- **Coords / facets:** `coord_fixed`, `coord_flip`, `coord_cartesian`, `facet_wrap`
- **Output:** `_repr_html_`, `to_svg`, `to_png`, `to_html`, `ggsave`, `LetsPlot.setup_html`

Not needed (skip): livemap / GIS / geocoding, Kotlin/JVM/Compose/Swing/Batik, `bistro` (corr_plot, qq_plot, ...),
waterfall, pie, contour/density2d, smooth/regression, polar coords, sampling UI, frontend contexts beyond Jupyter + standalone HTML.

## Must do

1. lets_plot-compatible Python spec layer (classes, `as_dict()`, `+`, module paths) with `as_dict()` parity tests against lets_plot.
2. Arrow data bridge (polars first, pandas, numpy dict), categorical / string / numeric / null handling.
3. Core pipeline: aesthetic mapping, stats, positions, scale training, coord, scene.
4. Scales: continuous, discrete, log2/log10, reverse; color: hue, brewer, viridis, gradient(2,n), manual, identity; size; nice breaks + labels.
5. Geoms + stats listed in Scope (point, path, segment, text, tile, raster, rect, polygon, vline/hline, line, bar/histogram, boxplot, violin, area_ridges, label, bracket, imshow, blank, function, text/label repel).
6. Layout: axes, ticks, grid, titles, legends (discrete + colorbar; position right/bottom/none; horizontal), `ggsize`, theme elements, `facet_wrap`, `gggrid` (ncol, widths/heights, align, shared guides).
7. Themes: classic (cellestial default look), bw, void, grey/minimal base.
8. Renderers: SVG, PNG (tiny-skia + resvg text), HTML (standalone + notebook).
9. Interactivity: tooltips with `layer_tooltips` (lines, format, title), zoom/pan toolbar (`ggtb`).
10. Visual golden-image test suite + side-by-side lets-plot comparison report.
11. Benchmarks vs lets-plot (spec build, render SVG/PNG/HTML, memory) at 10k / 100k / 1M points.
12. cellestial runs its full test suite on ggrust.

## Should do

1. Parallelism with rayon: per-layer, per-panel, per-subplot, per-group stats (KDE for many violins).
2. Canvas/WebGL point layer in HTML for >50k points; binary (base64 typed array) payload.
3. Caching: reuse Arrow conversion and computed stats when the same frame feeds many subplots (`gggrid` of 20 UMAPs).
4. PDF output (svg2pdf) for publication figures.
5. `facet_grid`, `geom_density`, `geom_errorbar`, `geom_jitter`, `scale_*_sqrt`, `coord_trans`-like transforms.
6. Optional lets-plot-like default sampling (behind a flag; ask user about default).
7. Good error messages that name the layer, aesthetic, and column.
8. Python type stubs (`.pyi`) for the extension module; full docstrings on the public API.
9. CI (GitHub Actions): fmt, clippy, tests, visual tests, wheels via maturin-action.
10. Text repel as a proper force-based solver (ggrepel algorithm) rather than greedy.

## Implementation plan

Each step = one or more commits on `main`, tests green, visuals checked, bug-hunt agents run.
Steps inside a phase are ordered; each builds on the previous.

### Phase 0: Foundation (do this very carefully)

- [x] **0.1 Workspace skeleton.** Cargo workspace (`ggrust-core`, `ggrust-python`), `pyproject.toml` (maturin), `python/ggrust/`, ruff + clippy config, `Makefile` with `check`, `.gitignore`, README. `maturin develop` builds; `import ggrust` works.
- [x] **0.2 Typed spec model.** `spec/` structs for plot, layer, mapping, scale, theme, guides, subplots, matching lets_plot `as_dict()` keys. Parsed from JSON (spec without data) via serde_json + serde_path_to_error. Unknown keys collected and reported (not silently dropped). Rust unit tests from JSON fixtures produced by real lets_plot.
- [x] **0.3 Data bridge.** Arrow PyCapsule import (`pyo3-arrow`), `data::Column` enum (numeric f64, categorical/string, bool, null mask), lookup by name. Tests: polars, pandas, numpy dict, nulls, categoricals, empty frames.
- [x] **0.4 Scene + SVG backend.** `scene::Primitive` (points batch, polyline, polygon, rect batch, text, group with clip) in pixel space; SVG writer; `text/` with embedded font + measurement. Golden test: hand-built scene -> SVG -> PNG.
- [x] **0.5 Trait registries.** `Stat`, `Geom`, `Position`, `Coord` traits + name registries (a `Scale` abstraction was NOT done here; see 0.10). Unknown geom/stat/position/coord/facet names (e.g. `coord_polar`) and unknown layer `parameters` (typos like `colr="red"`) must be reported, not ignored. `stat_identity`, `position_identity`, `coord_cartesian`, continuous linear scale, `geom_point` as the first vertical slice.
- [x] **0.6 Minimal layout.** Single panel, axes with nice breaks, axis titles, plot title, panel grid, `ggsize`.
- [x] **0.7 Python API slice.** `ggplot`, `aes`, `geom_point`, `ggsize`, `labs`, `PlotSpec.as_dict()`, `+`, `to_svg`, `to_png`, `_repr_html_` (static). `as_dict()` parity tests vs lets_plot. Spec JSON must handle NaN/inf (`json.dumps` emits invalid `NaN`) and numpy scalars; reject duplicate `color`/`colour` mapping keys.
- [x] **0.8 Test + visual + bench harness.** pytest suite, golden-image compare (pixel diff with tolerance, diff image on failure), side-by-side HTML report vs lets-plot, first benchmark (scatter 10k / 100k / 1M) with honest settings.
- [x] **0.9 Foundation review.** Bug-hunt agents on the whole foundation. Ask user to review the architecture before building on it.

- [ ] **0.10 Foundation v2 (from the 0.9 review; approved by the user).** Make "one file + one registry line" true for Phases 1-4:
  - [ ] 0.10.1 Staged build: map all layers -> resolve scales from spec (transforms, discrete positions) -> stat -> position -> geom setup -> train positions + style scales -> guides -> layout -> draw. Steps return warnings (no `&mut Vec`), ready for rayon.
  - [ ] 0.10.2 Scales (2a done: transforms, position scale specs, limits drop rows, nonlinear breaks; 2b done: discrete positions behind `PositionScale`, `scale_*_discrete`, staged map/scale/finish build, x label fallbacks two-rows/tilted/vertical; next 2c `StyleScale`): `PositionScale` (continuous + transform | discrete) owning breaks/labels/limits/expand, used by coord + layout; `StyleScale` trait (train, map to typed values, legend breaks) with a registry by aesthetic + `scale_mapper_kind`; defaults per aesthetic and column kind.
  - [x] 0.10.3 `LayerData` v2: `Variable::{Aesthetic, Computed}` (stat variables, `..count..`), `group_index` (explicit + implicit discrete grouping), `panel_index`, `source_rows` (tooltips).
  - [ ] 0.10.4 Coord owns all data->pixel math (layout uses `Panel::map`); `Coord::{flipped, aspect_ratio, limits}`; one flip representation.
  - [ ] 0.10.5 Python bridge v2: data objects replaced by `{"frame": i}` refs, frames deduplicated by identity (works for gggrid, enables caching); referenced columns computed once in Rust (`spec`), incl. tooltips/facets.
  - [x] 0.10.6 Typed style values: `Aesthetic::value_kind()` single source; `StyleValues` for color/number/shape/linetype/text; mismatch is an error.
  - [x] 0.10.7 Registries as one static list per kind (lookup + names derived, round-trip test).
  - [ ] 0.10.8 Layout into an area `Rect` with measure-then-place (gggrid, legends); deterministic SVG ids (explicit prefix).
  - Known gaps (bug hunt 0.10.2, deferred): sqrt axis with a single value keeps all breaks where lets-plot thins labels; an Enum's level order does not carry over to the same column in other layers.
  - [ ] 0.10.9 Small fixes from the review: ignored aesthetics checked before column fetch, `with_notes` keeps error kinds, batch consistency validated once, `is_axis_title_only` via defaults.

### Phase 1: Scatter-class plots (cellestial `dimensional`, `umap`, `scatter`, `xyplot`)

- [ ] 1.1 Discrete scales + `scale_color_hue`, `scale_color_manual`; discrete legend.
- [ ] 1.2 Continuous color: `scale_color_gradient`, `gradient2`, `gradientn`, `viridis`, `continuous`; colorbar legend.
- [ ] 1.3 `scale_*_brewer` palettes (all ColorBrewer sets lets-plot ships).
- [ ] 1.4 `theme()` + `element_*`, `theme_classic`, `theme_bw`, `theme_void`; `legend_position` (right, bottom, none, inside), horizontal legends; `guides`/`guide_legend` (ncol, override_aes).
- [ ] 1.5 `scale_size`, point shapes, alpha, stroke; `coord_fixed`, `coord_cartesian` limits; `scale_x/y_continuous` (limits, breaks, labels, expand, format), `scale_y_reverse`, `scale_y_log2`/`log10`.
- [ ] 1.6 `geom_text`, `geom_label`, `geom_path`, `geom_segment` (+ `arrow`), `geom_polygon`, `geom_vline`/`hline`, `geom_line`, `geom_blank`.
- [ ] 1.7 Port check: cellestial dimensional/scatter tests pass on ggrust (compat shim in a branch of cellestial, not edited on main without asking).

### Phase 2: Distributions (cellestial `violin`, `boxplot`, `ridge`, `bar`, `histogram`)

- [ ] 2.1 `stat_count` + `geom_bar`, `position_stack`, `position_dodge`; `stat_bin` + `geom_histogram`.
- [ ] 2.2 `stat_boxplot` + `geom_boxplot` (outliers, width, varwidth off).
- [ ] 2.3 `stat_ydensity` + `geom_violin` (KDE: gaussian, bw.nrd0 / scott, scale area/count/width, trim, quantiles). Parallel KDE per group.
- [ ] 2.4 `position_jitter`, `position_jitterdodge` (seeded, deterministic).
- [ ] 2.5 `stat_densityridges` + `geom_area_ridges`; `coord_flip`.
- [ ] 2.6 `geom_bracket` (significance brackets), `geom_function`.
- [ ] 2.7 Port check: cellestial distribution + quick tests.

### Phase 3: Heatmaps and grids (`heatmap`, `dotplot`, `stacked_violin`, `annotated_heatmap`)

- [ ] 3.1 `geom_tile`, `geom_raster`, `geom_rect`; discrete axes with many categories, label rotation.
- [ ] 3.2 `facet_wrap` (shared/free scales, strip labels).
- [ ] 3.3 `gggrid`: ncol, widths/heights, hspace/vspace, `align`, shared `guides="collect"`; `SupPlotsSpec` + `+` on grids.
- [ ] 3.4 Port check: cellestial heatmap, dotplot, annotated_heatmap tests; multi-plot `*s` functions.

### Phase 4: Interactivity and HTML

- [ ] 4.1 HTML renderer: SVG + embedded JS runtime, notebook `_repr_html_`, `to_html`, `LetsPlot.setup_html` no-op compat.
- [ ] 4.2 Tooltips: hit-testing index built in Rust (grid buckets), `layer_tooltips` (lines, format, title, anchor), default tooltips.
- [ ] 4.3 `ggtb` zoom/pan: re-render axes on zoom (Rust-precomputed break candidates, or JS tick generation).
- [ ] 4.4 Canvas point layer for dense scatter (>50k points); playwright tests for render + tooltip.

### Phase 5: Remaining cellestial features

- [ ] 5.1 `geom_text_repel` / `geom_label_repel` (volcano, ondata legend).
- [ ] 5.2 `geom_imshow` (spatial H&E images: RGB/RGBA arrays, cmap, extent).
- [ ] 5.3 `ggsave` (svg, png, html, pdf), `GGBunch` compat if still needed.
- [ ] 5.4 Full cellestial test suite green on ggrust; browser tests green.

### Phase 6: Performance

- [ ] 6.1 Profile (flamegraph via `cargo flamegraph`, `py-spy`, memory via `/usr/bin/time -v` + `tracemalloc`) on cellestial benchmark scripts.
- [ ] 6.1b Data bridge hot spots: integer -> f64 cast is single-threaded (50M rows ~0.6 s); string levels use SipHash (5M rows 91 ms vs polars unique 50 ms); multi-chunk columns are re-converted on every `column()` call (cache per frame).
- [ ] 6.1c PNG markers: ~1.3 us per marker via tiny-skia paths (1M points 1.4 s). Stamp pre-rendered subpixel sprites for uniform-style batches; consider parallel tiles.
- [ ] 6.2 Rayon parallelism (layers, panels, subplots, groups); measure, keep only wins.
- [ ] 6.3 Caching of Arrow conversion + stat results across subplots.
- [ ] 6.4 Final benchmark report vs lets-plot (methodology + raw numbers + plots).

### Phase 7: Release

- [ ] 7.1 Docs (API reference from docstrings, migration note for cellestial), CI, wheels for linux/macos/windows.
- [ ] 7.2 cellestial switches dependency `lets-plot` -> `ggrust` (with user approval).

## Performance strategy (honest)

- Biggest structural win: lets-plot serializes data to JSON in Python, then parses it in Kotlin/JS.
  ggrust passes Arrow buffers to Rust with no serialization.
- Native Rust stats (KDE, binning, boxplot) with rayon across groups and subplots.
- SVG writing with pre-sized buffers and `ryu` float formatting; PNG via tiny-skia directly from the scene (no SVG round trip).
- HTML: dense points as a base64 typed array drawn on canvas instead of 1M SVG nodes.
- No GPU on the dev server; GPU only appears client-side (canvas/WebGL). No server-side GPU work planned.
- Benchmarks always state N drawn, sampling settings, output format, hardware, cold vs warm.

## Open questions (ask when the step comes up)

- Copy lets-plot default sampling or not? (Phase 1.7)
- Exact visual parity with lets-plot vs "close and clean"? (Phase 0.8)
- Is `GGBunch` still needed or can cellestial drop it? (Phase 5.3)
