# patchwork extras: tags, design, insets, nested alignment (ex2.3)

- **Tags were not rendered before ex2.3.** `labs(tag=)` and `plot_tag*` were parsed and
  warned. Measured against lets-plot 4.11 `to_svg()` (26 cases): the box sits at the
  corner/side of its area (plot inside margin, panels, or margin band); `[x, y]` centres
  it, clamped inside; element margins only widen the box; `margin` reserves a band on
  each side the named position touches (`top-left` = top and left), below the plot margin
  and outside the caption; `margin` + `[x, y]` acts as `plot`; `angle` ignored; tag size
  16 inherits title colour but not title size. Code: `layout/tag.rs`, hooked into
  `plot_content` / `draw_titles` (facets share both).
- **Grid layout is two-phase now.** `PreparedGrid::layout(bounds) -> GridLayout` (recursive:
  nested grids are laid out, not just given a rect) then `draw_layout`. Alignment needs a
  nested grid's `frame()` (innermost outer panel edges) and `layout_panel(area, target)`,
  the grid twin of `PreparedPlot::arrange_panel`. Never draw before aligning.
- **Spans, not cells.** Every figure has a `Span` (`layout::grid::figure_spans`); design
  areas and plain grids share one path. Alignment and `sharex/sharey` row/col use the
  span's first (and last) row/column.
- **Tags are numbered before preparing** (`grid_tags.rs`, a `FigureTags` tree mirroring
  figures and insets) because plot specs are cloned and prepared in parallel; a nested
  grid gets its subtree through `GridContext.tags`. `tag_level="unit"` gives a nested
  grid `FigureTags::Unit(tag)`: the grid draws it through its own `labs(tag=)` path
  (`titles.tag`), so bands, theme and placement need no new code (user chose this over
  a `wrap_elements` function, 2026-11-13).
- **Insets** live in `grid/inset.rs` (child module so it reaches grid internals). A
  top-level plot with insets goes through the grid path (`Placed::Plot`), not
  `build_plot_with`. Insets with `on_top=False` draw after the host background, so the
  host's panel background covers them inside the panel, as patchwork.

**How to apply:** compare tags with lets-plot via `tests/test_patchwork.py`
(`_lets_plot_tag` reads the translate group of `class="plot-tag"`); 2 to 4 px offsets are
font metrics. New grid features: extend `Span`/`GridLayout`, not the drawing loop.
