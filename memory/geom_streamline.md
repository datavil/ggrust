# geom_streamline: one layer over velocity_grid + streamplot

`stat="streamline"` (`stat/streamline.rs`) runs per group (so per facet panel and per
`color`/`group`) and emits every line as its own output group via `per_group_pieces`.
`geom_streamline` (`geom/streamline.rs`) draws them with `path::draw_paths_with(arrow=None)`
and adds one head per line from `Streamline::arrow()` on the data-unit vertices, so the head
sits where matplotlib puts it; heads use `Arrow::apply` and only the heads are kept.

Decisions (2026-11-08):

- Aesthetics `u`, `v` (ggquiver, matplotlib), not metR's `dx`/`dy`. Number-kind, read by the
  stat only (like `weight`); `layer_data` carries their group means.
- Rows that form a complete regular grid (every pair of distinct x and y exactly once) are the
  field as given; anything else is scattered and goes through `velocity_grid`
  (`adjust_for_stream=True`). `gridded=True` builds the grid with NaN holes; `False` always
  smooths. Detection is exact equality of coordinates: a grid with float noise is "scattered".
- Core errors are relocated to the layer option (`density` from velocity_grid becomes
  `grid_density`, message prefix rewritten); other core paths (grid axes) land on
  `layers[i].data` with the core name in the message.
- Default arrow `arrow(angle=26.6, length=6, type="closed")` approximates matplotlib's `-|>`
  at `arrowsize=1` (4 pt long, 2 pt half width). `arrow=None` draws lines only.
- A group under 50 rows without `n_neighbors` raises (scVelo's `int(n/50)` default is 0).

Test pattern: `layer_data()` partitioned by `group` must equal `velocity_grid()` +
`streamplot()` called by hand, bit for bit (`tests/test_streamline.py`).
