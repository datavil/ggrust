---
name: secondary-axes
description: How sec_axis/dup_axis cross to Rust (callback), where the table is resolved, and how layout draws the second side
metadata:
  type: project
---

# Secondary axes (exc.4, 2026-11-05)

- **The transform is a Python callable called back from Rust** (user's choice over a
  formula string). `_render.split_functions` swaps each `scales[i].sec_axis.transform`
  callable for an index; the binding wraps them as `AxisFunction`s (`Python::attach`, one
  call per value) and `Figure::attach_functions` puts the table on every `PlotSpec`
  (`#[serde(skip)] functions`). Pure Rust tests attach Rust closures.
- **Resolve after the range is final**: `PositionScale::resolve_secondary` in
  `prepare_columns` (shared scale or each free group scale) and again in
  `grid_share::share_continuous` after `train_span`. Errors from there replace the
  prepared plot.
- **Sampling as ggplot2**: monotonic test on the trained range (strict), table over the
  expanded range trimmed where it stops being finite/monotonic, `approx` back in primary
  *data* units (exact for linear transforms on log axes).
- **Breaks**: a throwaway `ContinuousPositionScale` over the secondary range (primary's
  log/sqrt transform kept) goes through `choose_breaks`, then values map back
  (`layout/secondary.rs`). `dup_axis()` with all options derived mirrors the primary
  `AxisBreaks` exactly.
- **Layout**: `PlotLayout.secondary[index]` + `side_breaks(index, far)`; bands use the
  side's own breaks; `axis_title_bands` places primary and secondary titles (`Titles.
  x_secondary` / `y_secondary`, swapped under flip). `sides()` returns both with a
  secondary axis; `secondary_on_far_side()` puts it opposite `position=`.
- **Zoom**: `ZoomTile.secondary` holds fixed-break axes for the secondary side only; the
  runtime appends their nodes (no regenerated breaks).
- Visual goldens: none (report.py renders every case with lets-plot, which lacks it).

See [[axis-positions]], [[deviations-md]] row "Secondary axes".
