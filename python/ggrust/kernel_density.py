"""One-dimensional Gaussian kernel densities at given points, computed in Rust.

Not in lets-plot. ``kernel_density()`` evaluates the density of a set of values at
points the caller picks, as ``scipy.stats.gaussian_kde(values)(at)`` (Scott's or
Silverman's rule, or a bandwidth factor), so custom density shapes such as violins
drawn by hand need no scipy. ``stat_density`` and ``geom_violin`` keep lets-plot's
bandwidths; this function is for densities built outside a layer.
"""

from __future__ import annotations

import math
from collections.abc import Sequence
from numbers import Real
from typing import Literal, Protocol, TypeAlias

import polars as pl

from ggrust._ggrust import kernel_density as _kernel_density
from ggrust.errors import SpecError

__all__ = ["kernel_density"]


class _ArrayLike(Protocol):
    """Anything with ``tolist()``, such as a numpy array."""

    def tolist(self) -> object: ...


Values: TypeAlias = Sequence[float] | _ArrayLike | pl.Series
"""A one-dimensional set of numbers: a sequence, a numpy array or a polars Series."""

Bandwidth: TypeAlias = Literal["scott", "silverman"] | float
"""A bandwidth rule name, or a factor that multiplies the data standard deviation."""


def kernel_density(
    values: Values,
    at: Values,
    *,
    bandwidth: Bandwidth = "scott",
) -> pl.DataFrame:
    """Evaluate the Gaussian kernel density of ``values`` at the points ``at``.

    The kernel is a Gaussian whose standard deviation is the data standard
    deviation (unbiased) times the ``bandwidth`` factor, and the density is the
    mean of the kernels centered on the values, as ``scipy.stats.gaussian_kde``.
    The result matches ``gaussian_kde(values, bw_method=bandwidth)(at)`` to about
    1e-14 of the peak density for values centered within a few standard deviations
    of zero. Values offset from zero by ``k`` standard deviations lose about ``k``
    units in the last place in both scipy and ggrust (rounding of the scaled
    values), so they agree only to about ``k * 1e-15`` there. Use it to build
    density shapes outside a layer, for example violins placed by hand;
    ``stat_density`` and ``geom_violin`` follow lets-plot's bandwidths instead.

    Parameters
    ----------
    values : sequence of float, numpy array or polars Series
        The sample, one dimension (booleans count as 0 and 1). Non-finite and
        missing values are dropped; at
        least 2 must remain, and they must not all be equal.
    at : sequence of float, numpy array or polars Series
        Points to evaluate the density at, one dimension, in any order. A
        non-finite point gives a NaN density.
    bandwidth : {'scott', 'silverman'} or float, default='scott'
        Kernel bandwidth: Scott's or Silverman's rule (scipy's ``bw_method``), or a
        positive factor that multiplies the data standard deviation.

    Returns
    -------
    polars.DataFrame
        One row per point of ``at``, in its order: ``x`` (the point) and
        ``density`` (Float64). The density integrates to 1 over the real line.

    Raises
    ------
    ggrust.errors.SpecError
        If ``values`` or ``at`` is not a one-dimensional set of numbers, fewer than
        2 finite values remain, they are all equal, or ``bandwidth`` is invalid.

    Examples
    --------
    >>> from ggrust import *
    >>> waiting = [54, 55, 56, 57, 58, 60, 62, 75, 76, 78, 79, 80, 81, 82, 83, 85, 88]
    >>> grid = [40 + i for i in range(61)]
    >>> density = kernel_density(waiting, grid)
    >>> density.columns
    ['x', 'density']
    >>> density["x"][density["density"].arg_max()]
    80.0
    >>> plot = ggplot(density, aes("x", "density")) + geom_area(alpha=0.4) + geom_line()
    """
    factor = _bandwidth(bandwidth)
    value_list = _to_list(values)
    point_list = _to_list(at)
    try:
        x, density = _kernel_density(value_list, point_list, factor)
    except SpecError:
        raise
    except (TypeError, ValueError, OverflowError):
        # Not plain sequences of numbers: find the culprit (missing entries become NaN).
        x, density = _kernel_density(
            _numbers(value_list, "values"), _numbers(point_list, "at"), factor
        )
    return pl.DataFrame(
        {"x": x, "density": density},
        schema={"x": pl.Float64, "density": pl.Float64},
    )


def _spec_error(path: str, reason: str) -> SpecError:
    """Build a ``SpecError`` for argument ``path`` of ``kernel_density()``."""
    return SpecError(
        f"invalid kernel_density argument at '{path}': {reason}", path=path, reason=reason
    )


def _to_list(values: object) -> object:
    """Return numpy-like arrays (``tolist()``) and Series (``to_list()``) as lists.

    Other values are returned as they are.
    """
    for method in ("tolist", "to_list"):
        convert = getattr(values, method, None)
        if callable(convert):
            return convert()
    return values


def _bandwidth(value: object) -> str | float:
    """Return ``bandwidth`` as a rule name or a positive float, raising otherwise."""
    if isinstance(value, str):
        if value in ("scott", "silverman"):
            return value
        raise _spec_error(
            "bandwidth", f"unknown bandwidth rule {value!r}; pass 'scott', 'silverman' or a number"
        )
    if isinstance(value, bool) or not isinstance(value, Real):
        raise _spec_error(
            "bandwidth",
            f"expected 'scott', 'silverman' or a number, got {type(value).__name__} {value!r}",
        )
    try:
        factor = float(value)
    except OverflowError:
        raise _spec_error("bandwidth", f"{value!r} is too large for a float") from None
    if not (math.isfinite(factor) and factor > 0):
        raise _spec_error(
            "bandwidth", f"a bandwidth factor must be a positive finite number, got {value!r}"
        )
    return factor


def _numbers(values: object, path: str) -> list[float]:
    """Return one-dimensional ``values`` as floats; ``None`` (a missing entry) becomes NaN."""
    if not isinstance(values, Sequence) or isinstance(values, str):
        raise _spec_error(
            path, f"expected a one-dimensional sequence of numbers, got {type(values).__name__}"
        )
    numbers = []
    for index, value in enumerate(values):
        if value is None:
            numbers.append(math.nan)
        elif isinstance(value, Real):  # bools count as 0 and 1, as in numpy
            try:
                numbers.append(float(value))
            except OverflowError:
                raise _spec_error(
                    path, f"entry {index} is {value!r}, too large for a float"
                ) from None
        else:
            raise _spec_error(
                path,
                f"entry {index} is {type(value).__name__} {value!r}; expected numbers "
                "(pass a one-dimensional array)",
            )
    return numbers
