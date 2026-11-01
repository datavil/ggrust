# Deviations from lets-plot and ggplot2

Every place where ggrust behaves differently from lets-plot or from ggplot2. Rules in
[AGENTS.md](AGENTS.md) ("Record every deviation"). Add a row in the same commit that
introduces the deviation. Keep rows short; long reasoning goes in the topic `memory/` entry.

Kind: `chosen` (deliberate), `gap` (not implemented yet or known difference to fix),
`inherited` (ggrust follows lets-plot where ggplot2 differs).

| Topic | lets-plot | ggplot2 | ggrust | Kind | Why | Decided |
|---|---|---|---|---|---|---|
| Identity bar stacking (#1398) | `mode="groups"` default: rows of one group at one x overlap | stacks every row | identity `geom_bar`/`geom_histogram` stack every row; other geoms/stats keep `groups` (stats repeat x in a group) | chosen | overlapping bars hide data | plan step lp3.1, 2026-11-01 |
| Free facet stat ranges (#1443) | bins every panel over the whole plot's range | panel's own range | panel's own range on a free axis | chosen | follows ggplot2 | plan step lp3.2, 2026-11-01 |
| Axis title justification (#1504) | ignores `angle`, `vjust` | `rotate_just`; inherited `vjust=1` puts a horizontal y title at the top | `rotate_just`; unset hjust/vjust centre | chosen | ggplot2 placement, lets-plot's centred default | user (angle) and plan step lp3.5, 2026-11-01 |
| Collected legend overflow (#1506) | run off the canvas | (patchwork) run off | wrap, reflow, then cut to fit the grid; stacked legends split room by water-filling | chosen | legends never leave the canvas | plan step lp3.4, 2026-11-01 |
| Shared discrete axes (#1015) | `sharex`/`sharey` share continuous axes only | no gggrid | discrete axes share the union of categories | chosen | categories line up across plots | plan step lp3.6, 2026-11-01 |
| Wide end axis labels (#906) | cut at the canvas edge | cut at the device edge | the panel narrows so the label fits (only when it overflows) | chosen | labels are never cut | user, 2026-11-01 |
| Shared axes order of `as_discrete(levels=)` | n/a | n/a | union in order of appearance; explicit level orders not merged across plots | gap | rare | lp3.6 bug hunt, 2026-11-01 |
| Free scales in `facet_grid` stats | whole range | per grid column/row | facet_grid not built | gap | facet_grid parsed only | lp3.2, 2026-11-01 |
| `geom_step(direction="mid")` (#838) | not accepted (error) | steps halfway between x values | ggplot2's `stairstep` vertices and row styles | chosen | ggplot2 has it; lets-plot asks for it | plan step lp4.1, 2026-11-01 |
| `geom_step(padded=True)` with x = -inf / +inf | drops the infinite rows ("Removed 2 rows") | no `padded` | draws them at the panel's x edges; a transform or limits on x still drop them (reported) | chosen | the option's documented intent | lp4.1 bug hunt, 2026-11-01 |
| `geom_freqpoly` end bins | no padding | `pad = TRUE`: a zero-count bin added at both ends | no padding | inherited | parity | plan step lp4.1, 2026-11-01 |
