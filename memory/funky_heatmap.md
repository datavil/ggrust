# funky_heatmap: a funkyheatmap port on a unit canvas

`ggrust.bistro.funky_heatmap` ports the R package funkyheatmap 0.5 as plain layers
(rect, polygon, segment, text, image) on `coord_fixed`, 24 px per data unit (0.25 inch,
funkyheatmap's `width / 4` inches).

- **Set `plot_inset=0`** (and `plot_margin=0`) for a unit-exact canvas. `theme_void()`
  keeps lets-plot's 3 px plot inset; `coord_fixed` then shrinks the panel to ~23.8
  px/unit while text stays in px, so labels drift past the geometry and get cut.
- **Never pass `alpha=1` with a transparent fill.** A constant alpha replaces the fill's
  own alpha (lets-plot), so `#00000000` becomes opaque black. Pass `alpha` only below 1.
- **Text does not scale with the canvas.** funkyheatmap sizes the figure from text boxes,
  not text, so long names and legend titles overflow. The port measures text with the
  `_ggrust.text_width` binding (same shaper as the renderer) and grows the canvas.
- Rotated text is written with `text-anchor="middle"` around a shifted point; hjust
  is still honoured, but measure the extent from the anchor and hjust, not the SVG.
- funkyheatmap's `levelmatch` is a level's position among **all** `column_groups`
  columns (group and palette included): only the first column gets solid headers.
- Ranks in scIB: color value = `1 - (rank - 1) / (n - 1)` so rank 1 picks the last
  (lightest) palette color; palettes are listed dark to light, as funkyheatmap's `rev()`.
- The R source and the scIB metrics are not vendored: clone
  `github.com/funkyheatmap/funkyheatmap` to compare formulas (R/calculate_*.R,
  compose_ggplot.R, create_legends.R).
