# Contours follow lets-plot's triangle split (lp5.3)

Parity tests: `tests/test_contour.py`. Kotlin sources (no local checkout; fetch raw from
GitHub): `ContourStatUtil.kt`, `ContourFillHelper.kt`, `Contour.kt`,
`AbstractDensity2dStat.kt`, `Density2d(f)Stat.kt`.

- **Spec**: lets_plot emits `geom="contour"|"contourf"|"density2d"|"density2df"` with no
  stat; each geom defaults to the stat of the same name. Contour lines draw as paths
  (grey, size 0.5), bands as polygons (blue, transparent outline). No default
  `..level..` mapping in lets-plot.
- **Lines = lets-plot exactly**: each cell is four triangles around a centre valued at
  the mean of the two middle corners; a level is traced only in cells with
  `min < level <= max`. Ported, the vertex counts match lets-plot path for path.
- **Bands are not lets-plot's**: ContourFillHelper stitches lines to the plot border and
  breaks when the first line is a closed loop (fills the loop's inside). ggrust clips
  each triangle to `low <= z < high` with symbolic vertices (grid point, or grid edge x
  level index) so shared edges cancel exactly, then joins the rest into rings (outer
  CCW, holes CW, nonzero fill). lets-plot also simplifies polygons when drawing, so
  compare bands by ring count, fill and texts, not vertex counts.
- **Levels**: `zmin + (i + 0.5) * width`; band `..level..` = zmin, midpoints, zmax;
  density2df drops the band below the first level (lets-plot `subList(1)`).
- **density2d**: grid over the stat context ranges, R bandwidths (deviation). The dense
  `nx * ny * N` sum is chunked by 4096 points, partial grids summed in order (same bits
  serial or parallel). 100k points: 29 ms vs lets-plot 13 s.
- Polygon rings: `geom/polygon.rs` `draw_polygons(split_rings=true)` splits a group where
  a vertex repeats the ring start (lets-plot multi-ring polygons); `geom_polygon` itself
  does not split yet.

**How to apply:** read before touching contour stats, isobands, or multi-ring polygons.
