# lets-plot drops polars Categorical order

Source: `lets_plot/plot/util.py` (series annotations).

lets_plot emits `data_meta.series_annotations[].factor_levels` only for:
- pandas `category` dtype **with `ordered=True`**
- polars `Enum`
- `as_discrete(...)` levels

Plain polars `Categorical` and unordered pandas categoricals get no levels, so lets-plot
orders those categories by appearance or alphabetically, not by the categorical's order.

**How to apply:** in the Arrow data bridge (0.3), read category order from the Arrow
dictionary. This is a behavior difference from lets-plot (likely an improvement for
cellestial, which uses polars). Ask the user before deciding which order wins
when `factor_levels` and the Arrow dictionary disagree.
Fixtures `pandas_categorical`, `polars_enum` and `umap_categorical` pin the lets_plot behavior.

**Measured 2026-10-28 (polars 1.41 and 2.1, lets-plot 4.11):** polars exports a plain
`Categorical` with a compacted dictionary of the values present, in first-appearance order of
the exported rows, so ggrust's dictionary rule gives lets-plot's order for polars. Only pandas
*unordered* categoricals (and hand-built Arrow dictionaries) differ: ggrust takes the category
order, lets-plot first appearance. ggplot2 also uses factor levels, ordered or not.
A colour order that changes between runs is a data problem upstream (e.g. cellestial's
`mode().first()` on ties), not this rule: check `unique(maintain_order=True)` of the frame first.

**Decided by the user (2026-10-28):** keep the rule. Unordered pandas categoricals use their
category order (ggplot2), a recorded deviation from lets-plot. Do not "fix" it toward
first appearance.

