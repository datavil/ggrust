"""Quantile-quantile layers: ``geom_qq``, ``geom_qq_line``, ``geom_qq2``, ``geom_qq2_line``.

The ``stat_qq`` and ``stat_qq_line`` names follow ggplot2. The quantiles are computed
in the engine (``stat/qq.rs``).
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Literal

from ggrust.plot.geom import _geom

if TYPE_CHECKING:
    from collections.abc import Sequence

    from ggrust._typing import ManualKey, Number, PlotData, Position, Sampling, Tooltips
    from ggrust.plot.core import FeatureSpec, LayerSpec

__all__ = ["geom_qq", "geom_qq2", "geom_qq2_line", "geom_qq_line", "stat_qq", "stat_qq_line"]

Distribution = Literal["norm", "uniform", "t", "gamma", "exp", "chi2"]
"""Theoretical distributions of a Q-Q plot."""


def geom_qq(
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
    distribution: Distribution | None = None,
    dparams: Sequence[Number] | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Plot a sample against theoretical quantiles, one point per value.

    Map ``sample``. Computed variables: ``..theoretical..`` and ``..sample..``.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, default="qq"
        The statistical transformation.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        The key to show in the manual legend.
    sampling : str, int or FeatureSpec, optional
        Rows to draw: a ``sampling_*()`` spec, an int ``n`` (``sampling_random(n)``)
        or ``"none"``. Default: every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"``.
    distribution : {"norm", "uniform", "t", "gamma", "exp", "chi2"}, default="norm"
        The theoretical distribution.
    dparams : list of float, optional
        Its parameters: ``[mean, std]`` for ``"norm"`` (default ``[0, 1]``), ``[a, b]``
        for ``"uniform"`` (``[0, 1]``), ``[degrees of freedom]`` for ``"t"`` and
        ``"chi2"`` (``[1]``), ``[alpha, beta]`` (shape and rate) for ``"gamma"``
        (``[1, 1]``), ``[lambda]`` for ``"exp"`` (``[1]``).
    color_by : str, optional
        Aesthetic the ``color`` scale applies to.
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to.
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise with a warning.
    **other_args
        Constant aesthetics such as ``color="red"``, ``size=3``.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"height": [160.0, 172.0, 181.0, 168.0, 175.0], "other": [2.0, 4.0, 3.0, 6.0, 5.0]}
    >>> plot = ggplot(data, aes(sample="height")) + geom_qq()
    """
    return _geom(
        "qq",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        distribution=distribution,
        dparams=dparams,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_qq_line(
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
    distribution: Distribution | None = None,
    dparams: Sequence[Number] | None = None,
    quantiles: Sequence[Number] | None = None,
    color_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Draw the line through two quantiles of a sample, to read a Q-Q plot against.

    Map ``sample``. Computed variables: ``..theoretical..`` and ``..sample..``.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, default="qq_line"
        The statistical transformation.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        The key to show in the manual legend.
    sampling : str, int or FeatureSpec, optional
        Rows to draw: a ``sampling_*()`` spec, an int ``n`` (``sampling_random(n)``)
        or ``"none"``. Default: every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"``.
    distribution : {"norm", "uniform", "t", "gamma", "exp", "chi2"}, default="norm"
        The theoretical distribution.
    dparams : list of float, optional
        Its parameters: ``[mean, std]`` for ``"norm"`` (default ``[0, 1]``), ``[a, b]``
        for ``"uniform"`` (``[0, 1]``), ``[degrees of freedom]`` for ``"t"`` and
        ``"chi2"`` (``[1]``), ``[alpha, beta]`` (shape and rate) for ``"gamma"``
        (``[1, 1]``), ``[lambda]`` for ``"exp"`` (``[1]``).
    quantiles : list of float, default=[0.25, 0.75]
        The two probabilities the line passes through.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to.
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise with a warning.
    **other_args
        Constant aesthetics such as ``color="red"``, ``size=3``.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"height": [160.0, 172.0, 181.0, 168.0, 175.0], "other": [2.0, 4.0, 3.0, 6.0, 5.0]}
    >>> plot = ggplot(data, aes(sample="height")) + geom_qq_line()
    """
    return _geom(
        "qq_line",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        distribution=distribution,
        dparams=dparams,
        quantiles=quantiles,
        color_by=color_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_qq2(
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
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Plot one sample against another, one point per quantile.

    Map ``x`` and ``y``: each is sorted on its own and compared by quantiles.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, default="qq2"
        The statistical transformation.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        The key to show in the manual legend.
    sampling : str, int or FeatureSpec, optional
        Rows to draw: a ``sampling_*()`` spec, an int ``n`` (``sampling_random(n)``)
        or ``"none"``. Default: every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"``.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to.
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to.
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise with a warning.
    **other_args
        Constant aesthetics such as ``color="red"``, ``size=3``.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"height": [160.0, 172.0, 181.0, 168.0, 175.0], "other": [2.0, 4.0, 3.0, 6.0, 5.0]}
    >>> plot = ggplot(data, aes("height", "other")) + geom_qq2()
    """
    return _geom(
        "qq2",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_qq2_line(
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
    quantiles: Sequence[Number] | None = None,
    color_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Draw the line through two quantiles of two samples, to read ``geom_qq2`` against.

    Map ``x`` and ``y``.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, default="qq2_line"
        The statistical transformation.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        The key to show in the manual legend.
    sampling : str, int or FeatureSpec, optional
        Rows to draw: a ``sampling_*()`` spec, an int ``n`` (``sampling_random(n)``)
        or ``"none"``. Default: every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"``.
    quantiles : list of float, default=[0.25, 0.75]
        The two probabilities the line passes through.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to.
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise with a warning.
    **other_args
        Constant aesthetics such as ``color="red"``, ``size=3``.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"height": [160.0, 172.0, 181.0, 168.0, 175.0], "other": [2.0, 4.0, 3.0, 6.0, 5.0]}
    >>> plot = ggplot(data, aes("height", "other")) + geom_qq2_line()
    """
    return _geom(
        "qq2_line",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        quantiles=quantiles,
        color_by=color_by,
        na_rm=na_rm,
        **other_args,
    )


def stat_qq(
    mapping: FeatureSpec | None = None,
    *,
    geom: str | None = None,
    **options: Any,
) -> LayerSpec:
    """Plot theoretical quantiles of a sample (``stat="qq"``), ggplot2's ``stat_qq``.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``; map ``sample``.
    geom : str, default="qq"
        Geom that draws the quantiles, e.g. ``"point"`` or ``"line"``.
    **options
        As ``geom_qq()``.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"height": [160.0, 172.0, 181.0, 168.0, 175.0]}
    >>> plot = ggplot(data, aes(sample="height")) + stat_qq(distribution="t", dparams=[5])
    """
    return _geom(geom if geom is not None else "qq", mapping=mapping, stat="qq", **options)


def stat_qq_line(
    mapping: FeatureSpec | None = None,
    *,
    geom: str | None = None,
    **options: Any,
) -> LayerSpec:
    """Draw the fitting line of a Q-Q plot (``stat="qq_line"``), ggplot2's ``stat_qq_line``.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``; map ``sample``.
    geom : str, default="qq_line"
        Geom that draws the line, e.g. ``"path"``.
    **options
        As ``geom_qq_line()``.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"height": [160.0, 172.0, 181.0, 168.0, 175.0]}
    >>> plot = ggplot(data, aes(sample="height")) + stat_qq() + stat_qq_line()
    """
    return _geom(
        geom if geom is not None else "qq_line", mapping=mapping, stat="qq_line", **options
    )
