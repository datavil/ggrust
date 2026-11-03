# Minor ticks and exponent_format follow lets-plot (lp5.8)

- Minor ticks (measured on lets-plot 4.11 `to_svg()`): blank by default, 2 px long,
  1 px wide. Chain `axis_minor_ticks_x <- axis_minor_ticks <- axis_x <- axis <- line`:
  `axis_ticks` does NOT color them. Positions are the minor grid's
  (`minor_breaks`, half a step around the majors), kept inside the panel; they are
  drawn even when major ticks are blank. They follow the aesthetic under `coord_flip`.
- lets-plot reserves only the major tick length; ggrust reserves the longer tick
  (ggplot2), see deviations.md. The default layout is unchanged (2 < 4).
- `exponent_format` is plot-wide: parsed early by `theme::exponent_format(spec)`
  because scales are built before the theme is resolved. It is applied by
  `set_exponent_format` on position and style scales (rewrites their `format=`
  patterns), passed to text labels (`label_texts`) and tooltips (`ContentSource`).
  The theme style always replaces a pattern's own `&E/&P/&F`; given bounds replace
  the defaults (-7, and 6 or the pattern precision). `e` writes `1e+7`, `1e-9`.
- Comparing SVGs: ggrust ticks are `<path d="Mx yLx y">` with hex colors, lets-plot's
  `<line>` with `rgb()`; lets-plot puts a space before the superscript tspan.
