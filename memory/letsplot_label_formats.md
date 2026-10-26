# Axis labels: breaks, labels, format follow lets-plot (Phase 1.5a)

- `format` goes through lets-plot `FormatterUtil.byPattern`: braces -> template (`{.1f} %`,
  `{}` = Kotlin `toString`, so `2.0`), a valid number pattern -> one field, otherwise
  literal text. Axis exponents use the POW style: `x·\(10^{n}\)` -> label `x·10` with
  superscript `n` (U+2212 minus). Decimal rounding is half-up on the *shortest decimal
  digits* (0.445 -> .45), never binary float rounding.
- `scale/number_format.rs` is a port of lets-plot `NumberFormat`; its conformance vectors
  (`tests/fixtures/number_format.json`) are extracted from lets-plot's Kotlin tests.
  Re-extract rather than hand-edit when lets-plot changes.
- Continuous fixed breaks: given order kept, nulls and out-of-(expanded)-domain breaks
  dropped with their labels; default labels use `{,~g}` per value. `labels` without
  `breaks` are ignored; a length mismatch fails the plot (as lets-plot).
- Discrete: `breaks` filters categories; `labels` align with `breaks`, else with the axis
  order (reversed scales too); short lists leave empty labels.
- lets-plot also drops a label (and its tick/grid line) that overlaps a label drawn before
  it (`AxisUtil.TickLabelsMap`, unrotated boxes), on every axis kind. ggrust does not (open).
