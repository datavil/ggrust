# coord_flip and ridgelines follow lets-plot (Phase 2.5)

Parity tests: `tests/test_coord_flip_parity.py`, `tests/test_ridges_parity.py`.

coord_flip:
- Spec: `{"name": "flip", "flip": true}`, or `flip: true` on cartesian/fixed.
- Everything stays in data x/y until drawing: `Panel` holds the horizontal and vertical
  scales and swaps the inputs of `map(x, y)` when `flipped`. Geoms must draw through
  `panel.map` (reference lines check `panel.flipped`).
- lets-plot keeps titles and theme elements WITH THE AESTHETIC: under flip the x title,
  `axis_text_x`, `axis_line_x`, `panel_grid_major_x` all style the left axis
  (`Theme::flipped()` swaps every pair for layout).

Ridges:
- `densityridges` = ydensity with x/y swapped; `height` = density / layer max; untrimmed
  by default and spanning the overall x range of every layer.
- Ridge tops at `y + resolution(y) x scale x height`; drawn top row first; the band is
  simplified with lets-plot's Douglas-Peucker WEIGHTS (subdivide down to the smallest
  double, keep weights > 0.25 px): not classic DP. Points near 0.25 px flip between
  libraries because panels differ by a pixel (fonts): compare bands as subsets.
- `Geom::span_aesthetics` (setup columns training BEFORE position) is separate from
  `setup_aesthetics` (not trained after): bars span x only; ridges span y. Training bar
  ymin/ymax broke y limits.
