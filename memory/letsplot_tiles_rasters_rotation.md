# Tiles, rasters and rotated axis labels follow lets-plot (Phase 3.1)

Parity tests: `tests/test_tile_parity.py`, `tests/test_raster_parity.py`,
`tests/test_rotated_labels_parity.py`.

- **Defaults** come from `DefaultGeomTheme.forGeomKind`, not `AestheticsDefaults`:
  tile = pen fill, paper outline, size 0; rect = pen outline, pen fill at 10%,
  size 0.75; raster = brush fill. Tiles and rects draw rows in order (bars reverse).
- **Tile spans** (`DimensionsUtil.dimensionSpan`) train the axes before the
  position: width/height x resolution (`res`) or axis units (`identity`).
- **Raster** = one bitmap (`scene::Image`): SVG `<image>` with a straight-alpha PNG
  (never via tiny-skia's premultiplied `encode_png`: faint colors shift by ~5),
  PNG via `draw_pixmap` with nearest filtering. Axes cover whole cells (setup
  spans). `GeomProto.preferredCoordinateSystem`: a raster plot defaults to
  `coord_fixed(1)`, and `coord_flip()` flips that default (not cartesian).
  Under flip ggrust transposes the bitmap; lets-plot does not (its cells land
  in the wrong place).
- **Rotated labels**: `rotateLabels` is true when ANY of angle, hjust, vjust is set
  on the axis text, so `element_text(hjust=0)` alone switches layout. The vertical
  anchor at 90 degrees comes from hjust and the horizontal one from vjust; with
  `vjust=0.5` 90-degree labels centre in the band instead of ending at the tick.
  `rotatedLabelBounds` is not an exact bounding box (sqrt of squares); kept as is.
  Bands use real font metrics: panels differ from lets-plot by up to ~10 px.

**How to apply:** read before touching rectangle geoms, images (`geom_imshow`, 5.2)
or axis label layout.
