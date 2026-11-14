# joint_plot and qq_plot follow lets-plot

- `joint_plot` is a Python builder in lets_plot too (`bistro/joint.py`, `_plot2d_common.py`):
  port it line by line and test `as_dict()` (without `data`) for equality. Known spec
  differences: `..group..` -> `..level..` for 2D densities (ggrust has no contour index
  variable), `geom_pointdensity` is ggrust's `geom="point", stat="pointdensity"`.
- `qq_plot` is expanded by the Kotlin engine (no source at hand). Fast way to measure an
  engine-side bistro: build a candidate with plain lets-plot layers, normalise ids in both
  `to_svg()` outputs and `difflib` them; identical SVG means the expansion is known. The
  native bridge holds UTF-16 strings (`data.find("dens:tr".encode("utf-16-le"))`).
- Measured qq rules: points alpha 0.5 size 3, `aes(color=group, fill=group)` with constants
  winning; line `#FF0000` size 0.75, `color=group` when no `line_color`; margins are
  `geom_area(stat="density")`/histogram/boxplot over the *computed quantiles* with
  color/fill/alpha (0.5) of the points, default `"dens:tr"`; `theme_light()`; titles
  `"<dist>" distribution quantiles` / `<col> quantiles`. ggrust gets the quantiles from
  `layer_data(0)` of a points-only plot.
- lets-plot bugs not copied: pooled line with `group` + `line_color`; marginal legend
  with `show_legend=False` (deviations.md).
