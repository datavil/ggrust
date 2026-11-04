"""Stat layer functions.

Ported from lets-plot's ``lets_plot/plot/stat.py`` (MIT, see THIRD_PARTY_NOTICES.md).
Each builds a geom layer with a fixed stat.
"""

from __future__ import annotations

from numbers import Integral
from typing import TYPE_CHECKING, Any, Literal

from ggrust.errors import SpecError
from ggrust.plot.geom import _geom

if TYPE_CHECKING:
    from collections.abc import Mapping, Sequence

    from ggrust._typing import Number, PlotData, Position, Sampling, Tooltips
    from ggrust.plot.core import FeatureSpec, LayerSpec

__all__ = ["stat_ecdf", "stat_ellipse", "stat_sum", "stat_summary", "stat_summary_bin"]

SummaryFunction = Literal["count", "sum", "mean", "median", "min", "max", "lq", "mq", "uq"]
"""Aggregate names ``fun``, ``fun_min`` and ``fun_max`` accept."""

SummaryDataFunction = Literal["mean_se", "mean_sdl"]
"""ggplot2 ``fun.data`` shortcuts: set y, ymin and ymax together."""


def stat_summary(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    sampling: Sampling | None = None,
    tooltips: Tooltips | None = None,
    orientation: str | None = None,
    fun: SummaryFunction | None = None,
    fun_min: SummaryFunction | None = None,
    fun_max: SummaryFunction | None = None,
    fun_data: SummaryDataFunction | None = None,
    fun_args: Mapping[str, Number] | None = None,
    quantiles: Sequence[float] | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Summarise y at every distinct x (``stat="summary"``), drawn as point ranges.

    The stat computes ``y`` (``fun``, default mean), ``ymin`` (``fun_min``, default
    min) and ``ymax`` (``fun_max``, default max) per group and x.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, default="pointrange"
        Geom that draws the summary, e.g. ``"crossbar"``, ``"errorbar"``, ``"point"``.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    orientation : str, optional
        ``"y"`` summarises x per y (a discrete y and a continuous x pick it by default).
    fun : {"count", "sum", "mean", "median", "min", "max", "lq", "mq", "uq"}, default="mean"
        Aggregate for ``y``. ``lq``, ``mq`` and ``uq`` are the three ``quantiles``.
    fun_min : str, default="min"
        Aggregate for ``ymin``; the same names as ``fun``.
    fun_max : str, default="max"
        Aggregate for ``ymax``; the same names as ``fun``.
    fun_data : {"mean_se", "mean_sdl"}, optional
        ggplot2 shortcut that sets all three: the mean, plus and minus ``mult`` standard
        errors (``"mean_se"``, ``mult`` 1) or standard deviations (``"mean_sdl"``, ``mult``
        2). Cannot be combined with ``fun``, ``fun_min`` or ``fun_max``. Not in lets_plot.
    fun_args : dict, optional
        Arguments of ``fun_data``: ``{"mult": k}``.
    quantiles : list of float, default=[0.25, 0.5, 0.75]
        The probabilities of ``lq``, ``mq`` and ``uq``; three numbers in 0..1.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"color"`` or ``"fill"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"``, and the geom's options.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"day": ["mon", "mon", "tue", "tue", "tue"], "sales": [3, 5, 2, 6, 7]}
    >>> plot = ggplot(data, aes("day", "sales")) + stat_summary(fun_data="mean_se")
    """
    return _geom(
        geom if geom is not None else "pointrange",
        mapping=mapping,
        data=data,
        stat="summary",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        sampling=sampling,
        tooltips=tooltips,
        orientation=orientation,
        fun=fun,
        fun_min=fun_min,
        fun_max=fun_max,
        fun_data=fun_data,
        fun_args=dict(fun_args) if fun_args is not None else None,
        quantiles=quantiles,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def stat_summary_bin(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    sampling: Sampling | None = None,
    tooltips: Tooltips | None = None,
    orientation: str | None = None,
    fun: SummaryFunction | None = None,
    fun_min: SummaryFunction | None = None,
    fun_max: SummaryFunction | None = None,
    fun_data: SummaryDataFunction | None = None,
    fun_args: Mapping[str, Number] | None = None,
    quantiles: Sequence[float] | None = None,
    bins: int | None = None,
    binwidth: Number | None = None,
    center: Number | None = None,
    boundary: Number | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Summarise y in bins of x (``stat="summarybin"``), drawn as point ranges.

    x is binned as in ``geom_histogram``; every bin gets ``y``, ``ymin`` and ``ymax``
    (see ``stat_summary``). Empty bins are missing and not drawn.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, default="pointrange"
        Geom that draws the summary.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    orientation : str, optional
        ``"y"`` bins y and summarises x per bin.
    fun : str, default="mean"
        Aggregate for ``y``; see ``stat_summary``.
    fun_min : str, default="min"
        Aggregate for ``ymin``.
    fun_max : str, default="max"
        Aggregate for ``ymax``.
    fun_data : {"mean_se", "mean_sdl"}, optional
        ggplot2 shortcut that sets all three; see ``stat_summary``.
    fun_args : dict, optional
        Arguments of ``fun_data``: ``{"mult": k}``.
    quantiles : list of float, default=[0.25, 0.5, 0.75]
        The probabilities of ``lq``, ``mq`` and ``uq``.
    bins : int, default=30
        Number of bins; ``binwidth`` overrides it.
    binwidth : float, optional
        Width of the bins.
    center : float, optional
        x value at the centre of a bin.
    boundary : float, optional
        x value at the edge of a bin; overrides ``center``.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"color"`` or ``"fill"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"``, and the geom's options.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [21, 25, 33, 38, 41, 47, 52, 58], "income": [20, 28, 35, 40, 44, 47, 50, 49]}
    >>> plot = ggplot(data, aes("age", "income")) + stat_summary_bin(binwidth=10)
    """
    return _geom(
        geom if geom is not None else "pointrange",
        mapping=mapping,
        data=data,
        stat="summarybin",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        sampling=sampling,
        tooltips=tooltips,
        orientation=orientation,
        fun=fun,
        fun_min=fun_min,
        fun_max=fun_max,
        fun_data=fun_data,
        fun_args=dict(fun_args) if fun_args is not None else None,
        quantiles=quantiles,
        bins=bins,
        binwidth=binwidth,
        center=center,
        boundary=boundary,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def stat_ecdf(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    sampling: Sampling | None = None,
    tooltips: Tooltips | None = None,
    orientation: str | None = None,
    n: int | None = None,
    pad: bool | None = None,
    color_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Empirical cumulative distribution of x (``stat="ecdf"``), drawn as steps.

    Per group, y is the share of x values at or below each x.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, default="step"
        Geom that draws the distribution, e.g. ``"point"`` or ``"line"``.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    orientation : str, optional
        ``"y"`` uses y instead of x.
    n : int, optional
        Number of evenly spaced points from the smallest to the largest x; None uses
        every distinct x.
    pad : bool, default=True
        Add (-inf, 0) and (inf, 1), so steps run to the panel edges.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"``, and the geom's options.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"minutes": [12, 15, 15, 21, 30, 34, 41], "team": list("aabbaba")}
    >>> plot = ggplot(data, aes("minutes", color="team")) + stat_ecdf()
    """
    return _geom(
        geom if geom is not None else "step",
        mapping=mapping,
        data=data,
        stat="ecdf",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        sampling=sampling,
        tooltips=tooltips,
        orientation=orientation,
        n=n,
        pad=pad if pad is not None else True,
        color_by=color_by,
        na_rm=na_rm,
        **other_args,
    )


EllipseType = Literal["t", "norm", "euclid"]
"""``stat_ellipse`` fits: robust multivariate t, normal, or a circle."""


def stat_ellipse(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: Literal["path", "polygon"] | str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    tooltips: Tooltips | None = None,
    type: EllipseType | None = None,
    level: Number | None = None,
    segments: int | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Confidence ellipse of x and y per group (ggplot2 ``stat_ellipse``).

    lets-plot has none; this follows ggplot2, which fits the shape with
    ``MASS::cov.trob`` by default. A group needs at least 4 points; smaller groups,
    and groups whose points lie on a line, get no ellipse.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``; needs ``x`` and ``y``. Map ``color``,
        ``fill`` or ``group`` for one ellipse per group.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : {"path", "polygon"}, default="path"
        Geom that draws the outline; ``"polygon"`` fills it.
    position : str or FeatureSpec, optional
        Position adjustment.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content (HTML output), or ``"none"``.
    type : {"t", "norm", "euclid"}, default="t"
        ``"t"`` fits a robust multivariate t distribution (``MASS::cov.trob``),
        ``"norm"`` a multivariate normal, ``"euclid"`` draws a circle whose radius
        is ``level`` in data units.
    level : float, default=0.95
        Confidence level of the ellipse (for ``"euclid"``: the radius).
    segments : int, default=51
        Number of segments of the outline (it has ``segments + 1`` points).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"``, and the geom's options.

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If ``type`` is not one of the three, or ``segments`` is not a whole number of
        at least 1.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "height": [150, 155, 160, 163, 168, 158, 175, 180, 172, 185, 178, 190],
    ...     "weight": [50, 56, 55, 61, 63, 54, 70, 78, 69, 84, 74, 88],
    ...     "group": ["a"] * 6 + ["b"] * 6,
    ... }
    >>> plot = (
    ...     ggplot(data, aes("height", "weight", color="group"))
    ...     + geom_point()
    ...     + stat_ellipse(type="norm")
    ... )
    """
    if type is not None and type not in ("t", "norm", "euclid"):
        message = (
            f"stat_ellipse(): unknown type {type!r}; use 't' (robust t fit, default), "
            "'norm' (normal) or 'euclid' (a circle of radius level)"
        )
        raise SpecError(message, path="type", reason=message)
    if segments is not None and (
        isinstance(segments, bool) or not isinstance(segments, Integral) or segments < 1
    ):
        message = (
            f"stat_ellipse(): segments={segments!r} must be a whole number of at least 1 "
            "(default 51)"
        )
        raise SpecError(message, path="segments", reason=message)
    return _geom(
        geom if geom is not None else "path",
        mapping=mapping,
        data=data,
        stat="ellipse",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        tooltips=tooltips,
        type=type,
        level=level,
        segments=int(segments) if segments is not None else None,
        na_rm=na_rm,
        **other_args,
    )


def stat_sum(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    sampling: Sampling | None = None,
    tooltips: Tooltips | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Rows at each distinct (x, y) (``stat="sum"``), drawn as points sized by the count.

    Computes ``..n..`` (rows, or summed ``weight``, at the location), ``..prop..``
    (``n`` over the group's total) and ``..proppct..`` (the same in percent);
    ``size`` is mapped to ``..n..`` by default.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, default="point"
        Geom that draws the counts, e.g. ``"text"`` with ``aes(label="..n..")``.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"jitter"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"color"`` or ``"fill"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"``, and the geom's options.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"cylinders": [4, 4, 6, 8, 4, 6, 8, 8], "gears": [4, 4, 4, 3, 5, 4, 3, 3]}
    >>> plot = ggplot(data, aes("cylinders", "gears")) + stat_sum(aes(color="..prop.."))
    """
    return _geom(
        geom if geom is not None else "point",
        mapping=mapping,
        data=data,
        stat="sum",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        sampling=sampling,
        tooltips=tooltips,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )
