---
name: frame-column-cache
description: DataFrame caches converted columns per schema index; stat results are deliberately not cached
metadata:
  type: project
---

# Frame column cache (6.3)

`DataFrame::column` caches each converted `Column` in `Arc<[OnceLock<Option<Column>>]>` by schema index.

- Clones share the cache (gggrid subplots get clones of the same frame via `PlotData::from_frames`).
  Every derived frame (`with_column`, `from_columns`) goes through `DataFrame::new` and gets a fresh cache.
- Use `get_or_init` (blocking), not compute-then-set: parallel subplots all miss at once and would
  each convert, so a non-blocking cache saved nothing on grids.
- Failures store `None` and are converted again to rebuild the typed error (Error is not Clone).
- Columns are Arrow-backed, so a cache hit is a refcount bump; never add `&mut` methods to columns.
- Stat results are not cached across subplots (user decision, plan.md decisions table): cellestial
  grids map a different key per subplot, so a cache would never hit and its key would be fragile.
