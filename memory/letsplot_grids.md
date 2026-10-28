# gggrid follows lets-plot

Source: lets-plot `CompositeFigureBuildInfo.layoutedByOuterSize`, `CompositeFigureGridLayoutBase`,
`CompositeFigureGridAlignmentLayout`, `MonolithicCommon.buildCompositeFigure`,
`ThemeConfig.mergeUserOptions`, `LegendsBlockInfo.arrangeLegendBoxes`.

- **Prepare once, lay out many times.** A grid lays each plot out at its cell size and, with
  `align`, again around a target panel. `PreparedPlot` (build.rs) keeps stats/scales/legends;
  only `arrange`/`draw` rerun. Never rebuild a plot per layout pass.
- **Theme inheritance is partial.** A figure with no theme options takes the whole grid theme;
  one with options takes only name, flavor and legend placement. The grid's background fill
  (not its border) is pushed down unless the figure sets a flavor.
- **Collected legends keep their figure's theme** (lets-plot builds boxes in the figure);
  only the box arrangement (`legend_box`, spacing) comes from the grid. Using the grid theme
  made cellestial's size-9 legend text 13 px.
- **Dedupe needs NaN-aware equality**: unset alpha is NaN in key styles, so derived
  `PartialEq` never matches. Use `Styles::same_values`.
- **`guides`**: top grid collects only on "collect"; inside a collecting grid "auto" passes
  legends up, "collect" keeps them at its level, "keep" never collects.

**How to apply:** compare with `tests/test_grid_parity.py` (absolute panel rects through nested
`<svg>`/translate in lets-plot). Under `align` lets-plot sizes figures inside out, so compare
panels, not figure bounds. Few-px offsets are text metrics, not grid bugs.
