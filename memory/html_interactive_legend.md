# Interactive legend: keys by resolved style, marks tagged by a probe draw

ua.9 (2026-11-05). HTML only; static output never sees the tags.

- **Rows get keys by matching their resolved styles** to the legend entries (before
  `override_aes`), not by re-reading data columns: works after any stat, with merged
  legends (all aesthetics of a title must match) and manual scales. First key wins on ties.
  Boxplot outliers (companion layer, grey) take their box's key by group.
- **Ids are `title + U+001F + label`**, shared by `g.legend-key` and `g.keyed`
  (`data-key` = JSON array). A gggrid toggles every plot showing that title/label.
- **Do not draw a layer per key subset.** Tried first: it reorders stacking, splits canvas
  batches and breaks geoms that use all rows at draw time (raster grid step, half_point
  jitter, repel). Instead draw once, then draw again with `Styles::probe` (every color =
  `probe_color(combination)`, alpha kept) and walk both trees (`tag_marks`): marks stay
  exactly as drawn, in data order. A mark whose probe color decodes to nothing stays
  visible; tree shapes that differ leave the layer untagged.
- Mixed point batches keep one batch: `Points.keys` -> keyed `<g>` runs in SVG, a u8 key
  index per drawn marker in the canvas payload (`drawCanvas` skips hidden keys).
- Tooltips: `LayerInteraction.keys` (combinations + per-row index); `candidates()`
  filters hidden targets, `hover_x` lines skip a hidden line by its first vertex.
- Zoom: `collectMarks` must recurse into `g.keyed` like `g.layer`, or texts stretch.
- Clicks: the JS adds a `fill=none pointer-events=all` rect over the key's bbox; a
  double-click comes after two clicks (net no change), then isolates.
