# ggsci palettes: fixed tables, brewer overflow, warn after training

Step ex1.1 (2026-11-04). What works and what to reuse:

- Palettes are one table (`scale/style/ggsci_data.rs`, generated from ggsci 5.2.0
  `R/palettes.R` hex values only; ggsci is GPL, no code ported) and one mapper
  (`ggsci.rs`). Each family is its own `scale_mapper_kind` (`color_npg`, `color_d3`, ...),
  so a later scale call clears the options as any mapper pick does.
- More categories than colors: the brewer overflow rule (`brewer::extend_colors`,
  default generate) plus a warning. ggsci itself returns NA (grey). Recorded in deviations.md.
- The warning needs the category count, which is known only after training: mappers
  declare `StyleMapper::palette_size()`, and `train_style_scales` (build.rs) checks
  `StyleScale::palette_overflow()` (counts `limits` when given). Reuse this for any
  future fixed-size palette instead of warning inside `map_discrete` (no warnings there).
- `alpha` rounds like R `rgb()`: `floor(a * 255 + 0.5)`; apply it after extending, since
  generated colors are opaque.
