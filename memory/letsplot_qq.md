# Q-Q layers follow lets-plot

**Works:** `stat/qq.rs` is a line-by-line port of `QQStat`, `QQLineStat`, `QQ2Stat`,
`QQ2LineStat`, `QQStatUtil` (plotting positions `(i - 0.5) / n`, R-8 estimates for the
line). lets-plot's Kotlin source is not checked out next to ggrust any more; read it
with `gh api repos/JetBrains/lets-plot/contents/<path>`.

**Trap:** `sample` is a y position in lets-plot (`Aes.isPositionalY`), so a log y axis
transforms it before the stat, and the line is fitted in log units. Stat variables are
normally data units and go through the scale after the stat; `..sample..` is the
exception (`from_variables` skips it in `build.rs`), or the values are transformed
twice.

**Rows stay rows:** `geom_qq` reorders rows with `LayerData::take`, so a per-row color
follows its value (lets-plot's `Stats.INDEX`). `per_group` would average it.
