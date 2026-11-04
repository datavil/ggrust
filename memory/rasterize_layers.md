# Rasterized layers (ex1.5, ggrastr `rasterize()`)

`build.rs` wraps a `rasterize(dpi=)` layer's nodes in `scene::Raster` inside its
layer group. `render::rasterize_layers(scene, inches_per_pixel)` swaps each Raster
for an `Image` before SVG (Python binding, 1/96) and PDF (`to_pdf`, scale/dpi).
PNG and HTML draw the children as vector (user's choice: HTML keeps tooltips/zoom).

- **Alignment = same pixel grid as the PNG.** The bitmap covers the enclosing clip
  snapped outward to whole pixels of a whole-scene PNG at scale `dpi * inches`,
  rendered by translating the children by the snapped origin. Test: drawing the
  rasterized scene back at that scale equals `render_pixmap` of the original
  (`raster::tests`); a 0.7 px shift fails it, a 0.5 px shift does NOT (nearest
  sampling hits pixel-centre ties), so never mutation-test with half pixels.
- **Test coordinates must avoid quarter-pixel ties.** Sprites quantize `x * scale * 4`;
  coordinates like k/1000 at scale 3.125 land on .5 exactly and the translation's
  float noise flips the sprite phase (4 of 55k pixels differ).
- **`Image::upsample = false`** for rasters: the PDF Quartz-blur upsampling would
  otherwise double each side (cells are below a layout pixel but `ceil(cell * 4)` = 2).
- **Rasterizing only pays for dense layers.** Vector PDF markers are ~8 bytes each;
  the bitmap is a fixed cost per pixel, and the default `ggsave` PDF page is 12.5 in
  for a 600 px plot (lets-plot page size), so 300 dpi = 6.25 px per plot pixel.
  100k uniform points: raster PDF 1.4 MB > vector 0.85 MB. PNG row filters did not
  help (noise-like pixels); zlib level 6 saves ~25% but is slower (not done).

**How to apply:** a new renderer must handle `Node::Raster` (inline children, or call
`rasterize_layers` first); code walking layer contents should descend into it
(`Group::flattened` does).
