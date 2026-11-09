"""Layer sampling: ``sampling_random``, ``sampling_systematic``, ``sampling_fraction``, ...

ggrust draws every row by default (lets-plot samples large layers silently). Pass
``sampling=`` to a layer to keep fewer rows: a ``sampling_*()`` spec, several joined
with ``+`` (applied in order), ``"none"``, or an integer ``n`` as a shortcut for
``sampling_random(n)``. Sampling runs on each facet panel's stat output, as in
lets-plot, so ``geom_histogram`` samples its bins, not the input rows. The engine
picks the rows (its own random generator, so the rows differ from lets-plot's).

The specs of ``sampling_random``, ``sampling_systematic`` and the lets_plot samplers
ggrust does not apply (``sampling_random_stratified``, ``sampling_pick``,
``sampling_group_random``, ``sampling_group_systematic``, ``sampling_vertex_vw``,
``sampling_vertex_dp``) are lets_plot-identical. Those six are built so ported code
runs; rendering then warns (``GgrustWarning``) and skips them.
"""

from __future__ import annotations

import math
from numbers import Integral, Real
from typing import Literal

from ggrust.errors import SpecError, UnsupportedValueError
from ggrust.plot.core import FeatureSpec

__all__ = [
    "sampling_fraction",
    "sampling_group_random",
    "sampling_group_systematic",
    "sampling_none",
    "sampling_pick",
    "sampling_random",
    "sampling_random_stratified",
    "sampling_systematic",
    "sampling_vertex_dp",
    "sampling_vertex_vw",
]


def sampling_none() -> Literal["none"]:
    """Draw every row (no sampling).

    ggrust never samples by default, so this only states the default. It returns
    ``"none"``, lets_plot's spelling (``sampling="none"``), so the spec is the same.

    Returns
    -------
    str
        ``"none"``.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3], "y": [3, 1, 2]}
    >>> plot = ggplot(data, aes("x", "y")) + geom_point(sampling=sampling_none())
    """
    return "none"


def sampling_random(n: int, seed: int | None = None) -> FeatureSpec:
    """Keep ``n`` rows chosen at random.

    The kept rows keep their order. With the same ``seed`` the same rows are kept;
    without one a fixed default seed is used, so renders are reproducible. The rows
    are not the ones lets-plot picks. A layer with ``n`` rows or fewer is unchanged.

    Parameters
    ----------
    n : int
        Number of rows to keep, at least 1.
    seed : int, optional
        Seed of the random generator.

    Returns
    -------
    FeatureSpec
        The sampling, for a layer's ``sampling=``.

    Raises
    ------
    SpecError
        When ``n`` is below 1.
    UnsupportedValueError
        When ``n`` or ``seed`` is not an integer.

    Examples
    --------
    >>> import numpy as np
    >>> from ggrust import *
    >>> rng = np.random.default_rng(1)
    >>> data = {"x": rng.normal(size=10_000), "y": rng.normal(size=10_000)}
    >>> plot = ggplot(data, aes("x", "y")) + geom_point(
    ...     sampling=sampling_random(500, seed=42)
    ... )
    """
    return _sampling("random", n=_count(n, "sampling_random"), seed=_seed(seed, "sampling_random"))


def sampling_systematic(n: int) -> FeatureSpec:
    """Keep every k-th row, the smallest k that leaves at most ``n`` rows.

    The first row is always kept, then every k-th row after it
    (``k = ceil(rows / n)``). A layer with ``n`` rows or fewer is unchanged.

    Parameters
    ----------
    n : int
        Largest number of rows to keep, at least 1.

    Returns
    -------
    FeatureSpec
        The sampling, for a layer's ``sampling=``.

    Raises
    ------
    SpecError
        When ``n`` is below 1.
    UnsupportedValueError
        When ``n`` is not an integer.

    Examples
    --------
    >>> import numpy as np
    >>> from ggrust import *
    >>> days = np.arange(100_000)
    >>> data = {"day": days, "temperature": np.sin(days / 5_000) * 10 + 15}
    >>> plot = ggplot(data, aes("day", "temperature")) + geom_line(
    ...     sampling=sampling_systematic(2_000)
    ... )
    """
    return _sampling("systematic", n=_count(n, "sampling_systematic"))


