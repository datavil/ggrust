"""Outlines of a 2D point cloud at a fraction of its peak density, computed in Rust.

Not in lets-plot. ``density_outline()`` evaluates a Gaussian kernel density with a
full covariance (as ``scipy.stats.gaussian_kde``) on a padded grid and traces the
contour at ``level`` times its peak by marching squares (as
``skimage.measure.find_contours``), so outlines need neither scipy nor scikit-image.
The result is a polars frame to draw with ``geom_path``.
"""

from __future__ import annotations

import math
from collections.abc import Sequence
from numbers import Integral, Real
from typing import Literal, Protocol, TypeAlias

import polars as pl

from ggrust._ggrust import density_outline as _density_outline
from ggrust.errors import SpecError

__all__ = ["density_outline"]


class _ArrayLike(Protocol):
    """Anything with ``tolist()``, such as a numpy array."""

    def tolist(self) -> object: ...


Pairs: TypeAlias = Sequence[Sequence[float]] | _ArrayLike
"""Rows of two values (an ``(n, 2)`` array): nested sequences or a numpy array."""

Bandwidth: TypeAlias = Literal["scott", "silverman"] | float
"""A bandwidth rule name, or a factor that multiplies the data standard deviations."""


def density_outline(
    points: Pairs,
    *,
    level: float = 0.1,
    padding: float = 1.0,
    grid_size: int = 200,
    bandwidth: Bandwidth = "scott",
) -> pl.DataFrame:
    """Outline a 2D point cloud where its kernel density falls to ``level`` of the peak.

    The density is a Gaussian kernel estimate with a full covariance (the data
    covariance scaled by the ``bandwidth`` factor, as ``scipy.stats.gaussian_kde``).
    It is evaluated on a ``grid_size`` by ``grid_size`` grid spanning the points'
    bounding box widened by ``padding`` on every side, and the contour at
    ``level * peak`` is traced by marching squares, as
    ``skimage.measure.find_contours``. The vertices match that scipy and
    scikit-image recipe to about 1e-15. Draw the result with ``geom_path``.

    Parameters
    ----------
    points : (n, 2) array
        Point coordinates, one ``(x, y)`` row per point: a numpy array, nested
        sequences or a two-column data frame. Rows with a non-finite coordinate
        (or a masked entry) are dropped; at least 3 points must remain, and they
        must not lie on one line.
    level : float, default=0.1
        Outline level as a fraction of the peak density, between 0 and 1
        (exclusive). Lower values draw a wider outline that takes in sparse points.
    padding : float, default=1.0
        Space added to each side of the points' bounding box before gridding, in
        data units. Too little padding cuts low-level outlines open at the grid edge.
    grid_size : int, default=200
        Grid points per axis, from 2 to 3162. Higher values give smoother outlines;
        the work grows with ``grid_size ** 2`` times the number of points.
    bandwidth : {'scott', 'silverman'} or float, default='scott'
        Kernel bandwidth: Scott's or Silverman's rule (scipy's ``bw_method``), or a
        positive factor that multiplies the data standard deviations.

    Returns
    -------
    polars.DataFrame
        One row per outline vertex: ``x`` and ``y`` (Float64) and ``path`` (UInt32,
        the outline number from 0, for ``aes(group="path")``). Closed outlines
        repeat their first vertex. A cloud with several dense spots can give
        several outlines; a ``level`` the density never crosses gives none.

    Raises
    ------
    ggrust.errors.SpecError
        If ``points`` is not an ``(n, 2)`` array of numbers, fewer than 3 finite
        points remain, the points lie on one line, or an option is out of range.

    Examples
    --------
    >>> import math
    >>> from ggrust import *
    >>> points = [[math.cos(i) * (i % 7) / 7, math.sin(i) * (i % 5) / 5] for i in range(200)]
    >>> outline = density_outline(points, level=0.1, padding=0.5, grid_size=80)
    >>> outline.columns
    ['x', 'y', 'path']
    >>> outline["path"].unique().to_list()
    [0]
    >>> plot = (
    ...     ggplot()
    ...     + geom_point(aes("x", "y"), data={"x": [p[0] for p in points],
    ...                                       "y": [p[1] for p in points]}, size=1)
    ...     + geom_path(aes("x", "y", group="path"), data=outline, linetype="dashed")
    ...     + coord_fixed()
    ... )
    """
    options = (
        _number(level, "level"),
        _number(padding, "padding"),
        _grid_size(grid_size),
        _bandwidth(bandwidth),
    )
    rows = _to_list(points)
    try:
        x, y, path = _density_outline(rows, *options)
    except SpecError:
        raise
    except (TypeError, ValueError, OverflowError):
        # Not plain rows of two numbers: find the culprit (masked entries become NaN).
        x, y, path = _density_outline(_pairs(rows, "points"), *options)
    return pl.DataFrame(
        {"x": x, "y": y, "path": path},
        schema={"x": pl.Float64, "y": pl.Float64, "path": pl.UInt32},
    )


