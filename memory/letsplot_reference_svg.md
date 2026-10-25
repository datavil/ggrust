# Calibrate against real lets-plot output: `to_svg()` works offline

`lets_plot` (dev dependency) renders SVG without a browser through its bundled native
library: `(ggplot(...) + geom_point()).to_svg()`. Use it to read exact defaults
instead of guessing from Kotlin sources.

Measured so far (lets-plot 4.11, default theme):
- default size 600 x 400; points: r = 3.3 (size 3 x 2.2 / 2), fill `#474747`, no stroke
- axis text 13 px, title 16 px, grid lines `#E9E9E9` width 1, text color `#474747`
- continuous scales: 5% multiplicative expansion, zero-width domain -> +/- 0.5

**How to apply:** for visual parity work (0.6 layout, 0.8 comparison report) render
the same spec with lets-plot `to_svg()` and with ggrust, then compare numbers/pixels.

## Layout decision (user, Phase 0.6): "same rules, real fonts"
Copy lets-plot's constants and algorithms (breaks, label format, margins, spacing), but
measure text with the embedded font. Text bands use the real line box (~1.12 x size)
instead of lets-plot's "band = font size"; an explicit `axis_title_spacing` (3 px)
reproduces the gap lets-plot gets implicitly from its baseline placement (without it the
rotated y title touched the tick labels). Compare visually with
`cargo run --release --example compare_letsplot -- <dir> tests/visual/output <case>...`.

## Breaks parity (bug hunt 0.6)
Break *counts* are decided with lets-plot's `TextMetricsEstimator` port
(`text::estimated_width`) plus exact ports of `getFlexBreaks` and the union overlap test,
so labels match lets-plot (`tests/test_breaks_parity.py`). Remaining mismatches are
near-ties: real-font y labels make the panel a few px wider, which can flip an x
overlap decided by < 1 px. Fixing that would require estimated widths in layout (the
pixel clone the user declined).
