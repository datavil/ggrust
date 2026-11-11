# coord_polar

Layers draw flat and `coord/polar.rs` `Warp` bends the finished scene once. No geom
knows about polar, so a new geom works if its nodes are paths, rects, points or text.

- Flat x pixels are theta and flat y pixels the radius; the screen scales are set by
  `PolarView::adjust_scales` (lets-plot `adjustXYDomains`): no expansion, one extra
  slot on a discrete theta, radius x 1.21. The ring sits at 1/1.15 of the radial domain.
- A polar plot uses `space3d::layout_theme` for the layout, because axes are not flat.
- Bitmaps cannot be bent: `geom_raster` and `geom_imshow` raise at prepare time.
- Edges are resampled per 2 degrees of theta, capped so a zoomed edge far outside the
  panel cannot allocate without bound.
