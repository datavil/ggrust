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
- Mixed point batches keep one batch: `Points.keys` -> a u8 key index per drawn marker,
  in the canvas payload (`drawCanvas` skips hidden keys) and in SVG.
- **SVG point batches: no per-run `g.keyed` wrappers** (2026-11-06, HTML size). Keys
  interleave in real data (single-cell rows sorted by barcode), so runs were ~1 point:
  ~110 B of wrappers per ~66 B circle. Now the batch `<g>` holds the table
  (`data-keys` = JSON array of the key-id arrays) and each keyed marker `data-k="i"` (on
  its `data-anchor` group when anchored, else on every part); `[]` markers get none and
  stay visible, also without JS. The runtime tags batches `data-kb="<plot id>-n"` and
  hides with one `<style>` in the plot root (`g[data-kb=..]>[data-k=..]{display:none}`),
  never touching each marker. 4,000 points, 12 interleaved keys: 906 KB -> 664 KB.
  Uniform-key batches and other geoms keep `g.keyed`. Static SVG/PNG never see either.
- Tooltips: `LayerInteraction.keys` (combinations + per-row index); `candidates()`
  filters hidden targets, `hover_x` lines skip a hidden line by its first vertex.
- Zoom: `collectMarks` must recurse into `g.keyed` like `g.layer`, or texts stretch.
- Clicks: the JS adds a `fill=none pointer-events=all` rect over the key's bbox; a
  double-click comes after two clicks (net no change), then isolates.
- Canvas redraws are deferred (`scheduleKeyedDraw`: rAF + setTimeout, coalesced, skipped
  when `drawnHiddenKeys` already match). A redraw is one `drawImage` per marker (~150 ms
  for 200k); drawing synchronously on click blocked the page, and a double-click drew 3x.
  CPU-backed canvases and ImageBitmap sprites were measured: no faster. Tests must wait
  for the draw (`wait_for_function`), not read pixels right after the click.