def _spec_error(path: str, reason: str) -> SpecError:
    """Build a ``SpecError`` for argument ``path`` of ``density_outline()``."""
    return SpecError(
        f"invalid density_outline argument at '{path}': {reason}", path=path, reason=reason
    )


def _to_list(values: object) -> object:
    """Return numpy-like arrays (``tolist()``) and data frames (``to_numpy()``) as lists.

    Other values are returned as they are.
    """
    tolist = getattr(values, "tolist", None)
    if callable(tolist):
        return tolist()
    to_numpy = getattr(values, "to_numpy", None)
    if callable(to_numpy):
        return _to_list(to_numpy())
    return values


def _number(value: object, path: str) -> float:
    """Return ``value`` as a float, raising if it is not a real number."""
    if isinstance(value, bool) or not isinstance(value, Real):
        raise _spec_error(path, f"expected a number, got {type(value).__name__} {value!r}")
    return float(value)


def _grid_size(value: object) -> int:
    """Return ``grid_size`` as an int, raising if it is not an integer from 2 to 3162."""
    if isinstance(value, bool) or not isinstance(value, Integral) or not 2 <= int(value) <= 3162:
        raise _spec_error("grid_size", f"expected an integer from 2 to 3162, got {value!r}")
    return int(value)


def _bandwidth(value: object) -> str | float:
    """Return ``bandwidth`` as a rule name or a positive float, raising otherwise."""
    if isinstance(value, str):
        if value in ("scott", "silverman"):
            return value
        raise _spec_error(
            "bandwidth", f"unknown bandwidth rule {value!r}; pass 'scott', 'silverman' or a number"
        )
    factor = _number(value, "bandwidth")
    if not (math.isfinite(factor) and factor > 0):
        raise _spec_error(
            "bandwidth", f"a bandwidth factor must be a positive finite number, got {value!r}"
        )
    return factor


def _pairs(values: object, path: str) -> list[tuple[float, float]]:
    """Return ``(n, 2)`` values as float pairs; ``None`` (a masked entry) becomes NaN."""
    if not isinstance(values, Sequence) or isinstance(values, str):
        raise _spec_error(path, f"expected an (n, 2) array, got {type(values).__name__}")
    pairs = []
    for index, row in enumerate(values):
        row = _to_list(row)
        if not isinstance(row, Sequence) or isinstance(row, str) or len(row) != 2:
            raise _spec_error(
                path, f"row {index} must hold 2 values (x, y), got {row!r}; pass an (n, 2) array"
            )
        pair = []
        for value in row:
            if value is None:
                pair.append(math.nan)
            elif isinstance(value, Real):
                try:
                    pair.append(float(value))
                except OverflowError:
                    raise _spec_error(
                        path, f"row {index} holds {value!r}, too large for a float"
                    ) from None
            else:
                raise _spec_error(
                    path, f"row {index} holds {type(value).__name__} {value!r}; expected numbers"
                )
        pairs.append((pair[0], pair[1]))
    return pairs
