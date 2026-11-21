# Grid slices: `grid[:] += feature`

Status: done (plan.md step 14.1, agreed with the user 2026-11-21). The user chose the default proposal for all five decisions below.

## Goal

Let a feature be added to several cells of a `gggrid` in one statement, as an extension of
the shipped `grid[i] += feature` (`SupPlotsSpec.__setitem__`, deviations.md row "Changing
one cell of a grid"):

```python
grid[:] += scale_color_viridis()  # every cell
grid[0, :] += geom_smooth()  # first row
grid[:, 1] += theme(legend_position="none")  # second column
grid[1:3] += labs(x="weight")  # figures 1 and 2
```

Today `grid[:]` raises `TypeError` ("a grid cell is an int or a (row, column) pair of
ints, got slice") and `grid.plots() + layer` raises a tuple concatenation `TypeError`.
The workaround is `gggrid([p + shared for p in plots])` before building the grid.

Neither lets-plot nor ggplot2 has this. patchwork's `p & layer` is the closest (all plots,
recursive). The backlog rules out patchwork-style operators; slicing is Python's container
protocol, not a new operator, which is why the user picked it.

## How it works

`grid[:] += x` runs as `view = grid[:]`, `view = view + x`, `grid[:] = view`. So:

1. `__getitem__` accepts a slice, or a `(row, column)` pair where either part is a slice,
   and returns a small `GridCells` view (the selected cell positions and their figures).
   An int or an int pair still returns the figure, unchanged.
2. `GridCells.__add__(feature)` returns a new view with `feature` added to every non-empty
   figure in it (`figure + feature`), leaving empty cells empty.
3. `__setitem__` accepts the same slice forms with a `GridCells` value and writes its
   figures back to the same positions. A view from another grid, or of a different
   selection, raises an error naming both selections.

Estimated size: about 50 lines in `python/ggrust/plot/subplots.py` plus the view class.
No Rust change.

## Decisions to make before coding (ask the user)

1. **Nested grids.** Recurse (the feature reaches every plot inside a nested grid, as
   patchwork `&`) or add to the nested grid as a figure (so `theme` sets the nested grid's
   figure theme and layers fail as they do on any grid)? Default proposal: recurse for
   layers, scales, coords, labels; figure-level kinds follow the next point.
2. **Figure-level features through a slice** (`theme`, `ggsize`, `ggtitle`, `tag`, ...).
   Apply per plot (`grid[:] + theme(...)` themes each plot, unlike `grid + theme(...)`
   which themes the figure) or refuse? Default proposal: per plot, documented next to
   `grid + theme()` so the two meanings are side by side.
3. **Design areas that span several cells.** `grid[0, :]` on a `design=` grid can hit the
   same figure twice. Default proposal: deduplicate, each figure gets the feature once.
   `#` cells in a slice are skipped (an int pair on one still errors, as today).
4. **What `grid[:]` alone shows.** A `GridCells` repr (`GridCells(figures[0..3] of 2x2)`),
   not a tuple. Should it also be iterable / `len()`-able? Default proposal: yes, read-only.
5. **Steps and negative slices** (`grid[::2]`, `grid[::-1]`): accept everything Python's
   `slice.indices` accepts; order does not matter for `+`.

## Possible costs (noted at the user's request)

- **Stability is the real cost.** Once shipped, slicing semantics are frozen
  (docs/guide/stability.md): `GridCells` becomes a public name, and the answers to the five
  decisions above cannot change without a deprecation cycle. A wrong call on nested grids
  or on figure-level features is expensive to undo.
- **Two meanings of `+ theme()`.** If decision 2 is "per plot", `grid + theme()` and
  `grid[:] + theme()` do different things. That is easy to explain but easy to trip on.
  Refusing is safer but less useful.
- **Type checkers.** `grid[i] += layer` is already flagged because `grid[i]` is
  `PlotSpec | SupPlotsSpec | None`. Slices need `@overload`s on `__getitem__` and
  `__setitem__` (int/int-pair -> `Figure`, slice forms -> `GridCells`) or the noise
  doubles. The overloads must pass `ty` and keep the public API snapshot readable.
- **Mutation.** Like `grid[i] += x`, this changes the grid in place; grids copied earlier
  are unchanged. `grid[:] + x` without `=` returns a view, not a grid, which may surprise
  someone expecting a new grid. Document it; no copy-returning slice form is planned
  (`with_figure` stays the copy form for one cell).
- **Error surface.** New errors to write and test (Rust-style, with what/where/fix):
  bad slice part types, a foreign or mismatched view assigned back, a feature a cell
  rejects (name the cell's spec path, e.g. `figures[2].figures[1]`, as `plot_paths()` does).
- **Divergence from lets_plot.** Code using it no longer runs on lets_plot. Fine for a
  ggrust-only convenience, but cellestial should keep the list comprehension where it
  wants to stay portable.
- **Paperwork (normal for any step).** Public API snapshot re-bless (addition only),
  deviations.md row (update the existing "Changing one cell of a grid" row, which today
  says "No add-to-all-plots operator"), guide section in the grid docs, API reference page
  for `GridCells`, tests, bug-hunter pass.

## Tests

- `grid[:]`, `grid[r, :]`, `grid[:, c]`, `grid[a:b]`, negative and stepped slices add the
  feature to exactly the selected plots; others unchanged (compare `as_dict()`).
- Empty cells stay `None`; design areas spanning cells get the feature once; `#` cells skipped.
- Nested grids follow decision 1; figure-level features follow decision 2.
- Grids copied before the assignment are unchanged.
- Errors: type and key parts of each message.
- One rendered PNG of a 2x2 grid after `grid[:] += geom_smooth()`, inspected.
