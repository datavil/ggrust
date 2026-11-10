# Plot accessors

Read-only views on `PlotSpec` and `SupPlotsSpec`, in the spirit of ggplot2's
`p$data`, `p$layers`, `get_labs()`, `get_guide_data()`, `get_layer_data()` and
`get_panel_scales()`. lets-plot has none beyond `get_plot_shared_data()` /
`get_plot_layer_data()`, which stay for compatibility. Discussed with the user on
2026-11-04.

## Agreed, being built (session "plot-accessors")

`PlotSpec`: `data`, `mapping`, `layers` (each with `geom`, `stat`, `data`, `mapping`,
`params`), `scales`, `theme`, `labels`, `size`, `facet`, `coord`, `guides`;
`with_data(frame)`; engine-backed `computed_data()` (per layer, ggplot2 `layer_data`)
and `build()`.

`SupPlotsSpec`: `figures`, `layout`, resolved `ncol`/`nrow`, `size`, `theme`, `labels`,
`plots()` (nested grids flattened, empty cells skipped), `grid[i]`, `grid[row, col]`,
`with_figure(...)`, `with_layout(...)`. Figures return the theme they were built with,
not the grid's merged one. No `grid.build()` for now.

Rules: plain dicts and tuples, same objects (no copies), read-only; changes return a
new plot. Engine-backed accessors are methods, not properties.

## Proposed next

Moved to [../backlog.md](../backlog.md) section 5 (not scheduled).
