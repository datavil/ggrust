"""Beeswarm and quasirandom point layers (R package ggbeeswarm).

Not in lets-plot; ggplot2 needs the ggbeeswarm package. The layouts follow
ggbeeswarm (and the beeswarm and vipor packages it uses); the code is ggrust's own.
Both layers are point layers whose stat (``"beeswarm"``, ``"quasirandom"``) moves
each point sideways from its x, so points of one x show their distribution
without overlapping.
"""

from __future__ import annotations

from numbers import Integral, Real
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from ggrust.errors import SpecError, UnsupportedValueError, did_you_mean
from ggrust.plot.core import FeatureSpec
from ggrust.plot.geom import _geom

if TYPE_CHECKING:
    from ggrust._typing import ManualKey, Number, PlotData, Position, Sampling, Tooltips
    from ggrust.plot.core import LayerSpec

__all__ = ["geom_beeswarm", "geom_quasirandom"]

BeeswarmMethod: TypeAlias = Literal["swarm", "compactswarm", "center", "square", "hex"]
"""Layouts of ``geom_beeswarm``."""

Priority: TypeAlias = Literal["ascending", "descending", "density", "random", "none"]
"""Order in which ``geom_beeswarm`` places points."""

QuasirandomMethod: TypeAlias = Literal[
    "quasirandom", "pseudorandom", "smiley", "maxout", "frowney", "minout"
]
"""Spreads of ``geom_quasirandom``."""

_BEESWARM_METHODS: tuple[str, ...] = ("swarm", "compactswarm", "center", "square", "hex")
_PRIORITIES: tuple[str, ...] = ("ascending", "descending", "density", "random", "none")
_QUASIRANDOM_METHODS: tuple[str, ...] = (
    "quasirandom",
    "pseudorandom",
    "smiley",
    "maxout",
    "frowney",
    "minout",
)

# Dodge width of position_dodge() without a width: the violin and boxplot default,
# so swarms line up with dodged violins.
_DEFAULT_DODGE_WIDTH = 0.95


