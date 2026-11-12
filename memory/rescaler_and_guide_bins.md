# rescaler= and guide_bins()

- **Python callbacks reach the engine through one function table.** `split_functions`
  (`_render.py`) swaps every callable in the spec (sec_axis `transform`, facet
  `labeller.function`, scale `rescaler`) for its index; `PlotSpec.functions` carries
  them (`AxisFunction`: `call`, `call_text`, `rescale`). A new callback kind = one
  trait method with a refusing default + one branch in `split_functions`. A
  vectorised callback (rescaler) gets a Python wrapper that converts lists to a polars
  Series and back, so Rust only sees `Vec<f64>`.
- **Rescale at the StyleScale, not in mappers.** `Rescaler::apply` maps the rescaled
  0..1 value back into the scale's domain before `map_continuous`, so every mapper
  works unchanged; `map`, `map_numbers` and `bin_legend_entries` all go through it
  (the colorbar bins too). Out-of-0..1 results become missing (ggplot2 censors).
- **guide_bins is a key legend with `bin_edges`.** `Legend.bin_edges` (keys + 1
  labels) switches `fit_legend` to `measure_bins` / `Measured::Bins`; key drawing and
  the interactive-legend grouping are shared helpers (`draw_key`, `group_key`), so
  HTML toggles keep working. Edge labels come from `StyleScale::bin_edge_labels`.
- **Binned position scales are not a style-scale job.** ggplot2 bins x before the
  stat; ggrust trains position ranges after it (priorities 3.18, L).
