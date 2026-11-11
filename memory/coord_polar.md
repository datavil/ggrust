# coord_polar

Layers draw flat and `coord/polar.rs` `Warp` bends the finished scene once. No geom
knows about polar, so a new geom works if its nodes are paths, rects, points or text.

- Flat x pixels are theta and flat y pixels the radius; the screen scales are set by
  `PolarView::adjust_scales` (lets-plot `adjustXYDomains`): no expansion, one extra
  slot on a discrete theta, radius x 1.21. The ring sits at 1/1.15 of the radial domain.
- The layout blanks only the x axis (`layout::polar::layout_theme`): the theta axis lives
  inside the panel, the radius axis keeps its band and is drawn by `draw_axes` on a
  rectangle covering the upper half of the panel.
- Bitmaps cannot be bent: `geom_raster` and `geom_imshow` raise at prepare time.
- Edges are resampled per 2 degrees of theta, capped so a zoomed edge far outside the
  panel cannot allocate without bound.
