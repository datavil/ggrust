# ggforce marks (geom_mark_hull / ellipse / rect)

Step ex1.8 (2026-11-04). ggforce 0.5 is MIT; R's `concaveman` wrapper is GPL but the
algorithm (Park & Oh 2012, mapbox concaveman, ISC) was reimplemented from its description.
Reference renders: ggforce installed into a scratch `R_LIBS`; same data rendered in R and
ggrust matched (hull shapes, ellipses, boxes, 3 of 5 label positions; the rest differ by
group order).

- **Everything is screen space.** ggforce converts to mm inside `makeContent`; ggrust works
  in panel pixels at 96 px/in (`PIXELS_PER_MM`). Concavity is scale-free, so hulls match.
- **Read ggforce from R**, not the binary: `print(get(name, asNamespace("ggforce")))` dumps
  `makeContent.hull_enc`, `labelboxGrob`, `place_labels`, `elbow`, `get_end_points`.
- **Expansion without polygon clipping**: `shapeGrob` = mitre offset by `expand - radius`,
  then round offset by `radius`. Built as convex pieces (edge strips, mitre kites, the
  outline's edges) + interior, exact distance on a 0.5 px grid, marching squares at
  `radius + 0.5 px` (the half cell keeps sharp inner corners from cutting into `expand`),
  Douglas-Peucker. Never self-intersects on concave hulls; largest loop wins (ggforce: first).
- **Khachiyan stops early** (tol 0.01 on the weights): its ellipse can miss points. ggrust
  runs it on the hull vertices and scales the result up to hold every point.
  Degenerate: one point = zero ellipse; collinear = a half the length, b = a / 10.
- **Label boxes** use grid's text metrics, not the font's: 0.718 em a line, 1.2 x
  lineheight em between lines, 0.229 em descent gap under a lone label (measured in R).
  The 50 mm minimum width only applies when a description wraps.
- **Placement** = nearest free cell to the mark's bbox centre on the edge of the free
  region (grid + EDT + prefix sums). Order matters: ggrust places in its group order.
- **Facets look odd but are right**: short panels often have no room near a mark, so
  labels go far away or are skipped, as in R.
- `filter` is a new aesthetic (boolean column, kind Other, groups like any discrete
  column); `description` is a text style aesthetic resolved like `label` (`build.rs`).
