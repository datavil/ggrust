---
name: side-panels
description: How ggside side layers and ggmarginal margins are built (ex2.2): bands in the layout, scales, zoom, and the traps found
metadata:
  type: project
---

# Side panels (ggside, ggmarginal)

Step ex2.2 (2026-11-05), design approved by the user.

- **One engine, two spec forms.** ggside layers carry `"ggside": "x"|"y"` (placed by the
  plot's `"ggside": {x_pos, y_pos}`); `ggmarginal()` writes lets-plot's own keys
  (`marginal`, `margin_side`, `margin_size`), so `as_dict()` equals lets_plot's. Both
  parse in `spec/side.rs` into a `BandKey { shares_x, far }` + `BandStyle`. The layer key
  is `ggside`, not `side`: `side` is already a geom option (beeswarm, ggdist slabs).
- **Side layers stay in `tiles[0]`**, tagged `band: Option<usize>`: legends, tooltips,
  toggles and the boxplot companion all index layers by spec position. Main drawing and
  main training filter `band.is_none()`; `build/side.rs` trains the bands.
- **Scales:** along the shared axis a side layer maps through the main scale (ggside
  layers train it, margins do not, as lets-plot); across through the band's own scale,
  built by `axis_scale` from `scale_xsidey_*()` (`PositionScales.xside_y`) or, for a margin
  below/left, `AxisScaleSpec::away_from_panel()` (reversed: values grow away). Every
  lookup "which scale does this aesthetic use" goes through `band_on_cross` /
  `uses_main_axis`; `default_scale_name` skips side layers' cross aesthetic (else
  `..density..` or a track mapping renames the main axis).
- **Tracks:** a side tile/raster without its own cross mapping gets a constant discrete
  column (label = its fill/color column) in `map_layer_into`, after dropping the
  inherited plot mapping on that axis.
- **Layout:** `PlotLayout.bands` (one per screen side); `band_rect` derives from the panel,
  so `fit_ratio` and `gggrid(align=True)` move bands along. `AxisEdge::of`, titles and
  legend origins add `band_extent(side)`. ggside size is a share of the panel, a margin's
  of panel + margins (`side_panel::split`). A top/bottom band's own axis widens the y axis
  band (`vertical_axis_width`); a left/right band's horizontal axis must fit the existing
  x band (single-line labels).
- **HTML zoom:** band layers live in `g.side-x` / `g.side-y` (screen axis shared), and the
  runtime's `treeGroups` treats them like axis dendrograms (inner group, one-axis
  matrix). Tooltip layers carry `side`; `moveTargets` moves them along that axis when the
  band's span equals the tile panel's. Points/text stretch while zoomed (gap).
- **Facets** drop side layers with a warning in `prepare_plot` (layers and their frames
  removed together, so indices stay consistent).
- `validate_parameters` accepts flipped constant positions under `orientation="y"`
  (`geom_boxplot(y=0, orientation="y")`, ggmarginal's top boxplots).

See [[deviations-md]] rows "Side panels ...", "`ggmarginal()` ..."; related:
[[dendrograms]], [[axis-positions]], [[ggnewscale-scale-groups]].
