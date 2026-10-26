# Discrete legends and hue/manual scales follow lets-plot (Phase 1.1)

Verified against lets-plot 4.11 (`tests/test_legend_parity.py`, `tests/test_style_parity.py`).
Kotlin sources: `plot-builder/.../assemble/LegendAssembler.kt`, `PlotAssemblerUtil.createLegends`,
`guide/LegendComponentLayout.kt`; clone with `git clone --depth 1 --filter=blob:none --sparse`.

- **Discrete domain** = explicit `breaks` first, then data levels (first appearance);
  `limits` replace it (other values -> `na_value`). Legend = domain filtered by breaks;
  labels align with breaks (or the domain); with limits + breaks, labels follow breaks.
- **Manual dict values** (Python): keys become `breaks`, so the dict order is the legend order.
- **Legends group by scale name** (title: `labs()` > scale `name` > default scale name, which
  joins several columns with ", "); every layer mapping the aesthetic adds a key part for
  *every* label of the shared scale (not only its own values). Keys merge by label, so
  -0.0 and 0.0 (both "0") share a key.
- **Order** follows the geom's rendered aesthetics (`GeomMeta`: point = size, color, alpha):
  a size legend comes before a color legend.
- **Key styles**: legend values, else layer constants for shape/color/fill only (a constant
  size is ignored), else legend defaults (point size 5), else geom defaults.
- **Layout**: key cell >= 23 px, label gap = width("_")/2, 15 rows per column, 5 px margin,
  white background without border, at the right inside the plot margin, centred on the panel.
- Hue: n = max(levels, 2), full circle drops the last step; one level -> middle (#7cae00).
  Hue ranges are sorted (`DoubleSpan`) after adding `h_start` and after the full-circle cut.
- `na_value` only reaches scales built from options (values, a mapper kind); lets-plot's
  default mappers ignore it.
- Labels wrap at 30 characters by words (newlines break too); 5+ lines -> 5 + "...".
- Placement is relative to the panel (right edge + inset): with a minimum-size panel the
  legend runs past the plot edge rather than covering the data.

## Colorbars (Phase 1.2)
- Every continuous color/fill scale gets a colorbar (even manual colors: a stepped bar)
  unless a legend is asked for; continuous size/alpha get a key legend of ~5 breaks.
- Bar 23 x 115 px (1 x 5 key sizes), or label font size x (breaks + 1) if longer: lets-plot
  uses the *font size* here, not the line box. 20 bins, each rect runs to the bar bottom
  (no seams), the first is 2 px longer; ticks 2 px from the ends, white, at both edges.
- Colorbars come before key legends; an equal colorbar under the same title is shown once,
  a different one is titled "name (aes)".
- Continuous style limits: domain = data range within limits, widened to the limits;
  values outside -> NA. Colormap tables are generated from lets-plot's `Cm*.kt`.
- Guides of an aesthetic: `guides(...)` first, then the scale's own `guide` (title, name,
  colorbar options). A colorbar asked for on a non-continuous-color scale is an error, as
  in lets-plot's `checkFitsColorBar`. Colorbar labels split at newlines; the bar grows by
  lines x font size.
- viridis `begin`/`end` are sorted (`DoubleSpan`).
- Brewer: palette name picks the family (type ignored); an index needs `type` and wraps;
  no palette -> Set1 for categories, Blues for continuous data (quantized into the largest
  set). More categories than the set has: qualitative generates, others interpolate (LAB),
  unless `overflow` says otherwise. Tables generated from lets-plot's `ColorSets.kt`.

## Placement and grids (Phase 1.4b)
Measured from lets-plot SVGs (`LegendBoxesLayoutUtil`, `PlotLegendsLayoutUtil`, `LegendBoxLayout`):
- Outside legends sit at the plot margin edge (beyond `plot_inset`); right/left are justified
  along the panel height, top/bottom along the panel width (`legend_justification`, default
  centre). Top legends go below the title. The plot area gives up block + `legend_box_spacing`.
- Inside (`legend_position=[x, y]`): the justification point of the block is placed at
  (x, y) of the *panel* (0, 0 = bottom-left); takes no space; may run past the plot top.
- `legend_direction` auto = horizontal at top/bottom. Horizontal boxes put the title at the
  left (no gap), title and graph centred vertically on each other.
- Several legends: stacked (left-justified) even at the bottom; `legend_box="horizontal"`
  puts them side by side, top-justified; `legend_spacing` between them.
- Grid (`computeLayoutGrid`): column-major unless `byrow`; defaults 15 rows per column
  (vertical) or 5 columns per row (horizontal: rows = ceil(n / 5)). ncol/nrow capped at n.
  Options of all aesthetics in one legend combine: last count wins, byrow from any.
- Horizontal colorbar: 5 x 1 key sizes (or estimated label width x (breaks + 1)), low at the
  left, labels a third of a line below, centred on ticks.
- `override_aes` (1.4c): maps of all aesthetics in one legend merge (later wins); values apply
  by key *position* in each aesthetic's entries; lists shorter than the keys repeat their
  last value, `None` keeps the scale value, `[]` overrides nothing; an aesthetic the legend
  does not map gets entries for the first aesthetic's labels.
- Layout order from the outside in (lets-plot `PlotFigureLayouter`, `PlotSvgComponent.addAxisTitle`):
  plot margin, plot title/subtitle, legends, axis titles, `plot_inset`, axes, panel. The inset
  never moves titles or legends. Guide options: the scale's guide comes first, `guides()` last;
  legend counts combine (last wins), colorbar options are the last whole `guide_colorbar`.
