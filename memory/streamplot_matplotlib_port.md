# streamplot is a bit-exact matplotlib port

`ggrust.streamplot()` (`crates/ggrust-core/src/streamplot.rs`) ports matplotlib 3.10's
`lib/matplotlib/streamplot.py` line by line. Lines and arrows are bit-identical on
`tests/fixtures/streamplot_matplotlib.json` (regenerate with
`tests/fixtures/generate_streamplot_reference.py` in an env with matplotlib).

What it takes to stay exact:

- Python `round()` is half to even: `round_ties_even`, not `round`.
- `StreamMask._current_xy` survives across lines and is not reset by `undo`; keep it.
- Masked arrays: an interpolation is missing if any of its 4 corners is non-finite,
  whatever its weight. Missing speed or velocity ends the line.
- Same operation order as numpy (`x / (1/dx)` for grid to data, `powf(0.5)` for `** 0.5`,
  `hypot`, sequential cumsum for the arrow's half arc length).
- `forward` only drops the seed vertex (`xyt[1:]`), as matplotlib.

Deliberate difference: `density` below 1/15 raises (matplotlib divides by zero).
The class is `StreamplotSet` because `Streamplot.md` and `streamplot.md` collide on
case-insensitive file systems in the API reference.

float32 grids (scVelo's `compute_velocity_on_grid` returns float32 `X_grid`) differ by
< 1e-6: matplotlib takes `dx` and the origin in float32, ggrust in float64. Same lines
and vertex counts (checked on pancreas: 77/77 lines); not worth emulating.
