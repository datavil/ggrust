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
