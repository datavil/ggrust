"""Density slabs and point intervals (R package ggdist).

Not in lets-plot; ggplot2 needs the ggdist package. The numbers follow ggdist 3.3
for sample data: slabs are ``density_bounded()`` densities (reflection at bounds
estimated from the data, Sheather-Jones ``"dpi"`` bandwidth, trimmed to the data,
501 points), intervals are equal-tailed quantile intervals around a median, mean or
mode. ggdist is GPL; ggrust implements the documented algorithms, not its code.

``.width`` is spelled ``widths`` here (``.width`` is not a Python name); the
computed variable keeps ggdist's name: ``"...width.."``.
"""

from __future__ import annotations

from collections.abc import Iterable
from numbers import Integral, Real
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from ggrust.errors import SpecError, UnsupportedValueError
from ggrust.plot.core import aes
from ggrust.plot.geom import _geom
from ggrust.plot.geom_beeswarm import _check_choice, _check_positive

if TYPE_CHECKING:
    from collections.abc import Sequence

    from ggrust._typing import ManualKey, Number, PlotData, Position, Tooltips
    from ggrust.plot.core import FeatureSpec, LayerSpec

__all__ = ["stat_halfeye", "stat_interval", "stat_pointinterval", "stat_slab"]

Density: TypeAlias = Literal["bounded", "unbounded"]
"""Density estimator of a slab."""

PointInterval: TypeAlias = Literal["median_qi", "mean_qi", "mode_qi"]
"""Point summary and interval type."""

SlabSide: TypeAlias = Literal["topright", "top", "right", "bottomleft", "bottom", "left", "both"]
"""Where a slab grows from its baseline."""

Normalize: TypeAlias = Literal["all", "groups", "none"]
"""How slab thicknesses are scaled."""

_DENSITIES: tuple[str, ...] = ("bounded", "unbounded")
_POINT_INTERVALS: tuple[str, ...] = ("median_qi", "mean_qi", "mode_qi")
_SIDES: tuple[str, ...] = ("topright", "top", "right", "bottomleft", "bottom", "left", "both")
_NORMALIZE: tuple[str, ...] = ("all", "groups", "none")
_BANDWIDTHS: tuple[str, ...] = ("dpi", "nrd0", "nrd")
_SLAB_STYLE = ("fill", "alpha")
"""Constant aesthetics ``stat_halfeye`` gives the slab; the others go to the interval."""


