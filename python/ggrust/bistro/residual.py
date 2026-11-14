"""``residual_plot``: the residuals of a fitted model against ``x``, with margins.

Follows lets-plot's bistro ``residual_plot`` (``lets_plot/bistro/residual.py`` and
``_plot2d_common.py``; MIT, see THIRD_PARTY_NOTICES.md). lets_plot fits the model in
Python and builds an ordinary plot, so ggrust does the same: the result is one
``ggplot`` of a frame with a ``..residual..`` column, a main layer, a dashed line at
zero and ``ggmarginal`` layers.

Fits: ``"lm"`` is a least-squares polynomial of degree ``deg`` (lets_plot uses
statsmodels' OLS; ggrust solves the normal equations of the centred and scaled x in
polars, which gives the same fitted values without numpy). ``"loess"`` uses
statsmodels' ``lowess`` and scipy's ``interp1d`` as lets_plot does, so it needs both
packages. ``"none"`` keeps ``y`` as it is.
"""

from __future__ import annotations

import math
import numbers
from collections.abc import Sequence
from typing import TYPE_CHECKING, Literal

import polars as pl

from ggrust._data import as_arrow
from ggrust.bistro._marginal import parse_marginal
from ggrust.bistro.joint import _GEOMS, _main_layer, _marginal_layers, _pair
from ggrust.errors import ColumnNotFoundError, SpecError, did_you_mean
from ggrust.plot.core import DummySpec, aes
from ggrust.plot.geom import geom_hline
from ggrust.plot.label import ylab
from ggrust.plot.plot import ggplot
from ggrust.plot.theme import element_line, element_text, theme

if TYPE_CHECKING:
    from ggrust._typing import PlotData
    from ggrust.bistro.joint import JointGeom
    from ggrust.plot.core import FeatureSpec, PlotSpec

__all__ = ["residual_plot"]

#: How the model is fitted.
ResidualMethod = Literal["lm", "loess", "lowess", "none"]

_METHODS: tuple[str, ...] = ("lm", "loess", "lowess", "none")
_RESIDUAL = "..residual.."
_BINS_DEFAULT = 30
_HLINE_COLOR = "magenta"
_HLINE_LINETYPE = "dashed"
_MARGINAL_DEFAULT = "dens:r"


def residual_plot(
    data: PlotData,
    x: str,
    y: str,
    *,
    method: ResidualMethod = "lm",
    deg: int = 1,
    span: float = 0.5,
    seed: int | None = None,
    max_n: int | None = None,
    geom: JointGeom = "point",
    bins: int | Sequence[int] | None = None,
    binwidth: float | Sequence[float] | None = None,
    color: str | None = None,
    size: float | None = None,
    alpha: float | None = None,
    color_by: str | None = None,
    show_legend: bool | None = None,
    hline: bool = True,
    marginal: str = _MARGINAL_DEFAULT,
) -> PlotSpec:
    """Plot the residuals of ``y`` after a fit on ``x``, against ``x``.

    As lets_plot's ``residual_plot``: the residuals (``y`` minus the fitted value) as
    points, 2D bins or a 2D density, a dashed magenta line at zero and a marginal
    density of the residuals on the right.

    Parameters
    ----------
    data : DataFrame or dict
        The data. Rows with a missing, NaN or infinite ``x`` or ``y`` are left out.
    x, y : str
        Names of the predictor and response columns.
    method : {"lm", "loess", "lowess", "none"}, default="lm"
        The model: a polynomial least-squares fit, a local regression (needs
        statsmodels and scipy), or none (plots ``y`` itself).
    deg : int, default=1
        Degree of the ``"lm"`` polynomial.
    span : float, default=0.5
        Share of the data in each local ``"loess"`` fit.
    seed : int, optional
        Random seed for ``max_n`` sampling.
    max_n : int, optional
        Fit ``"loess"`` on this many randomly chosen rows (of each group).
    geom : {"point", "tile", "hex", "density2d", "density2df", "pointdensity", "none"}, \
default="point"
        The main layer.
    bins : int or list of int, optional
        Number of bins along x and y (one value for both), for ``"tile"``/``"hex"``
        and histogram margins. Overridden by ``binwidth``.
    binwidth : float or list of float, optional
        Bin width along x and y (one value for both). By default both are the larger
        of (x range / 30) and (y range / 30) of the data, when ``bins`` is not given.
    color : str, optional
        Color of the main layer and of the margins.
    size : float, optional
        Size of the main layer's marks.
    alpha : float, optional
        Transparency of the main layer, in 0..1.
    color_by : str, optional
        Column whose values color (and fill) the layers; one fit per value.
    show_legend : bool, default=True
        False hides the legend of the main and marginal layers.
    hline : bool, default=True
        Draw the dashed line at zero.
    marginal : str, default="dens:r"
        The margins as ``"geom:sides[:size]"`` items separated by commas (see
        ``joint_plot``); ``"none"`` draws no margins.

    Returns
    -------
    PlotSpec
        The plot; add more layers, scales and themes with ``+``.

    Raises
    ------
    SpecError
        If ``method``, ``geom``, ``deg`` or a marginal item is not accepted, ``x`` or
        ``y`` is not numeric, a group has a single row, or ``"loess"`` is asked for
        without statsmodels and scipy.
    ColumnNotFoundError
        If ``x``, ``y`` or ``color_by`` is not a column of ``data``.

    Examples
    --------
    >>> import numpy as np
    >>> from ggrust import *
    >>> from ggrust.bistro.residual import residual_plot
    >>> rng = np.random.default_rng(42)
    >>> hours = rng.uniform(0, 10, 150)
    >>> data = {"hours": hours, "score": 50 + 4 * hours + rng.normal(0, 6, 150)}
    >>> plot = residual_plot(data, "hours", "score")
    """
    if method not in _METHODS:
        message = (
            f"residual_plot: unknown method {method!r}{did_you_mean(method, _METHODS)}; "
            f"expected one of {', '.join(repr(name) for name in _METHODS)}"
        )
        raise SpecError(message, path="method")
    if geom not in _GEOMS:
        message = (
            f"residual_plot: unknown geom {geom!r}{did_you_mean(geom, _GEOMS)}; "
            f"expected one of {', '.join(repr(name) for name in _GEOMS)}"
        )
        raise SpecError(message, path="geom")
    if isinstance(deg, bool) or not isinstance(deg, numbers.Integral) or deg < 0:
        message = f"residual_plot(deg={deg!r}): expected a whole number >= 0, e.g. 1 or 2"
        raise SpecError(message, path="deg")
    deg = int(deg)
    if marginal != "none":
        parse_marginal(marginal, "residual_plot")

    frame = _complete_rows(data, x, y, color_by)
    if color_by is None:
        stat_data = _with_residuals(frame, x, y, method, deg, span, seed, max_n)
    else:
        groups = frame.partition_by(color_by, maintain_order=True)
        parts = [_with_residuals(group, x, y, method, deg, span, seed, max_n) for group in groups]
        stat_data = pl.concat(parts) if parts else frame.with_columns(pl.lit(None).alias(_RESIDUAL))

    bins_xy = _pair(bins, "bins")
    binwidth_xy = _pair(binwidth, "binwidth")
    if binwidth_xy is None and bins_xy is None:
        binwidth_xy = _default_binwidth(frame, x, y)

    mapping: dict[str, str] = {"x": x, "y": _RESIDUAL}
    if color_by is not None:
        mapping["color"] = color_by
        mapping["fill"] = color_by

    layers: FeatureSpec = DummySpec()
    main_layer = _main_layer(geom, binwidth_xy, bins_xy, color, color_by, size, alpha, show_legend)
    if main_layer is not None:
        layers += main_layer
    if hline:
        layers += geom_hline(yintercept=0, color=_HLINE_COLOR, linetype=_HLINE_LINETYPE)
    if marginal != "none":
        layers += _marginal_layers(marginal, binwidth_xy, bins_xy, color, color_by, show_legend)
    title = ylab(y) if method == "none" else ylab(f"{y} residual")
    axes = theme(
        axis="blank",
        axis_text_x=element_text(),
        axis_title_x=element_text(),
        axis_line_y=element_line(),
        axis_ticks_y=element_line(),
        axis_text_y=element_text(),
        axis_title_y=element_text(),
    )
    return ggplot(stat_data, aes(**mapping)) + layers + title + axes


def _complete_rows(data: PlotData, x: str, y: str, color_by: str | None) -> pl.DataFrame:
    """Return ``data`` as a frame without rows whose ``x`` or ``y`` is missing, NaN or infinite.

    Raises ``ColumnNotFoundError`` for a missing column and ``SpecError`` when ``x``
    or ``y`` is not numeric, temporal or boolean.
    """
    frame = pl.DataFrame(as_arrow(data))
    for name in (x, y, color_by):
        if name is not None and name not in frame.columns:
            message = (
                f"residual_plot: column {name!r} not found{did_you_mean(name, frame.columns)}; "
                f"available columns: {', '.join(frame.columns)}"
            )
            raise ColumnNotFoundError(message, column=name, available=frame.columns)
    complete = pl.lit(True)
    for name in (x, y):
        dtype = frame.schema[name]
        if not (dtype.is_numeric() or dtype.is_temporal() or dtype == pl.Boolean):
            message = (
                f"residual_plot: column {name!r} holds {dtype}, not numbers; "
                "the fit needs numeric x and y (cast the column first)"
            )
            raise SpecError(message, path="x" if name == x else "y")
        column = pl.col(name)
        complete = complete & column.is_not_null()
        if dtype.is_float():
            # NaN and infinite values cannot be fitted (they would make every residual NaN).
            complete = complete & column.is_finite()
    return frame.filter(complete)


def _with_residuals(
    frame: pl.DataFrame,
    x: str,
    y: str,
    method: str,
    deg: int,
    span: float,
    seed: int | None,
    max_n: int | None,
) -> pl.DataFrame:
    """Return ``frame`` with the ``..residual..`` column of one fit."""
    if frame.height == 1:
        message = (
            "residual_plot: a fit needs at least two rows with x and y"
            + ("" if method == "none" else f" (method={method!r})")
            + "; remove the group or add data"
        )
        raise SpecError(message, path="data")
    if frame.height == 0:
        return frame.with_columns(pl.lit(None, dtype=pl.Float64).alias(_RESIDUAL))
    xs = frame[x].cast(pl.Float64)
    ys = frame[y].cast(pl.Float64)
    if method == "lm":
        fitted = _polynomial_fit(xs, ys, deg)
    elif method == "none":
        fitted = pl.zeros(frame.height, dtype=pl.Float64, eager=True)
    else:
        fitted = _loess_fit(xs, ys, span, seed, max_n)
    return frame.with_columns((ys - fitted).alias(_RESIDUAL))


def _polynomial_fit(xs: pl.Series, ys: pl.Series, deg: int) -> pl.Series:
    """Fitted values of the least-squares polynomial of degree ``deg``.

    The fit uses ``t = (x - mean) / sd``: the polynomials in t and in x span the
    same functions, so the fitted values are the same, and the normal equations of
    t are well conditioned. Any least-squares solution gives the same fitted values,
    so a column that adds nothing (all x equal) is dropped from the solve.
    """
    mean = xs.mean()
    spread = xs.std()
    center = float(mean) if isinstance(mean, (int, float)) else 0.0
    scale = float(spread) if isinstance(spread, (int, float)) else 0.0
    if not math.isfinite(scale) or scale <= 0:
        scale = 1.0
    t = (xs - center) / scale
    frame = pl.DataFrame({"t": t, "y": ys})
    powers = frame.select(
        [(pl.col("t") ** power).sum().alias(f"p{power}") for power in range(2 * deg + 1)]
        + [
            ((pl.col("t") ** power) * pl.col("y")).sum().alias(f"q{power}")
            for power in range(deg + 1)
        ]
    ).row(0)
    size = deg + 1
    matrix = [[float(powers[row + column]) for column in range(size)] for row in range(size)]
    rhs = [float(powers[2 * deg + 1 + row]) for row in range(size)]
    coefficients = _solve(matrix, rhs)
    fitted = pl.lit(0.0)
    for coefficient in reversed(coefficients):
        fitted = fitted * pl.col("t") + coefficient
    return frame.select(fitted.alias("fitted"))["fitted"]


def _solve(matrix: list[list[float]], rhs: list[float]) -> list[float]:
    """Solve the symmetric positive semi-definite system by Gaussian elimination.

    Partial pivoting; a pivot below 1e-12 of the largest diagonal is a direction
    without data, and its coefficient is 0 (still a least-squares solution).
    """
    size = len(rhs)
    rows = [[*matrix[index], rhs[index]] for index in range(size)]
    tolerance = 1e-12 * max((abs(rows[index][index]) for index in range(size)), default=0.0)
    pivots: list[int | None] = [None] * size
    row = 0
    for column in range(size):
        best = max(range(row, size), key=lambda index: abs(rows[index][column]), default=None)
        if best is None or abs(rows[best][column]) <= tolerance:
            continue
        rows[row], rows[best] = rows[best], rows[row]
        for other in range(size):
            if other != row and rows[other][column] != 0.0:
                factor = rows[other][column] / rows[row][column]
                rows[other] = [a - factor * b for a, b in zip(rows[other], rows[row], strict=True)]
        pivots[column] = row
        row += 1
    return [
        0.0 if pivot is None else rows[pivot][size] / rows[pivot][column]
        for column, pivot in enumerate(pivots)
    ]


def _loess_fit(
    xs: pl.Series, ys: pl.Series, span: float, seed: int | None, max_n: int | None
) -> pl.Series:
    """Fitted values of statsmodels' ``lowess``, interpolated at ``xs`` (as lets_plot)."""
    try:
        import numpy as np
        import statsmodels.api as sm  # ty: ignore[unresolved-import]
        from scipy.interpolate import interp1d  # ty: ignore[unresolved-import]
    except ImportError as error:
        message = (
            "residual_plot(method='loess') needs numpy, statsmodels and scipy "
            f"({error.name} is missing); install them, or use method='lm'"
        )
        raise SpecError(message, path="method") from error
    x_train = xs.to_numpy()
    y_train = ys.to_numpy()
    if max_n is not None:
        np.random.seed(seed)
        indices = np.random.choice(range(x_train.size), size=max_n, replace=False)
        x_train = x_train[indices]
        y_train = y_train[indices]
    smoothed = sm.nonparametric.lowess(y_train, x_train, frac=span)
    model = interp1d(smoothed[:, 0], smoothed[:, 1], bounds_error=False)
    return pl.Series("fitted", [float(model(value)) for value in xs.to_numpy()], dtype=pl.Float64)


def _default_binwidth(frame: pl.DataFrame, x: str, y: str) -> list[float] | None:
    """lets_plot's default ``[w, w]`` from the data's x and y (not the residuals)."""
    if frame.height == 0:
        return None
    xs = frame[x].cast(pl.Float64)
    ys = frame[y].cast(pl.Float64)
    width_x = (float(xs.max()) - float(xs.min())) / _BINS_DEFAULT  # ty: ignore[invalid-argument-type]
    width_y = (float(ys.max()) - float(ys.min())) / _BINS_DEFAULT  # ty: ignore[invalid-argument-type]
    width = max(width_x, width_y)
    return [width, width]
