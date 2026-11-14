"""``joint_plot``: a bivariate layer with univariate marginal layers around it.

Follows lets-plot's bistro ``joint_plot`` (``lets_plot/bistro/joint.py`` and
``_plot2d_common.py``; MIT, see THIRD_PARTY_NOTICES.md). lets_plot builds this plot in
Python from ordinary layers, so ggrust does the same: the result is one ``ggplot``
with a main layer, an optional regression line and ``ggmarginal`` layers, and its
``as_dict()`` matches lets_plot's for the same arguments.
"""

from __future__ import annotations

import math
from collections.abc import Sequence
from typing import TYPE_CHECKING, Literal, TypeVar, cast

import polars as pl

from ggrust._data import as_arrow
from ggrust.bistro._marginal import parse_marginal
from ggrust.errors import ColumnNotFoundError, SpecError, did_you_mean
from ggrust.plot.core import DummySpec, aes
from ggrust.plot.geom import (
    geom_area,
    geom_bin2d,
    geom_boxplot,
    geom_density2d,
    geom_density2df,
    geom_hex,
    geom_histogram,
    geom_point,
    geom_smooth,
)
from ggrust.plot.geom_pointdensity import geom_pointdensity
from ggrust.plot.label import xlab, ylab
from ggrust.plot.plot import ggplot
from ggrust.plot.side import ggmarginal

if TYPE_CHECKING:
    from ggrust._typing import PlotData
    from ggrust.plot.core import FeatureSpec, FeatureSpecArray, LayerSpec, PlotSpec

__all__ = ["joint_plot"]

_Number = TypeVar("_Number", int, float)

#: The main layer's geometry.
JointGeom = Literal["point", "tile", "hex", "density2d", "density2df", "pointdensity", "none"]

_GEOMS: tuple[str, ...] = (
    "point",
    "tile",
    "hex",
    "density2d",
    "density2df",
    "pointdensity",
    "none",
)
_BINS_DEFAULT = 30
_COLOR_DEFAULT = "pen"
_MARGINAL_ALPHA = 0.1
_REGRESSION_METHOD = "lm"
_REGRESSION_COLOR = "magenta"
_REGRESSION_LINETYPE = "dashed"


