# corr_plot follows lets-plot's measured output

lets-plot's `corr_plot` is expanded engine-side (Kotlin `CorrPlotOptionsBuilder`, not in
the Python package). Without its source, every rule in `python/ggrust/bistro/corr.py`
was measured from `to_svg()` output of lets-plot 4.11 (decode cells from tile/circle/
text centres and the axis labels; `strings` on the native bridge gives constants:
`#B3412C`/`#EDEDED`/`#326C81`, `.2f`, `rgba(0,0,0,0)`).

- Types: one layer is full; tiles lower, points/labels upper; an unset layer takes the
  other triangle of its partner; labels with their own `color` are full. "upper" is
  above the diagonal as drawn, so `flip=False` swaps it in index terms.
- Diagonal default: only on full layers. Labels white over tiles/points, sized by
  `|r|` only over points.
- Threshold drops cells with `|r| < t` (NaN kept) and variables without any kept cell;
  axes are cropped by type/diag, not by the threshold.
- Size: 40 px per variable in 150..700, + 5.7 px per character of the longest x name,
  + 70 legend; height + y-name width / 2 when wider than a cell, else 20 (floored).
- Panel: `coord_fixed`, categories padded by 0.6; ggrust needs explicit coord limits
  for that (its discrete padding differs with tiles).
- lets-plot errors on a single distinct value (degenerate range): pick test data with
  several values.