def sampling_fraction(fraction: float, seed: int | None = None) -> FeatureSpec:
    """Keep a random share of the rows (ggrust extension, not in lets_plot).

    ``fraction=0.2`` keeps 20 percent of the rows (rounded, at least one), chosen
    as ``sampling_random`` chooses them.

    Parameters
    ----------
    fraction : float
        Share of the rows to keep, greater than 0 and at most 1.
    seed : int, optional
        Seed of the random generator.

    Returns
    -------
    FeatureSpec
        The sampling, for a layer's ``sampling=``.

    Raises
    ------
    SpecError
        When ``fraction`` is not in (0, 1] (``20`` suggests ``0.2``).
    UnsupportedValueError
        When ``fraction`` is not a number or ``seed`` not an integer.

    Examples
    --------
    >>> import numpy as np
    >>> from ggrust import *
    >>> rng = np.random.default_rng(1)
    >>> data = {"x": rng.normal(size=10_000), "y": rng.normal(size=10_000)}
    >>> plot = ggplot(data, aes("x", "y")) + geom_point(sampling=sampling_fraction(0.1))
    """
    function = "sampling_fraction"
    if isinstance(fraction, bool) or not isinstance(fraction, Real):
        raise UnsupportedValueError(
            f"{function}(fraction={fraction!r}): {type(fraction).__name__} is not accepted; "
            "use a number in (0, 1], such as 0.2 for 20%",
            path="sampling.fraction",
        )
    value = float(fraction)
    if not (math.isfinite(value) and 0 < value <= 1):
        hint = (
            f"for {fraction}% use fraction={value / 100:g}"
            if 1 < value <= 100
            else "such as 0.2 for 20%"
        )
        raise SpecError(
            f"{function}(fraction={fraction!r}): fraction is a share of the rows in (0, 1]; {hint}",
            path="sampling.fraction",
        )
    return _sampling("fraction", fraction=value, seed=_seed(seed, function))


def sampling_random_stratified(
    n: int, seed: int | None = None, min_subsample: int | None = None
) -> FeatureSpec:
    """lets_plot's stratified random sampling. Not applied by ggrust.

    Built as lets_plot builds it, so ported code runs; rendering warns
    (``GgrustWarning``) and draws every row. Use ``sampling_random`` instead.

    Parameters
    ----------
    n : int
        Number of rows to keep.
    seed : int, optional
        Seed of the random generator.
    min_subsample : int, optional
        Smallest sample per group.

    Returns
    -------
    FeatureSpec
        The lets_plot spec.

    Examples
    --------
    >>> from ggrust import *
    >>> sampling_random_stratified(100, seed=1).as_dict()
    {'name': 'random_stratified', 'n': 100, 'seed': 1}
    """
    return _sampling("random_stratified", n=n, seed=seed, min_subsample=min_subsample)


def sampling_pick(n: int) -> FeatureSpec:
    """lets_plot's first-n sampling. Not applied by ggrust.

    Built as lets_plot builds it, so ported code runs; rendering warns
    (``GgrustWarning``) and draws every row.

    Parameters
    ----------
    n : int
        Number of rows to keep.

    Returns
    -------
    FeatureSpec
        The lets_plot spec.

    Examples
    --------
    >>> from ggrust import *
    >>> sampling_pick(100).as_dict()
    {'name': 'pick', 'n': 100}
    """
    return _sampling("pick", n=n)


def sampling_group_random(n: int, seed: int | None = None) -> FeatureSpec:
    """lets_plot's random sampling of whole groups. Not applied by ggrust.

    Built as lets_plot builds it, so ported code runs; rendering warns
    (``GgrustWarning``) and draws every row.

    Parameters
    ----------
    n : int
        Number of groups to keep.
    seed : int, optional
        Seed of the random generator.

    Returns
    -------
    FeatureSpec
        The lets_plot spec.

    Examples
    --------
    >>> from ggrust import *
    >>> sampling_group_random(3, seed=1).as_dict()
    {'name': 'group_random', 'n': 3, 'seed': 1}
    """
    return _sampling("group_random", n=n, seed=seed)


def sampling_group_systematic(n: int) -> FeatureSpec:
    """lets_plot's systematic sampling of whole groups. Not applied by ggrust.

    Built as lets_plot builds it, so ported code runs; rendering warns
    (``GgrustWarning``) and draws every row.

    Parameters
    ----------
    n : int
        Number of groups to keep.

    Returns
    -------
    FeatureSpec
        The lets_plot spec.

    Examples
    --------
    >>> from ggrust import *
    >>> sampling_group_systematic(3).as_dict()
    {'name': 'group_systematic', 'n': 3}
    """
    return _sampling("group_systematic", n=n)


