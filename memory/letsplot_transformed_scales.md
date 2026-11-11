# Transformed position scales: lets-plot traps (Phase 0.10.2a)

Verified against lets-plot 4.11 `to_svg()` (`tests/test_breaks_parity.py`, 0/96 random
transformed axes differ).

- **Data is transformed at mapping time**; domain, limits and stats are in transformed units.
- **ggrust no longer follows this (user, 2026-11-11):** it pads in transformed units, as
  ggplot2; parity tests against lets-plot use `expand=[0, 0]` on transformed axes.
  lets-plot's rule: **expansion is computed in data units** (`PlotUtil.rangeWithExpand`): inverse the ends,
  pad by 5% of the *data* span, transform back; an end that becomes NaN (log/sqrt below
  0) stays unexpanded. Expanding in transformed units gives wrong breaks (log10 3..30).
- **Log axes accept every finite positive value** (ggplot2): lets-plot's domain stops at
  `MAX_VALUE = f64::MAX / 1e5` and drops 1e304. `LOG_MAX_VALUE` is `f64::MAX`; the inverse
  is capped there because `powf` at the clamped top can round past it.
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
