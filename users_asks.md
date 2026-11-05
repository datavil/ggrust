# What plotting users ask for

Survey of open issues on lets-plot, plotnine and ggplot2, sorted by reactions
(checked 2026-11-04). Reaction counts are small (5 to 19 at the top), so read them
as direction, not votes. Not scheduled: pick items into [plan.md](plan.md) or
[extensions.md](extensions.md) with the user.

## Already covered by ggrust

Interactive output (plotnine #262, 19 reactions), Polars support (plotnine #602, 16),
speed on millions of rows (plotnine #482), ggnewscale (plotnine #803), half violins
(ggplot2 #6844). Planned: polar coordinates (lp5.5), `geom_curve` (lp6.2),
gghighlight (ex2.5), Sankey (ex2.6), patchwork-style grids (ex2.3).

## Candidates, ranked

1. **Custom fonts.** ggrust embeds one font. Journals ask for Arial or Helvetica,
   companies for brand fonts, CJK users for glyphs the embedded font lacks.
   `element_text(family=...)` is accepted but there is only one font to draw with.
   A `register_font(path)` used for both measuring and drawing.
2. **Distribution beyond PyPI.** conda-forge (lets-plot #785, the most-reacted
   lets-plot issue; also #982). Pyodide/WASM wheel (lets-plot #1196) for JupyterLite,
   Pyodide and in-browser marimo.
3. **Notebook integrations.** Panel (lets-plot #870), marimo, anywidget. Today
   `_repr_html_` is one-way; an anywidget wrapper would let selection and zoom flow
   back to Python.
4. **More interactivity.** Interactive legend, click a key to hide its group
   (lets-plot #1080); selection tool returning the selected rows (lets-plot #998).
   Builds on the layer toggles and the tooltip hit index.
5. **ggplot2's top asks.**
   - Jittered boxplot outliers (ggplot2 #4480, 13).
   - Missing values shown in the legend when `na_value` is set (ggplot2 #4567, 12).
   - Clipping per layer (ggplot2 #6773, 9).
   - Common scale options set once, for example one palette for every color scale
     (ggplot2 #4269, 5).
6. **Docs gallery with an example for every geom** (plotnine #101, 6). Doubles as
   visual regression coverage.
7. **Grid titles, captions and tags** (plotnine #966, patchwork's `plot_annotation`).
   ggrust has grid titles but warns on caption and tag: a gap in our own code.

Other requests seen: axis capping (lets-plot #1184), vertical fill gradient
(lets-plot #994), waffle charts (lets-plot #1336), treemaps (lets-plot #1309),
`gggrid` `sharex`/`sharey` with discrete scales (lets-plot #1015), dark theme
(plotnine #164), methods-based interface (plotnine #1047).

## Sources

- [lets-plot issues](https://github.com/JetBrains/lets-plot/issues)
- [plotnine issues](https://github.com/has2k1/plotnine/issues)
- [ggplot2 issues](https://github.com/tidyverse/ggplot2/issues)
