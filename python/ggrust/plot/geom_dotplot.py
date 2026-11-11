"""``geom_dotplot``, ``geom_ydotplot`` and ``stat_bindot``: stacks of dots.

Ported from lets-plot's ``lets_plot/plot/geom.py`` (MIT, see THIRD_PARTY_NOTICES.md).
``stat_bindot`` is plotnine's name for the layer: a dot plot given by its stat.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Literal

from ggrust.plot.geom import _geom

if TYPE_CHECKING:
    from ggrust._typing import ManualKey, Number, PlotData, Sampling, Tooltips
    from ggrust.plot.core import FeatureSpec, LayerSpec

__all__ = ["geom_dotplot", "geom_ydotplot", "stat_bindot"]


def geom_dotplot(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    manual_key: ManualKey | None = None,
    sampling: Sampling | None = None,
    tooltips: Tooltips | None = None,
    binwidth: Number | None = None,
    bins: int | None = None,
    method: Literal["dotdensity", "histodot"] | None = None,
    stackdir: Literal["up", "down", "center", "centerwhole"] | None = None,
    stackratio: Number | None = None,
    dotsize: Number | None = None,
    stackgroups: bool | None = None,
    center: Number | None = None,
    boundary: Number | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Draw a dot plot: each observation a dot, stacked within its bin.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    show_legend : bool, default=True
        False hides this layer from the legend.
    inherit_aes : bool, default=True
        False ignores the plot's mapping.
    manual_key : str or FeatureSpec, optional
        A manual legend key for this layer.
    sampling : FeatureSpec or int, optional
        Result of a ``sampling_*()`` call.
    tooltips : FeatureSpec or str, optional
        Accepted for compatibility; dot plots have no tooltips yet.
    binwidth : float, optional
        Width of a bin, and of a dot; overrides ``bins``.
    bins : int, default=30
        Number of bins.
    method : {"dotdensity", "histodot"}, default="dotdensity"
        ``dotdensity`` places stacks where the data is, ``histodot`` uses equal bins.
    stackdir : {"up", "down", "center", "centerwhole"}, default="up"
        Which way dots stack.
    stackratio : float, default=1
        Distance between dots, as a share of a dot.
    dotsize : float, default=1
        Diameter of a dot, as a share of ``binwidth``.
    stackgroups : bool, default=False
        Stack dots across groups (with ``method="histodot"``).
    center : float, optional
        Center of a bin (with ``method="histodot"``).
    boundary : float, optional
        Boundary between two bins (with ``method="histodot"``).
    color_by : str, optional
        Aesthetic the ``color`` scale of this layer reads.
    fill_by : str, optional
        Aesthetic the ``fill`` scale of this layer reads.
    na_rm : bool, default=False
        True drops missing values without a warning.
    **other_args
        Constant aesthetics: ``color``, ``fill``, ``alpha``, ``stroke``.

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If ``method`` or ``stackdir`` is not accepted (when the plot is built).

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"height": [1.2, 1.3, 1.3, 1.7, 2.0, 2.1]}
    >>> plot = ggplot(data, aes(x="height")) + geom_dotplot(binwidth=0.2)
    """
    return _geom(
        "dotplot",
        mapping=mapping,
        data=data,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        binwidth=binwidth,
        bins=bins,
        method=method,
        stackdir=stackdir,
        stackratio=stackratio,
        dotsize=dotsize,
        stackgroups=stackgroups,
        center=center,
        boundary=boundary,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_ydotplot(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    manual_key: ManualKey | None = None,
    sampling: Sampling | None = None,
    tooltips: Tooltips | None = None,
    orientation: Literal["x", "y"] | None = None,
    binwidth: Number | None = None,
    bins: int | None = None,
    method: Literal["dotdensity", "histodot"] | None = None,
    stackdir: Literal["left", "right", "center", "centerwhole"] | None = None,
    stackratio: Number | None = None,
    dotsize: Number | None = None,
    stackgroups: bool | None = None,
    center: Number | None = None,
    boundary: Number | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Draw a dot plot along y, one column of stacks at each x.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    show_legend : bool, default=True
        False hides this layer from the legend.
    inherit_aes : bool, default=True
        False ignores the plot's mapping.
    manual_key : str or FeatureSpec, optional
        A manual legend key for this layer.
    sampling : FeatureSpec or int, optional
        Result of a ``sampling_*()`` call.
    tooltips : FeatureSpec or str, optional
        Accepted for compatibility; dot plots have no tooltips yet.
    orientation : {"x", "y"}, optional
        The axis the layer runs along.
    binwidth : float, optional
        Width of a bin, and of a dot; overrides ``bins``.
    bins : int, default=30
        Number of bins.
    method : {"dotdensity", "histodot"}, default="dotdensity"
        ``dotdensity`` places stacks where the data is, ``histodot`` uses equal bins.
    stackdir : {"left", "right", "center", "centerwhole"}, default="center"
        Which way dots stack.
    stackratio : float, default=1
        Distance between dots, as a share of a dot.
    dotsize : float, default=1
        Diameter of a dot, as a share of ``binwidth``.
    stackgroups : bool, default=False
        Stack dots across groups rather than dodging them.
    center : float, optional
        Center of a bin (with ``method="histodot"``).
    boundary : float, optional
        Boundary between two bins (with ``method="histodot"``).
    color_by : str, optional
        Aesthetic the ``color`` scale of this layer reads.
    fill_by : str, optional
        Aesthetic the ``fill`` scale of this layer reads.
    na_rm : bool, default=False
        True drops missing values without a warning.
    **other_args
        Constant aesthetics: ``color``, ``fill``, ``alpha``, ``stroke``.

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If ``method`` or ``stackdir`` is not accepted (when the plot is built).

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"group": ["a", "a", "b"], "height": [1.2, 1.3, 1.7]}
    >>> plot = ggplot(data, aes("group", "height")) + geom_ydotplot(binwidth=0.2)
    """
    return _geom(
        "ydotplot",
        mapping=mapping,
        data=data,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        orientation=orientation,
        binwidth=binwidth,
        bins=bins,
        method=method,
        stackdir=stackdir,
        stackratio=stackratio,
        dotsize=dotsize,
        stackgroups=stackgroups,
        center=center,
        boundary=boundary,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def stat_bindot(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str = "dotplot",
    binwidth: Number | None = None,
    bins: int | None = None,
    method: Literal["dotdensity", "histodot"] | None = None,
    center: Number | None = None,
    boundary: Number | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Draw a layer from the stacks of a dot plot.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, default="dotplot"
        The geom that draws the stacks.
    binwidth : float, optional
        Width of a bin; overrides ``bins``.
    bins : int, default=30
        Number of bins.
    method : {"dotdensity", "histodot"}, default="dotdensity"
        ``dotdensity`` places stacks where the data is, ``histodot`` uses equal bins.
    center : float, optional
        Center of a bin (with ``method="histodot"``).
    boundary : float, optional
        Boundary between two bins (with ``method="histodot"``).
    **other_args
        Other layer options and constant aesthetics.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"height": [1.2, 1.3, 1.3, 1.7, 2.0, 2.1]}
    >>> plot = ggplot(data, aes(x="height")) + stat_bindot(binwidth=0.2)
    """
    return _geom(
        geom,
        mapping=mapping,
        data=data,
        stat="dotplot",
        binwidth=binwidth,
        bins=bins,
        method=method,
        center=center,
        boundary=boundary,
        **other_args,
    )
