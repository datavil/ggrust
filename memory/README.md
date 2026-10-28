# Memory index

Critical lessons: approaches that work, or that break things. One file per lesson.
Add a line here for every new file. Format: `- [Title](file.md): one-line hook`.

- [lets-plot silently samples data](letsplot_default_sampling.md): points capped at 100k, text 5k, etc. Disable or match it in benchmarks.
- [cellestial reads PlotSpec internals](cellestial_spec_coupling.md): `as_dict()` layout, `FeatureSpec` classes and `+` semantics must match lets_plot.
- [One build path: uv](build_single_path.md): never `uv run --no-sync` after `maturin develop`; it tests a stale extension (false green).
- [lets-plot drops polars Categorical order](categorical_order_lets_plot.md): `factor_levels` only for ordered pandas / polars Enum / as_discrete. Read order from Arrow dictionaries; polars exports first appearance, only unordered pandas differs.
- [serde loses error paths through tagged enums](spec_error_paths.md): dispatch `kind` by hand; leaves use `try_from = "Value"` + `from_value_tracked`, never plain untagged.
- [polars Null columns break arrow-rs FFI](polars_null_column_ffi.md): cast pl.Null to Float64 in `as_arrow` before export. Float64 crosses zero-copy.
- [Never cast strings to Utf8](arrow_string_casts.md): i32 offsets overflow past 2 GiB (panic); iterate string types in place; never concat dictionary chunks.
- [PNG: rasterise directly, never via big SVG](png_direct_raster.md): usvg node limit breaks 1M points; direct tiny-skia + text via resvg; consistency test guards SVG/PNG parity.
- [Calibrate against real lets-plot SVG](letsplot_reference_svg.md): `to_svg()` works offline; measured defaults (point r 3.3 #474747, text 13px, grid #E9E9E9).
- [lets-plot semantics traps](lets_plot_semantics_traps.md): alpha replaces (not multiplies), layer/plot data merge, constant positions, colour alias, precision test, write SVG attributes once.
- [Never let a failing check reach a commit](check_before_commit.md): don't pipe `make check` into grep and then `;` commit; gate on its exit status.
- [Transformed scales follow lets-plot exactly](letsplot_transformed_scales.md): log/sqrt/reverse scales, data-space expansion, half-up label rounding, limits drop rows.
- [Discrete axes follow lets-plot](letsplot_discrete_axes.md): 0.2 expansion, levels joined over layers, label fallbacks and the one-reflow layout order.
- [Visual cases need their own seeded generator](visual_case_determinism.md): shared `RANDOM` at call time made goldens depend on test order.
- [Default style scales follow lets-plot](letsplot_style_scales.md): Set1 + generated colors, LAB gradient, size/alpha ranges, NA values; benchmark machine changed.
- [Discrete legends follow lets-plot](letsplot_legends.md): breaks-first domain, legends grouped by title, keys for every scale label, rendered-aes order, key style precedence.
- [Axis labels follow lets-plot](letsplot_label_formats.md): format patterns, POW exponents, decimal rounding, fixed breaks, overlap dropping (open).
- [Text and line geoms follow lets-plot](letsplot_text_and_lines.md): line width from `size` x 2.2, text 2 px per size, vjust 0.7 em, label box placement, `,~g` numeric labels.
- [Port checks need a mirrored env](cellestial_port_check.md): worktree + import swap + poetry's exact versions; fresh resolves give false failures.
- [Bars, stats and positions follow lets-plot](letsplot_bars_and_positions.md): per-group stats, `(low, high]` breaks, stack/dodge formulas, dry-run axis ranges, zero-based y.
- [When lets-plot looks wrong, ggplot2 decides](deviate_from_lets_plot_via_ggplot2.md): parity by default; quirks follow ggplot2, recorded in plan.md.
- [Boxplots follow lets-plot, with ggplot2 corrections](letsplot_boxplots.md): two layers, NaN outlier placeholders, dodge 0.95, type 7 quartiles, coef on outliers, overridden mappings still group.
- [Violins follow lets-plot, bandwidth follows R](letsplot_violins.md): per-x KDE, layer-wide violinwidth, band edges repeated, R bw.nrd0, rayon per group/bin.
- [Jitter follows lets-plot bit for bit](letsplot_jitter.md): Kotlin XorWow port, offset formula, jitterdodge around the unjittered x, fixed default seed.
- [coord_flip and ridgelines follow lets-plot](letsplot_flip_and_ridges.md): flip in Panel, theme follows the aesthetic, ridge heights, lets-plot DP weights, span vs setup columns.
- [Brackets follow lets-plot](letsplot_brackets.md): no inherited mapping/legend by default, tips below the bar, segment paint fallbacks, label lifted half its height.
- [Lists in a mapping are data](letsplot_mapping_vectors.md): `aes(x=[..])` becomes column `x`/`x1`.. (not column names, except `group`); frame references carry every column name for naming.
- [Tiles, rasters, rotated labels follow lets-plot](letsplot_tiles_rasters_rotation.md): geom-theme defaults, raster bitmap + default coord_fixed, straight-alpha PNG, rotateLabels on any of angle/hjust/vjust.
- [facet_wrap follows lets-plot](letsplot_facets.md): per-panel stats with plot-wide group and level order, shared stat context, grid layout rules, default faceted size.
- [gggrid follows lets-plot](letsplot_grids.md): prepare once/lay out many, partial theme inheritance, collected legends keep figure theme, NaN-aware dedupe.
- [HTML output: explicit ids, inlined runtime](html_renderer.md): per-display id prefix (not the content hash), runtime is a function expression without trailing `;`, `vertical-align:top` on the root.