def joint_plot(
    data: PlotData,
    x: str,
    y: str,
    *,
    geom: JointGeom | None = None,
    bins: int | Sequence[int] | None = None,
    binwidth: float | Sequence[float] | None = None,
    color: str | None = None,
    size: float | None = None,
    alpha: float | None = None,
    color_by: str | None = None,
    show_legend: bool | None = None,
    reg_line: bool | None = None,
    se: bool | None = None,
    marginal: str | None = None,
) -> PlotSpec:
    """Draw a bivariate plot of ``x`` and ``y`` with their distributions in the margins.

    As lets_plot's ``joint_plot``: a main layer (points, 2D bins or a 2D density), a
    dashed linear regression line for point layers, and marginal densities,
    histograms or boxplots (``ggmarginal``) on the chosen sides.

    Parameters
    ----------
    data : DataFrame or dict
        The data.
    x, y : str
        Names of the columns on the x and y axes.
    geom : {"point", "tile", "hex", "density2d", "density2df", "pointdensity", "none"}, \
default="point"
        The main layer; ``"none"`` draws only the regression line and margins.
    bins : int or list of int, optional
        Number of bins along x and y (one value for both), for ``"tile"``/``"hex"``
        and the histogram margins. Overridden by ``binwidth``.
    binwidth : float or list of float, optional
        Bin width along x and y (one value for both). By default both are the larger
        of (x range / 30) and (y range / 30), when ``bins`` is not given.
    color : str, optional
        Color of the main layer and of the margins.
    size : float, optional
        Size of the main layer's marks.
    alpha : float, optional
        Transparency of the main layer, in 0..1.
    color_by : str, optional
        Column whose values color (and fill) the layers, one regression line and one
        marginal shape per value.
    show_legend : bool, default=True
        False hides the legend of the main and marginal layers.
    reg_line : bool, optional
        Draw a linear regression line; by default only for ``"point"`` and
        ``"pointdensity"``.
    se : bool, default=True
        Draw the confidence band around the regression line.
    marginal : str, optional
        The margins as ``"geom:sides[:size]"`` items separated by commas: geom one of
        ``"dens"``/``"density"``, ``"hist"``/``"histogram"``, ``"box"``/``"boxplot"``,
        sides any of ``"t"``, ``"b"``, ``"l"``, ``"r"``, size a share of the plot (see
        ``ggmarginal``), e.g. ``"hist:tr:0.3"`` or ``"box:tr:.05, dens:bl"``. Default
        ``"dens:tr"`` for 2D densities or with ``color_by``, else ``"hist:tr"``;
        ``"none"`` draws no margins.

    Returns
    -------
    PlotSpec
        The plot; add more layers, scales and themes with ``+``.

    Raises
    ------
    SpecError
        If ``geom`` or a marginal item is not one of the accepted values.
    ColumnNotFoundError
        If ``x`` or ``y`` is not a column of ``data`` (only when default bin widths
        must be computed from the data).

    Examples
    --------
    >>> import numpy as np
    >>> from ggrust import *
    >>> from ggrust.bistro.joint import joint_plot
    >>> rng = np.random.default_rng(42)
    >>> data = {"height": rng.normal(170, 8, 200), "weight": rng.normal(70, 10, 200)}
    >>> plot = joint_plot(data, "height", "weight")
    """
    geom_kind = geom or "point"
    if geom_kind not in _GEOMS:
        message = (
            f"joint_plot: unknown geom {geom_kind!r}{did_you_mean(geom_kind, _GEOMS)}; "
            f"expected one of {', '.join(repr(name) for name in _GEOMS)}"
        )
        raise SpecError(message, path="geom")
    bins_xy = _pair(bins, "bins")
    binwidth_xy = _pair(binwidth, "binwidth")
    if binwidth_xy is None and bins_xy is None:
        binwidth_xy = _default_binwidth(data, x, y)

    mapping: dict[str, str] = {"x": x, "y": y}
    if color_by is not None:
        mapping["color"] = color_by
        mapping["fill"] = color_by

    layers: FeatureSpec = DummySpec()
    main_layer = _main_layer(
        geom_kind, binwidth_xy, bins_xy, color, color_by, size, alpha, show_legend
    )
    if main_layer is not None:
        layers += main_layer
    draw_regression = reg_line if reg_line is not None else geom_kind in ("point", "pointdensity")
    if draw_regression:
        layers += geom_smooth(
            aes(group=color_by),
            method=_REGRESSION_METHOD,
            se=se,
            color=_REGRESSION_COLOR,
            linetype=_REGRESSION_LINETYPE,
        )
    if _row_count(data, x) == 0:
        marginal = "none"
    if marginal is None:
        marginal = "dens:tr" if geom_kind in ("density2d", "density2df") or color_by else "hist:tr"
    if marginal != "none":
        layers += _marginal_layers(marginal, binwidth_xy, bins_xy, color, color_by, show_legend)
    return ggplot(data, aes(**mapping)) + layers + xlab(x) + ylab(y)


def _pair(value: _Number | Sequence[_Number] | None, name: str) -> list[_Number] | None:
    """Return a bins or binwidth argument as an ``[x, y]`` list (lets_plot's rule)."""
    if value is None:
        return None
    if not isinstance(value, Sequence):
        return [value, value]
    pair = list(value)
    if len(pair) != 2:
        message = (
            f"joint_plot({name}={value!r}): expected one number or a pair [x, y], "
            f"got {len(pair)} values"
        )
        raise SpecError(message, path=name)
    return pair


def _column(data: PlotData, name: str) -> pl.Series:
    """Return the column ``name`` of ``data``, or raise ``ColumnNotFoundError``."""
    frame = pl.DataFrame(as_arrow(data))
    if name not in frame.columns:
        message = (
            f"joint_plot: column {name!r} not found{did_you_mean(name, frame.columns)}; "
            f"available columns: {', '.join(frame.columns)}"
        )
        raise ColumnNotFoundError(message, column=name, available=frame.columns)
    return frame[name]


def _row_count(data: PlotData, name: str) -> int:
    """Return the number of rows of ``data`` (0 for an empty frame)."""
    return _column(data, name).len()


