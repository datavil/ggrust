# Spec accessors and layer_data

**Works:** read-only properties over the stored spec (`PlotSpec.data`, `.layers`, ...,
`SupPlotsSpec.figures`, ...) that return new dicts and tuples, so mutating a result
never reaches the plot; changes go through `with_*` copies (`PlotSpec.duplicate`,
`SupPlotsSpec.duplicate` copy the lists). `layer_data(i)` / `build()` live on the
plot, not the layer: one `LayerSpec` object is shared by every plot built from it
with `+`, so a back-reference would be wrong.

**Engine side:** `crates/ggrust-core/src/build/inspect.rs` is a child module of
`build.rs`, so it reads `PreparedPlot`'s private tiles and `Arrangement` without new
public API. It runs `prepare_plot` + `arrange` at the plot size (breaks depend on the
axis length) and draws nothing. Layer tables are one Arrow `RecordBatch` per spec
layer over all panels (`panel`, `group`, then `LayerData::columns()`); discrete
columns become `Dictionary<UInt32, Utf8>` (polars `Categorical`) with levels merged
over panels. y-oriented layers store x and y swapped: flip the names back. Layout
breaks are screen axes: swap them under `coord_flip`.

**PyO3:** return `pyo3_arrow::PyTable` objects; `pl.DataFrame(table)` reads them via
`__arrow_c_stream__` (no pyarrow or arro3 needed).

**Traps:** `geom_*(data=dict)` stores a converted dict, so `layer.data is dict` is
False (a polars frame keeps its identity). lets_plot's `data_meta` key is internal:
`params` leaves it out. The doctest suite only runs reference entries (class
docstrings), so `tests/test_accessors.py` runs every accessor's own example.

**Tier 2 (`panels()`, `defaults`, `theme_defaults()`, `repr`):** `PlotLayout.panel` is already
in plot pixels for every facet tile, so `panels()` reads it straight from the arrangement.
`scale_x_reverse` negates the transformed units, so the domain still runs left to right;
only a decreasing `coord_trans` warp sets `reversed`. Test the mapping against the
`<circle cx cy>` of `to_svg()`, not against hand-computed numbers
(`tests/test_accessors_tier2.py`). `defaults` and `theme_defaults` cross PyO3 as JSON
strings (NaN becomes null, and unset NaN props are dropped).
