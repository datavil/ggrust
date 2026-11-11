"""Standalone ``stat_*`` layers for stats lets_plot reaches only through ``stat=``.

``stat_bin``, ``stat_bin2d``, ``stat_boxplot``, ``stat_count``, ``stat_density``,
``stat_density2d``, ``stat_identity``, ``stat_sina``, ``stat_smooth`` and
``stat_ydensity``, as in ggplot2 and plotnine. Each one is the stat's usual geom
function with that stat (``stat_bin()`` is ``geom_histogram(stat="bin")``), or, with
``geom=``, a layer of that geom drawing the stat's rows. They add no stat logic.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Literal

from ggrust.plot.geom import (
    _geom,
    geom_bar,
    geom_bin2d,
    geom_boxplot,
    geom_density,
    geom_density2d,
    geom_histogram,
    geom_point,
    geom_sina,
    geom_smooth,
    geom_violin,
)

if TYPE_CHECKING:
    from collections.abc import Callable, Mapping, Sequence

    from ggrust._typing import Breaks, Number, PlotData, Position, Tooltips
    from ggrust.plot.core import FeatureSpec

__all__ = [
    "stat_bin",
    "stat_bin2d",
    "stat_boxplot",
    "stat_count",
    "stat_density",
    "stat_density2d",
    "stat_identity",
    "stat_sina",
    "stat_smooth",
    "stat_ydensity",
]


def _stat_layer(
    stat: str,
    default_geom: Callable[..., FeatureSpec],
    geom: str | None,
    /,
    **arguments: Any,
) -> FeatureSpec:
    """Return the default geom function's layer with ``stat``, or a ``geom`` layer.

    Parameters
    ----------
    stat : str
        The stat's name in the spec.
    default_geom : callable
        The ``geom_*`` function drawing this stat by default.
    geom : str, optional
        Another geom, by name; None uses ``default_geom``.
    **arguments
        The layer's arguments.

    Returns
    -------
    FeatureSpec
        The layer.

    Raises
    ------
    TypeError
        If ``stat`` is passed again in ``arguments``.
    """
    if "stat" in arguments:
        message = (
            f"stat_{stat}() sets stat='{stat}' itself and takes no 'stat' argument; "
            f"use geom_*(stat=...) for another stat, or geom= to draw '{stat}' differently."
        )
        raise TypeError(message)
    if geom is None:
        return default_geom(stat=stat, **arguments)
    return _geom(geom, stat=stat, **arguments)


def stat_bin(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    tooltips: Tooltips | None = None,
    orientation: str | None = None,
    bins: int | None = None,
    binwidth: Number | None = None,
    center: Number | None = None,
    boundary: Number | None = None,
    breaks: Breaks | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> FeatureSpec:
    """Count rows in bins of x (``stat="bin"``); drawn as a histogram by default.

    Computes ``..count..``, ``..density..``, ``..sumprop..`` and ``..sumpct..``.

    ``stat_bin(...)`` is ``geom_histogram(stat="bin", ...)``; another ``geom`` draws the
    same computed rows with that geom.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, optional
        Geom that draws the computed rows, by name; ``"histogram"`` when None.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    orientation : str, optional
        Set to ``"y"`` to draw the geom along the y axis.
    bins : int, optional
        Number of bins (30 when neither ``bins`` nor ``binwidth`` is set).
    binwidth : float, optional
        Width of the bins; overrides ``bins``.
    center : float, optional
        A bin center, to align the bins.
    boundary : float, optional
        A bin boundary, to align the bins.
    breaks : list of float, optional
        Bin boundaries; overrides ``bins``, ``binwidth``, ``center`` and ``boundary``.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Options of the geom and constant aesthetics such as ``color="red"``.

    Returns
    -------
    FeatureSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"height": [150, 152, 158, 160, 161, 163, 165, 168, 170, 171, 175, 182]}
    >>> plot = ggplot(data, aes("height")) + stat_bin(bins=5)
    """
    return _stat_layer(
        "bin",
        geom_histogram,
        geom,
        mapping=mapping,
        data=data,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        tooltips=tooltips,
        orientation=orientation,
        bins=bins,
        binwidth=binwidth,
        center=center,
        boundary=boundary,
        breaks=breaks,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def stat_bin2d(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    tooltips: Tooltips | None = None,
    bins: int | Sequence[int] | None = None,
    binwidth: float | Sequence[float | None] | None = None,
    drop: bool | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> FeatureSpec:
    """Count rows in rectangular bins of x and y (``stat="bin2d"``); drawn as tiles by default.

    Computes ``..count..`` and ``..density..``.

    ``stat_bin2d(...)`` is ``geom_bin2d(stat="bin2d", ...)``; another ``geom`` draws the
    same computed rows with that geom.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``: ``x``, ``y`` (required), ``fill``, ``color``,
        ``alpha``, ``size``, ``linetype``, ``width``, ``height``, ``weight``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, optional
        Geom that draws the computed rows, by name; ``"bin2d"`` when None.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"identity"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    bins : int or list of int, default=[30, 30]
        Number of bins along x and y; one number is used for both axes.
    binwidth : float or list of float, optional
        Bin width along x and y (a None entry keeps ``bins`` for that axis); wins over
        ``bins``.
    drop : bool, default=True
        False also draws the empty bins (count 0).
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Options of the geom and constant aesthetics such as ``color="red"``.

    Returns
    -------
    FeatureSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 2, 3, 3, 3, 4, 4, 5], "y": [2, 1, 2, 3, 2, 3, 4, 3, 5]}
    >>> plot = ggplot(data, aes("x", "y")) + stat_bin2d(bins=[4, 4])
    """
    return _stat_layer(
        "bin2d",
        geom_bin2d,
        geom,
        mapping=mapping,
        data=data,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        tooltips=tooltips,
        bins=bins,
        binwidth=binwidth,
        drop=drop,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def stat_boxplot(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    tooltips: Tooltips | None = None,
    orientation: str | None = None,
    coef: Number | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> FeatureSpec:
    """Quartiles, whiskers and outliers of y per x (``stat="boxplot"``), drawn as boxplots.

    Outliers are drawn by a second point layer, as ``geom_boxplot``.

    ``stat_boxplot(...)`` is ``geom_boxplot(stat="boxplot", ...)``; another ``geom`` draws the
    same computed rows with that geom.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, optional
        Geom that draws the computed rows, by name; ``"boxplot"`` when None.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    orientation : str, optional
        ``"y"`` draws along the y axis; a discrete y with a continuous x picks it by default.
    coef : float, default=1.5
        Whisker length as a multiple of the interquartile range; rows beyond the
        whiskers are outliers.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Options of the geom and constant aesthetics such as ``color="red"``.

    Returns
    -------
    FeatureSpec
        The layer; the default boxplot geom adds the outlier layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"day": ["Mon"] * 5 + ["Tue"] * 5,
    ...         "minutes": [12, 15, 14, 30, 13, 20, 22, 19, 21, 18]}
    >>> plot = ggplot(data, aes("day", "minutes")) + stat_boxplot(coef=1.0)
    """
    return _stat_layer(
        "boxplot",
        geom_boxplot,
        geom,
        mapping=mapping,
        data=data,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        tooltips=tooltips,
        orientation=orientation,
        coef=coef,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def stat_count(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    tooltips: Tooltips | None = None,
    orientation: str | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> FeatureSpec:
    """Count rows per x (``stat="count"``); drawn as bars by default.

    Computes ``..count..``, ``..prop..`` and ``..proppct..``.

    ``stat_count(...)`` is ``geom_bar(stat="count", ...)``; another ``geom`` draws the
    same computed rows with that geom.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, optional
        Geom that draws the computed rows, by name; ``"bar"`` when None.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    orientation : str, optional
        ``"y"`` draws along the y axis; a discrete y with a continuous x picks it by default.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Options of the geom and constant aesthetics such as ``color="red"``.

    Returns
    -------
    FeatureSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"fruit": ["apple", "pear", "apple", "plum", "apple", "pear"]}
    >>> plot = ggplot(data, aes("fruit")) + stat_count(geom="point", size=5)
    """
    return _stat_layer(
        "count",
        geom_bar,
        geom,
        mapping=mapping,
        data=data,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        tooltips=tooltips,
        orientation=orientation,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def stat_density(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    tooltips: Tooltips | None = None,
    orientation: str | None = None,
    trim: bool | None = None,
    kernel: str | None = None,
    adjust: Number | None = None,
    bw: str | Number | None = None,
    n: int | None = None,
    fs_max: Number | None = None,
    quantiles: Sequence[float] | None = None,
    bounds: Sequence[Number | None] | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> FeatureSpec:
    """Kernel density estimate of x (``stat="density"``); drawn as a density curve by default.

    Computes ``..density..``, ``..count..`` and ``..scaled..``.

    ``stat_density(...)`` is ``geom_density(stat="density", ...)``; another ``geom`` draws the
    same computed rows with that geom.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. ``weight`` weighs the rows.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, optional
        Geom that draws the computed rows, by name; ``"density"`` when None.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"stack"`` or a ``position_*()`` spec;
        ``"identity"`` when None.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    orientation : str, optional
        ``"y"`` estimates the density of ``y``.
    trim : bool, default=False
        True evaluates the density over the group's data range only; otherwise over
        the x range of every layer.
    kernel : str, default="gaussian"
        Kernel: ``"gaussian"``, ``"rectangular"``, ``"triangular"``, ``"biweight"``,
        ``"epanechikov"``, ``"optcosine"`` or ``"cosine"``.
    adjust : float, default=1
        Multiplier of the bandwidth.
    bw : str or float, default="nrd0"
        Bandwidth: a positive number, or ``"nrd0"`` or ``"nrd"``.
    n : int, default=512
        Number of points the density is evaluated at (2 to 1024).
    fs_max : float, default=5000
        Above this many rows, only rows within 5 bandwidths of a point are summed.
    quantiles : list of float, default=[0.25, 0.5, 0.75]
        Quantiles that split the area into bands (``..quantile..``), each in [0, 1].
    bounds : list of [float or None, float or None], optional
        Known lower and upper bounds of the data (``None`` or ``inf`` for an open end),
        as ggplot2: values outside are dropped and the kernels are reflected at each
        finite bound, so the density does not leak past it. Not in lets_plot.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Options of the geom and constant aesthetics such as ``color="red"``.

    Returns
    -------
    FeatureSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"height": [150, 152, 158, 160, 161, 163, 165, 168, 170, 171, 175, 182]}
    >>> plot = ggplot(data, aes("height")) + stat_density(geom="line", adjust=0.8)
    """
    return _stat_layer(
        "density",
        geom_density,
        geom,
        mapping=mapping,
        data=data,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        tooltips=tooltips,
        orientation=orientation,
        trim=trim,
        kernel=kernel,
        adjust=adjust,
        bw=bw,
        n=n,
        fs_max=fs_max,
        quantiles=quantiles,
        bounds=bounds,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def stat_density2d(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    tooltips: Tooltips | None = None,
    kernel: str | None = None,
    adjust: float | None = None,
    bw: str | float | Sequence[float] | None = None,
    n: int | Sequence[int] | None = None,
    bins: int | None = None,
    binwidth: float | None = None,
    breaks: Sequence[float] | None = None,
    contour_var: Literal["density", "ndensity", "count"] | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> FeatureSpec:
    """Contours of a 2D kernel density estimate of x and y (``stat="density2d"``).

    Computes ``..level..``.

    ``stat_density2d(...)`` is ``geom_density2d(stat="density2d", ...)``; another ``geom`` draws the
    same computed rows with that geom.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``: ``x``, ``y`` (required), ``color``, ``alpha``,
        ``size``, ``linetype``, ``weight``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, optional
        Geom that draws the computed rows, by name; ``"density2d"`` when None.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"identity"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    kernel : str, default="gaussian"
        Smoothing kernel: ``"gaussian"``, ``"rectangular"``, ``"triangular"``,
        ``"biweight"``, ``"epanechikov"``, ``"optcosine"`` or ``"cosine"``.
    adjust : float, default=1
        Multiplies the bandwidth; above 1 smooths more.
    bw : str or float or list of float, default="nrd0"
        Bandwidth: a rule (``"nrd0"`` or ``"nrd"``, per axis), one number for both
        axes, or ``[x, y]``.
    n : int or list of int, default=100
        Grid points along x and y (2 to 999); one number is used for both axes.
    bins : int, default=10
        Number of levels, evenly spaced over the z range.
    binwidth : float, optional
        Distance between levels; wins over ``bins``.
    breaks : list of float, optional
        Explicit levels in the units of ``contour_var``; win over ``bins`` and
        ``binwidth`` (ggplot2).
    contour_var : {"density", "ndensity", "count"}, default="density"
        What the levels are levels of: the density, the density divided by its maximum
        (``"ndensity"``, comparable across groups and samples), or the density times the
        number of points (``"count"``). ggplot2's parameter; lets_plot has none.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Options of the geom and constant aesthetics such as ``color="red"``.

    Returns
    -------
    FeatureSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 2, 3, 3, 3, 4, 4, 5], "y": [2, 1, 2, 3, 2, 3, 4, 3, 5]}
    >>> plot = ggplot(data, aes("x", "y")) + stat_density2d(bins=5)
    """
    return _stat_layer(
        "density2d",
        geom_density2d,
        geom,
        mapping=mapping,
        data=data,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        tooltips=tooltips,
        kernel=kernel,
        adjust=adjust,
        bw=bw,
        n=n,
        bins=bins,
        binwidth=binwidth,
        breaks=breaks,
        contour_var=contour_var,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def stat_identity(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    tooltips: Tooltips | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> FeatureSpec:
    """Leave the data as it is (``stat="identity"``); drawn as points by default.

    Useful to draw raw rows with any geom, for example ``stat_identity(geom="line")``.

    ``stat_identity(...)`` is ``geom_point(stat="identity", ...)``; another ``geom`` draws the
    same computed rows with that geom.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``: ``x``, ``y`` (required), ``color``, ``fill``,
        ``size``, ``shape``, ``alpha``, ``stroke``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, optional
        Geom that draws the computed rows, by name; ``"point"`` when None.
    position : str or FeatureSpec, optional
        Position adjustment (``"identity"``).
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Options of the geom and constant aesthetics such as ``color="red"``.

    Returns
    -------
    FeatureSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"hour": [6, 9, 12, 15, 18], "temp": [9, 14, 19, 21, 16]}
    >>> plot = ggplot(data, aes("hour", "temp")) + stat_identity(geom="line")
    """
    return _stat_layer(
        "identity",
        geom_point,
        geom,
        mapping=mapping,
        data=data,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        tooltips=tooltips,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def stat_sina(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    tooltips: Tooltips | None = None,
    orientation: str | None = None,
    seed: int | None = None,
    quantiles: Sequence[float] | None = None,
    scale: Number | None = None,
    trim: bool | None = None,
    tails_cutoff: Number | None = None,
    kernel: str | None = None,
    bw: str | Number | None = None,
    adjust: Number | None = None,
    n: int | None = None,
    fs_max: Number | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> FeatureSpec:
    """Points spread within the density of y per x (``stat="sina"``), as a sina plot.

    ``stat_sina(...)`` is ``geom_sina(stat="sina", ...)``; another ``geom`` draws the
    same computed rows with that geom.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, optional
        Geom that draws the computed rows, by name; ``"sina"`` when None.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    orientation : str, optional
        ``"y"`` draws along the y axis; a discrete y with a continuous x picks it by default.
    seed : int, optional
        Random seed, for reproducible results.
    quantiles : list of float, optional
        Quantiles to draw lines at, each in [0, 1].
    scale : float, optional
        Multiplier of the pixel size.
    trim : bool, optional
        True trims the density to the data range.
    tails_cutoff : float, optional
        Density tails are cut where the density falls under this share of the maximum.
    kernel : str, optional
        Kernel of the density estimate, e.g. ``"gaussian"``.
    bw : str or float, optional
        Bandwidth: a number or a rule name such as ``"nrd0"``.
    adjust : float, optional
        Multiplier of the bandwidth.
    n : int, optional
        Number of points the density is evaluated at.
    fs_max : float, optional
        Upper bound of the number of points the density is evaluated at.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Options of the geom and constant aesthetics such as ``color="red"``.

    Returns
    -------
    FeatureSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"day": ["Mon"] * 6 + ["Tue"] * 6,
    ...         "minutes": [12, 15, 14, 16, 13, 15, 20, 22, 19, 21, 18, 20]}
    >>> plot = ggplot(data, aes("day", "minutes")) + stat_sina(seed=1)
    """
    return _stat_layer(
        "sina",
        geom_sina,
        geom,
        mapping=mapping,
        data=data,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        tooltips=tooltips,
        orientation=orientation,
        seed=seed,
        quantiles=quantiles,
        scale=scale,
        trim=trim,
        tails_cutoff=tails_cutoff,
        kernel=kernel,
        bw=bw,
        adjust=adjust,
        n=n,
        fs_max=fs_max,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def stat_smooth(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    tooltips: Tooltips | None = None,
    orientation: str | None = None,
    method: Literal["lm", "loess", "lowess"] | None = None,
    n: int | None = None,
    se: bool | None = None,
    level: Number | None = None,
    span: Number | None = None,
    deg: int | None = None,
    seed: int | None = None,
    max_n: int | None = None,
    fullrange: bool | None = None,
    formula: str | None = None,
    method_args: Mapping[str, Number] | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> FeatureSpec:
    """Fitted curve with a confidence band (``stat="smooth"``); drawn as a smooth by default.

    Computes ``x``, ``y``, ``ymin``, ``ymax`` and ``..se..``.

    ``stat_smooth(...)`` is ``geom_smooth(stat="smooth", ...)``; another ``geom`` draws the
    same computed rows with that geom.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, optional
        Geom that draws the computed rows, by name; ``"smooth"`` when None.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    orientation : str, optional
        ``"y"`` smooths x along y.
    method : {"lm", "loess", "lowess"}, default="lm"
        Fitting method: least squares (``"lm"``) or local regression (``"loess"``,
        ``"lowess"``).
    n : int, default=80
        Number of points the curve is evaluated at.
    se : bool, default=True
        False leaves out the confidence band.
    level : float, default=0.95
        Confidence level of the band, in [0.01, 0.99].
    span : float, default=0.5
        Share of the points each loess fit uses, in (0, 1].
    deg : int, default=1
        Degree of the ``"lm"`` polynomial; 2 or more fits a polynomial.
    seed : int, default=37
        Seed of the row sampling for loess on more than ``max_n`` rows.
    max_n : int, default=1000
        Loess fits a random sample of this many rows per group when it has more.
    fullrange : bool, default=False
        True draws the curve over the plot's whole x range, not only the group's
        data, as ggplot2: lines and polynomials extrapolate, loess stops at its data.
        Not in lets_plot.
    formula : str, optional
        ``"y ~ x"`` or ``"y ~ poly(x, k)"`` (the same as ``deg=k``, with
        ``method="lm"``), as ggplot2. Not in lets_plot.
    method_args : dict, optional
        Arguments of the method, as ggplot2's ``method.args``: ``{"span": 0.3}`` for
        loess. Not in lets_plot.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Options of the geom and constant aesthetics such as ``color="red"``.

    Returns
    -------
    FeatureSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> frame = {"hours": [1, 2, 3, 4, 5, 6], "score": [52, 55, 61, 64, 70, 71]}
    >>> plot = ggplot(frame, aes("hours", "score")) + geom_point() + stat_smooth(se=False)
    """
    return _stat_layer(
        "smooth",
        geom_smooth,
        geom,
        mapping=mapping,
        data=data,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        tooltips=tooltips,
        orientation=orientation,
        method=method,
        n=n,
        se=se,
        level=level,
        span=span,
        deg=deg,
        seed=seed,
        max_n=max_n,
        fullrange=fullrange,
        formula=formula,
        method_args=method_args,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def stat_ydensity(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    tooltips: Tooltips | None = None,
    orientation: str | None = None,
    quantiles: Sequence[float] | None = None,
    scale: Number | None = None,
    trim: bool | None = None,
    tails_cutoff: Number | None = None,
    kernel: str | None = None,
    bw: str | Number | None = None,
    adjust: Number | None = None,
    n: int | None = None,
    fs_max: Number | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> FeatureSpec:
    """Kernel density of y per x (``stat="ydensity"``); drawn as violins by default.

    Computes ``..violinwidth..``, ``..density..``, ``..count..`` and ``..scaled..``.

    ``stat_ydensity(...)`` is ``geom_violin(stat="ydensity", ...)``; another ``geom`` draws the
    same computed rows with that geom.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, optional
        Geom that draws the computed rows, by name; ``"violin"`` when None.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    orientation : str, optional
        ``"y"`` draws along the y axis; a discrete y with a continuous x picks it by default.
    quantiles : list of float, optional
        Quantiles to draw lines at, each in [0, 1].
    scale : float, optional
        Multiplier of the pixel size.
    trim : bool, optional
        True trims the density to the data range.
    tails_cutoff : float, optional
        Density tails are cut where the density falls under this share of the maximum.
    kernel : str, optional
        Kernel of the density estimate, e.g. ``"gaussian"``.
    bw : str or float, optional
        Bandwidth: a number or a rule name such as ``"nrd0"``.
    adjust : float, optional
        Multiplier of the bandwidth.
    n : int, optional
        Number of points the density is evaluated at.
    fs_max : float, optional
        Upper bound of the number of points the density is evaluated at.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Options of the geom and constant aesthetics such as ``color="red"``.

    Returns
    -------
    FeatureSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"day": ["Mon"] * 6 + ["Tue"] * 6,
    ...         "minutes": [12, 15, 14, 16, 13, 15, 20, 22, 19, 21, 18, 20]}
    >>> plot = ggplot(data, aes("day", "minutes")) + stat_ydensity(trim=False)
    """
    return _stat_layer(
        "ydensity",
        geom_violin,
        geom,
        mapping=mapping,
        data=data,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        tooltips=tooltips,
        orientation=orientation,
        quantiles=quantiles,
        scale=scale,
        trim=trim,
        tails_cutoff=tails_cutoff,
        kernel=kernel,
        bw=bw,
        adjust=adjust,
        n=n,
        fs_max=fs_max,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )
