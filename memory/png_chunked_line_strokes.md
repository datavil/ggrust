# PNG: stroke long polylines in chunks

`geom_line` to PNG at 1M rows took 17 s (100k: 0.9 s), slower than ggplot2 + ragg.
`sample` showed 99% of CPU in `tiny_skia::scan::path::fill_path_impl` itself: the
stroker and edge builder were noise.

- **Why:** tiny-skia (Skia's scan converter) keeps active edges in a linked list and
  walks all of them on every quarter-pixel scanline. Six x-sorted zigzag lines of 166k
  segments each, every segment spanning much of the panel height, keep ~100k edges
  active in a ~25 MB edge array: the walk is cache misses. Cost grows faster than the
  data (10x rows, 20x time).
- **Fix:** `Canvas::stroke_chunked` in `render/png.rs` strokes open, undashed paths over
  1024 segments in 256-segment chunks (1M: 17.5 s -> 2.0 s, 100k: 0.97 s -> 0.24 s).
  A tiny-skia micro-benchmark found 64 to 1024 segments all good; 4096 already loses
  most of the gain.
- **Same picture:** each chunk repeats the previous chunk's last segment so the join
  is drawn. Translucent strokes go opaque into a layer, composited at the stroke alpha
  (without it, overlaps blend twice: mean channel diff 40). The layer covers the path's
  padded bounds and costs ~6 ms per path on a 3000 x 2400 canvas, so translucent paths
  chunk only past 4096 segments (at 1100 segments x 300 paths, layers made it 30%
  slower than the single stroke; found by the bug hunt). Only AA edge pixels move
  (overlapping chunks combine coverage as 1-(1-a)(1-b)).
- **Guard:** `render_parity.rs::long_dense_lines_match_the_svg_stroke` (opaque and
  translucent, NaN gap) compares with resvg's single stroke of the SVG.

**How to apply:** when a PNG of long or dense paths is slow, profile before blaming
the stroker. The same edge-list cost applies to any huge single fill (e.g. big polygons
from contours): split it when the pieces do not need to be one path.
