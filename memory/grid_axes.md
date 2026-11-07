---
name: grid-axes
description: How gggrid(axes=, axis_titles=) collects axes and titles (patchwork rules, mask on the theme, NaN-safe keys)
metadata:
  type: project
---

# Collected axes in gggrid (plan 10.4, 2026-11-07)

Source: patchwork 1.3 `collect_axes`, `collect_axis_titles`, `rle_2d`, `grob_id`
(read with `Rscript -e 'print(getNamespace("patchwork")$collect_axes)'`).

- **Rules (patchwork's, per side):** runs of equal cells down each column for x, along
  each row for y (`rle_2d`, then joined across neighbours with the same rows/columns).
  Axes: equal = same scale + axis theme + **same span extent** (x: columns, y: rows;
  patchwork `merge = FALSE`); the outer cell of each run keeps it. Titles: text +
  style only (`merge = TRUE`); the first edge cell keeps it; when any run spans
  several columns (x) / rows (y), every kept title is drawn by the grid over its
  run's panels (`RunTitle`). Empty cells, nested grids, faceted plots: no key, break runs.
- **Mask lives on the plot's screen theme** (`Theme.grid_collected`, a
  `CollectedSides`), not a theme option: `layout_with` zeroes the side's bands,
  `draw_axes` callers pass `drawn(axis_sides)`, nested-opposite guides and trees on a
  hidden side go too. Titles: `Dropped` (no room) or `Hidden` (room, not drawn).
  A removed title stays `Hidden` when another plot on the same grid line keeps a title:
  without `align` that keeps a row's panels level (patchwork's shared gtable row).
- **Keys compare as `Debug` text**: unset `TextStyle.angle` is NaN, so derived
  `PartialEq` never matched (same trap as legend dedupe, [[letsplot-grids]]).
- **Never `Debug` a `HashSet`**: `DiscretePositionScale.known` printed in random order,
  so discrete axes never matched; `axis_identity()` clears it (bug hunt 10.4).
- **Scales compare through `axis_identity()`**: `sharex` widens with
  `train_beyond_limits`, leaving each plot's own `domain`; join them first or shared
  axes never match.

Open: panels that drop an axis grow (cells stay equal); patchwork keeps panels equal.

See [[patchwork-extras]], [[axis-positions]], deviations.md row "Collected axes in a grid".
