# PNG marker sprites: coverage per geometry, colors at stamp time

Step 6.1c. `render/sprite.rs` stamps point batches of 1000+ markers (`SPRITE_MIN_MARKERS`
in `png.rs`); smaller batches are traced, so small plots and goldens did not move.

- **Cache coverage, not colored sprites.** Key = (shape, size, outline width); each part's
  fill and outline coverage is a layer, rendered once by tiny-skia in white (alpha =
  coverage). Colors are blended per item at stamp time, so continuous color scales share
  one sprite. Layers keep part order: overlapping translucent parts blend like traced ones.
- **1/16 px steps, rounded.** Centres on exact steps match traced output bit for bit
  (tested). Off-step, tiny-skia's quarter-scanline AA flips edge pixels by tens of levels
  even for sub-1/32 px moves (thin line tips of cross/star flip whole pixels). Finer steps
  do not help: test against the noise of shifting traced markers, never against zero.
- **Rare geometries stay traced** (`STAMP_AFTER` = 64 sightings): a sprite costs about one
  traced marker, so continuous `size` scales gained nothing and were slower when stamped.
  `size_alpha` 1M PNG is still ~8.7 s for that reason.
- Budgets: 256 geometries, 64 MB coverage, sprites up to 256 px; past them, trace. The
  off-canvas cull margin includes the largest sprite (tiny canvases).
