# lets-plot semantics that are easy to get wrong

Found by the Phase 0.5 bug hunt; each has a regression test in
`crates/ggrust-core/tests/build_regressions.rs`.

- **alpha replaces, never multiplies.** `Color.withOpacity` sets the alpha. `alpha` on a
  layer replaces the color's alpha only when given (lets-plot's default 0.999887 means
  "unset"; ggrust uses NaN as the unset default). `"red / 0.5"` and `"transparent / 0.5"`
  also replace (`transparent / 0.5` = rgba(0,0,0,128)).
- **Layer data + plot data merge** (`DataConfigUtil`): if both exist and have the same
  row count, plot columns fill in what the layer data lacks. Different row counts: layer
  data only.
- **Constant positions are allowed:** `geom_point(x=1.5, y=1.5)` draws; with no data, one
  row.
- **Option aliases:** `colour` / `col` -> `color` in layer options too, not only mappings.
- **Zero-width test is lets-plot's `isBeyondPrecision`** (abs 1e-50, 12 decimal places),
  never `f64::EPSILON`-relative with a floor of 1 (that collapsed [1e-20, 3e-20]).
- **SVG attributes must be written once.** Decide composite styles (stroke color +
  width) in one place; the XML well-formedness test parses every scene with roxmltree.

## Grouping (`DataProcessing.computeGroups`, `LayerConfig.explicitGroupingVarNames`)
- Explicit `group` from plot and layer is **combined** (plot + layer, distinct), not overridden like other aesthetics; `inherit_aes=False` drops the plot's. `group=[]` turns grouping off.
- Default grouping: mapped, non-positional, discrete columns (distinct by column name). Continuous style columns never group.
- One variable: group = index in distinct values (discrete: level order; continuous: first appearance), missing values one extra group last. Several: combinations numbered by first appearance (so variable order does not matter).
