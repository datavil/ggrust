"""Stats from ggplot2 and plotnine that lets_plot does not have.

``stat_unique``, ``stat_hull`` and ``stat_summary_2d``.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Literal

from ggrust.plot.geom import _geom

if TYPE_CHECKING:
    from collections.abc import Sequence

    from ggrust._typing import Number, PlotData, Position, Tooltips
    from ggrust.plot.core import FeatureSpec, LayerSpec

__all__ = ["stat_hull", "stat_summary_2d", "stat_unique"]

Summary2dFunction = Literal["count", "sum", "mean", "median", "min", "max"]
"""Aggregate names ``stat_summary_2d`` accepts."""


def stat_unique(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    tooltips: Tooltips | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Drop duplicate rows before drawing (``stat="unique"``), as ggplot2.

    Rows count as duplicates when every mapped aesthetic and the group are equal;
    the first one is kept. Useful to draw one label or point per repeated value.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, default="point"
        Geom that draws the rows.
    position : str or FeatureSpec, optional
        Position adjustment: a name or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content, or ``"none"`` (HTML output).
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics and the geom's options.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"city": ["Oslo", "Oslo", "Rome", "Rome"], "temp": [4, 4, 18, 18]}
    >>> plot = ggplot(data, aes("city", "temp")) + stat_unique(geom="text", label="here")
    """
    return _geom(
        geom if geom is not None else "point",
        mapping=mapping,
        data=data,
        stat="unique",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        tooltips=tooltips,
        na_rm=na_rm,
        **other_args,
    )


def stat_hull(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    tooltips: Tooltips | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Draw the convex hull of each group (``stat="hull"``), as plotnine.

    The hull's vertices run counter-clockwise and the first is repeated at the end,
    so the default ``geom_path`` closes it; ``geom="polygon"`` fills it. Computes
    ``..area..``, the enclosed area. Groups of fewer than three non-collinear points
    draw nothing.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, default="path"
        Geom that draws the hull, e.g. ``"polygon"``.
    position : str or FeatureSpec, optional
        Position adjustment: a name or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content, or ``"none"`` (HTML output).
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics and the geom's options.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"height": [150, 160, 170, 165, 180, 175], "weight": [50, 62, 70, 58, 85, 72],
    ...         "team": ["a", "a", "a", "b", "b", "b"]}
    >>> plot = ggplot(data, aes("height", "weight", color="team")) + geom_point()
    >>> plot = plot + stat_hull()
    """
    return _geom(
        geom if geom is not None else "path",
        mapping=mapping,
        data=data,
        stat="hull",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        tooltips=tooltips,
        na_rm=na_rm,
        **other_args,
    )


def stat_summary_2d(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    tooltips: Tooltips | None = None,
    fun: Summary2dFunction | None = None,
    bins: Number | Sequence[Number] | None = None,
    binwidth: Number | Sequence[Number] | None = None,
    drop: bool | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Summarise ``z`` over a grid of rectangles (``stat="summary2d"``), as ggplot2.

    The grid follows ``geom_bin2d``. Computes ``..value..`` (``fun`` of the bin's
    ``z``), which ``fill`` is mapped to, and the bin sizes.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``; needs ``x``, ``y`` and ``z``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, default="bin2d"
        Geom that draws the bins (tiles sized by the bin).
    position : str or FeatureSpec, optional
        Position adjustment: a name or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content, or ``"none"`` (HTML output).
    fun : {"mean", "median", "sum", "count", "min", "max"}, default="mean"
        The summary of each bin's ``z``.
    bins : int or list of int, default=30
        Number of bins along x and y (one number for both).
    binwidth : float or list of float, optional
        Bin width along x and y; wins over ``bins``.
    drop : bool, default=True
        False keeps bins without data, with a missing value.
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics and the geom's options.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"lon": [1, 2, 2, 3, 4, 4], "lat": [1, 1, 2, 3, 3, 4],
    ...         "rain": [20, 35, 30, 10, 5, 8]}
    >>> plot = ggplot(data, aes("lon", "lat", z="rain")) + stat_summary_2d(bins=4)
    """
    return _geom(
        geom if geom is not None else "bin2d",
        mapping=mapping,
        data=data,
        stat="summary2d",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        tooltips=tooltips,
        fun=fun,
        bins=bins,
        binwidth=binwidth,
        drop=drop,
        na_rm=na_rm,
        **other_args,
    )
