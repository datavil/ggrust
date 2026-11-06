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
- **Layout:** `PlotLayout.bands` (any number per screen side, see 9.10 below); `band_rect` derives from the panel,
  so `fit_ratio` and `gggrid(align=True)` move bands along. `AxisEdge::of`, titles and
  legend origins add `band_extent(side)`. ggside size is a share of the panel, a margin's
  of panel + margins (`side_panel::split`). A top/bottom band's own axis widens the y axis
  band (`vertical_axis_width`); a left/right band's horizontal axis grows the x axis band
  on its side (`horizontal_labels_height`, measured by a first pass and fed back to
  `layout_with` as `band_labels_height`; up to `BAND_LABEL_PASSES` more passes, since a
  thinner band can turn its labels). Band x labels use the main axis's lets-plot rules
  (vertical on a narrow band, user's choice over ggside's overlapping flat labels).
  `keep_labels_inside` never blanks category labels (they name tracks); before this fix
  it blanked all but the middle one on narrow bands.
  Continuous band axes: lets-plot's count rule allows one break on a thin band (the
  first, often 0 at the edge, which `keep_labels_inside` blanks), so when no label
  survives `layout_bands` retries `scale.breaks(2..=3)` and keeps the first set with a
  visible, non-overlapping label (a density band had ticks but no number).
- **Band axis theme (9.8):** `Theme::ggside_axis` holds ggside's `ggside_axis_*` elements
  resolved per screen side of the band's axis (`_x_top/_x_bottom` = y band, `_y_left/_y_right`
  = x band), each chain ending in the main `axis_*` chain. Every band axis helper in
  `side_panel.rs` (measure and draw) goes through `band_axis_theme(theme, side)`, a theme copy
  with those swapped into the main axis fields, so a blank band label takes no room. New
  band axis code must use it too, never `theme.axis_text_*` directly.
- **Several bands per axis (9.10, user's API):** a ggside layer's `side_pos` overrides
  `ggside()`; `side_band` names a band. `BandKey.slot` (0 nearest the panel) is set in
  `band_setups` from first appearance per (axis, side, name), after `layer_side` (which
  leaves it 0), then setups are sorted by key and layer indices remapped. Size: `side_size`
  counts only from layers of the band's final style (a `margin_size` is not a panel share).
  Layout: bands of one side are requested and laid out in key order, so
  `BandLayout.offset` (inner bands' extent, set in `layout_bands`) places them and
  `band_extent` sums a side; `build.rs::band_layout` finds a band as the n-th laid band
  on its screen side, never by side alone. Side scales: `PositionScales.targeted`
  (`SideTarget`, most specific match replaces `xside_y`/`yside_x`). One-band output is
  byte-identical (offset 0); keep it so.
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
