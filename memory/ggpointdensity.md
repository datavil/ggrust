# ggpointdensity: measured rules, exact grid counts

Step ex1.4 (2026-11-04). ggpointdensity 0.2.1 is GPL: its R and C code were read for behaviour only, and
reimplemented (grid instead of its O(n^2) loop). The stat is `stat/pointdensity.rs`; Python emits
`geom="point", stat="pointdensity"` (no new geom).

- The rule (from the C source, read for behaviour): j counts if it is a duplicate, or same
  x and `xy*dy*dy <= r2`, or same y and `xy*dx*dx <= r2` (quirk: `xy`, not `1/xy`, so
  same-y reach is wider when `xy < 1`; kept for parity), or `(1/xy)*dx*dx + xy*dy*dy
  <= r2`. `r2 = (w + h) / 70` (not squared), `xy = w / h`, `w`, `h` = plot-wide scale
  limits (every layer, zero-width stays zero) times `adjust`. Per group. A zero x range
  counts same-x or same-y points, a zero y range only duplicates. Probing with
  `count_neighbors` alone misled twice: test ties and degenerate ranges against R.
- kde2d: `MASS::kde2d(n = 100, lims = scale limits, h = max(sqrt(eps), bandwidth.nrd) *
  adjust)`; kde2d divides `h` by 4 (sd), `bandwidth.nrd` = 4 x `bw.nrd` with no zero-IQR
  fallback (ggrust's `Bandwidth::Nrd` has one, so it is not reused). Grid = R `seq()`
  (both ends exact, or `findInterval` of the max point moves a node); lookup is the node
  below-left, not the nearest. `Density2dOptions::density` reused.
- `method="auto"` (default): kde2d when any group has more than 20000 rows.
- Fast exact counting: cells 1/16 of the ellipse semi-axes (max 2048 per axis), points
  sorted by cell so the cell offsets are row prefix sums; per query and grid row, cells
  wholly inside add their count, border cells test points. A 1e-6-cell margin plus 4 ulps of the
  coordinates keeps it exact (1e15 offsets broke a cells-only margin). Rule 3 extras come
  from same-y runs sorted by x (binary search). 1M UMAP points: 0.45 s over geom_point (10-core M4); R's O(n^2)
  counter takes 2.7 s at 50k.
- Reading numbers back in pytest: `geom="text"` labels with a marker color; they carry
  6 significant digits (`label_format` is not supported on geom_text).
