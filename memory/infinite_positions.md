# Infinite positions: map at the scale, send constants as a marker

**Works:** ggplot2's "-Inf / Inf mean the panel edge" for every geom at once with three
small changes, no per-geom code:

- `ContinuousPositionScale::map` / `DiscretePositionScale::map` send -inf to 0 and +inf
  to 1 (`scale::continuous::infinite_edge`). Every geom maps through `Panel::map`, so
  text, rects, segments, reference lines, coord_flip, coord_fixed and facets follow.
- `transform_value` keeps infinities (negated by a decreasing transform, so reverse
  swaps the edges) and `limit()` keeps them (ggplot2 `oob` is `only.finite`). Training
  already skipped non-finite values, so they never widen the axis.
- `report_missing_values` counts infinite *position* values as drawn for geoms; stats
  still treat them as missing.

**Trap:** JSON has no infinity. `_render._plain` turned every inf into null, so
`geom_hline(yintercept=inf)` failed as "missing yintercept". Only position constants
under `layers[i]` are sent as `{"$float": "inf"}` and read by `Parameters::position`;
every other infinite number stays null (sizes etc. must not become infinite). Annotate
positions are data columns, so they cross through Arrow and needed nothing.

**Side effect:** `stat_ecdf` padding (-inf, +inf rows) now reaches the panel edges also
under x limits (lets-plot drops them there, so its ECDF stops short of 1).
