# Arrow axes are theme axis lines, not layers

`theme(axis_line=element_line(arrow=arrow(), extent=0.25))` (decided with the user,
2026-11-03; plan 9.4, deviations row). Lessons:

- **Pixel space, one node builder.** `layout::axis_line_node` is used by the static
  axes and by the zoom template (`layout/zoom.rs` `line`). The JS runtime stamps
  `axis.line` unchanged, so anything drawn there is zoom-invariant for free. Keep new
  axis decorations in that node, not in data-space layers.
- **Arrow and extent only from the axis_line chain** (`keys[..2]`: `axis_line_x`,
  `axis_line`). Reading them through `line`/`axis` would put arrows on every axis line
  of a theme that styles `line`; other elements warn instead.
- **Point order matters only with heads.** A plain y line keeps lets-plot's top-to-bottom
  order (dash phase, existing goldens); with heads it runs from the origin so
  `ends="last"` points up (ggplot2).
- `ggsave` with a relative path writes under `lets-plot-images/` (lets-plot rule): use
  absolute paths for scratch renders.
