"""ggpp's ``stat_centroid`` and ``stat_summary_xy``: one row per group at its centre.

Not in lets-plot; in ggplot2 they need the ggpp package. ggpp takes R functions
(``.fun``, ``.fun.x``, ``.fun.y``); ggrust takes their names, ``"mean"`` or
``"median"``. Typical use puts each cluster's name at the cluster's centre instead of
in a legend: ``geom_text(aes(label="cluster"), stat="centroid")``.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Literal

from ggrust.plot.geom import _geom

if TYPE_CHECKING:
    from ggrust._typing import PlotData, Position, Tooltips
    from ggrust.plot.core import FeatureSpec, LayerSpec

__all__ = ["stat_centroid", "stat_summary_xy"]

CentroidFunction = Literal["mean", "median"]
"""Summaries ``fun``, ``fun_x`` and ``fun_y`` accept."""


def stat_centroid(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = False,
    inherit_aes: bool | None = None,
    tooltips: Tooltips | None = None,
    fun: CentroidFunction | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """One row per group at the centroid of its x and y (``stat="centroid"``), as ggpp.

    Each group (and facet panel) becomes one row at ``fun`` of its x and of its y, over
    the rows where both are finite. Other mapped columns keep one value per group, so
    map ``label`` (or ``color``) to the column that defines the groups. Also usable
    as ``geom_text(stat="centroid")`` or ``geom_label(stat="centroid")``. Positions
    are summarised in axis units, as ggplot2: on a log axis the mean is geometric.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``: ``x``, ``y`` (required), and the geom's own,
        such as ``label`` or ``color``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, default="point"
        Geom that draws the centres, e.g. ``"text"`` or ``"label"``.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"identity"`` or a ``position_*()`` spec.
    show_legend : bool, default=False
        True adds this layer to the legend (ggpp leaves it out).
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    fun : {"mean", "median"}, default="mean"
        Summary of x and of y. ``"median"`` resists outlying cells.
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``size=6``, and the geom's options.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> import numpy as np
    >>> rng = np.random.default_rng(1)
    >>> centres = {"apples": (0, 0), "pears": (4, 1), "plums": (2, 4)}
    >>> fruit = [name for name in centres for _ in range(100)]
    >>> data = {
    ...     "width": [centres[name][0] + rng.normal() for name in fruit],
    ...     "height": [centres[name][1] + rng.normal() for name in fruit],
    ...     "fruit": fruit,
    ... }
    >>> plot = (
    ...     ggplot(data, aes("width", "height"))
    ...     + geom_point(aes(color="fruit"), alpha=0.4, show_legend=False)
    ...     + stat_centroid(aes(label="fruit"), geom="label", fun="median", size=6)
    ... )
    """
    return _geom(
        geom if geom is not None else "point",
        mapping=mapping,
        data=data,
        stat="centroid",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        tooltips=tooltips,
        fun=fun,
        na_rm=na_rm,
        **other_args,
    )


def stat_summary_xy(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = False,
    inherit_aes: bool | None = None,
    tooltips: Tooltips | None = None,
    fun_x: CentroidFunction | None = None,
    fun_y: CentroidFunction | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """One row per group at a summary of x and of y (``stat="summary_xy"``), as ggpp.

    As ``stat_centroid()``, with a summary per axis: ``fun_x`` of x and ``fun_y`` of
    y, over the rows where both are finite.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``: ``x``, ``y`` (required), and the geom's own,
        such as ``label`` or ``color``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, default="point"
        Geom that draws the summaries, e.g. ``"text"`` or ``"label"``.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"identity"`` or a ``position_*()`` spec.
    show_legend : bool, default=False
        True adds this layer to the legend (ggpp leaves it out).
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    fun_x : {"mean", "median"}, default="mean"
        Summary of x.
    fun_y : {"mean", "median"}, default="mean"
        Summary of y.
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``size=6``, and the geom's options.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"day": [1, 2, 3, 1, 2, 3], "visits": [5, 9, 40, 2, 3, 4], "shop": list("aaabbb")}
    >>> plot = (
    ...     ggplot(data, aes("day", "visits", color="shop"))
    ...     + geom_point()
    ...     + stat_summary_xy(fun_x="mean", fun_y="median", shape=4, size=6)
    ... )
    """
    return _geom(
        geom if geom is not None else "point",
        mapping=mapping,
        data=data,
        stat="summary_xy",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        tooltips=tooltips,
        fun_x=fun_x,
        fun_y=fun_y,
        na_rm=na_rm,
        **other_args,
    )
