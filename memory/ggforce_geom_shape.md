# geom_shape: fillets, not ggforce's offsets

Step 2026-11-13 (residence2 Fig 4A cell outlines). Neither lets-plot nor ggplot2 4.0.1
has a corner radius; ggforce `geom_shape(expand, radius)` is the reference.

- **Round corners as exact fillets** (`geom/shape.rs::round_corners`): each convex corner
  becomes an arc tangent to both edges. ggforce's "mitre in by r, round out by r" gives
  the same outline when the radius fits, but makes the shape collapse when it does not
  (a stadium at exactly half height is fragile in R). ggrust reduces the radius to fit
  the edges instead (half an edge per rounded end), so `radius=1000` on a rectangle is a
  stadium. Recorded in deviations.md.
- **Round before expanding**: fillet by `radius - expand` on the original edges, then the
  round offset of `mark_shape::expand_outline` grows those arcs to `radius`. Filleting
  after `expand_outline` fails: its grid-traced outline has tiny edges at the corners, so
  the fitting rule clamps the radius to almost nothing.
- Works in panel pixels after `panel.map`, so corners are round under any coord; the
  geom post-processes `polygon::draw_polygons` paths (no duplicated grouping code).
