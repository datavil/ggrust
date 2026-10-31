# Rayon parallel levels: gated, ordered, byte-identical

Step 6.2 put grid figures, facet panels, layers and tooltip jobs on rayon. Rules that made it work:

- Every site goes through `crate::parallel::ordered_map(items, parallel, f)`: results in input
  order, sequential when `parallel` is false or there is one item. Results are then consumed in
  order, so warnings, tooltip notes and the first error are those of a sequential run. Never
  `collect::<Result<Vec<_>>>()` on a parallel iterator: it returns any error, not the first.
- Gate on size: `parallel::is_worth(rows or marks)` (10k). Waking the pool from the Python thread
  costs ~50 us per call; ungated, a 5-panel 300-row facet went 0.23 -> 0.43 ms.
- Sites: `grid.rs` (plots prepare; figures draw, each with its own `Interaction` whose zoom has
  empty `plots`, merged in order), `build.rs` (tiles x layers in `compute_layer` and
  `style_layer`, panel and layer draws, one tooltip job per panel and layer), `render/svg.rs`
  (`SvgWriter::nodes`: heavy siblings into sub-writers with clip numbers pre-assigned by
  `clip_count`, id positions shifted). Stats were already per group (`stat::groups::per_group`).
- HTML canvas batches stay sequential: their numbers follow writing order.
- Outputs stayed byte-identical on every `benchmarks/parallel.py` case (SVG, PNG, HTML); check
  that again after touching any site.
- PNG is not parallel: one pixmap raster plus deflate. Parallel tiles belong to 6.1c / 6.1f.
