# ggnewscale: scale groups

- Style scales are keyed by `ScaleKey { aesthetic, group }` (`scale/style/mod.rs`).
  Group 0 is the plain plot: a spec without `new_scale_*()` must serialize and render
  exactly as before (no `scale_group` fields, plain guide keys).
- Python resolves the `+` order, not Rust: `PlotSpec` counts markers per aesthetic and
  tags what is added later (layers/scales: `"scale_group": {"color": 1}`; guides and
  `labs()`: key `"color:1"`). `as_dict()` loses the interleaving of layers and scales,
  so the tag must be written at `+` time.
- Every lookup of a style scale, scale spec or guide by aesthetic must go through the
  layer's group (`scale_key(&layer.scale_groups, aesthetic)`): training, styling,
  legends, legend titles (`default_scale_name` filters layers by group), tooltips,
  `forces_discrete`, facet level order. A new lookup by bare `Aesthetic` is a bug.
- Legends and colorbars are keyed by (title, group) during assembly only; the
  `LegendBox` carries no group, so `gggrid(guides="collect")` dedupes by content.
