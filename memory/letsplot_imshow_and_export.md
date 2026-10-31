# geom_imshow and export sizes follow lets-plot (Phase 5.2, 5.3)

Parity tests: `tests/test_imshow_parity.py`, `tests/test_export_parity.py`.

- **imshow is a Python port**: the array becomes an RGBA PNG data URL on a
  `geom='image'` layer; our `zlib` encoder gives byte-identical hrefs to pypng, so
  `as_dict()` equals lets_plot's. Keep it that way (cellestial compares specs).
- **Grey colorbar** (`color_grey`): interpolate lightness L* first, round last. Building
  a two-color LAB gradient from rounded grey endpoints is off by one in places.
- **Images like rasters**: no axis expansion, default `coord_fixed`, flip/reverse keep
  pixels at their data positions (lets-plot does not; recorded in plan.md 5.2).
- **Export sizing** (lets-plot `PlotImageExport`, measured, not documented right):
  `scale` defaults to 2 only when none of w/h, dpi, unit is given; it still multiplies
  when w/h are given; physical units default to 300 dpi; unitless w/h are inches and
  >= 20 raises; the layout size is rounded up to whole pixels, then scaled and
  floored (`new_pixmap`).
- **PDF without deps**: PDF `FlateDecode` + `/Predictor 15` reads a PNG's joined IDAT
  data directly. Composite over white first (PDF image has no alpha). Test PDF bytes
  as bytes: `from_utf8_lossy` shifts xref offsets.

**How to apply:** read before touching `geom_image`, color scales on `paint_*`, or
`to_png`/`to_pdf`/`ggsave` sizing. Measure lets-plot with `to_png(BytesIO, ...)` +
Pillow `info["dpi"]` when in doubt.
