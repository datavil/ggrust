"""Streamlines of a 2D vector field, computed in Rust (matplotlib's ``streamplot``).

Not in lets-plot. ``streamplot()`` integrates the same lines and arrows as
``matplotlib.pyplot.streamplot`` without matplotlib, and returns them as polars
frames to draw with ``geom_path`` and ``geom_segment(arrow=arrow())``.
"""

from __future__ import annotations

import math
from collections.abc import Sequence
from dataclasses import dataclass
from numbers import Real
from typing import Literal, Protocol, TypeAlias

import polars as pl

from ggrust._ggrust import streamplot as _streamplot
from ggrust.errors import SpecError

__all__ = ["StreamplotSet", "streamplot"]


class _ArrayLike(Protocol):
    """Anything with ``tolist()``, such as a numpy array."""

    def tolist(self) -> object: ...


Vector: TypeAlias = Sequence[float] | _ArrayLike
"""One-dimensional values: a sequence or a numpy array."""

Matrix: TypeAlias = Sequence[Sequence[float]] | _ArrayLike
"""Two-dimensional values (rows of columns): nested sequences or a numpy array."""

IntegrationDirection: TypeAlias = Literal["forward", "backward", "both"]


@dataclass(frozen=True, eq=False)
class StreamplotSet:
    """Streamlines and their arrows, as matplotlib's ``StreamplotSet``.

    Attributes
    ----------
    lines : pl.DataFrame
        Columns ``x``, ``y`` and ``group``: the vertices of each line in order,
        one ``group`` per line (draw with ``geom_path(aes(group="group"))``).
    arrows : pl.DataFrame
        Columns ``x``, ``y``, ``xend``, ``yend`` and ``group``: one short segment
        per line at half its arc length, pointing along the flow (draw with
        ``geom_segment(arrow=arrow())``).

    Examples
    --------
    >>> from ggrust import *
    >>> x = [0, 1, 2, 3]
    >>> y = [0, 1, 2]
    >>> streams = streamplot(x, y, [[1.0] * 4] * 3, [[0.5] * 4] * 3)
    >>> streams.lines.columns
    ['x', 'y', 'group']
    >>> streams.arrows.columns
    ['x', 'y', 'xend', 'yend', 'group']
    """

    lines: pl.DataFrame
    arrows: pl.DataFrame


def streamplot(
    x: Vector | Matrix,
    y: Vector | Matrix,
    u: Matrix,
    v: Matrix,
    *,
    density: float | tuple[float, float] = 1,
    minlength: float = 0.1,
    maxlength: float = 4.0,
    integration_direction: IntegrationDirection = "both",
    broken_streamlines: bool = True,
    start_points: Matrix | None = None,
) -> StreamplotSet:
    """Streamlines of the vector field ``(u, v)``, as ``matplotlib.pyplot.streamplot``.

    A line-by-line port of matplotlib's integrator (adaptive Heun steps, a
    ``30 * density`` occupancy mask, seeds spiralling in from the boundary), so
    the lines and arrows match matplotlib's for the same input.

    Parameters
    ----------
    x, y : sequence of float or 2D array
        Evenly spaced, strictly increasing grid coordinates. 2D arrays as from
        ``numpy.meshgrid`` are accepted when every row of ``x`` (column of ``y``)
        is the same.
    u, v : 2D array
        x and y components of the field, with one row per ``y`` value and one
        column per ``x`` value. Non-finite values and masked entries (numpy masked
        arrays) are missing: lines stop there.
    density : float or tuple of float, default=1
        Closeness of the lines. ``density=1`` divides the domain into a 30x30
        mask, each cell crossed by at most one line. A tuple sets x and y apart.
    minlength : float, default=0.1
        Lines shorter than this (in axes coordinates, the domain being 1 x 1)
        are dropped.
    maxlength : float, default=4.0
        Longest line, in axes coordinates.
    integration_direction : {"both", "forward", "backward"}, default="both"
        Integrate along the field, against it, or both (each half as long).
    broken_streamlines : bool, default=True
        Stop a line when it comes too close to another. ``False`` lets lines run on.
    start_points : 2D array, optional
        ``(x, y)`` seed points in data coordinates, inside the grid. By default
        seeds spiral in from the boundary of the mask.

    Returns
    -------
    StreamplotSet
        ``lines`` (``x``, ``y``, ``group``) and ``arrows`` (``x``, ``y``, ``xend``,
        ``yend``, ``group``) frames.

    Raises
    ------
    ggrust.errors.SpecError
        If the grid is not evenly spaced and increasing, ``u`` or ``v`` does not
        match it, ``density`` is too small, ``integration_direction`` is unknown,
        or a start point lies outside the grid.

    Examples
    --------
    >>> from ggrust import *
    >>> x = [i / 10 for i in range(-20, 21)]
    >>> y = x
    >>> u = [[-yi for xi in x] for yi in y]
    >>> v = [[xi for xi in x] for yi in y]
    >>> streams = streamplot(x, y, u, v, density=1.5)
    >>> plot = (
    ...     ggplot()
    ...     + geom_path(aes("x", "y", group="group"), data=streams.lines)
    ...     + geom_segment(
    ...         aes("x", "y", xend="xend", yend="yend"), data=streams.arrows, arrow=arrow()
    ...     )
    ...     + coord_fixed()
    ... )
    """
    if integration_direction not in ("both", "forward", "backward"):
        raise _spec_error(
            "integration_direction",
            f"unknown integration_direction {integration_direction!r}; "
            "accepted: 'both', 'forward', 'backward'",
        )
    density_pair = _density(density)
    x_values = _grid_axis(x, "x")
    y_values = _grid_axis(y, "y")
    points = (
        None
        if start_points is None
        else [_point(point) for point in _rows(start_points, "start_points")]
    )
    (line_x, line_y, line_group), arrows = _streamplot(
        x_values,
        y_values,
        _rows(u, "u", missing=True),
        _rows(v, "v", missing=True),
        density_pair,
        float(minlength),
        float(maxlength),
        integration_direction,
        bool(broken_streamlines),
        points,
    )
    lines = pl.DataFrame(
        {"x": line_x, "y": line_y, "group": line_group},
        schema={"x": pl.Float64, "y": pl.Float64, "group": pl.UInt32},
    )
    arrow_frame = pl.DataFrame(
        dict(zip(("x", "y", "xend", "yend", "group"), arrows, strict=True)),
        schema={
            "x": pl.Float64,
            "y": pl.Float64,
            "xend": pl.Float64,
            "yend": pl.Float64,
            "group": pl.UInt32,
        },
    )
    return StreamplotSet(lines=lines, arrows=arrow_frame)


