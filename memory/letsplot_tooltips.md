# Tooltips follow lets-plot (measured by hovering real lets-plot)

How to measure: `LetsPlot`'s offline JS ships in the wheel
(`lets_plot/package_data/lets-plot.min.js`). Write one page per plot with
`<script src='lets-plot.min.js'>` + the body of `plot.to_html()`, serve the folder
(`uv run python -m http.server 8765 --directory tests/visual/output`; big pages do
not open as files in the browser pane), hover with the
real mouse (synthetic `dispatchEvent` does not trigger lets-plot), read the tooltip
`<text>` elements. One page per plot: a long page scrolls under you. Kotlin source:
sparse clone of `JetBrains/lets-plot` tag `v4.11.0` (`plot-base/.../tooltip`).

Rules that are easy to get wrong:
- Default lines: bound aesthetics minus axis ones (and any bound to an axis'
  variable), minus hidden, minus discrete with < 5 levels (legend shows them).
  A single `x`/`y` line has no label (histogram "3", line "62").
- Labels are the scale name (axis title / legend title), stat vars by name;
  boxplot stats are "y max", "upper", "middle", "lower", "y min".
- Positions format with the scale's formatter for **100 breaks** over the domain
  (170.125 -> "170.1"); other numbers `{,~g}`; `format()` on a variable also
  formats the aesthetic it is mapped to, but never the axis tooltip.
- Values are **before** the position adjustment (stacked bar shows its own count;
  keep `unadjusted` data, not the drawn data).
- Bars (and lines) look up along x: every stacked segment gets its own box.
  Points: nearest within radius + 5.1 px; violins/ridges: outline vertices,
  reach 5.1 px (hard to hit in lets-plot too). Text layers: only with user lines.
- `tooltips="none"` removes axis tooltips too. Blank axis text hides that axis' tooltip.
- `@gene|...`: text before `|` is a label, never a field.

Traps hit while building it:
- The payload's `lines` key collided with a runtime-internal `layer.lines`:
  keep runtime state under names the payload never uses.
- A tooltip-only column that cannot be converted to Arrow must not fail SVG/PNG:
  tooltip columns are exported for HTML only and dropped with a warning.
