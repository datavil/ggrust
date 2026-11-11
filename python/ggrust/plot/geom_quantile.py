"""Quantile regression lines: ``geom_quantile`` and ``stat_quantile`` (ggplot2).

Linear lines ``y ~ x`` fitted per group and quantile by minimising the check loss
(the coefficients of ``quantreg::rq``), drawn over the group's x range. Not in
lets-plot.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from ggrust.errors import SpecError
from ggrust.plot.geom import _geom
from ggrust.plot.stat_layer import _stat_layer

if TYPE_CHECKING:
    from collections.abc import Sequence

    from ggrust._typing import ManualKey, PlotData, Position, Sampling, Tooltips
    from ggrust.plot.core import FeatureSpec, LayerSpec

__all__ = ["geom_quantile", "stat_quantile"]

#: ggplot2's only supported model here.
_FORMULAS = ("y ~ x", "y~x")


def _check_model(function: str, formula: str | None, method: str | None) -> None:
    """Reject models other than the linear ``y ~ x`` fitted by ``rq``.

    Raises
    ------
    SpecError
        If ``formula`` or ``method`` asks for another model.
    """
    if formula is not None and formula.replace(" ", "") != "y~x":
        reason = "only the linear formula 'y ~ x' is supported (no splines or other terms)"
        raise SpecError(f"{function}(formula={formula!r}): {reason}", path="formula", reason=reason)
    if method is not None and method != "rq":
        reason = "only method='rq' (linear quantile regression) is supported"
        raise SpecError(f"{function}(method={method!r}): {reason}", path="method", reason=reason)


def _quantile_list(quantiles: float | Sequence[float] | None) -> list[float] | None:
    """``quantiles`` as a list (ggplot2 also takes one number)."""
    if quantiles is None:
        return None
    if isinstance(quantiles, int | float):
        return [float(quantiles)]
    return list(quantiles)


def geom_quantile(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    stat: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    manual_key: ManualKey | None = None,
    sampling: Sampling | None = None,
    tooltips: Tooltips | None = None,
    quantiles: float | Sequence[float] | None = None,
    formula: str | None = None,
    method: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Draw quantile regression lines (``stat="quantile"``, ggplot2).

    One straight line per group and quantile: for quantile tau, the line below
    which about a share tau of the points lie (``quantreg::rq(y ~ x)``). The stat
    computes ``..quantile..``, so ``aes(color="..quantile..")`` tells the lines
    apart.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``: ``x``, ``y`` (required), ``color``, ``alpha``,
        ``size``, ``linetype``, ``group``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name; ``"quantile"`` when None.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"identity"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        This layer's key in a manual legend: a label, or ``layer_key()``.
    sampling : str, int or FeatureSpec, optional
        Rows to draw: a ``sampling_*()`` spec, an int ``n`` or ``"none"``.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"``.
    quantiles : float or list of float, default=[0.25, 0.5, 0.75]
        Probabilities strictly between 0 and 1, one line each.
    formula : str, default="y ~ x"
        Only the linear ``"y ~ x"`` is supported.
    method : str, default="rq"
        Only ``"rq"`` is supported.
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise with a warning.
    **other_args
        Constant aesthetics such as ``color="black"`` or ``size=1``.

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If ``formula`` or ``method`` asks for another model.

    Examples
    --------
    >>> from ggrust import *
    >>> import numpy as np
    >>> rng = np.random.default_rng(1)
    >>> engine = rng.uniform(1, 6, 200)
    >>> data = {"engine": engine, "mpg": 40 - 4 * engine + rng.normal(0, 1, 200) * engine}
    >>> plot = ggplot(data, aes("engine", "mpg")) + geom_point() + geom_quantile()
    """
    _check_model("geom_quantile", formula, method)
    return _geom(
        "quantile",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        quantiles=_quantile_list(quantiles),
        na_rm=na_rm,
        **other_args,
    )


def stat_quantile(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    tooltips: Tooltips | None = None,
    quantiles: float | Sequence[float] | None = None,
    formula: str | None = None,
    method: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> FeatureSpec:
    """Quantile regression lines of y on x (``stat="quantile"``, ggplot2).

    Computes ``..quantile..``. ``stat_quantile(...)`` is
    ``geom_quantile(stat="quantile", ...)``; another ``geom`` draws the same rows.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``: ``x``, ``y`` (required), ``color``, ``group``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, optional
        Geom that draws the computed rows, by name; ``"quantile"`` when None.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"identity"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"``.
    quantiles : float or list of float, default=[0.25, 0.5, 0.75]
        Probabilities strictly between 0 and 1, one line each.
    formula : str, default="y ~ x"
        Only the linear ``"y ~ x"`` is supported.
    method : str, default="rq"
        Only ``"rq"`` is supported.
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise with a warning.
    **other_args
        Options of the geom and constant aesthetics such as ``color="red"``.

    Returns
    -------
    FeatureSpec
        The layer.

    Raises
    ------
    SpecError
        If ``formula`` or ``method`` asks for another model.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3, 4, 5, 6, 7, 8], "y": [1, 3, 2, 5, 4, 7, 5, 9]}
    >>> plot = ggplot(data, aes("x", "y")) + geom_point() + stat_quantile(quantiles=[0.1, 0.9])
    """
    _check_model("stat_quantile", formula, method)
    return _stat_layer(
        "quantile",
        geom_quantile,
        geom,
        mapping=mapping,
        data=data,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        tooltips=tooltips,
        quantiles=_quantile_list(quantiles),
        na_rm=na_rm,
        **other_args,
    )
