# cellestial reads PlotSpec internals

cellestial does not only call `geom_*`; it inspects and rebuilds specs:

- `plot.as_dict()["mapping"]`, `["layers"][i]["mapping"]`, `["data"]`, `["figures"][i]["data"]`
  (`cellestial/util/operations.py`, `util/utilities.py`).
- `mapping.as_dict()` on `aes(...)` results, merged with `|` / `update`.
- Imports `PlotSpec`, `LayerSpec`, `FeatureSpec`, `FeatureSpecArray` from
  `lets_plot.plot.core` and `SupPlotsSpec` from `lets_plot.plot.subplots`.
- `cellestial/layers/_deferred.py` relies on `PlotSpec.__add__` raising `TypeError` for
  unknown types so `__radd__` of deferred layers is tried.
- `gggrid` must accept `PlotSpec` objects, not `as_dict()` output.

**How to apply:** ggrust keeps these class names, module paths (`ggrust.plot.core`,
`ggrust.plot.subplots`), dict keys and `+` semantics identical to lets_plot. Python API
compatibility tests assert `as_dict()` equality against lets_plot for the same calls.