def _default_binwidth(data: PlotData, x: str, y: str) -> list[float] | None:
    """Return lets_plot's default ``[w, w]``: the larger of the two ranges over 30.

    None for temporal columns (their axis unit is not the column's), so the
    layers' own default bins apply.
    """
    temporal = (pl.Date, pl.Datetime, pl.Time, pl.Duration)
    if any(isinstance(_column(data, name).dtype, temporal) for name in (x, y)):
        return None
    x_values = _column(data, x).cast(pl.Float64, strict=False).drop_nulls().drop_nans()
    y_values = _column(data, y).cast(pl.Float64, strict=False).drop_nulls().drop_nans()
    if x_values.is_empty() or y_values.is_empty():
        return None
    width_x = (_float(x_values.max()) - _float(x_values.min())) / _BINS_DEFAULT
    width_y = (_float(y_values.max()) - _float(y_values.min())) / _BINS_DEFAULT
    width = max(width_x, width_y)
    if not math.isfinite(width) or width <= 0:
        return None
    return [width, width]


def _float(value: object) -> float:
    """Return a polars scalar as a float."""
    return float(value)  # ty: ignore[invalid-argument-type]


def _main_layer(
    geom_kind: str,
    binwidth: list[float] | None,
    bins: list[int] | None,
    color: str | None,
    color_by: str | None,
    size: float | None,
    alpha: float | None,
    show_legend: bool | None,
) -> LayerSpec | None:
    """Return the bivariate layer for ``geom_kind`` (None for ``"none"``)."""
    if geom_kind == "point":
        return geom_point(color=color, size=size, alpha=alpha, show_legend=show_legend)
    if geom_kind in ("tile", "hex"):
        layer = geom_bin2d if geom_kind == "tile" else geom_hex
        return layer(
            aes(fill="..count.." if color_by is None else color_by),
            bins=bins,
            binwidth=binwidth,
            color=color,
            size=size,
            alpha=alpha,
            show_legend=show_legend,
        )
    if geom_kind == "density2d":
        return geom_density2d(
            aes(color="..level.." if color_by is None else color_by),
            color=color,
            size=size,
            alpha=alpha,
            show_legend=show_legend,
        )
    if geom_kind == "density2df":
        return geom_density2df(
            aes(fill="..level.." if color_by is None else color_by),
            color=color,
            size=size,
            alpha=alpha,
            show_legend=show_legend,
        )
    if geom_kind == "pointdensity":
        return geom_pointdensity(
            aes(color="..density.." if color_by is None else color_by),
            color=color,
            size=size,
            alpha=alpha,
            show_legend=show_legend,
        )
    return None


def _marginal_layers(
    marginal: str,
    binwidth: list[float] | None,
    bins: list[int] | None,
    color: str | None,
    color_by: str | None,
    show_legend: bool | None,
) -> FeatureSpec:
    """Parse ``marginal`` (``"geom:sides[:size], ..."``) into ``ggmarginal`` layers."""
    marginal_color = None if color_by is not None else (color or _COLOR_DEFAULT)
    result: FeatureSpec = DummySpec()
    for item in parse_marginal(marginal, "joint_plot"):
        for side in item.sides:
            layer = _marginal_layer(item.geom, side, binwidth, bins, marginal_color, show_legend)
            result += ggmarginal(side, size=item.size, layer=layer)
    return result


def _marginal_layer(
    geom_kind: str,
    side: str,
    binwidth: list[float] | None,
    bins: list[int] | None,
    color: str | None,
    show_legend: bool | None,
) -> LayerSpec | FeatureSpecArray:
    """Return one marginal layer (lets_plot's alpha 0.1, color and fill alike)."""
    if geom_kind in ("dens", "density"):
        return geom_area(
            stat="density",
            position="identity",
            color=color,
            fill=color,
            alpha=_MARGINAL_ALPHA,
            show_legend=show_legend,
        )
    if geom_kind in ("hist", "histogram"):
        index = 0 if side in ("t", "b") else 1
        return geom_histogram(
            bins=None if bins is None else bins[index],
            binwidth=None if binwidth is None else binwidth[index],
            color=color,
            fill=color,
            alpha=_MARGINAL_ALPHA,
            show_legend=show_legend,
        )
    boxplot = geom_boxplot(color=color, fill=color, alpha=_MARGINAL_ALPHA, show_legend=show_legend)
    return cast("FeatureSpecArray", boxplot)