def stat_slab(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    manual_key: ManualKey | None = None,
    tooltips: Tooltips | None = None,
    orientation: str | None = None,
    density: Density | None = None,
    bandwidth: Literal["dpi", "nrd0", "nrd"] | Number | None = None,
    adjust: Number | None = None,
    trim: bool | None = None,
    n: int | None = None,
    normalize: Normalize | None = None,
    scale: Number | None = None,
    side: SlabSide | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Density slab of y per x: a filled density on one side of each x.

    As ``stat_slab`` in the R package ggdist, for sample data. Computes
    ``..pdf..``, ``..cdf..``, ``..thickness..`` (the pdf, drawn) and ``..n..``
    per group and x. The slab's thickness is normalized so the layer's largest
    reaches ``scale`` of the x slot.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Needs ``y`` (and usually ``x``), or ``x``
        with a discrete ``y`` for horizontal slabs.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, default="slab"
        Geom that draws the density.
    position : str or FeatureSpec, optional
        Position adjustment, e.g. ``position_dodge(width=0.9)``; none by default.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        This layer's key in a manual legend: a label, or ``layer_key()``.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"``.
    orientation : str, optional
        ``"y"`` draws along the y axis; a discrete y with a continuous x picks it.
    density : {"bounded", "unbounded"}, default="bounded"
        ``"bounded"`` reflects the density at bounds estimated from the data, so
        bounded data (proportions, positive values) keep their edges.
        ``"unbounded"`` is R's ``density()``.
    bandwidth : {"dpi", "nrd0", "nrd"} or float, default="dpi"
        Bandwidth rule (Sheather-Jones direct plug-in by default; it falls back
        to ``"nrd0"`` for samples too sparse for it) or a bandwidth.
    adjust : float, default=1
        Multiplies the bandwidth.
    trim : bool, default=True
        False extends an unbounded density 3 bandwidths beyond the data and keeps
        a bounded one up to its bounds.
    n : int, default=501
        Number of points the density is evaluated at.
    normalize : {"all", "groups", "none"}, default="all"
        ``"all"``: the layer's largest thickness reaches ``scale``; ``"groups"``:
        every slab's does; ``"none"``: thickness as computed.
    scale : float, default=0.9
        Share of the x slot a thickness of 1 fills.
    side : {"topright", "bottomleft", "both"}, default="topright"
        Where the slab grows from x (``"top"``, ``"right"``, ``"bottom"`` and
        ``"left"`` are aliases).
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics (``fill``, ``alpha``, ``color``, ``size``) and
        ``width`` (the slot, in x resolutions, default 1).

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If ``density``, ``side``, ``normalize`` or ``bandwidth`` is not an accepted
        name, or a number is not positive.
    UnsupportedValueError
        If a number option is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"diet": ["veg"] * 6 + ["meat"] * 6,
    ...         "weight": [61, 58, 66, 70, 63, 59, 72, 80, 77, 69, 85, 74]}
    >>> plot = ggplot(data, aes("diet", "weight", fill="diet")) + stat_slab()
    """
    _reject_stat("stat_slab", other_args)
    _check_slab("stat_slab", density, bandwidth, adjust, n, normalize, scale, side)
    return _geom(
        geom if geom is not None else "slab",
        mapping=mapping,
        data=data,
        stat="slab",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=None,
        tooltips=tooltips,
        orientation=orientation,
        density=density,
        bandwidth=bandwidth,
        adjust=adjust,
        trim=trim,
        n=n,
        normalize=normalize,
        scale=scale,
        side=side,
        na_rm=na_rm,
        **other_args,
    )


def stat_pointinterval(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    manual_key: ManualKey | None = None,
    tooltips: Tooltips | None = None,
    orientation: str | None = None,
    point_interval: PointInterval | None = None,
    widths: Sequence[float] | None = None,
    interval_size_range: tuple[float, float] | None = None,
    fatten_point: Number | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Point summary of y with quantile intervals around it, per x.

    As ``stat_pointinterval`` in the R package ggdist, for sample data. One row
    per interval level, widest first, with ``y`` the point, ``ymin``/``ymax`` the
    interval, ``"...width.."`` the level and ``"..level.."`` the level as a
    discrete value. Narrower intervals draw thicker.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Needs ``y`` (and usually ``x``).
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, default="pointinterval"
        Geom that draws the intervals.
    position : str or FeatureSpec, optional
        Position adjustment, e.g. ``position_dodge(width=0.9)``; none by default.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        This layer's key in a manual legend: a label, or ``layer_key()``.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"``.
    orientation : str, optional
        ``"y"`` draws along the y axis; a discrete y with a continuous x picks it.
    point_interval : {"median_qi", "mean_qi", "mode_qi"}, default="median_qi"
        The point (median, mean, or mode) with equal-tailed quantile intervals.
    widths : list of float, default=[0.66, 0.95]
        Interval levels (ggdist's ``.width``), each in (0, 1].
    interval_size_range : tuple of float, default=(0.78, 1.81)
        Line widths of the widest and the narrowest interval (ggdist's 0.6 and
        1.4, converted to ggrust's units so plots look as in R).
    fatten_point : float, default=2.5
        Point size as a multiple of the line width (ggdist's 1.8, converted).
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics (``color``, ``alpha``, ``shape``; ``linewidth`` and
        ``size`` replace the computed widths).

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If ``point_interval`` is not an accepted name, or ``widths`` or
        ``interval_size_range`` holds an invalid value.
    UnsupportedValueError
        If ``widths`` is not a list of numbers.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"diet": ["veg"] * 6 + ["meat"] * 6,
    ...         "weight": [61, 58, 66, 70, 63, 59, 72, 80, 77, 69, 85, 74]}
    >>> plot = ggplot(data, aes("diet", "weight")) + stat_pointinterval(widths=[0.5, 0.9])
    """
    function = "stat_pointinterval"
    _reject_stat(function, other_args)
    _check_interval(function, point_interval, widths)
    _check_size_range(function, interval_size_range, fatten_point)
    return _geom(
        geom if geom is not None else "pointinterval",
        mapping=mapping,
        data=data,
        stat="pointinterval",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=None,
        tooltips=tooltips,
        orientation=orientation,
        point_interval=point_interval,
        widths=_widths(widths),
        interval_size_range=list(interval_size_range) if interval_size_range else None,
        fatten_point=fatten_point,
        na_rm=na_rm,
        **other_args,
    )


