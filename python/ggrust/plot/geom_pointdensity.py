"""Point layers colored by local point density (R package ggpointdensity).

Not in lets-plot; ggplot2 needs the ggpointdensity package. The densities follow
ggpointdensity 0.2.1; the code is ggrust's own. Every point keeps its place and
gets ``..density..`` (mapped to ``color`` by default) and ``..ndensity..``
(density over the group's maximum), computed per group by ``stat="pointdensity"``.
"""

from __future__ import annotations

import math
from numbers import Real
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from ggrust.errors import SpecError
from ggrust.plot.geom import _geom
from ggrust.plot.geom_beeswarm import _check_choice, _check_positive

if TYPE_CHECKING:
    from ggrust._typing import ManualKey, Number, PlotData, Position, Sampling, Tooltips
    from ggrust.plot.core import FeatureSpec, LayerSpec

__all__ = ["geom_pointdensity", "stat_pointdensity"]

PointDensityMethod: TypeAlias = Literal["auto", "neighbors", "kde2d"]
"""How ``stat="pointdensity"`` estimates the density at each point."""

_METHODS: tuple[str, ...] = ("auto", "neighbors", "kde2d")


def geom_pointdensity(
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
    method: PointDensityMethod | None = None,
    adjust: Number | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Scatter plot with each point colored by the density of points around it.

    As ``geom_pointdensity`` in the R package ggpointdensity: a point layer over
    ``stat="pointdensity"``, with ``color`` mapped to ``..density..``. Useful for
    dense scatter plots such as UMAP embeddings, where overplotting hides how many
    points sit in a region.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Needs ``x`` and ``y``. Map ``color`` to
        ``"..ndensity.."`` for densities scaled to 1 per group.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name; ``"pointdensity"`` by default.
    position : str or FeatureSpec, optional
        Position adjustment, applied after the densities are computed.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        This layer's key in a manual legend: a label, or ``layer_key()``.
    sampling : str, int or FeatureSpec, optional
        Rows to draw: a ``sampling_*()`` spec, an int ``n`` (``sampling_random(n)``)
        or ``"none"``. Default: every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"``.
    method : {"auto", "neighbors", "kde2d"}, default="auto"
        ``"neighbors"`` counts the points (the point itself included) within an
        ellipse around each point whose size follows the x and y ranges.
        ``"kde2d"`` reads a 2D Gaussian kernel density estimate (``MASS::kde2d``,
        100 x 100 grid) at each point. ``"auto"`` uses ``"kde2d"`` when a group has
        more than 20000 rows, as ggpointdensity does, else ``"neighbors"``.
    adjust : float, default=1
        Multiplies the neighbour radius (``"neighbors"``) or the bandwidth
        (``"kde2d"``). Larger values give smoother densities.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics (``size``, ``alpha``, ``shape``, ...). A constant
        ``color`` replaces the density coloring.

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If ``method`` is not one of the accepted names or ``adjust`` is not positive.
    UnsupportedValueError
        If ``method`` is not a string or ``adjust`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> import numpy as np
    >>> generator = np.random.default_rng(1)
    >>> data = {"height": generator.normal(170, 8, 2000),
    ...         "weight": generator.normal(70, 10, 2000)}
    >>> plot = (ggplot(data, aes("height", "weight")) + geom_pointdensity(size=1.5)
    ...         + scale_color_viridis())
    """
    _check("geom_pointdensity", method, adjust)
    return _geom(
        "point",
        mapping=mapping,
        data=data,
        stat=stat if stat is not None else "pointdensity",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        method=method,
        adjust=adjust,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def stat_pointdensity(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    sampling: Sampling | None = None,
    tooltips: Tooltips | None = None,
    method: PointDensityMethod | None = None,
    adjust: Number | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Density of points around each point (``stat="pointdensity"``), drawn as points.

    As ``stat_pointdensity`` in the R package ggpointdensity. Computes
    ``..density..`` (neighbour count or kernel density, mapped to ``color`` by
    default) and ``..ndensity..`` (density over the group's maximum) for every row.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Needs ``x`` and ``y``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, default="point"
        Geom that draws the rows, e.g. ``"text"`` with ``aes(label="..density..")``.
    position : str or FeatureSpec, optional
        Position adjustment, applied after the densities are computed.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    sampling : str, int or FeatureSpec, optional
        Rows to draw: a ``sampling_*()`` spec, an int ``n`` (``sampling_random(n)``)
        or ``"none"``. Default: every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"``.
    method : {"auto", "neighbors", "kde2d"}, default="auto"
        See ``geom_pointdensity``.
    adjust : float, default=1
        Multiplies the neighbour radius or the kernel bandwidth.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics and the geom's options.

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If ``method`` is not one of the accepted names or ``adjust`` is not positive.
    UnsupportedValueError
        If ``method`` is not a string or ``adjust`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1.0, 1.1, 1.2, 3.0, 3.1, 5.0], "y": [2.0, 2.1, 1.9, 4.0, 4.2, 1.0]}
    >>> plot = ggplot(data, aes("x", "y")) + stat_pointdensity(aes(size="..ndensity.."))
    """
    _check("stat_pointdensity", method, adjust)
    return _geom(
        geom if geom is not None else "point",
        mapping=mapping,
        data=data,
        stat="pointdensity",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        sampling=sampling,
        tooltips=tooltips,
        method=method,
        adjust=adjust,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def _check(function: str, method: object, adjust: object) -> None:
    """Raise for an unknown ``method`` or an ``adjust`` that is not a positive finite number."""
    _check_choice(function, "method", method, _METHODS)
    _check_positive(function, "adjust", adjust)
    if isinstance(adjust, Real) and not math.isfinite(adjust):
        reason = f"adjust must be a finite positive number, got {adjust}"
        raise SpecError(f"{function}(adjust={adjust!r}): {reason}", path="adjust", reason=reason)
