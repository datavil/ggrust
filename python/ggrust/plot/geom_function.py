"""``geom_function()``: draw a Python function as a line.

Ported from lets-plot's ``lets_plot/plot/geom_function_.py`` (MIT, see
THIRD_PARTY_NOTICES.md). The function is evaluated in Python; the result is an
ordinary layer (``geom_line`` by default) on columns ``x`` and ``y``.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, cast

from ggrust.errors import UnsupportedDataError
from ggrust.plot.core import LayerSpec, aes
from ggrust.plot.geom import _geom

if TYPE_CHECKING:
    from collections.abc import Sequence

    from ggrust._typing import Function, ManualKey, Number, PlotData, Position, Tooltips
    from ggrust.plot.core import FeatureSpec


__all__ = ["geom_function"]

_FUNCTION_X, _FUNCTION_Y = "x", "y"
_DEFAULT_XLIM = (0.0, 1.0)
_DEFAULT_N = 512


def geom_function(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    stat: str | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    manual_key: ManualKey | None = None,
    tooltips: Tooltips | None = None,
    fun: Function | None = None,
    xlim: Sequence[Number] | None = None,
    n: int | None = None,
    color_by: str | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Evaluate ``fun`` at x values and draw the result.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping; ``x`` may name a column of ``data`` or be a list of values.
    data : DataFrame or dict, optional
        Data holding the x column; a ``y`` column with the results is added.
    stat : str, default "identity"
        Stat of the layer.
    geom : str, default "line"
        Geom of the layer.
    fun : callable, optional
        Function of one number; None gives an empty ``y``.
    xlim : list of float, optional
        ``[low, high]`` of the evaluated x range when ``x`` is not mapped (default [0, 1]).
    n : int, optional
        Number of evaluated points in ``xlim`` (default 512).
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        geom if geom is not None else "line",
        mapping=_function_mapping(mapping),
        data=_function_data(mapping, data, fun, xlim, n),
        stat=stat if stat is not None else "identity",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=None,
        tooltips=tooltips,
        color_by=color_by,
        **other_args,
    )


def _linspace(start: float, stop: float, count: int) -> list[float]:
    if count == 1:
        return [start]
    step = (stop - start) / (count - 1)
    return [start + step * index for index in range(count)]


def _function_data(
    mapping: FeatureSpec | None,
    data: PlotData | None,
    fun: Function | None,
    xlim: Sequence[Number] | None,
    n: int | None,
) -> PlotData:
    x_mapping = mapping.as_dict().get("x") if mapping is not None else None
    # pandas or polars frames, used by duck typing as lets_plot does.
    table = cast("Any", data)
    if isinstance(x_mapping, str) and data is not None:
        xs = table[x_mapping]
    elif hasattr(x_mapping, "__iter__"):
        xs = x_mapping
    else:
        start, stop = xlim if xlim is not None else _DEFAULT_XLIM
        xs = _linspace(start, stop, n if n is not None else _DEFAULT_N)
    ys = [None] * len(xs) if fun is None else [fun(x) for x in xs]
    if data is None:
        return {_FUNCTION_X: xs, _FUNCTION_Y: ys}
    if isinstance(data, dict):
        return {**table, _FUNCTION_Y: ys}
    module = type(data).__module__.split(".")[0]
    if module == "pandas":
        return table.assign(**{_FUNCTION_Y: ys})
    if module == "polars":
        import polars as pl

        return table.with_columns(pl.Series(_FUNCTION_Y, ys))
    message = (
        f"geom_function(): unsupported data type {type(data).__name__}; pass a dict, a pandas "
        "DataFrame or a polars DataFrame"
    )
    raise UnsupportedDataError(message)


def _function_mapping(mapping: FeatureSpec | None) -> FeatureSpec:
    mapping_dict = mapping.as_dict() if mapping is not None else {}
    return aes(**{"x": _FUNCTION_X, **mapping_dict, "y": _FUNCTION_Y})
