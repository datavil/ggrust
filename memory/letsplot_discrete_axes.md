# Discrete position axes: lets-plot rules (Phase 0.10.2b)

Verified against lets-plot 4.11 (`tests/test_discrete_parity.py`: positions, limits,
reverse, expand, numbers, and 238/238 label arrangements in a size sweep).

- **Domain**: categories at 0..n-1 (levels joined across layers in first appearance),
  expansion **additive 0.2, multiplicative 0** (not ggplot2's 0.6). Numbers on a
  discrete axis pass through unchanged and widen the domain.
- An axis is discrete when any layer maps a discrete column to it or
  `scale_*_discrete()` is used; levels must be joined over *all* layers before stats,
  hence the build stages `map_layer` -> `axis_scale` -> `finish_layer`.
- `limits` pick and order categories; others are dropped (rows removed, like
  continuous limits). `scale_x_discrete()` always emits `reverse: False`, and repeated
  scales merge later-wins, so a later plain call undoes `_reversed()`.
- **x labels** (`HorizontalFixedBreaksLabelsLayout`): simple -> two rows -> tilted
  30 deg -> vertical. A layout fails if labels overlap (8 px; 60 px within a row for
  two rows; tilted needs ticks >= (h + 5) / sin 30 apart) **or leave the axis span**:
  labels may reach only 10 px past the right end; on the left, the y axis width
  (`geomAreaInsets`). Decide with estimated widths and label height = font size.
- **Layout order** (`TopDownTileLayout.computeAxisInfos`): y axis, then x axis; if
  x got thicker, lay out y once more and **keep the x breaks/arrangement**. A
  fixpoint loop oscillates (vertical band -> narrower y labels -> simple -> ...).
- y axis discrete labels have no fallbacks: all categories, stacked.
