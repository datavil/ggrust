# Memory index

Critical lessons: approaches that work, or that break things. One file per lesson.
Add a line here for every new file. Format: `- [Title](file.md): one-line hook`.

- [lets-plot silently samples data](letsplot_default_sampling.md): points capped at 100k, text 5k, etc. Disable or match it in benchmarks.
- [cellestial reads PlotSpec internals](cellestial_spec_coupling.md): `as_dict()` layout, `FeatureSpec` classes and `+` semantics must match lets_plot.
- [One build path: uv](build_single_path.md): never `uv run --no-sync` after `maturin develop`; it tests a stale extension (false green).
- [lets-plot drops polars Categorical order](categorical_order_lets_plot.md): `factor_levels` only for ordered pandas / polars Enum / as_discrete. Read order from Arrow dictionaries.
- [serde loses error paths through tagged enums](spec_error_paths.md): dispatch `kind` by hand; leaves use `try_from = "Value"` + `from_value_tracked`, never plain untagged.
- [polars Null columns break arrow-rs FFI](polars_null_column_ffi.md): cast pl.Null to Float64 in `as_arrow` before export. Float64 crosses zero-copy.
- [Never cast strings to Utf8](arrow_string_casts.md): i32 offsets overflow past 2 GiB (panic); iterate string types in place; never concat dictionary chunks.
- [PNG: rasterise directly, never via big SVG](png_direct_raster.md): usvg node limit breaks 1M points; direct tiny-skia + text via resvg; consistency test guards SVG/PNG parity.
- [Calibrate against real lets-plot SVG](letsplot_reference_svg.md): `to_svg()` works offline; measured defaults (point r 3.3 #474747, text 13px, grid #E9E9E9).
