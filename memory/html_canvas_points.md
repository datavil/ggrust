# Canvas points: placeholder in the SVG, sprites at quarter pixels

Plan 4.4 (decided with the user): in HTML only, a point batch with more than
5,000 drawable markers (2,000 for zoomable `ggtb()` plots) (`render/canvas.rs`, same drawability + scene clip box
as the SVG writer) becomes an empty `<foreignObject class="ggrust-canvas">`
over the whole scene; its markers travel as base64 Float32 xy + a style table
+ u16/u32 style indices. SVG and PNG output never change.

- **foreignObject, not an overlay canvas.** It sits where the batch was, so
  z-order with other layers and the panel's clip path still hold, and zoom's
  `collectMarks` finds it in the panel like any mark.
- **Marker parts come from Rust** (`MarkerShape::for_each_part` at 0,0): canvas
  and SVG geometry cannot drift. A unit test pins that parts translate exactly.
- **Sprites per quarter-pixel offset, stamped at whole pixels.** One sprite per
  style stamped at fractional positions is visibly blurred (bilinear); floor
  instead of round to the nearest quarter added a bias. Mean pixel difference
  vs the browser's own SVG render went 2.5 -> 1.0. Stamping keeps data order,
  so translucent markers stack as in the SVG. Past 4096 styles: traced paths.
- **Cull before quantizing**: zoomed pixels pass 2^31 and `x & 3` truncates.
- **Strokes keep their width on zoom** (the SVG uses non-scaling strokes);
  geometry scales by the zoom size factor `k`.
- Without JavaScript the dense layer is missing (static notebook viewers).
- Tests: `tests/test_canvas.py` compares browser screenshots of the canvas page
  against the same browser rendering `to_svg()`; the zoom test compares the
  panel interior (grid off) with a static `coord_cartesian(xlim, ylim)` render.
  Both fail on a 1 px shift or a missing redraw (mutation-checked).
- Playwright: `uv run playwright install chromium` (the browser revision must
  match the package; an older cached revision is not used).

- **Zoom lag came from the browser, not our JS** (2026-11-03). With 6,700 SVG
  circles a wheel step cost ~2 ms of runtime JS but ~7 ms of browser layout
  (every circle's cx/cy/r changes) plus paint, so zoomable plots go to the
  canvas from 2,000 points. On the canvas, creating ~120 sprite canvases per
  step (k changes every step with `size_zoomin`) cost ~4 ms: sprites of one k
  now share atlas pages. The tooltip index is rebuilt lazily on the next
  lookup, not on every zoom step.
- **Paint every sprite before the first stamp.** Painting an atlas page in
  between `drawImage` stamps made Chromium sync it with the target each time:
  6.6 ms of script per step at dpr 2, 1.4 ms with two passes. Measure with
  playwright + CDP `Performance.getMetrics` (Script/RecalcStyle/Layout/Task
  durations) at `device_scale_factor=2`; the hidden browser pane runs no rAF
  and dpr 1 hides costs.

- **Threshold lowered from 50,000 to 5,000** (2026-11-06, user). The cellestial
  docs page was 97 MB: its ~17k-point UMAPs stayed SVG, at ~66 B per circle
  plus ~110 B of interactive-legend `<g class="keyed">` wrapper per point.
  For one such plot lets-plot's HTML is 1.07 MB, ggrust's was 3.42 MB. The
  canvas payload is ~11-14 B per point. Trade-off kept: without JavaScript
  (static notebook viewers such as GitHub) these mid-sized layers are now
  missing too, not only the >50k ones. SVG and PNG output are unchanged.

- **Tooltip targets refer to their canvas batch** (2026-11-06, HTML size). Point
  targets that are exactly a batch's markers (same count, order and Float32
  centres: `canvas::Batch::same_points`, a content check in `html::canvas_targets`)
  are written `{"kind":"points","canvas":n,"radius":..,"rows":..}`; the runtime
  (`canvasTargets`) rebuilds x/y from the decoded `xy`, **rounded to 0.01 px** like
  inline targets: with Float32 centres, 4 of 2,833 cursor positions picked the
  other point of a near-tie. Plain arrays, not Float32Array (zoom maps them).
  Anything else keeps x/y inline: SVG points, markers culled from the canvas
  (zero/NaN size, past the clip margin), other geoms. Matching by content means
  facets, grids and several batches need no bookkeeping. Radii stay inline (the
  canvas has sizes per style, not hover radii). 17k-point UMAP, 12 colors, id
  tooltip: 1,485,803 -> 1,255,956 B. Test: `test_canvas_targets_answer_as_inline_targets`
  compares `ggrust.lookup` on a grid against the same page with inline arrays.
