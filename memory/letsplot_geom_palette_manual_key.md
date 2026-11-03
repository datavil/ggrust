# Geom colors and manual legends follow lets-plot (2026-11-04)

- **pen / brush / paper** (`style::GeomPalette`, `theme::geom_palette`): pen = the
  flavor's ink (`Symbol::Black`), paper = its white, brush = `#118ed8` in every
  flavor (measured from lets-plot SVG); `theme(geom=element_geom(...))` overrides
  each, and names inside element_geom read the flavor. Geom defaults are
  `StyleDefault::PEN/BRUSH/PAPER` (rect and ridge fill: `Themed(Pen, 26)`), never
  literal `#474747`/white, or dark flavors draw light-theme geoms. Before this, every
  default geom color ignored the flavor. Scale values, `na_value` and `halo_color`
  still use the standard palette (deviations.md).
- **manual_key / layer_key** (`build.rs` `manual_key_contribution`): port of
  `PlotAssemblerUtil.createLegends` + `LegendAssembler.createLegend`. Legends are
  keyed by title (a group titled like a scale joins it); a layer's manual key comes
  after its scale legends; keys sort by `index` (missing last, stable) before
  merging by label; key = group `override_aes` (list by index, else layer position)
  + layer_key values + ALL layer constants + defaults; shows with
  `show_legend=False`; `guides(<group>="none")` hides. Default group "manual" has
  no title.
- **How to measure lets-plot colors:** its SVG writes `<circle fill="#rrggbb">`
  with a CSS block on top, so diffing color sets of whole SVGs is noise; read the
  geom element itself, or screenshot both (playwright) side by side.