def _spec_error(path: str, reason: str) -> SpecError:
    """Build a ``SpecError`` for argument ``path`` of ``streamplot()``."""
    location = f" at '{path}'" if path else ""
    return SpecError(f"invalid streamplot argument{location}: {reason}", path=path, reason=reason)


def _to_list(values: object) -> object:
    """Return ``values.tolist()`` for numpy-like arrays, the values themselves otherwise."""
    tolist = getattr(values, "tolist", None)
    return tolist() if callable(tolist) else values


def _sequence(values: object, path: str, what: str) -> Sequence[object]:
    """Return ``values`` (or its ``tolist()``) as a sequence, raising if it is not one."""
    values = _to_list(values)
    if not isinstance(values, Sequence) or isinstance(values, str):
        raise _spec_error(path, f"expected {what}, got {type(values).__name__}")
    return values


def _float(value: object, path: str) -> float:
    """Return ``value`` as a float, raising if it is not a real number."""
    if not isinstance(value, Real):
        raise _spec_error(path, f"expected numbers, got {type(value).__name__} {value!r}")
    return float(value)


def _rows(values: object, path: str, *, missing: bool = False) -> list[list[float]]:
    """Return two-dimensional values as a list of float rows.

    With ``missing``, ``None`` (a masked entry of a numpy masked array) becomes NaN.
    """
    return [
        [
            math.nan if missing and value is None else _float(value, path)
            for value in _sequence(row, path, "a 2D array")
        ]
        for row in _sequence(values, path, "a 2D array")
    ]


def _point(point: list[float]) -> tuple[float, float]:
    """Return a start point as ``(x, y)``."""
    if len(point) != 2:
        raise _spec_error(
            "start_points", f"each start point needs 2 values (x, y), got {len(point)}"
        )
    return float(point[0]), float(point[1])


def _density(density: float | tuple[float, float]) -> tuple[float, float]:
    """Return ``density`` as an ``(x, y)`` pair."""
    if isinstance(density, Real):
        return float(density), float(density)
    values = _sequence(density, "density", "a number or 2 numbers")
    if len(values) != 2:
        raise _spec_error("density", f"density must be a number or 2 numbers, got {density!r}")
    return _float(values[0], "density"), _float(values[1], "density")


def _grid_axis(values: object, name: Literal["x", "y"]) -> list[float]:
    """Return 1D grid coordinates from a 1D or a meshgrid-style 2D ``x`` or ``y``."""
    sequence = _sequence(values, name, "1D or 2D grid values")
    if not sequence or isinstance(sequence[0], Real):
        return [_float(value, name) for value in sequence]
    rows = _rows(sequence, name)
    # x varies along rows (every row the same), y along columns (every column the same).
    lines = rows if name == "x" else [list(column) for column in zip(*rows, strict=True)]
    first = lines[0]
    for line in lines[1:]:
        # np.allclose: rtol 1e-5, atol 1e-8.
        if len(line) != len(first) or any(
            abs(a - b) > 1e-8 + 1e-5 * abs(b) for a, b in zip(line, first, strict=True)
        ):
            what = "rows" if name == "x" else "columns"
            raise _spec_error(name, f"the {what} of a 2D '{name}' must be equal (as from meshgrid)")
    return first
