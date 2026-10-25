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
