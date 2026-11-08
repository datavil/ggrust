"""Cell velocities averaged onto a regular grid, computed in Rust (scVelo's gridding).

Not in lets-plot. ``velocity_grid()`` ports scVelo's ``compute_velocity_on_grid``
(scvelo 0.3.4), so RNA velocity streams need neither scVelo nor matplotlib: its
result feeds straight into ``streamplot()``.
"""

from __future__ import annotations

import math
from collections.abc import Sequence
from dataclasses import dataclass
from numbers import Integral, Real
from typing import Protocol, TypeAlias

from ggrust._ggrust import velocity_grid as _velocity_grid
from ggrust.errors import SpecError

__all__ = ["VelocityGrid", "velocity_grid"]


class _ArrayLike(Protocol):
    """Anything with ``tolist()``, such as a numpy array."""

    def tolist(self) -> object: ...


Pairs: TypeAlias = Sequence[Sequence[float]] | _ArrayLike
"""Rows of two values (an ``(n, 2)`` array): nested sequences or a numpy array."""


@dataclass(frozen=True, eq=False)
class VelocityGrid:
    """Velocities on a regular grid, laid out for ``streamplot(x, y, u, v)``.

    Attributes
    ----------
    x : list of float
        Grid x coordinates, increasing (constant when the points do not spread in
        x, which only ``adjust_for_stream=False`` accepts, as scVelo).
    y : list of float
        Grid y coordinates, increasing.
    u : list of list of float
        x component of the velocity, one row per ``y`` value and one column per
        ``x`` value. Hidden grid points are NaN.
    v : list of list of float
        y component of the velocity, laid out as ``u``. NaN where ``u`` is NaN
        only when ``adjust_for_stream=False`` (as scVelo, which hides stream
        points through ``u`` alone; ``streamplot`` treats either as missing).

    Examples
    --------
    >>> from ggrust import *
    >>> points = [[i % 10, i // 10] for i in range(100)]
    >>> vectors = [[1.0, 0.5]] * 100
    >>> grid = velocity_grid(points, vectors, n_neighbors=5)
    >>> len(grid.x), len(grid.y), len(grid.u), len(grid.u[0])
    (50, 50, 50, 50)
    """

    x: list[float]
    y: list[float]
    u: list[list[float]]
    v: list[list[float]]


def velocity_grid(
    points: Pairs,
    vectors: Pairs,
    *,
    density: float = 1,
    smooth: float = 0.5,
    n_neighbors: int | None = None,
    min_mass: float = 1,
    cutoff_percentile: float = 5,
    adjust_for_stream: bool = True,
) -> VelocityGrid:
    """Average cell velocities onto a regular grid, as scVelo's ``compute_velocity_on_grid``.

    Each grid point takes the Gaussian-weighted mean velocity of its
    ``n_neighbors`` nearest cells (exact Euclidean neighbours). The grid spans the
    points' range widened by 1 percent on each side. With ``adjust_for_stream``
    (the default), grid points with almost no velocity, or whose neighbours move
    slowest (below the ``cutoff_percentile``), are hidden, as scVelo prepares the
    field for a stream plot. Pass the result to ``streamplot()``.

    Parameters
    ----------
    points : (n, 2) array
        Cell coordinates in the embedding (e.g. UMAP), one ``(x, y)`` row per cell:
        a numpy array, nested sequences or a two-column data frame.
        Rows with a non-finite coordinate or velocity (or a masked entry) are dropped.
    vectors : (n, 2) array
        Cell velocities in the embedding, one ``(dx, dy)`` row per cell.
    density : float, default=1
        Grid points per axis: ``int(50 * density)``, from 2 to 3162 (``density``
        from 0.04 to about 63).
    smooth : float, default=0.5
        Width of the Gaussian kernel, in units of the mean grid step.
    n_neighbors : int, optional
        Cells averaged per grid point. By default ``int(n_cells / 50)``. From
        ``n_cells // 2`` on, scikit-learn (and so scVelo) switches to less accurate
        brute-force distances; ggrust keeps exact ones, so the results differ slightly.
    min_mass : float, default=1
        Hide grid points whose velocity is below ``10 ** (min_mass - 6)`` (at
        most 90 percent of the largest). With ``adjust_for_stream=False``, hide
        those whose kernel mass is at most ``min_mass`` percent of the 99th
        percentile mass, as scVelo selects quiver arrows.
    cutoff_percentile : float, default=5
        Hide grid points whose neighbours' mean absolute velocity is below this
        percentile of all grid points (``adjust_for_stream`` only).
    adjust_for_stream : bool, default=True
        Hide points as scVelo does for stream plots (``True``) or for quiver
        arrows (``False``; scVelo's ``autoscale`` is not applied).

    Returns
    -------
    VelocityGrid
        Grid ``x`` and ``y`` and velocity components ``u`` and ``v``.

    Raises
    ------
    ggrust.errors.SpecError
        If ``points`` or ``vectors`` is not an ``(n, 2)`` array of numbers, they
        differ in length, the points have no spread on an axis (streams) or on
        both axes, ``n_neighbors``
        is not between 1 and the number of finite points (the default needs at
        least 50 points), or an option is out of range.

    Examples
    --------
    >>> import math
    >>> from ggrust import *
    >>> points = [[math.cos(i / 20) * (1 + i / 400), math.sin(i / 20) * (1 + i / 400)]
    ...           for i in range(400)]
    >>> vectors = [[-y, x] for x, y in points]
    >>> grid = velocity_grid(points, vectors, density=0.6)
    >>> streams = streamplot(grid.x, grid.y, grid.u, grid.v)
    >>> plot = (
    ...     ggplot()
    ...     + geom_path(aes("x", "y", group="group"), data=streams.lines)
    ...     + geom_segment(
    ...         aes("x", "y", xend="xend", yend="yend"), data=streams.arrows, arrow=arrow()
    ...     )
    ...     + coord_fixed()
    ... )
    """
    options = (
        _number(density, "density"),
        _number(smooth, "smooth"),
        _neighbors(n_neighbors),
        _number(min_mass, "min_mass"),
        _number(cutoff_percentile, "cutoff_percentile"),
        bool(adjust_for_stream),
    )
    point_rows = _to_list(points)
    vector_rows = _to_list(vectors)
    try:
        x, y, u, v = _velocity_grid(point_rows, vector_rows, *options)
    except SpecError:
        raise
    except (TypeError, ValueError, OverflowError):
        # Not plain rows of two numbers: find the culprit (masked entries become NaN).
        x, y, u, v = _velocity_grid(
            _pairs(point_rows, "points"), _pairs(vector_rows, "vectors"), *options
        )
    return VelocityGrid(x=x, y=y, u=u, v=v)


def _spec_error(path: str, reason: str) -> SpecError:
    """Build a ``SpecError`` for argument ``path`` of ``velocity_grid()``."""
    return SpecError(
        f"invalid velocity_grid argument at '{path}': {reason}", path=path, reason=reason
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


def _neighbors(value: object) -> int | None:
    """Return ``n_neighbors`` as an int (or ``None``), raising if it is not a positive int."""
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, Integral) or not 1 <= int(value) < 2**63:
        raise _spec_error("n_neighbors", f"expected a positive integer or None, got {value!r}")
    return int(value)


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
