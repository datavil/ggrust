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

### Tier 1: users ask for these a lot

- **`p.guide_data("color")`**: polars frame of legend keys, each value with its color
  and label (ggplot2 `get_guide_data()`). Answers "which colors did my categories
  get?", so colors can be reused elsewhere (cellestial: a cluster's UMAP color in
  heatmap annotations).
- **`p.scale_info("x")`**: trained limits, breaks and labels after the build, per panel
  under free facet scales (ggplot2 `layer_scales()`, `get_panel_scales()`).
- **`p.columns`**: every data column the spec uses. `referenced_columns` already exists
  in the extension module.
- **`p.validate()`**: the warnings, without rendering. `validate_spec` already exists.

### Tier 2: for advanced users

- **`p.panels()`**: pixel boxes for each panel after layout, plus how each panel's data
  range maps to pixels. For overlays, HTML widgets over a panel, or aligning images.
- **`p.layers[i].defaults`**: the geom's default aesthetics (point size, color, ...).
- **`ggrust.theme_defaults("bw")`**: a named theme as a dict.
- **Readable `repr`**: for example
  `<PlotSpec: 2 layers (point, smooth), 3 scales, facet_wrap(~species)>`.

### Not worth it

- **`last_plot()`**: global state, ruled out by AGENTS.md.
- **Setters on everything**: `with_data` plus `+` cover changes.
