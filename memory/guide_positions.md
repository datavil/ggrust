# Per-guide legend positions

`guide_*(position=)` (ggplot2 3.5) is built as **one legend block per position**, not a
new layout: `PreparedPlot::legend_blocks` (build.rs) groups the boxes by
`LegendBox::position(theme)` and measures each group with a theme clone whose
`legend_position` is that position. Everything downstream (`legend_space`,
`legend_origin`, auto key direction, `draw_legends`) already works per side, so the
group theme carries the side; legend space is summed over blocks.

- Plots without positioned guides take the old path exactly (one borrowed theme), so
  goldens do not move.
- Blocks on opposite sides must split the budget (`shared_legend_budget`), or top +
  bottom legends overflow the canvas (bug hunt).
- Inside point: guide theme's `legend_position_inside`, else the plot's, else a numeric
  `legend_position`, else `legend_justification` (ggplot2 3.5).
- `legend_position="none"` hides every guide, positioned ones too (ggplot2).
- gggrid `guides="collect"` ignores positions (gap, deviations.md).
