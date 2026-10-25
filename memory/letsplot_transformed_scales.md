# Transformed position scales: lets-plot traps (Phase 0.10.2a)

Verified against lets-plot 4.11 `to_svg()` (`tests/test_breaks_parity.py`, 0/96 random
transformed axes differ).

- **Data is transformed at mapping time**; domain, limits and stats are in transformed units.
- **Expansion is computed in data units** (`PlotUtil.rangeWithExpand`): inverse the ends,
  pad by 5% of the *data* span, transform back; an end that becomes NaN (log/sqrt below
  0) stays unexpanded. Expanding in transformed units gives wrong breaks (log10 3..30).
- **Zero-width ranges are widened after expansion**, unpadded (`ensureApplicableRange`):
  a single value v spans exactly v +/- 0.5.
- **Breaks**: identity/reverse linear in data units; sqrt/log linear in transformed units,
  log10 switching to whole powers when that gives >= 3 breaks. Labels use the step to the
  neighbouring break (`NonlinearBreaksHelper.createMultiFormatter`).
- **Number rounding is half-up on the shortest decimal digits** (Kotlin `Double.toString`
  + `Arithmetic.round`), not on the exact binary value: 0.125 -> "0.13", 0.25 -> "0.3".
  Rust `{:.N e}` rounds the exact value half-to-even: do not use it for labels.
- Kotlin `log(x, base)` is `ln(x) / ln(base)`; keep that rounding (not `log10()`).
- **Limits drop rows**: values outside `xlim`/`limits` (or the transform domain) become
  missing, and lets-plot reports "Removed N rows ... outside the scale limits".
- `labs(x=)` beats `scale_x_*(name=)` regardless of order; repeated scales for one
  aesthetic merge key by key, later wins.
