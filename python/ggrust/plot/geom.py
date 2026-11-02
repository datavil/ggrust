"""Geom layer functions.

Ported from lets-plot's ``lets_plot/plot/geom.py`` (MIT, see THIRD_PARTY_NOTICES.md).
Geoms the engine does not draw yet still build lets_plot-identical specs, so code
written for lets_plot imports and composes; rendering them reports the geom.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Literal, cast

from ggrust.errors import SpecError
from ggrust.plot.core import FeatureSpec, LayerSpec
from ggrust.plot.tooltip import layer_tooltips
from ggrust.plot.util import as_annotated_data, key_int2str

if TYPE_CHECKING:
    from collections.abc import Sequence

    from ggrust._typing import (
        Breaks,
        Color,
        Labels,
        ManualKey,
        Number,
        PlotData,
        Position,
        Sampling,
        Shape,
        Tooltips,
    )


__all__ = [
    "geom_abline",
    "geom_area_ridges",
    "geom_bar",
    "geom_blank",
    "geom_boxplot",
    "geom_bracket",
    "geom_crossbar",
    "geom_density",
    "geom_errorbar",
    "geom_freqpoly",
    "geom_histogram",
    "geom_hline",
    "geom_jitter",
    "geom_label",
    "geom_label_repel",
    "geom_line",
    "geom_linerange",
    "geom_path",
    "geom_point",
    "geom_pointrange",
    "geom_polygon",
    "geom_raster",
    "geom_rect",
    "geom_segment",
    "geom_sina",
    "geom_smooth",
    "geom_step",
    "geom_text",
    "geom_text_repel",
    "geom_tile",
    "geom_violin",
    "geom_vline",
]


def geom_point(
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
    size_unit: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Scatter points.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``: ``x``, ``y`` (required), ``color``, ``fill``,
        ``size``, ``shape``, ``alpha``, ``stroke``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Stat name (``"identity"``).
    position : str or FeatureSpec, optional
        Position adjustment (``"identity"``).
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    size_unit : {"x", "y"}, optional
        Size relative to one axis unit, as in lets_plot; not supported for points yet
        (ggrust warns).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"``, ``size=2``, ``alpha=0.5``, or
        constant positions ``x=1, y=2``.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "point",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        size_unit=size_unit,
        na_rm=na_rm,
        **other_args,
    )


def geom_path(
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
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Connect observations in data order.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "path",
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
        na_rm=na_rm,
        **other_args,
    )


def geom_line(
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
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Connect observations sorted by x.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "line",
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
        na_rm=na_rm,
        **other_args,
    )


def geom_step(
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
    direction: Literal["hv", "vh", "mid"] | None = None,
    color_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Connect observations sorted by x with stairs.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    direction : {"hv", "vh", "mid"}, default="hv"
        ``"hv"`` steps along x first, then y; ``"vh"`` along y first; ``"mid"`` steps
        halfway between adjacent x values (as ggplot2).
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options, e.g. ``pad=True`` to draw rows at x = -inf / inf at the panel edges.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"day": [1, 2, 3, 4, 5], "stock": [10, 7, 7, 4, 9]}
    >>> plot = ggplot(data, aes("day", "stock")) + geom_step(direction="mid")
    """
    return _geom(
        "step",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        direction=direction,
        color_by=color_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_freqpoly(
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
    orientation: str | None = None,
    color_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Frequency polygon: a line through the counts of binned x (``stat="bin"``).

    Takes the binning options of ``geom_histogram`` (``bins``, ``binwidth``,
    ``center``, ``boundary``, ``breaks``) as keyword arguments.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name; ``"bin"`` when None.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    orientation : str, optional
        ``"y"`` bins y instead of x.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and binning options.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"height": [160, 172, 181, 168, 175, 190, 158, 177]}
    >>> plot = ggplot(data, aes("height")) + geom_freqpoly(bins=5)
    """
    return _geom(
        "freqpoly",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        orientation=orientation,
        color_by=color_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_polygon(
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
    """Draw filled polygons, one per group.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
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
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "polygon",
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


def geom_abline(
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
    intercept: Number | None = None,
    slope: Number | None = None,
    color_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Straight reference lines ``y = intercept + slope * x`` across the panel.

    The lines do not train the axes. With neither ``intercept`` nor ``slope``
    (mapped or given) the line is ``y = x``, as in ggplot2.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    intercept : float, optional
        Value of y at x = 0 (axis units, after a scale transform); 0 when None.
    slope : float, optional
        Change of y per unit of x; 1 when None.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "abline",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        intercept=intercept,
        slope=slope,
        color_by=color_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_hline(
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
    yintercept: Number | None = None,
    color_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Horizontal reference lines at ``yintercept``.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    yintercept : float, optional
        Position of the line on the y axis.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "hline",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        yintercept=yintercept,
        color_by=color_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_vline(
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
    xintercept: Number | None = None,
    color_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Vertical reference lines at ``xintercept``.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    xintercept : float, optional
        Position of the line on the x axis.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "vline",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        xintercept=xintercept,
        color_by=color_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_errorbar(
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
    width_unit: str | None = None,
    color_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Error bars from ymin to ymax at x, with fences ``width`` wide (default 0.45).

    ``width`` (a constant or a mapped aesthetic) is a share of the x resolution.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    width_unit : str, optional
        Unit of ``width``; only ``"res"`` (the x resolution, default) is supported.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import aes, geom_errorbar, ggplot
    >>> data = {"x": ["a", "b"], "y": [2.0, 3.0], "low": [1.0, 2.5], "high": [3.0, 4.0]}
    >>> plot = ggplot(data, aes(x="x", y="y", ymin="low", ymax="high")) + geom_errorbar()
    """
    return _geom(
        "errorbar",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        width_unit=width_unit,
        color_by=color_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_crossbar(
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
    labels: FeatureSpec | None = None,
    fatten: Number | None = None,
    width_unit: str | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Boxes from ymin to ymax at x with a midline at y, dodged by group.

    ``width`` (default 0.9) is a share of the x resolution.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    labels : FeatureSpec, optional
        Midline labels from ``layer_labels()`` (lets_plot); not drawn by ggrust yet.
    fatten : float, optional
        The midline width as a multiple of the outline width (default 2.5); 0 hides it.
    width_unit : str, optional
        Unit of ``width``; only ``"res"`` (the x resolution, default) is supported.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"color"`` or ``"fill"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import aes, geom_crossbar, ggplot
    >>> data = {"x": ["a", "b"], "y": [2.0, 3.0], "low": [1.0, 2.5], "high": [3.0, 4.0]}
    >>> plot = ggplot(data, aes(x="x", y="y", ymin="low", ymax="high")) + geom_crossbar()
    """
    return _geom(
        "crossbar",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        labels=labels,
        fatten=fatten,
        width_unit=width_unit,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_pointrange(
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
    fatten: Number | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Draw a vertical line from ymin to ymax at x with a point at y.

    ``linewidth`` sets the line, ``size`` the point (times ``fatten``).

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    fatten : float, optional
        The point size as a multiple of ``size`` (default 5).
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"color"`` or ``"fill"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import aes, geom_pointrange, ggplot
    >>> data = {"x": ["a", "b"], "y": [2.0, 3.0], "low": [1.0, 2.5], "high": [3.0, 4.0]}
    >>> plot = ggplot(data, aes(x="x", y="y", ymin="low", ymax="high")) + geom_pointrange()
    """
    return _geom(
        "pointrange",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        fatten=fatten,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_linerange(
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
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Draw a vertical line from ymin to ymax at x.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import aes, geom_linerange, ggplot
    >>> data = {"x": ["a", "b"], "y": [2.0, 3.0], "low": [1.0, 2.5], "high": [3.0, 4.0]}
    >>> plot = ggplot(data, aes(x="x", y="y", ymin="low", ymax="high")) + geom_linerange()
    """
    return _geom(
        "linerange",
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
        na_rm=na_rm,
        **other_args,
    )


def geom_segment(
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
    arrow: FeatureSpec | None = None,
    color_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Line segments from (x, y) to (xend, yend), optionally with arrows.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    arrow : FeatureSpec, optional
        Arrow spec from ``arrow()``.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "segment",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        arrow=arrow,
        color_by=color_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_text(
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
    label_format: str | None = None,
    na_text: str | None = None,
    nudge_x: Number | None = None,
    nudge_y: Number | None = None,
    check_overlap: bool | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Text labels at (x, y); the text comes from the ``label`` aesthetic.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    label_format : str, optional
        Format pattern of the label text, e.g. ``".2f"`` or ``"{d} %"``.
    na_text : str, optional
        Text shown for missing values.
    nudge_x : float, optional
        Horizontal offset of the labels.
    nudge_y : float, optional
        Vertical offset of the labels.
    check_overlap : bool, optional
        True skips labels that overlap labels drawn before them.
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "text",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        label_format=label_format,
        na_text=na_text,
        nudge_x=nudge_x,
        nudge_y=nudge_y,
        check_overlap=check_overlap,
        na_rm=na_rm,
        **other_args,
    )


def geom_label(
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
    label_format: str | None = None,
    na_text: str | None = None,
    nudge_x: Number | None = None,
    nudge_y: Number | None = None,
    label_padding: Number | None = None,
    label_r: Number | None = None,
    label_size: Number | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Text labels in a rounded box at (x, y).

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    label_format : str, optional
        Format pattern of the label text, e.g. ``".2f"`` or ``"{d} %"``.
    na_text : str, optional
        Text shown for missing values.
    nudge_x : float, optional
        Horizontal offset of the labels.
    nudge_y : float, optional
        Vertical offset of the labels.
    label_padding : float, optional
        Space between the text and the label border.
    label_r : float, optional
        Radius of the label corners.
    label_size : float, optional
        Width of the label border line.
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "label",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        label_format=label_format,
        na_text=na_text,
        nudge_x=nudge_x,
        nudge_y=nudge_y,
        label_padding=label_padding,
        label_r=label_r,
        label_size=label_size,
        na_rm=na_rm,
        **other_args,
    )


def geom_blank(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    stat: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool = False,
    manual_key: ManualKey | None = None,
    sampling: Sampling | None = None,
    map: PlotData | None = None,
    map_join: str | Sequence[str] | None = None,
    use_crs: str | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Draw nothing; the mapped data still trains the scales. No tooltips (as lets_plot).

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, default=False
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    map : DataFrame or dict, optional
        Spatial data (lets_plot ``map``); not supported by ggrust yet.
    map_join : str or list of str, optional
        Columns joining ``data`` to ``map`` (lets_plot); not supported by ggrust yet.
    use_crs : str, optional
        Coordinate reference system for ``map`` (lets_plot); not supported by ggrust yet.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "blank",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips="none",
        map=map,
        map_join=map_join,
        use_crs=use_crs,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


# Geoms below build lets_plot-identical specs; the engine does not draw them yet and
# rendering reports which one (plan.md names the step that adds each).


def geom_bar(
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
    labels: Labels | None = None,
    orientation: str | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Bars, by default the count of rows per x (``stat="count"``), stacked.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    labels : list or dict, optional
        Texts at the breaks: a list, or a dict of break values to texts.
    orientation : str, optional
        ``"y"`` draws along the y axis; a discrete y with a continuous x picks it by default.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "bar",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        labels=labels,
        orientation=orientation,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_histogram(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    stat: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    manual_key: ManualKey | None = None,
    sampling: Sampling | None = None,
    threshold: Number | None = None,
    tooltips: Tooltips | None = None,
    labels: Labels | None = None,
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
) -> LayerSpec:
    """Histogram of binned x values (``stat="bin"``), stacked.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    threshold : float, optional
        Densities under this value are not drawn.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    labels : list or dict, optional
        Texts at the breaks: a list, or a dict of break values to texts.
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
    breaks : list or dict, optional
        Positions of the ticks or legend keys: a list, or a dict of labels to values.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "histogram",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        threshold=threshold,
        tooltips=tooltips,
        labels=labels,
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


def geom_boxplot(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    stat: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    manual_key: ManualKey | None = None,
    tooltips: Tooltips | None = None,
    orientation: str | None = None,
    fatten: Number | None = None,
    outlier_alpha: Number | None = None,
    outlier_color: Color | None = None,
    outlier_fill: Color | None = None,
    outlier_shape: Shape | None = None,
    outlier_size: Number | None = None,
    outlier_stroke: Number | None = None,
    varwidth: bool | None = None,
    whisker_width: Number | None = None,
    width_unit: str | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> FeatureSpec:
    """Box and whiskers plot, dodged by group.

    With the default stat, a second point layer (``stat="boxplot_outlier"``) draws
    the outliers, as in lets_plot. Unlike lets_plot, ``coef`` also reaches the
    outlier layer, so points beyond shortened whiskers are not lost.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    outlier_alpha, outlier_color, outlier_fill : optional
        Style of the outlier points (also ``outlier_shape``, ``outlier_size``,
        ``outlier_stroke``); ``color``, ``fill``, ``shape``, ``size`` and
        ``stroke`` in ``other_args`` are the fallback.
    stat : str, optional
        Statistical transformation, by name.
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
    orientation : str, optional
        ``"y"`` draws along the y axis; a discrete y with a continuous x picks it by default.
    fatten : float, optional
        Multiplier of the median line width.
    outlier_shape : int or str, optional
        Shape of the outlier points.
    outlier_size : float, optional
        Size of the outlier points.
    outlier_stroke : float, optional
        Stroke width of the outlier points.
    varwidth : bool, optional
        True makes box widths proportional to the square root of the group size.
    whisker_width : float, optional
        Width of the whisker ends, relative to the box width.
    width_unit : str, optional
        Unit of ``width``: ``"res"``, ``"identity"``, ``"size"`` or ``"px"``.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec or FeatureSpecArray
        The box layer, plus the outlier layer with the default stat.
    """
    boxplot_layer = _geom(
        "boxplot",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=None,
        tooltips=tooltips,
        orientation=orientation,
        fatten=fatten,
        varwidth=varwidth,
        whisker_width=whisker_width,
        width_unit=width_unit,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )
    if stat is None or stat == "boxplot":

        def outlier_parameter(name: str, value: Any) -> Any:  # noqa: ANN401 (user constants)
            return value if value is not None else other_args.get(name)

        outlier_fatten = 4
        size = outlier_parameter("size", outlier_size)
        if size is not None:
            size *= outlier_fatten
        boxplot_layer += _geom(
            "point",
            mapping=mapping,
            data=data,
            stat="boxplot_outlier",
            position=position,
            show_legend=False,
            inherit_aes=inherit_aes,
            sampling=None,
            orientation=orientation,
            alpha=outlier_alpha,
            color=outlier_parameter("color", outlier_color),
            fill=outlier_parameter("fill", outlier_fill),
            shape=outlier_parameter("shape", outlier_shape),
            size=size,
            stroke=outlier_parameter("stroke", outlier_stroke),
            # lets_plot leaves `coef` out here, so its outliers use the default 1.5.
            coef=other_args.get("coef"),
            color_by=color_by,
            fill_by=fill_by,
            na_rm=True,
        )
    return boxplot_layer


def geom_violin(
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
    orientation: str | None = None,
    show_half: Number | None = None,
    quantiles: Sequence[float] | None = None,
    quantile_lines: bool | None = None,
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
) -> LayerSpec:
    """Violin plot: mirrored kernel density per x (``stat="ydensity"``). Not drawn yet.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    orientation : str, optional
        ``"y"`` draws along the y axis; a discrete y with a continuous x picks it by default.
    show_half : float, optional
        Draw only half of each violin: ``-1`` left, ``1`` right, ``0`` both.
    quantiles : list of float, optional
        Quantiles to draw lines at, each in [0, 1].
    quantile_lines : bool, optional
        True draws the quantile lines.
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
    fs_max : float, optional
        Upper bound of the number of points the density is evaluated at.
    n : int, optional
        Number of points the density is evaluated at.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.
        ``draw_quantiles`` is rejected, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    if "draw_quantiles" in other_args:
        message = (
            "The parameter 'draw_quantiles' is no longer supported.\n"
            "Use parameters 'quantile_lines', 'quantiles' as needed."
        )
        raise SpecError(message)
    return _geom(
        "violin",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        orientation=orientation,
        show_half=show_half,
        quantiles=quantiles,
        quantile_lines=quantile_lines,
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


def geom_sina(
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
    orientation: str | None = None,
    seed: int | None = None,
    show_half: Number | None = None,
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
) -> LayerSpec:
    """Sina plot: points spread within the violin outline.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    orientation : str, optional
        ``"y"`` draws along the y axis; a discrete y with a continuous x picks it by default.
    seed : int, optional
        Random seed, for reproducible results.
    show_half : float, optional
        Draw only half of each violin: ``-1`` left, ``1`` right, ``0`` both.
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
    fs_max : float, optional
        Upper bound of the number of points the density is evaluated at.
    n : int, optional
        Number of points the density is evaluated at.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "sina",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        orientation=orientation,
        seed=seed,
        show_half=show_half,
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


def geom_density(
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
    orientation: str | None = None,
    trim: bool | None = None,
    kernel: str | None = None,
    adjust: Number | None = None,
    bw: str | Number | None = None,
    n: int | None = None,
    fs_max: Number | None = None,
    quantiles: Sequence[float] | None = None,
    quantile_lines: bool | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Kernel density estimate of ``x``, drawn as a filled area (``stat="density"``).

    One density per group. The computed variables are ``..density..`` (y by default),
    ``..count..`` (density x total weight, which is the number of rows without
    ``weight``), ``..scaled..`` (density / its maximum) and ``..quantile..``.
    Bandwidths follow R's ``bw.nrd0`` and ``bw.nrd``.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. ``weight`` weighs the rows.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name; ``"density"`` when None.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"stack"`` or a ``position_*()`` spec;
        ``"identity"`` when None.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
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
    quantile_lines : bool, default=False
        True draws a line from the curve down to the base at each quantile.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``fill="steelblue"`` or ``alpha=0.5``, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> frame = {"height": [160, 165, 170, 172, 175, 180, 185], "sex": list("FFFMMMM")}
    >>> plot = ggplot(frame, aes("height", fill="sex")) + geom_density(alpha=0.4)
    """
    return _geom(
        "density",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        orientation=orientation,
        trim=trim,
        kernel=kernel,
        adjust=adjust,
        bw=bw,
        n=n,
        fs_max=fs_max,
        quantiles=quantiles,
        quantile_lines=quantile_lines,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_smooth(
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
    labels: FeatureSpec | None = None,
    orientation: str | None = None,
    method: Literal["lm", "loess", "lowess"] | None = None,
    n: int | None = None,
    se: bool | None = None,
    level: Number | None = None,
    span: Number | None = None,
    deg: int | None = None,
    seed: int | None = None,
    max_n: int | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Draw a fitted curve with a confidence band (``stat="smooth"``), one per group.

    The computed variables are ``x``, ``y`` (the fitted value), ``ymin`` and ``ymax``
    (the band) and ``..se..`` (the standard error). The band uses each model's own
    standard error, as ggplot2 (lets_plot uses the straight-line one for every
    method, so its polynomial and loess bands differ).

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name; ``"smooth"`` when None.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    labels : FeatureSpec, optional
        lets_plot's model summary annotation (``smooth_labels()``); not supported yet
        (ggrust warns).
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
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``fill="steelblue"``, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> frame = {"hours": [1, 2, 3, 4, 5, 6], "score": [52, 55, 61, 64, 70, 71]}
    >>> plot = ggplot(frame, aes("hours", "score")) + geom_point() + geom_smooth()
    """
    return _geom(
        "smooth",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        labels=labels,
        orientation=orientation,
        method=method,
        n=n,
        se=se,
        level=level,
        span=span,
        deg=deg,
        seed=seed,
        max_n=max_n,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_area(
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
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Fill the area under a line, stacked by default (``position="stack"``).

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import aes, geom_area, ggplot
    >>> data = {"day": [1, 2, 3, 1, 2, 3], "sales": [3, 5, 4, 1, 2, 2], "shop": list("aaabbb")}
    >>> plot = ggplot(data, aes(x="day", y="sales", fill="shop")) + geom_area()
    """
    return _geom(
        "area",
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
        na_rm=na_rm,
        **other_args,
    )


def geom_ribbon(
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
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Fill the band between ymin and ymax along x.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import aes, geom_ribbon, ggplot
    >>> data = {"day": [1, 2, 3], "low": [1.0, 2.0, 1.5], "high": [3.0, 4.0, 3.5]}
    >>> plot = ggplot(data, aes(x="day", ymin="low", ymax="high")) + geom_ribbon()
    """
    return _geom(
        "ribbon",
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
        na_rm=na_rm,
        **other_args,
    )


def geom_area_ridges(
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
    trim: bool | None = None,
    tails_cutoff: Number | None = None,
    kernel: str | None = None,
    adjust: Number | None = None,
    bw: str | Number | None = None,
    n: int | None = None,
    fs_max: Number | None = None,
    min_height: Number | None = None,
    scale: Number | None = None,
    quantiles: Sequence[float] | None = None,
    quantile_lines: bool | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Ridgeline plot: one density curve per y level. Not drawn yet.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    trim : bool, optional
        True trims the density to the data range.
    tails_cutoff : float, optional
        Density tails are cut where the density falls under this share of the maximum.
    kernel : str, optional
        Kernel of the density estimate, e.g. ``"gaussian"``.
    adjust : float, optional
        Multiplier of the bandwidth.
    bw : str or float, optional
        Bandwidth: a number or a rule name such as ``"nrd0"``.
    fs_max : float, optional
        Upper bound of the number of points the density is evaluated at.
    n : int, optional
        Number of points the density is evaluated at.
    min_height : float, optional
        Ridges lower than this are not drawn.
    scale : float, optional
        Multiplier of the pixel size.
    quantiles : list of float, optional
        Quantiles to draw lines at, each in [0, 1].
    quantile_lines : bool, optional
        True draws the quantile lines.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "area_ridges",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        trim=trim,
        tails_cutoff=tails_cutoff,
        kernel=kernel,
        adjust=adjust,
        bw=bw,
        n=n,
        fs_max=fs_max,
        min_height=min_height,
        scale=scale,
        quantiles=quantiles,
        quantile_lines=quantile_lines,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_count(
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
    """Points at each distinct (x, y), sized by the number of rows there (``stat="sum"``).

    The stat computes ``..n..`` (rows, or summed ``weight``, at the location),
    ``..prop..`` (``n`` over the group's total) and ``..proppct..`` (the same in
    percent); ``size`` is mapped to ``..n..`` by default.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``: ``x``, ``y`` (required), ``color``, ``fill``,
        ``size``, ``shape``, ``alpha``, ``stroke``, ``weight``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name; ``"sum"`` when None.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"jitter"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
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
        Constant aesthetics such as ``color="red"`` or ``alpha=0.5``.

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"cylinders": [4, 4, 6, 8, 4, 6, 8, 8], "gears": [4, 4, 4, 3, 5, 4, 3, 3]}
    >>> plot = ggplot(data, aes("cylinders", "gears")) + geom_count()
    """
    return _geom(
        "point",
        mapping=mapping,
        data=data,
        stat=stat if stat is not None else "sum",
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


def geom_jitter(
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
    width: Number | None = None,
    height: Number | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    seed: int | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Points with random noise added to their positions. Not drawn yet.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    width, height : float, optional
        Jitter amount on each axis, as a share of the data resolution (0.4 by default).
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    seed : int, optional
        Random seed, for reproducible results.
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "jitter",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        width=width,
        height=height,
        color_by=color_by,
        fill_by=fill_by,
        seed=seed,
        na_rm=na_rm,
        **other_args,
    )


def geom_tile(
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
    width_unit: str | None = None,
    height_unit: str | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Rectangles centred on (x, y), e.g. heatmap cells. Not drawn yet.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    width_unit : str, optional
        Unit of ``width``: ``"res"``, ``"identity"``, ``"size"`` or ``"px"``.
    height_unit : str, optional
        Unit of ``height``: ``"res"``, ``"identity"``, ``"size"`` or ``"px"``.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "tile",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        width_unit=width_unit,
        height_unit=height_unit,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_bin2d(
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
    bins: int | Sequence[int] | None = None,
    binwidth: float | Sequence[float | None] | None = None,
    drop: bool | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Tiles counting the rows in a grid of rectangles (``stat="bin2d"``).

    The grid spans the x and y ranges, widened by half a bin on each side. The stat
    computes ``..count..`` (rows, or summed ``weight``, per bin; ``fill`` by default)
    and ``..density..`` (integrates to 1 over the grid).

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``: ``x``, ``y`` (required), ``fill``, ``color``,
        ``alpha``, ``size``, ``linetype``, ``width``, ``height``, ``weight``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name; ``"bin2d"`` when None.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"identity"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
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
        Constant aesthetics such as ``alpha=0.8``, and ``width``/``height`` (tile size
        as a share of the bin).

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"height": [160, 172, 181, 165, 170, 158], "weight": [55, 70, 82, 61, 68, 52]}
    >>> plot = ggplot(data, aes("height", "weight")) + geom_bin2d(bins=[4, 3])
    """
    return _geom(
        "bin2d",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        bins=bins,
        binwidth=binwidth,
        drop=drop,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_hex(
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
    bins: int | Sequence[int] | None = None,
    binwidth: float | Sequence[float | None] | None = None,
    drop: bool | None = None,
    width_unit: Literal["res", "identity"] | None = None,
    height_unit: Literal["res", "identity"] | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Hexagons counting the rows in a hexagonal grid (``stat="binhex"``).

    Hexagons stand on a point; every other row is shifted by half a bin. The stat
    computes ``..count..`` (rows, or summed ``weight``, per hexagon; ``fill`` by
    default) and ``..density..`` (integrates to 1 over the grid).

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``: ``x``, ``y`` (required), ``fill``, ``color``,
        ``alpha``, ``size``, ``linetype``, ``width``, ``height``, ``weight``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name; ``"binhex"`` when None.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"identity"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    bins : int or list of int, default=[30, 30]
        Number of hexagons along x and of rows along y; one number is used for both.
    binwidth : float or list of float, optional
        Hexagon width and height (a None entry keeps ``bins`` for that axis); wins
        over ``bins``.
    drop : bool, default=True
        False also draws the empty hexagons (count 0).
    width_unit : {"res", "identity"}, default="res"
        Unit of ``width``: the bin width (``"res"``) or x axis units (``"identity"``).
    height_unit : {"res", "identity"}, default="res"
        Unit of ``height``: the row spacing (``"res"``) or y axis units (``"identity"``).
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``alpha=0.8``, and ``width``/``height`` (hexagon
        size in ``width_unit``/``height_unit``).

    Returns
    -------
    LayerSpec
        The layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"height": [160, 172, 181, 165, 170, 158], "weight": [55, 70, 82, 61, 68, 52]}
    >>> plot = ggplot(data, aes("height", "weight")) + geom_hex(bins=[4, 3])
    """
    return _geom(
        "hex",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        bins=bins,
        binwidth=binwidth,
        drop=drop,
        width_unit=width_unit,
        height_unit=height_unit,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_raster(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    stat: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    manual_key: ManualKey | None = None,
    sampling: Sampling | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Equal-sized tiles on a regular grid. Not drawn yet.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "raster",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_rect(
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
    map: PlotData | None = None,
    map_join: str | Sequence[str] | None = None,
    use_crs: str | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Rectangles from ``xmin``, ``xmax``, ``ymin``, ``ymax``. Not drawn yet.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    map, map_join, use_crs : optional
        GIS options of lets_plot; ggrust does not support maps.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
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
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "rect",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        map=map,
        map_join=map_join,
        use_crs=use_crs,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_bracket(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    stat: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    manual_key: ManualKey | None = None,
    sampling: Sampling | None = None,
    orientation: str | None = None,
    label_format: str | None = None,
    na_text: str | None = None,
    nudge_x: Number | None = None,
    nudge_y: Number | None = None,
    nudge_unit: str | None = None,
    size_unit: str | None = None,
    bracket_shorten: Number | None = None,
    tiplength_unit: str | None = None,
    color_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Labelled brackets from ``xmin`` to ``xmax`` at height ``y`` (significance marks).

    Unlike most layers, the plot mapping is not inherited and no legend is shown
    unless ``show_legend=True``, as in lets_plot.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``: ``xmin``, ``xmax``, ``y``, ``label``, and
        ``color``/``alpha``. Tip lengths (``lenstart``, ``lenend``) and the bracket
        line (``segment_color``, ``segment_size``, ``segment_alpha``) are constants
        only in ggrust yet, e.g. ``segment_size=0.5``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    orientation : str, optional
        Only ``"x"`` (horizontal brackets) is drawn by ggrust yet.
    nudge_x, nudge_y : float, optional
        Shift of the whole bracket in data units.
    bracket_shorten : float, default 0
        Pull both ends toward the middle: 0 keeps the full width, 1 collapses it.
    tiplength_unit : {"size", "res", "identity", "px"}, default "size"
        Unit of ``lenstart``/``lenend``: point size units (2.2 px), the y resolution,
        data units, or pixels.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    label_format : str, optional
        Format pattern of the label text, e.g. ``".2f"`` or ``"{d} %"``.
    na_text : str, optional
        Text shown for missing values.
    nudge_unit : str, optional
        Unit of the nudge: ``"identity"`` (data units), ``"size"``, ``"px"``.
    size_unit : str, optional
        Relate ``size`` to an axis: ``"x"``, ``"y"``, ``"min"`` or ``"max"``.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``segment_size=1``, and the
        geom's options. As in lets_plot; options ggrust does not read yet are
        reported as not supported.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "bracket",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=None,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=None,
        orientation=orientation,
        label_format=label_format,
        na_text=na_text,
        nudge_x=nudge_x,
        nudge_y=nudge_y,
        nudge_unit=nudge_unit,
        size_unit=size_unit,
        bracket_shorten=bracket_shorten,
        tiplength_unit=tiplength_unit,
        color_by=color_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_text_repel(
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
    map: PlotData | None = None,
    map_join: str | Sequence[str] | None = None,
    use_crs: str | None = None,
    label_format: str | None = None,
    na_text: str | None = None,
    halo_width: Number | None = None,
    halo_color: Color | None = None,
    nudge_x: Number | None = None,
    nudge_y: Number | None = None,
    size_unit: str | None = None,
    nudge_unit: str | None = None,
    check_overlap: bool | None = None,
    color_by: str | None = None,
    seed: int | None = None,
    max_iter: int | None = None,
    max_time: Number | None = None,
    direction: int | None = None,
    point_padding: Number | None = None,
    box_padding: Number | None = None,
    max_overlaps: int | None = None,
    min_segment_length: Number | None = None,
    arrow: FeatureSpec | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Text labels pushed apart so they do not overlap each other or the points.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    map : DataFrame or dict, optional
        Spatial data (lets_plot ``map``); not supported by ggrust yet.
    map_join : str or list of str, optional
        Columns joining ``data`` to ``map`` (lets_plot); not supported by ggrust yet.
    use_crs : str, optional
        Coordinate reference system for ``map`` (lets_plot); not supported by ggrust yet.
    label_format : str, optional
        Format pattern of the label text, e.g. ``".2f"`` or ``"{d} %"``.
    na_text : str, optional
        Text shown for missing values.
    halo_width : float, optional
        Width of the halo around text.
    halo_color : str, optional
        Color of the halo around text.
    nudge_x : float, optional
        Horizontal offset of the labels.
    nudge_y : float, optional
        Vertical offset of the labels.
    size_unit : str, optional
        Relate ``size`` to an axis: ``"x"``, ``"y"``, ``"min"`` or ``"max"``.
    nudge_unit : str, optional
        Unit of the nudge: ``"identity"`` (data units), ``"size"``, ``"px"``.
    check_overlap : bool, optional
        True skips labels that overlap labels drawn before them.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    seed : int, optional
        Random seed, for reproducible results.
    max_iter : int, optional
        Maximum number of iterations of the repel layout.
    max_time : float, optional
        Maximum time of the repel layout, in seconds.
    direction : int, optional
        Repel direction: ``"both"``, ``"x"`` or ``"y"``.
    point_padding : float, optional
        Space kept free around each point.
    box_padding : float, optional
        Space kept free around each label.
    max_overlaps : int, optional
        Labels overlapping more than this many items are dropped.
    min_segment_length : float, optional
        Shorter connector segments are not drawn.
    arrow : FeatureSpec, optional
        Arrow spec from ``arrow()``.
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "text_repel",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        map=map,
        map_join=map_join,
        use_crs=use_crs,
        label_format=label_format,
        na_text=na_text,
        halo_width=halo_width,
        halo_color=halo_color,
        nudge_x=nudge_x,
        nudge_y=nudge_y,
        size_unit=size_unit,
        nudge_unit=nudge_unit,
        check_overlap=check_overlap,
        color_by=color_by,
        max_iter=max_iter,
        max_time=max_time,
        seed=seed,
        direction=direction,
        point_padding=point_padding,
        box_padding=box_padding,
        max_overlaps=max_overlaps,
        min_segment_length=min_segment_length,
        arrow=arrow,
        na_rm=na_rm,
        **other_args,
    )


def geom_label_repel(
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
    map: PlotData | None = None,
    map_join: str | Sequence[str] | None = None,
    use_crs: str | None = None,
    label_format: str | None = None,
    na_text: str | None = None,
    nudge_x: Number | None = None,
    nudge_y: Number | None = None,
    label_padding: Number | None = None,
    label_r: Number | None = None,
    label_size: Number | None = None,
    alpha_stroke: bool | None = None,
    size_unit: str | None = None,
    nudge_unit: str | None = None,
    check_overlap: bool | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    seed: int | None = None,
    max_iter: int | None = None,
    max_time: Number | None = None,
    direction: int | None = None,
    point_padding: Number | None = None,
    box_padding: Number | None = None,
    max_overlaps: int | None = None,
    min_segment_length: Number | None = None,
    arrow: FeatureSpec | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Boxed text labels pushed apart so they do not overlap each other or the points.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name.
    position : str or FeatureSpec, optional
        Position adjustment: a name such as ``"dodge"`` or a ``position_*()`` spec.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        A legend key for this layer, as in lets_plot; not supported yet (ggrust warns).
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"`` to disable
        tooltips (HTML output).
    map : DataFrame or dict, optional
        Spatial data (lets_plot ``map``); not supported by ggrust yet.
    map_join : str or list of str, optional
        Columns joining ``data`` to ``map`` (lets_plot); not supported by ggrust yet.
    use_crs : str, optional
        Coordinate reference system for ``map`` (lets_plot); not supported by ggrust yet.
    label_format : str, optional
        Format pattern of the label text, e.g. ``".2f"`` or ``"{d} %"``.
    na_text : str, optional
        Text shown for missing values.
    nudge_x : float, optional
        Horizontal offset of the labels.
    nudge_y : float, optional
        Vertical offset of the labels.
    label_padding : float, optional
        Space between the text and the label border.
    label_r : float, optional
        Radius of the label corners.
    label_size : float, optional
        Width of the label border line.
    alpha_stroke : bool, optional
        True applies ``alpha`` to the label border too.
    size_unit : str, optional
        Relate ``size`` to an axis: ``"x"``, ``"y"``, ``"min"`` or ``"max"``.
    nudge_unit : str, optional
        Unit of the nudge: ``"identity"`` (data units), ``"size"``, ``"px"``.
    check_overlap : bool, optional
        True skips labels that overlap labels drawn before them.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    seed : int, optional
        Random seed, for reproducible results.
    max_iter : int, optional
        Maximum number of iterations of the repel layout.
    max_time : float, optional
        Maximum time of the repel layout, in seconds.
    direction : int, optional
        Repel direction: ``"both"``, ``"x"`` or ``"y"``.
    point_padding : float, optional
        Space kept free around each point.
    box_padding : float, optional
        Space kept free around each label.
    max_overlaps : int, optional
        Labels overlapping more than this many items are dropped.
    min_segment_length : float, optional
        Shorter connector segments are not drawn.
    arrow : FeatureSpec, optional
        Arrow spec from ``arrow()``.
    na_rm : bool, optional
        True removes rows with missing values silently; otherwise they are removed with a warning.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "label_repel",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        map=map,
        map_join=map_join,
        use_crs=use_crs,
        label_format=label_format,
        na_text=na_text,
        nudge_x=nudge_x,
        nudge_y=nudge_y,
        label_padding=label_padding,
        label_r=label_r,
        label_size=label_size,
        alpha_stroke=alpha_stroke,
        size_unit=size_unit,
        nudge_unit=nudge_unit,
        check_overlap=check_overlap,
        color_by=color_by,
        fill_by=fill_by,
        max_iter=max_iter,
        max_time=max_time,
        seed=seed,
        direction=direction,
        point_padding=point_padding,
        box_padding=box_padding,
        max_overlaps=max_overlaps,
        min_segment_length=min_segment_length,
        arrow=arrow,
        na_rm=na_rm,
        **other_args,
    )


def _geom(
    name: str,
    *,
    mapping: FeatureSpec | None = None,
    data: PlotData | None = None,
    stat: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    na_rm: bool | None = None,
    manual_key: ManualKey | None = None,
    sampling: Sampling | None = None,
    tooltips: Tooltips | None = None,
    **kwargs: Any,
) -> LayerSpec:
    if mapping and not (isinstance(mapping, FeatureSpec) and mapping.kind == "mapping"):
        message = (
            f"geom_{name}(): 'mapping' must be an aes() mapping, got {type(mapping).__name__}; "
            "pass e.g. aes('x', 'y'), and data with data=..."
        )
        raise SpecError(message, path="mapping", reason=message)
    data = key_int2str(data)
    data, mapping, data_meta = as_annotated_data(data, mapping)
    if isinstance(tooltips, list | tuple):
        tooltips = layer_tooltips(cast("Sequence[str]", tooltips))
    return LayerSpec(
        geom=name,
        stat=stat,
        data=data,
        mapping=mapping,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        na_rm=na_rm,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        **data_meta,
        **kwargs,
    )