def stat_interval(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    manual_key: ManualKey | None = None,
    tooltips: Tooltips | None = None,
    orientation: str | None = None,
    point_interval: PointInterval | None = None,
    widths: Sequence[float] | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Stacked quantile intervals of y per x, colored by level.

    As ``stat_interval`` in the R package ggdist, for sample data: thick
    ``geom_linerange`` lines (``size`` 5.2, ggdist's 4 in ggrust's units), the
    widest drawn first, with
    ``color`` mapped to ``"..level.."`` (discrete, widest first). Add a color
    scale (``scale_color_brewer``, ``scale_color_viridis``) for ggdist's
    sequential look.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Needs ``y`` (and usually ``x``).
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, default="linerange"
        Geom that draws the intervals.
    position : str or FeatureSpec, optional
        Position adjustment; none by default.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        This layer's key in a manual legend: a label, or ``layer_key()``.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"``.
    orientation : str, optional
        ``"y"`` draws along the y axis; a discrete y with a continuous x picks it.
    point_interval : {"median_qi", "mean_qi", "mode_qi"}, default="median_qi"
        The point summary the intervals belong to.
    widths : list of float, default=[0.5, 0.8, 0.95]
        Interval levels (ggdist's ``.width``), each in (0, 1].
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics (``size``, ``alpha``; a constant ``color`` replaces
        the level coloring).

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If ``point_interval`` is not an accepted name or a width is outside (0, 1].
    UnsupportedValueError
        If ``widths`` is not a list of numbers.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"diet": ["veg"] * 6 + ["meat"] * 6,
    ...         "weight": [61, 58, 66, 70, 63, 59, 72, 80, 77, 69, 85, 74]}
    >>> plot = (ggplot(data, aes("diet", "weight")) + stat_interval()
    ...         + scale_color_brewer(palette="Blues"))
    """
    _reject_stat("stat_interval", other_args)
    _check_interval("stat_interval", point_interval, widths)
    other_args.setdefault("size", 5.2)
    return _geom(
        geom if geom is not None else "linerange",
        mapping=mapping,
        data=data,
        stat="interval",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=None,
        tooltips=tooltips,
        orientation=orientation,
        point_interval=point_interval,
        widths=_widths(widths),
        na_rm=na_rm,
        **other_args,
    )


def stat_halfeye(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    manual_key: ManualKey | None = None,
    tooltips: Tooltips | None = None,
    orientation: str | None = None,
    density: Density | None = None,
    bandwidth: Literal["dpi", "nrd0", "nrd"] | Number | None = None,
    adjust: Number | None = None,
    trim: bool | None = None,
    n: int | None = None,
    normalize: Normalize | None = None,
    scale: Number | None = None,
    side: SlabSide | None = None,
    point_interval: PointInterval | None = None,
    widths: Sequence[float] | None = None,
    interval_size_range: tuple[float, float] | None = None,
    fatten_point: Number | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> FeatureSpec:
    """Half-eye plot: a density slab with a point interval at its base, per x.

    As ``stat_halfeye`` in the R package ggdist: a ``stat_slab`` layer and a
    ``stat_pointinterval`` layer over the same data and position. Constant
    ``fill`` and ``alpha`` go to the slab; other constants (``color``,
    ``size``, ``linewidth``, ``shape``) go to the interval.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``, shared by both layers.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    position : str or FeatureSpec, optional
        Position adjustment of both layers, e.g. ``position_dodge(width=0.9)``.
    show_legend : bool, optional
        False hides both layers from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for these layers.
    manual_key : str or FeatureSpec, optional
        The slab's key in a manual legend: a label, or ``layer_key()``.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content of both layers.
    orientation : str, optional
        ``"y"`` draws along the y axis; a discrete y with a continuous x picks it.
    density, bandwidth, adjust, trim, n, normalize, scale, side : optional
        The slab's options; see ``stat_slab``.
    point_interval, widths, interval_size_range, fatten_point : optional
        The interval's options; see ``stat_pointinterval``.
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics, split between the layers as above, and ``width``.

    Returns
    -------
    FeatureSpecArray
        The slab layer, then the point interval layer.

    Raises
    ------
    SpecError
        If an option of either layer is invalid (see ``stat_slab`` and
        ``stat_pointinterval``).
    UnsupportedValueError
        If an option has the wrong type.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"diet": ["veg"] * 6 + ["meat"] * 6,
    ...         "weight": [61, 58, 66, 70, 63, 59, 72, 80, 77, 69, 85, 74]}
    >>> plot = ggplot(data, aes("diet", "weight")) + stat_halfeye()
    """
    function = "stat_halfeye"
    _check_slab(function, density, bandwidth, adjust, n, normalize, scale, side)
    _check_interval(function, point_interval, widths)
    _check_size_range(function, interval_size_range, fatten_point)
    _reject_stat(function, other_args)
    slab_mapping, interval_mapping = _split_mapping(mapping)
    slab_style = {name: other_args.pop(name) for name in _SLAB_STYLE if name in other_args}
    width = other_args.pop("width", None)
    slab = _geom(
        "slab",
        mapping=slab_mapping,
        data=data,
        stat="slab",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=None,
        tooltips=tooltips,
        orientation=orientation,
        density=density,
        bandwidth=bandwidth,
        adjust=adjust,
        trim=trim,
        n=n,
        normalize=normalize,
        scale=scale,
        side=side,
        width=width,
        na_rm=na_rm,
        **slab_style,
    )
    interval = _geom(
        "pointinterval",
        mapping=interval_mapping,
        data=data,
        stat="pointinterval",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        sampling=None,
        tooltips=tooltips,
        orientation=orientation,
        point_interval=point_interval,
        widths=_widths(widths),
        interval_size_range=list(interval_size_range) if interval_size_range else None,
        fatten_point=fatten_point,
        na_rm=na_rm,
        **other_args,
    )
    return slab + interval


_SLAB_VARIABLES = ("..pdf..", "..cdf..", "..thickness..", "..n..")
"""Stat variables only the slab layer of ``stat_halfeye`` computes."""
_INTERVAL_VARIABLES = ("..level..", "...width..")
"""Stat variables only the interval layer of ``stat_halfeye`` computes."""


def _split_mapping(mapping: FeatureSpec | None) -> tuple[FeatureSpec | None, FeatureSpec | None]:
    """Return ``mapping`` for the slab and for the interval layer of ``stat_halfeye``.

    Aesthetics mapped to a stat variable of one layer are left out of the other's
    mapping (ggdist computes both in one layer).
    """
    if mapping is None:
        return None, None
    entries = mapping.as_dict()
    slab = {key: value for key, value in entries.items() if value not in _INTERVAL_VARIABLES}
    interval = {key: value for key, value in entries.items() if value not in _SLAB_VARIABLES}
    if len(slab) == len(interval) == len(entries):
        return mapping, mapping
    return aes(**slab), aes(**interval)


def _reject_stat(function: str, other_args: dict[str, Any]) -> None:
    """Raise when ``stat`` is passed: these functions are their stat."""
    if "stat" in other_args:
        reason = f"{function}() is stat_{function.removeprefix('stat_')}; it takes no 'stat'"
        reason += " (use geom_* or layer() with stat=... for another stat)"
        raise SpecError(
            f"{function}(stat={other_args['stat']!r}): {reason}", path="stat", reason=reason
        )


def _check_slab(
    function: str,
    density: object,
    bandwidth: object,
    adjust: object,
    n: object,
    normalize: object,
    scale: object,
    side: object,
) -> None:
    """Raise for an invalid slab option."""
    _check_choice(function, "density", density, _DENSITIES)
    _check_choice(function, "normalize", normalize, _NORMALIZE)
    _check_choice(function, "side", side, _SIDES)
    if isinstance(bandwidth, str):
        _check_choice(function, "bandwidth", bandwidth, _BANDWIDTHS)
    else:
        _check_positive(function, "bandwidth", bandwidth)
    _check_positive(function, "adjust", adjust)
    _check_positive(function, "scale", scale)
    if n is not None and (not isinstance(n, Integral) or isinstance(n, bool) or n < 2):
        reason = f"n must be a whole number of 2 or more, got {n!r}"
        raise SpecError(f"{function}(n={n!r}): {reason}", path="n", reason=reason)


def _check_interval(function: str, point_interval: object, widths: object) -> None:
    """Raise for an unknown ``point_interval`` or a width outside (0, 1]."""
    _check_choice(function, "point_interval", point_interval, _POINT_INTERVALS)
    if widths is None:
        return
    if isinstance(widths, Real):
        values: list[object] = [widths]
    elif isinstance(widths, Iterable) and not isinstance(widths, str | bytes):
        values = list(widths)  # lists, tuples, numpy arrays, polars series
    else:
        reason = f"expected a list of interval widths, got {type(widths).__name__}"
        raise UnsupportedValueError(
            f"{function}(widths={widths!r}): {reason}", path="widths", reason=reason
        )
    bad = [value for value in values if isinstance(value, bool) or not isinstance(value, Real)]
    # numpy scalars are Real; numpy bools are not, and fall in `bad` too.
    if bad or not values:
        reason = f"expected a non-empty list of numbers, got {widths!r}"
        raise UnsupportedValueError(
            f"{function}(widths={widths!r}): {reason}", path="widths", reason=reason
        )
    numbers = [float(value) for value in values if isinstance(value, Real)]
    outside = [value for value in numbers if not 0 < value <= 1]
    if outside:
        reason = f"interval widths must lie in (0, 1], got {outside}; e.g. widths=[0.66, 0.95]"
        raise SpecError(f"{function}(widths={widths!r}): {reason}", path="widths", reason=reason)


def _check_size_range(function: str, size_range: object, fatten_point: object) -> None:
    """Raise unless ``size_range`` is two non-negative numbers and ``fatten_point`` positive."""
    _check_positive(function, "fatten_point", fatten_point)
    if size_range is None:
        return
    valid = (
        isinstance(size_range, list | tuple)
        and len(size_range) == 2
        and all(isinstance(value, Real) and not isinstance(value, bool) for value in size_range)
        and all(float(value) >= 0 for value in size_range)
    )
    if not valid:
        reason = f"expected two non-negative line widths, e.g. (0.6, 1.4), got {size_range!r}"
        raise SpecError(
            f"{function}(interval_size_range={size_range!r}): {reason}",
            path="interval_size_range",
            reason=reason,
        )


def _widths(widths: Sequence[float] | None) -> list[float] | None:
    """``widths`` as a list of floats for the spec (a single number too)."""
    if widths is None:
        return None
    if isinstance(widths, Real):
        return [float(widths)]
    return [float(value) for value in widths]
