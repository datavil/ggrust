# Text and line geoms follow lets-plot (measured with `to_svg()`, Phase 1.6)

- Lines (path, line, segment, hline, vline, polygon outline) use `size`, not
  `linewidth`: width = size x 2.2 px (default 0.75 -> 1.65). `linewidth` is ignored
  by lets-plot 4.x for these geoms. Dashes are the linetype pattern x line width.
- Polygon: fill `#118ED8`, outline white; `alpha` fades the fill only.
- Arrows: barbs `length` px at +-`angle` from the line, the line end pulled back
  by the line width; closed heads are filled with the line color. A line split
  into styled pieces gets heads only at its outer ends.
- Text: font px = size x 2 (default 7 -> 14 px), NOT 2.8. `vjust` moves the
  baseline 0.7 em from 0 to 1; lines are one font size apart, each aligned by
  `hjust`. `geom_label` is placed by its box (top at the point for vjust = 1),
  box = text + 0.25 x font px padding, radius 0.15 x box height, 1 px border;
  `alpha` fades the box fill only. hjust/vjust names (`left`, `top`, ...) are numbers.
- Mapped `hjust`/`vjust` (lp5.11, #1143): no scale, values drawn as they are
  (`build::mapped_justification`). lets-plot snaps text hjust to start/middle/end
  (only exact 0 and 1); ggrust keeps any number (ggplot2). Compare vjust by the
  baseline shift between rows at one point: it matches lets-plot to 0.05 px.
  lets-plot fails on a missing value; ggrust centres it.
- Numeric labels: `,~g` with power exponents (`1.23457·10^8`, `10^20`), missing
  values `n/a`. Do not use Rust `f64::to_string` (`0.30000000000000004`).

**How to apply:** check any new text/line geom against these before guessing.
The parity tests live in `tests/test_line_text_geoms_parity.py`.

## Text layer options (lp5.9, measured on lets-plot 4.11)

- `label_format` / `na_text` go through `build::LabelTextOptions` while styles are
  resolved (read only when the geom lists them): numbers and strings through
  `LabelPattern::format_text` (strings that parse as numbers are formatted, `{}`
  writes Kotlin's `1234567.0`, unknown patterns are literal text), constant `label=`
  too, missing values -> `na_text` (`n/a`). Identical texts to lets-plot.
- `check_overlap`: per layer and panel, draw order, first wins; lets-plot
  `TextHelper` + `GeometryUtils.arePolygonsIntersected` (SAT, touching counts).
  Box heights match lets-plot exactly; widths are the real font (lets-plot's
  estimator is ~15% wider), so ggrust keeps a few more labels (93 vs 103 of 200).
- **How to measure:** binary-search the smallest gap at which two equal labels both
  stay (`tests/test_text_options.py::smallest_gap`); counts alone mix width and height.