def sampling_vertex_vw(n: int, polygon: bool | None = None) -> FeatureSpec:
    """lets_plot's Visvalingam-Whyatt path simplification. Not applied by ggrust.

    Built as lets_plot builds it, so ported code runs; rendering warns
    (``GgrustWarning``) and draws every vertex.

    Parameters
    ----------
    n : int
        Number of vertices to keep.
    polygon : bool, optional
        Whether the paths are polygon rings.

    Returns
    -------
    FeatureSpec
        The lets_plot spec.

    Examples
    --------
    >>> from ggrust import *
    >>> sampling_vertex_vw(500).as_dict()
    {'name': 'vertex_vw', 'n': 500}
    """
    return _sampling("vertex_vw", n=n, polygon=polygon)


def sampling_vertex_dp(n: int, polygon: bool | None = None) -> FeatureSpec:
    """lets_plot's Douglas-Peucker path simplification. Not applied by ggrust.

    Built as lets_plot builds it, so ported code runs; rendering warns
    (``GgrustWarning``) and draws every vertex.

    Parameters
    ----------
    n : int
        Number of vertices to keep.
    polygon : bool, optional
        Whether the paths are polygon rings.

    Returns
    -------
    FeatureSpec
        The lets_plot spec.

    Examples
    --------
    >>> from ggrust import *
    >>> sampling_vertex_dp(500).as_dict()
    {'name': 'vertex_dp', 'n': 500}
    """
    return _sampling("vertex_dp", n=n, polygon=polygon)


def resolve_sampling(sampling: object) -> str | FeatureSpec | None:
    """Return a layer's ``sampling=`` as the spec stores it, the int shortcut expanded.

    Parameters
    ----------
    sampling : object
        The layer option, as given: ``"none"``, a ``sampling_*()`` spec, or an integer ``n``
        meaning ``sampling_random(n)``.

    Returns
    -------
    str, FeatureSpec or None
        ``sampling`` with an integer replaced by ``sampling_random(n)``.

    Raises
    ------
    SpecError
        When ``sampling`` is a string other than ``"none"`` or an integer below 1.
    UnsupportedValueError
        When ``sampling`` is of another type (a float suggests ``sampling_fraction``).
    """
    if sampling is None or isinstance(sampling, FeatureSpec):
        return sampling
    accepted = 'use "none", an integer n (sampling_random(n)) or a sampling_*() spec'
    if isinstance(sampling, str):
        if sampling != "none":
            raise SpecError(f"sampling={sampling!r} is not accepted; {accepted}", path="sampling")
        return sampling
    if isinstance(sampling, Integral) and not isinstance(sampling, bool):
        return sampling_random(int(sampling))
    if isinstance(sampling, Real) and not isinstance(sampling, bool):
        raise UnsupportedValueError(
            f"sampling={sampling!r}: a float is not accepted; for a share of the rows use "
            f"sampling_fraction({sampling!r}), for a row count an integer",
            path="sampling",
        )
    raise UnsupportedValueError(
        f"sampling={sampling!r}: {type(sampling).__name__} is not accepted; {accepted}",
        path="sampling",
    )


def _sampling(name: str, **options: object) -> FeatureSpec:
    """Build a ``sampling`` spec; ``None`` options are left out of ``as_dict()``."""
    return FeatureSpec("sampling", name, **options)


def _count(n: object, function: str) -> int:
    """``n`` as an int >= 1, or raise."""
    if isinstance(n, bool) or not isinstance(n, Integral):
        raise UnsupportedValueError(
            f"{function}(n={n!r}): {type(n).__name__} is not accepted; "
            "use an integer number of rows >= 1",
            path="sampling.n",
        )
    if n < 1:
        raise SpecError(
            f"{function}(n={n!r}): n is the number of rows to keep; use an integer >= 1",
            path="sampling.n",
        )
    return int(n)


def _seed(seed: object, function: str) -> int | None:
    """``seed`` as an int or None, or raise."""
    if seed is None:
        return None
    if isinstance(seed, bool) or not isinstance(seed, Integral):
        raise UnsupportedValueError(
            f"{function}(seed={seed!r}): {type(seed).__name__} is not accepted; "
            "use an integer seed or leave it out",
            path="sampling.seed",
        )
    return int(seed)