def geom_beeswarm(
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
    orientation: Literal["x", "y"] | None = None,
    method: BeeswarmMethod | None = None,
    cex: Number | None = None,
    side: Literal[-1, 0, 1] | None = None,
    priority: Priority | None = None,
    dodge_width: Number | None = None,
    seed: int | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Beeswarm plot: points of each x moved sideways so that none overlap.

    As ``geom_beeswarm`` in the R package ggbeeswarm. A point is 1/100 of each
    axis range across (times ``cex``; a discrete axis counts its levels), so the
    layout depends on the data ranges, not on the plot size.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Needs ``x`` and ``y``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name; ``"beeswarm"`` by default.
    position : str or FeatureSpec, optional
        Position adjustment. ``"dodge"`` or ``position_dodge(width)`` sets
        ``dodge_width`` (0.95 without a width); others apply after the swarm.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        This layer's key in a manual legend: a label, or ``layer_key()``.
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"``.
    orientation : {"x", "y"}, optional
        ``"y"`` swarms along the y axis; a discrete y with a continuous x picks it
        by default.
    method : {"swarm", "compactswarm", "center", "square", "hex"}, default="swarm"
        ``"swarm"`` places each point (in ``priority`` order) at the free spot
        nearest the centre; ``"compactswarm"`` places the point that fits nearest
        the centre first, for tighter swarms. ``"center"``, ``"square"`` and
        ``"hex"`` bin y into rows one point high (moving points to the row middle)
        and lay each row out centred, on a square grid or on a hexagonal grid.
    cex : float, default=1
        Point spacing, as a multiple of the default.
    side : {-1, 0, 1}, default=0
        ``1`` swarms to the right only, ``-1`` to the left only, ``0`` both ways.
    priority : {"ascending", "descending", "density", "random", "none"}, default="ascending"
        Order of placement for the swarm methods: by y, by density (densest
        first), random (seeded), or data order.
    dodge_width : float, optional
        Dodge groups at one x side by side, ``position_dodge(width)`` style, and
        swarm each group on its own. Without it, every group at an x shares one swarm.
    seed : int, optional
        Random seed of ``priority="random"``.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics (``color``, ``size``, ``shape``, ...).

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If ``method``, ``priority``, ``side``, ``cex`` or ``dodge_width`` is not
        one of the accepted values.
    UnsupportedValueError
        If a numeric parameter is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"day": ["Mon"] * 8 + ["Tue"] * 8,
    ...         "minutes": [31, 35, 35, 36, 38, 40, 41, 47, 25, 28, 30, 30, 30, 33, 37, 45]}
    >>> plot = ggplot(data, aes("day", "minutes")) + geom_beeswarm(cex=3)
    """
    function = "geom_beeswarm"
    _check_choice(function, "method", method, _BEESWARM_METHODS)
    _check_choice(function, "priority", priority, _PRIORITIES)
    if side is not None and (isinstance(side, bool) or side not in (-1, 0, 1)):
        reason = f"side must be -1 (left), 0 (both sides) or 1 (right), got {side!r}"
        raise SpecError(f"{function}(side={side!r}): {reason}", path="side", reason=reason)
    _check_positive(function, "cex", cex)
    _check_seed(function, seed)
    position, dodge_width = _dodge_from_position(function, position, dodge_width)
    return _geom(
        "beeswarm",
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
        method=method,
        cex=cex,
        side=side,
        priority=priority,
        dodge_width=dodge_width,
        seed=seed,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_quasirandom(
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
    orientation: Literal["x", "y"] | None = None,
    method: QuasirandomMethod | None = None,
    width: Number | None = None,
    varwidth: bool | None = None,
    bandwidth: Number | None = None,
    nbins: int | None = None,
    dodge_width: Number | None = None,
    seed: int | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Quasirandom plot: points spread sideways within the density of their x.

    As ``geom_quasirandom`` in the R package ggbeeswarm: like a violin made of
    points. Each point moves by up to ``width`` times the density at its y
    (scaled to a maximum of 1), along a van der Corput sequence by rank, so the
    layout is deterministic and evenly spread.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Needs ``x`` and ``y``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name; ``"quasirandom"`` by default.
    position : str or FeatureSpec, optional
        Position adjustment. ``"dodge"`` or ``position_dodge(width)`` sets
        ``dodge_width`` (0.95 without a width); others apply after the spread.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        This layer's key in a manual legend: a label, or ``layer_key()``.
    sampling : str or FeatureSpec, optional
        lets_plot sampling; accepted and ignored, as ggrust draws every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"``.
    orientation : {"x", "y"}, optional
        ``"y"`` spreads along the y axis; a discrete y with a continuous x picks it
        by default.
    method : {"quasirandom", "pseudorandom", "smiley", "maxout", "frowney", "minout"}, \
default="quasirandom"
        ``"quasirandom"``: van der Corput offsets; ``"pseudorandom"``: seeded
        uniform offsets; ``"smiley"`` (alias ``"maxout"``) and ``"frowney"``
        (``"minout"``): within each density bin, alternate points to the outside,
        low values outermost (smiley) or high values outermost (frowney).
    width : float, optional
        Largest offset, in x units; 0.4 times the x resolution by default.
    varwidth : bool, default=False
        True narrows each x's spread by the square root of its share of the
        largest group's count.
    bandwidth : float, default=0.5
        Multiplier of R's default (``nrd0``) density bandwidth.
    nbins : int, optional
        Number of density points (1024 for quasirandom and pseudorandom; for
        smiley and frowney, the bins, one per 5 points by default).
    dodge_width : float, optional
        Dodge groups at one x side by side, ``position_dodge(width)`` style, and
        spread each group on its own. Without it, every group at an x shares one spread.
    seed : int, optional
        Random seed of ``method="pseudorandom"``.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics (``color``, ``size``, ``shape``, ...).

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If ``method`` is unknown or a numeric parameter is out of range.
    UnsupportedValueError
        If a numeric parameter is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"day": ["Mon"] * 8 + ["Tue"] * 8,
    ...         "minutes": [31, 35, 35, 36, 38, 40, 41, 47, 25, 28, 30, 30, 30, 33, 37, 45]}
    >>> plot = ggplot(data, aes("day", "minutes")) + geom_violin() + geom_quasirandom()
    """
    function = "geom_quasirandom"
    if method in ("tukey", "tukeyDense"):
        reason = (
            f"method {method!r} is not supported (it needs R's random generator); "
            f"use one of {', '.join(map(repr, _QUASIRANDOM_METHODS))}"
        )
        raise SpecError(f"{function}(method={method!r}): {reason}", path="method", reason=reason)
    _check_choice(function, "method", method, _QUASIRANDOM_METHODS)
    _check_positive(function, "width", width)
    _check_seed(function, seed)
    _check_positive(function, "bandwidth", bandwidth)
    if nbins is not None and (
        isinstance(nbins, bool) or not isinstance(nbins, Integral) or nbins < 2
    ):
        reason = f"nbins must be a whole number of at least 2, got {nbins!r}"
        raise SpecError(f"{function}(nbins={nbins!r}): {reason}", path="nbins", reason=reason)
    position, dodge_width = _dodge_from_position(function, position, dodge_width)
    return _geom(
        "quasirandom",
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
        method=method,
        width=width,
        varwidth=varwidth,
        bandwidth=bandwidth,
        nbins=nbins,
        dodge_width=dodge_width,
        seed=seed,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def _check_choice(function: str, name: str, value: object, accepted: tuple[str, ...]) -> None:
    """Raise unless ``value`` is None or one of ``accepted``."""
    if value is None or (isinstance(value, str) and value in accepted):
        return
    listing = ", ".join(map(repr, accepted))
    if isinstance(value, str):
        reason = f"unknown {name} {value!r}; use one of {listing}{did_you_mean(value, accepted)}"
        raise SpecError(f"{function}({name}={value!r}): {reason}", path=name, reason=reason)
    reason = f"expected one of {listing}, got {type(value).__name__}"
    raise UnsupportedValueError(f"{function}({name}={value!r}): {reason}", path=name, reason=reason)


def _check_positive(function: str, name: str, value: object) -> None:
    """Raise unless ``value`` is None or a positive number."""
    if value is None:
        return
    if isinstance(value, bool) or not isinstance(value, Real):
        reason = f"expected a positive number, got {type(value).__name__}"
        raise UnsupportedValueError(
            f"{function}({name}={value!r}): {reason}", path=name, reason=reason
        )
    if not float(value) > 0:
        reason = f"{name} must be a positive number, got {value}"
        raise SpecError(f"{function}({name}={value!r}): {reason}", path=name, reason=reason)


def _check_seed(function: str, seed: object) -> None:
    """Raise unless ``seed`` is None or a whole number."""
    if seed is None or (isinstance(seed, Integral) and not isinstance(seed, bool)):
        return
    reason = f"seed must be a whole number, got {seed!r}"
    raise SpecError(f"{function}(seed={seed!r}): {reason}", path="seed", reason=reason)


def _dodge_from_position(
    function: str, position: Position | None, dodge_width: Number | None
) -> tuple[Position | None, Number | None]:
    """Turn ``position="dodge"`` / ``position_dodge(width)`` into ``dodge_width``.

    The swarm must know about the dodge (each group gets its own swarm), so the
    dodge is the stat's, as in ggbeeswarm. Other positions are kept.
    """
    _check_positive(function, "dodge_width", dodge_width)
    if position == "dodge":
        return None, dodge_width if dodge_width is not None else _DEFAULT_DODGE_WIDTH
    if isinstance(position, FeatureSpec) and position.as_dict().get("name") == "dodge":
        width = position.as_dict().get("width")
        if dodge_width is None:
            dodge_width = width if width is not None else _DEFAULT_DODGE_WIDTH
        return None, dodge_width
    return position, dodge_width
