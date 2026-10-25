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
