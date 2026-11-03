# Canvas points: placeholder in the SVG, sprites at quarter pixels

Plan 4.4 (decided with the user): in HTML only, a point batch with more than
50,000 drawable markers (2,000 for zoomable `ggtb()` plots) (`render/canvas.rs`, same drawability + scene clip box
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
