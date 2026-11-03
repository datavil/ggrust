# Vector PDF output (lp5.13)

`render/pdf.rs` writes the scene as vector PDF with `pdf-writer` + `subsetter`
(typst's crates). The raster PDF of 5.3 is gone; the page size is unchanged
(the PNG's pixels / dpi, lets-plot's page), so `test_export_parity` still compares
MediaBoxes with lets-plot.

**Why these crates:** smallest weight and fastest. krilla pulls a second skrifa
version and wants its own text setup; svg2pdf goes through usvg (node limit, see
`png_direct_raster.md`) and an SVG string. Glyphs come from `text::shape` (the
harfrust shaping behind `measure`), so PDF widths equal layout widths; kerning is
written as `TJ` adjustments against `hmtx` advances.

**Traps found:**
- Text state (`Ts` rise, `Tf`) survives `ET`. Reset the superscript rise to 0 or
  every later label is raised.
- macOS Quartz (Preview, `sips`) ignores `/Interpolate false` and blurs upscaled
  images. Images are repeated per pixel (`upsampling`, up to 16x, long side 4096).
- `sips -s format png x.pdf` writes Display P3 PNGs. Convert to sRGB (PIL
  `ImageCms`) before comparing with ggrust's PNG, or colours look shifted.
- A form XObject inherits fill/stroke colour and alpha but also dash: set
  `[] 0 d` and the line width inside each marker form.
- 100k circles inline are ~180 bytes each (8 MB, 190 ms); one form per distinct
  marker + `q 1 0 0 1 x y cm /M0 Do Q` gives 1.2 MB, 79 ms. Past 1024 distinct
  markers (continuous sizes) points fall back to inline paths.

**How to apply:** check PDFs by rasterising with `sips` (in a Python subprocess:
shell loops over variables are refused in worktrees) and extract text with a
PDFKit swift one-liner (`PDFDocument(url:).string`).
