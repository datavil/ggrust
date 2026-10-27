# When lets-plot's behaviour looks wrong, ggplot2 decides

The user's rule (2026-10-27): do whatever makes more sense; when stuck between lets-plot
parity and a saner behaviour, check how ggplot2 behaves and do that.

**Why:** parity is the default (cellestial's port should be an import swap), but
lets-plot quirks that make plots depend on accidents (e.g. axis order changing when
`fill` is added) are not worth copying.

**How to apply:** match lets-plot by default. When a lets-plot behaviour is a quirk,
look up ggplot2's (source or docs), follow it, record the deviation in plan.md and
in the topic memory, and keep a test that pins ggrust's choice. First use:
discrete axis order with a stat ([[letsplot_bars_and_positions]]).
