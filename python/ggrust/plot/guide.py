"""``guides()``, the guide spec helpers and ``layer_key()`` (manual legend keys).

Ported from lets-plot's ``lets_plot/plot/guide.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from __future__ import annotations

import math
from collections.abc import Iterable, Mapping
from dataclasses import dataclass
from numbers import Real
from typing import TYPE_CHECKING, Literal

from ggrust.errors import SpecError, UnsupportedValueError, did_you_mean
from ggrust.plot.core import FeatureSpec

if TYPE_CHECKING:
    from collections.abc import Sequence
    from typing import Any

    from ggrust._typing import Number

__all__ = [
    "RangeKey",
    "guide_axis",
    "guide_axis_mark",
    "guide_axis_nested",
    "guide_bins",
    "guide_colorbar",
    "guide_legend",
    "guides",
    "key_range_manual",
    "layer_key",
]


def _guide(name: str | None, **kwargs: Any) -> FeatureSpec:
    if "title" in kwargs and isinstance(kwargs["title"], int):
        message = (
            "guide_*(): only 'title' may be passed by position; pass the other options by "
            "keyword (e.g. guide_legend('Title', ncol=2))"
        )
        raise SpecError(message)
    return FeatureSpec("guide", name=name, **kwargs)


_CAPS: tuple[str, ...] = ("none", "both", "upper", "lower")


def guide_axis(
    title: str | None = None,
    *,
    cap: Literal["none", "both", "upper", "lower"] | bool | None = None,
) -> FeatureSpec:
    """Axis guide for a position scale, with axis capping (ggplot2 ``guide_axis``).

    ``cap`` cuts the axis line back to the outermost breaks. It shows only where the
    theme draws an axis line (``theme_classic()``, or
    ``theme(axis_line=element_line())``); ticks, labels and the grid are unchanged.

    Parameters
    ----------
    title : str, optional
        Axis title, as ``labs(x=...)``.
    cap : {"none", "both", "upper", "lower"} or bool, default="none"
        Which ends of the axis line stop at the outermost break: ``"both"``, the
        right (top) end only (``"upper"``), the left (bottom) end only
        (``"lower"``), or none. ``True`` is ``"both"``, ``False`` is ``"none"``.
        The ends are visual: ``"upper"`` is the right end also on a reversed scale.
        Under ``coord_flip()``, ``guides(x=...)`` caps the axis drawn horizontally
        (as ggplot2); a scale's own ``guide=`` caps that scale's axis.

    Returns
    -------
    FeatureSpec
        Guide specification for ``guides(x=...)`` / ``guides(y=...)`` or a discrete
        position scale's ``guide=``.

    Raises
    ------
    SpecError
        If ``cap`` is an unknown name.
    UnsupportedValueError
        If ``cap`` is neither a name nor a boolean.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3]}
    >>> plot = (
    ...     ggplot(data, aes("age", "weight"))
    ...     + geom_point()
    ...     + theme_classic()
    ...     + guides(x=guide_axis(cap="both"), y=guide_axis(cap="upper"))
    ... )
    """
    if cap is not None and not isinstance(cap, bool):
        listing = ", ".join(map(repr, _CAPS))
        if not isinstance(cap, str):
            reason = f"expected one of {listing} or a bool, got {cap.__class__.__name__}"
            raise UnsupportedValueError(
                f"guide_axis(cap={cap!r}): {reason}", path="cap", reason=reason
            )
        if cap not in _CAPS:
            reason = f"unknown cap {cap!r}; use one of {listing}{did_you_mean(cap, _CAPS)}"
            raise SpecError(f"guide_axis(cap={cap!r}): {reason}", path="cap", reason=reason)
    return _guide("axis", title=title, cap=cap)


_SPREADS: tuple[str, ...] = ("nearest", "even")


def guide_axis_mark(
    *,
    spread: Literal["nearest", "even"] | None = None,
    link_width: Number | None = None,
    padding: Number | None = None,
) -> FeatureSpec:
    """Axis guide that labels the scale's breaks with leader lines (ComplexHeatmap ``anno_mark``).

    Pick the breaks to label with the scale's ``breaks=`` (for example a few genes
    of a heatmap's rows), and its ``limits=`` to keep the axis order (without
    limits a discrete axis moves its breaks to the front, as in lets-plot). Their
    labels are spread along the axis so they never
    overlap, and a line links each break's true position to its label: straight
    out for a third of the link, slanted over the middle third, straight into the
    label. Works on any axis side (``position=``), under ``coord_flip()``, and on
    side panel axes (``scale_xsidey_*()``, ``scale_ysidex_*()``). Labels take the
    axis text theme (``axis_text_y=element_text(face="italic")``); on top and
    bottom axes they read upwards, and the theme's text angle is not applied. The
    link takes the axis ticks' line (the label colour when ticks are blank). A
    ggrust guide: lets-plot and ggplot2 have no equivalent.

    In HTML output with zoom (``ggtb()``), zoomed labels sit at their breaks
    without links; resetting the zoom restores the spread layout.

    Parameters
    ----------
    spread : {"nearest", "even"}, default="nearest"
        ``"nearest"``: each label at its break when there is room, pushed apart
        only as far as needed (``anno_mark``). ``"even"``: equal slots over the
        whole axis, in break order.
    link_width : float, default=20
        Pixels across the axis the link takes past the ticks (``anno_mark``'s 5 mm).
    padding : float, default=4
        Pixels kept between neighbouring labels (``anno_mark``'s 1 mm).

    Returns
    -------
    FeatureSpec
        Guide specification for ``scale_*(guide=...)`` or ``guides(x=...)``.

    Raises
    ------
    SpecError
        If ``spread`` is unknown.
    UnsupportedValueError
        If ``spread`` is not a string, or ``link_width`` or ``padding`` is not a
        number of 0 or more.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"fruit": ["apple", "banana", "cherry", "date", "fig", "grape"],
    ...         "month": ["Jan"] * 6, "sales": [3, 5, 2, 8, 6, 4]}
    >>> plot = (
    ...     ggplot(data, aes("month", "fruit", fill="sales"))
    ...     + geom_tile()
    ...     + scale_y_discrete(
    ...         breaks=["banana", "cherry", "grape"],
    ...         limits=data["fruit"],
    ...         guide=guide_axis_mark(),
    ...     )
    ... )
    """
    if spread is not None and not (isinstance(spread, str) and spread in _SPREADS):
        listing = ", ".join(map(repr, _SPREADS))
        if isinstance(spread, str):
            reason = (
                f"unknown spread {spread!r}; use one of {listing}{did_you_mean(spread, _SPREADS)}"
            )
            raise SpecError(
                f"guide_axis_mark(spread={spread!r}): {reason}", path="spread", reason=reason
            )
        reason = f"expected one of {listing}, got {type(spread).__name__}"
        raise UnsupportedValueError(
            f"guide_axis_mark(spread={spread!r}): {reason}", path="spread", reason=reason
        )
    for name, value in (("link_width", link_width), ("padding", padding)):
        if value is None:
            continue
        if (
            isinstance(value, bool)
            or not isinstance(value, Real)
            or not math.isfinite(value)
            or not value >= 0
        ):
            reason = f"expected pixels (a number of 0 or more), got {value!r}"
            raise UnsupportedValueError(
                f"guide_axis_mark({name}={value!r}): {reason}", path=name, reason=reason
            )
    return FeatureSpec(
        "guide", name="axis_mark", spread=spread, link_width=link_width, padding=padding
    )


KeyBound = str | int | float
"""One end (or member) of a nested-guide group: a category or a data value."""


@dataclass(frozen=True)
class RangeKey:
    """Groups of a nested axis guide given as ranges (``key_range_manual()``).

    Attributes
    ----------
    start : tuple of str or float
        First category (or value) of each group.
    end : tuple of str or float
        Last category (or value) of each group.
    name : tuple of str or None
        Each group's name, or ``None`` for brackets without names.
    """

    start: tuple[KeyBound, ...]
    end: tuple[KeyBound, ...]
    name: tuple[str, ...] | None

    def as_spec(self) -> list[dict[str, KeyBound | None]]:
        """Write the groups as the spec does.

        Returns
        -------
        list of dict
            One ``{"name", "start", "end"}`` per group.
        """
        names: Sequence[str | None] = self.name or [None] * len(self.start)
        return [
            {"name": name, "start": start, "end": end}
            for name, start, end in zip(names, self.start, self.end, strict=True)
        ]


def _bounds(value: object, parameter: str) -> tuple[KeyBound, ...]:
    """``value`` as a tuple of categories or numbers (a scalar becomes one)."""
    if isinstance(value, (str, int, float)):
        items: list[object] = [value]
    elif isinstance(value, Iterable):
        items = list(value)
    else:
        reason = f"expected a category, a number or a sequence of them, got {type(value).__name__}"
        raise UnsupportedValueError(
            f"key_range_manual({parameter}=...): {reason}", path=parameter, reason=reason
        )
    bounds: list[KeyBound] = []
    for item in items:
        if isinstance(item, bool) or not isinstance(item, (str, int, float)):
            reason = f"expected categories (str) or numbers, got {type(item).__name__}"
            raise UnsupportedValueError(
                f"key_range_manual({parameter}=...): {reason}", path=parameter, reason=reason
            )
        if isinstance(item, float) and not math.isfinite(item):
            reason = f"expected finite numbers, got {item!r}"
            raise UnsupportedValueError(
                f"key_range_manual({parameter}=...): {reason}", path=parameter, reason=reason
            )
        bounds.append(item)
    return tuple(bounds)


def key_range_manual(
    start: KeyBound | Sequence[KeyBound],
    end: KeyBound | Sequence[KeyBound],
    name: str | Sequence[str] | None = None,
) -> RangeKey:
    r"""Give the groups of ``guide_axis_nested()`` by their first and last break (legendry).

    Parameters
    ----------
    start : str, float or sequence of them
        First category (discrete axes) or value (continuous axes) of each group.
    end : str, float or sequence of them
        Last category or value of each group, aligned with ``start``.
    name : str or sequence of str, optional
        Each group's name (``"\n"`` breaks lines). Without names only brackets
        are drawn.

    Returns
    -------
    RangeKey
        The key for ``guide_axis_nested(key=...)``.

    Raises
    ------
    SpecError
        If ``start``, ``end`` and ``name`` differ in length.
    UnsupportedValueError
        If a bound is not a category or a finite number.

    Examples
    --------
    >>> from ggrust import *
    >>> key = key_range_manual(start=["apple", "date"], end=["cherry", "fig"],
    ...                        name=["Early", "Late"])
    >>> len(key.as_spec())
    2
    """
    starts, ends = _bounds(start, "start"), _bounds(end, "end")
    names = None if name is None else ((name,) if isinstance(name, str) else tuple(name))
    lengths = {len(starts), len(ends)} | ({len(names)} if names is not None else set())
    if len(lengths) > 1:
        reason = (
            f"start, end and name must have one entry per group; got {len(starts)} starts, "
            f"{len(ends)} ends" + ("" if names is None else f" and {len(names)} names")
        )
        raise SpecError(f"key_range_manual(): {reason}", path="start", reason=reason)
    if names is not None:
        for item in names:
            if not isinstance(item, str):
                reason = f"expected names (str), got {type(item).__name__}"
                raise UnsupportedValueError(
                    f"key_range_manual(name=...): {reason}", path="name", reason=reason
                )
    return RangeKey(start=starts, end=ends, name=names)


_NESTED_CHOICES: dict[str, tuple[str, ...]] = {
    "type": ("bracket", "box"),
    "bracket": ("line", "square", "chevron"),
    "side": ("axis", "opposite"),
}


def _nested_key(key: object) -> list[dict[str, Any]]:
    """Return the spec form of ``guide_axis_nested(key=...)``."""
    if isinstance(key, RangeKey):
        return key.as_spec()
    if not isinstance(key, Mapping) or isinstance(key, str):
        reason = (
            "expected a mapping {group: [levels]} or key_range_manual(start, end, name), "
            f"got {type(key).__name__}"
        )
        raise UnsupportedValueError(
            f"guide_axis_nested(key=...): {reason}", path="key", reason=reason
        )
    groups: list[dict[str, Any]] = []
    owner: dict[KeyBound, str] = {}
    for group, levels in key.items():
        if not isinstance(group, str):
            reason = f"group names are str, got {type(group).__name__} {group!r}"
            raise UnsupportedValueError(
                f"guide_axis_nested(key=...): {reason}", path="key", reason=reason
            )
        if isinstance(levels, (str, bytes)) or not hasattr(levels, "__iter__"):
            reason = (
                f"the levels of group {group!r} must be a sequence (e.g. [{levels!r}]), "
                f"not {type(levels).__name__}"
            )
            raise UnsupportedValueError(
                f"guide_axis_nested(key=...): {reason}", path=f"key.{group}", reason=reason
            )
        members = _bounds(levels, f"key.{group}")
        for level in members:
            if owner.get(level) == group:
                continue  # listed twice in one group: one membership
            if level in owner:
                reason = (
                    f"level {level!r} is in groups {owner[level]!r} and {group!r}; "
                    "each level belongs to one group"
                )
                raise SpecError(
                    f"guide_axis_nested(key=...): {reason}", path=f"key.{group}", reason=reason
                )
            owner[level] = group
        groups.append({"name": group, "levels": list(members)})
    return groups


def guide_axis_nested(
    key: Mapping[str, Sequence[KeyBound]] | RangeKey,
    *,
    type: Literal["bracket", "box"] | None = None,
    bracket: Literal["line", "square", "chevron"] | None = None,
    side: Literal["axis", "opposite"] | None = None,
    pad_discrete: Number | None = None,
    angle: Number | None = None,
) -> FeatureSpec:
    r"""Axis guide with brackets naming groups of breaks (R legendry ``guide_axis_nested``).

    Each group gets a bracket (or a box) spanning its breaks and its name past
    it, outside the axis labels (``side="axis"``) or across the panel
    (``side="opposite"``, e.g. gene groups above a dot plot whose gene names are
    below). Works on any axis side (``position=``), under ``coord_flip()``, in
    facets (beside the axis only), in ``gggrid()`` and on side panel axes
    (``scale_xsidey_*()``). Names take the axis text theme and may hold
    ``"\n"`` line breaks; the bracket takes ``theme(legendry_bracket=...)``,
    its depth ``legendry_bracket_size``, the box ``legendry_box``. One level of
    groups. In HTML output with zoom (``ggtb()``) the brackets follow the
    breaks; names of groups that leave the panel are hidden.

    Parameters
    ----------
    key : mapping or RangeKey
        The groups: a mapping of group name to its categories
        (``{"Citrus": ["lemon", "lime"]}``; a group spans its first to last
        category on the axis), or ``key_range_manual(start, end, name)``.
        Categories not on the axis are skipped; groups with none are dropped.
    type : {"bracket", "box"}, default="bracket"
        Bracket beside the breaks with the name past it, or a box holding the name.
    bracket : {"line", "square", "chevron"}, default="line"
        Bracket shape (legendry's ``bracket_line()``, ``bracket_square()``,
        ``bracket_chevron()``).
    side : {"axis", "opposite"}, default="axis"
        Past the axis labels, or across the panel from the axis.
    pad_discrete : float, default=0.4
        Categories a group reaches past its first and last category.
    angle : float, default=0
        Angle of the group names, in degrees.

    Returns
    -------
    FeatureSpec
        Guide specification for ``scale_*(guide=...)`` or ``guides(x=...)``.

    Raises
    ------
    SpecError
        If an option value is unknown, or a level is in two groups.
    UnsupportedValueError
        If ``key`` or an option has the wrong type.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"fruit": ["lemon", "lime", "apple", "pear", "plum"],
    ...         "sales": [4, 3, 8, 5, 2]}
    >>> plot = (
    ...     ggplot(data, aes("fruit", "sales"))
    ...     + geom_col()
    ...     + scale_x_discrete(
    ...         limits=data["fruit"],
    ...         guide=guide_axis_nested(
    ...             {"Citrus": ["lemon", "lime"], "Pome": ["apple", "pear"]},
    ...             bracket="square",
    ...         ),
    ...     )
    ... )
    """
    options: dict[str, str | None] = {"type": type, "bracket": bracket, "side": side}
    for option, value in options.items():
        accepted = _NESTED_CHOICES[option]
        if value is None or (isinstance(value, str) and value in accepted):
            continue
        listing = ", ".join(map(repr, accepted))
        if isinstance(value, str):
            reason = (
                f"unknown {option} {value!r}; use one of {listing}{did_you_mean(value, accepted)}"
            )
            raise SpecError(
                f"guide_axis_nested({option}={value!r}): {reason}", path=option, reason=reason
            )
        reason = f"expected one of {listing}, got {value.__class__.__name__}"
        raise UnsupportedValueError(
            f"guide_axis_nested({option}={value!r}): {reason}", path=option, reason=reason
        )
    for option, number in (("pad_discrete", pad_discrete), ("angle", angle)):
        if number is not None and (
            isinstance(number, bool) or not isinstance(number, Real) or not math.isfinite(number)
        ):
            reason = f"expected a finite number, got {number!r}"
            raise UnsupportedValueError(
                f"guide_axis_nested({option}={number!r}): {reason}", path=option, reason=reason
            )
    return FeatureSpec(
        "guide",
        name="axis_nested",
        key=_nested_key(key),
        type=type,
        bracket=bracket,
        side=side,
        pad_discrete=pad_discrete,
        angle=angle,
    )


def guides(**kwargs: object) -> FeatureSpec:
    """Set guides (legends, colorbars) per aesthetic.

    Parameters
    ----------
    **kwargs
        Aesthetic name to guide (``"none"``, ``guide_legend()``, ...).

    Returns
    -------
    FeatureSpec
        The guides feature.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + guides(color=guide_legend(title="Animal", ncol=3))
    """
    return FeatureSpec("guides", name=None, **kwargs)


def guide_legend(
    title: str | None = None,
    *,
    nrow: int | None = None,
    ncol: int | None = None,
    byrow: bool | None = None,
    override_aes: Mapping[str, Any] | None = None,
    reverse: bool | None = None,
    direction: Literal["horizontal", "vertical"] | None = None,
    position: Literal["top", "right", "bottom", "left", "inside"] | None = None,
    order: int | None = None,
    theme: FeatureSpec | None = None,
) -> FeatureSpec:
    """Show a key legend for an aesthetic.

    Parameters
    ----------
    title : str, optional
        Legend title.
    nrow, ncol : int, optional
        Number of rows or columns of legend keys.
    byrow : bool, optional
        True fills the keys row by row instead of column by column.
    override_aes : dict, optional
        Constant aesthetics for the legend keys only, e.g. ``{"size": 5}``.
    reverse : bool, optional
        True lists the keys in reverse order (ggplot2).
    direction : {"horizontal", "vertical"}, optional
        Key layout of this legend; None follows ``theme(legend_direction=...)``
        (ggplot2).
    position : {"top", "right", "bottom", "left", "inside"}, optional
        Side of the plot this guide goes to, over ``theme(legend_position=...)``;
        ``"inside"`` puts it in the panel at ``theme(legend_position_inside=...)``
        (ggplot2 3.5). Guides at one position are stacked in one block. None
        follows ``legend_position``; ``legend_position="none"`` still hides every
        guide.
    order : int, optional
        Place among the legends, 1 to 99: legends with a lower order come first,
        legends without one (or 0) last, in their usual order (ggplot2).
    theme : FeatureSpec, optional
        ``theme(legend_*=...)`` options for this legend only, e.g.
        ``theme(legend_text=element_text(color="red"))`` (ggplot2).

    Returns
    -------
    FeatureSpec
        The guide.

    Raises
    ------
    SpecError
        If ``direction``, ``order`` or ``theme`` is not accepted.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + guides(color=guide_legend(nrow=1, override_aes={"size": 8}))
    >>> plot = plot + guides(color=guide_legend(reverse=True, order=1))
    """
    _check_common("guide_legend", reverse, direction, order, position)
    return _guide("legend", **_with_theme(locals(), "guide_legend"))


def guide_colorbar(
    title: str | None = None,
    *,
    barwidth: Number | None = None,
    barheight: Number | None = None,
    nbin: int | None = None,
    label_position: Literal["ends"] | None = None,
    reverse: bool | None = None,
    direction: Literal["horizontal", "vertical"] | None = None,
    position: Literal["top", "right", "bottom", "left", "inside"] | None = None,
    order: int | None = None,
    theme: FeatureSpec | None = None,
    display: Literal["gradient", "rectangles", "raster"] | None = None,
    alpha: Number | None = None,
    draw_ulim: bool | None = None,
    draw_llim: bool | None = None,
    shape: Literal["triangle"] | None = None,
    show: bool | tuple[bool, bool] | list[bool] | None = None,
    cap_size: Number | None = None,
    minor_ticks: bool | None = None,
) -> FeatureSpec:
    """Show a colorbar for a continuous color or fill scale.

    Parameters
    ----------
    title : str, optional
        Colorbar title.
    barwidth, barheight : float, optional
        Bar size in pixels.
    nbin : int, optional
        Number of color segments (default 20).
    label_position : {"ends"}, optional
        ``"ends"`` draws only the labels of the lowest and highest breaks, beside
        the bar's ends: left and right of a horizontal bar (``RNA  Min ▬ Max``),
        below and above a vertical one. Other breaks keep their ticks. The default
        (None) puts every label below a horizontal bar or right of a vertical one.
        A ggrust option; lets-plot and ggplot2 have no equivalent.
    reverse : bool, optional
        True puts the high values at the bottom (or left) of the bar (ggplot2).
    direction : {"horizontal", "vertical"}, optional
        Bar orientation of this colorbar; None follows
        ``theme(legend_direction=...)`` (ggplot2).
    position : {"top", "right", "bottom", "left", "inside"}, optional
        Side of the plot this guide goes to, over ``theme(legend_position=...)``;
        ``"inside"`` puts it in the panel at ``theme(legend_position_inside=...)``
        (ggplot2 3.5). Guides at one position are stacked in one block. None
        follows ``legend_position``; ``legend_position="none"`` still hides every
        guide.
    order : int, optional
        Place among the legends, 1 to 99, as in ``guide_legend()``.
    theme : FeatureSpec, optional
        ``theme(legend_*=...)`` options for this colorbar only (ggplot2).
    display : {"gradient", "rectangles", "raster"}, optional
        ``"rectangles"`` (the default, as lets-plot) draws ``nbin`` flat color
        segments; ``"gradient"`` and ``"raster"`` blend smoothly between them
        (ggplot2).
    alpha : float, optional
        Opacity of the bar, between 0 and 1 (ggplot2).
    draw_ulim, draw_llim : bool, optional
        False hides the ticks of a break at the upper (lower) end of the bar
        (ggplot2).
    shape : {"triangle"}, optional
        Caps past the bar's ends: ``"triangle"`` draws a pointed, equilateral cap on
        the bar's thickness, filled with that end's color, like matplotlib's
        ``colorbar(extend=...)`` (values beyond the limits). Named after legendry's
        ``guide_colbar(shape=)``; a ggrust option, not in lets-plot or ggplot2.
        None (the default) draws no caps.
    show : bool or (bool, bool), optional
        Which caps ``shape`` draws: True both (the default), False none, or
        ``(lower, upper)`` for the low and high ends of the scale (legendry). Ends
        follow the transformed scale: with ``trans="reverse"`` the upper end holds
        the smallest data value. Needs ``shape``.
    cap_size : float, optional
        Length of each cap along the bar, in pixels (legendry's ``size``). None
        makes the caps equilateral; matplotlib's look is about 5 percent of the
        bar length (``extendfrac``). Needs ``shape``.
    minor_ticks : bool, optional
        True adds shorter ticks between the breaks: 2 to 9 times each power of ten
        on a log10 scale (none past 50 decades), else halfway between neighbouring
        breaks (like ggplot2's axis ``minor.ticks``). A ggrust option; stepped
        (binned) colorbars draw none.

    Returns
    -------
    FeatureSpec
        The guide.

    Raises
    ------
    SpecError
        If ``label_position``, ``direction``, ``order``, ``theme``, ``display``,
        ``alpha``, ``shape``, ``show``, ``cap_size`` or ``minor_ticks`` is not
        accepted.

    Examples
    --------
    >>> from ggrust import *
    >>> spec = guides(fill=guide_colorbar(label_position="ends"))
    >>> spec = guides(fill=guide_colorbar(reverse=True, display="gradient", alpha=0.8))
    >>> spec = guides(
    ...     color=guide_colorbar(
    ...         shape="triangle",
    ...         minor_ticks=True,
    ...         theme=theme(legend_title_position="bottom"),
    ...     )
    ... )
    """
    arguments = dict(locals())
    if label_position is not None and label_position != "ends":
        message = (
            f"guide_colorbar(label_position={label_position!r}) is not supported; "
            f"expected 'ends' or None{did_you_mean(str(label_position), ['ends'])}"
        )
        raise SpecError(message)
    _check_common("guide_colorbar", reverse, direction, order, position)
    if display is not None and display not in _DISPLAYS:
        message = (
            f"guide_colorbar(display={display!r}) is not supported; expected one of "
            f"{', '.join(repr(name) for name in _DISPLAYS)}"
            f"{did_you_mean(str(display), list(_DISPLAYS))}"
        )
        raise SpecError(message)
    if alpha is not None and (
        isinstance(alpha, bool) or not isinstance(alpha, Real) or not 0 <= alpha <= 1
    ):
        raise SpecError(f"guide_colorbar(alpha={alpha!r}): expected a number between 0 and 1")
    for name, flag in (
        ("draw_ulim", draw_ulim),
        ("draw_llim", draw_llim),
        ("minor_ticks", minor_ticks),
    ):
        if flag is not None and not isinstance(flag, bool):
            raise SpecError(f"guide_colorbar({name}={flag!r}): expected True or False")
    if shape is not None and shape != "triangle":
        message = (
            f"guide_colorbar(shape={shape!r}) is not supported; expected 'triangle' or "
            f"None{did_you_mean(str(shape), ['triangle'])}"
        )
        raise SpecError(message)
    if show is not None:
        if shape is None:
            raise SpecError(
                f"guide_colorbar(show={show!r}): show picks the caps drawn by shape; "
                "add shape='triangle'"
            )
        if isinstance(show, (tuple, list)):
            if len(show) != 2 or not all(isinstance(flag, bool) for flag in show):
                raise SpecError(
                    f"guide_colorbar(show={show!r}): expected True, False or "
                    "(lower, upper) booleans"
                )
            arguments["show"] = list(show)
        elif not isinstance(show, bool):
            raise SpecError(
                f"guide_colorbar(show={show!r}): expected True, False or (lower, upper) booleans"
            )
    if cap_size is not None:
        if shape is None:
            raise SpecError(
                f"guide_colorbar(cap_size={cap_size!r}): cap_size sets the length of the "
                "caps drawn by shape; add shape='triangle'"
            )
        if (
            isinstance(cap_size, bool)
            or not isinstance(cap_size, Real)
            or not math.isfinite(cap_size)
            or cap_size <= 0
        ):
            raise SpecError(
                f"guide_colorbar(cap_size={cap_size!r}): expected a positive size in pixels"
            )
    return _guide("colorbar", **_with_theme(arguments, "guide_colorbar"))


def guide_bins(
    title: str | None = None,
    *,
    show_limits: bool | None = None,
    reverse: bool | None = None,
    direction: Literal["horizontal", "vertical"] | None = None,
    position: Literal["top", "right", "bottom", "left", "inside"] | None = None,
    order: int | None = None,
    theme: FeatureSpec | None = None,
) -> FeatureSpec:
    """Show a binned scale's keys touching, labelled on their edges (ggplot2).

    The default guide of ``scale_size_binned`` and ``scale_alpha_binned``: one key
    per bin in a single column (or row), an axis line beside them with a tick at
    each labelled edge, and the break values on the edges between the keys. Not in
    lets-plot. Scales merged into one legend (the same title) need the same bins,
    and a guide too large for its room is drawn as a plain legend instead. There is
    no ``ncol`` / ``nrow``: the keys always form one column (or row).

    Parameters
    ----------
    title : str, optional
        Legend title.
    show_limits : bool, optional
        Also label the two outer edges. None follows the scale's ``show_limits``.
    reverse : bool, optional
        True lists the bins from high to low.
    direction : {"horizontal", "vertical"}, optional
        Key layout of this guide; None follows ``theme(legend_direction=...)``.
    position : {"top", "right", "bottom", "left", "inside"}, optional
        Side of the plot this guide goes to, over ``theme(legend_position=...)``;
        ``"inside"`` puts it in the panel at ``theme(legend_position_inside=...)``
        (ggplot2 3.5). Guides at one position are stacked in one block. None
        follows ``legend_position``; ``legend_position="none"`` still hides every
        guide.
    order : int, optional
        Place among the legends, 1 to 99, as in ``guide_legend``.
    theme : FeatureSpec, optional
        ``theme(legend_*=...)`` options for this guide only.

    Returns
    -------
    FeatureSpec
        The guide.

    Raises
    ------
    SpecError
        If ``direction``, ``order`` or ``theme`` is not accepted, or
        ``show_limits`` is not True, False or None.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3, 4, 5], "y": [2, 4, 3, 5, 1], "score": [3, 18, 41, 66, 97]}
    >>> plot = ggplot(data, aes("x", "y", size="score")) + geom_point()
    >>> plot = plot + scale_size_binned(guide=guide_bins(show_limits=True))
    """
    _check_common("guide_bins", reverse, direction, order, position)
    if show_limits is not None and not isinstance(show_limits, bool):
        raise SpecError(f"guide_bins(show_limits={show_limits!r}): expected True or False")
    return _guide("bins", **_with_theme(locals(), "guide_bins"))


_DIRECTIONS: tuple[str, ...] = ("horizontal", "vertical")
_POSITIONS: tuple[str, ...] = ("top", "right", "bottom", "left", "inside")
_DISPLAYS: tuple[str, ...] = ("gradient", "rectangles", "raster")
# Elements that place or arrange the whole legend block, not one guide.
_BLOCK_ELEMENTS: frozenset[str] = frozenset(
    {
        "legend_position",
        "legend_justification",
        "legend_box",
        "legend_box_just",
        "legend_box_spacing",
        "legend_spacing",
        "legend_spacing_x",
        "legend_spacing_y",
        "legend_interactive",
    }
)


def _check_common(
    function: str,
    reverse: bool | None,
    direction: str | None,
    order: int | None,
    position: str | None = None,
) -> None:
    """Validate the options every legend guide takes.

    Parameters
    ----------
    function : str
        Name of the guide function, for the message.
    reverse : bool, optional
        The ``reverse`` option.
    direction : str, optional
        The ``direction`` option.
    order : int, optional
        The ``order`` option.
    position : str, optional
        The ``position`` option.

    Raises
    ------
    SpecError
        If an option has the wrong type or value.
    """
    if reverse is not None and not isinstance(reverse, bool):
        raise SpecError(f"{function}(reverse={reverse!r}): expected True or False")
    if direction is not None and direction not in _DIRECTIONS:
        message = (
            f"{function}(direction={direction!r}) is not supported; expected 'horizontal' "
            f"or 'vertical'{did_you_mean(str(direction), list(_DIRECTIONS))}"
        )
        raise SpecError(message)
    if order is not None and (
        isinstance(order, bool) or not isinstance(order, int) or not 0 <= order <= 99
    ):
        raise SpecError(f"{function}(order={order!r}): expected an integer from 0 to 99")
    if position is not None and position not in _POSITIONS:
        message = (
            f"{function}(position={position!r}) is not supported; expected one of "
            f"{', '.join(repr(name) for name in _POSITIONS)}"
            f"{did_you_mean(str(position), list(_POSITIONS))}"
        )
        raise SpecError(message)


def _with_theme(options: dict[str, Any], function: str) -> dict[str, Any]:
    """Return guide options with ``theme`` as its dict of ``legend_*`` elements.

    Parameters
    ----------
    options : dict
        The guide function's arguments.
    function : str
        Name of the guide function, for the message.

    Returns
    -------
    dict
        ``options`` with ``theme`` replaced by its elements (or dropped).

    Raises
    ------
    SpecError
        If ``theme`` is not a ``theme(...)`` spec, is a complete theme, or sets
        elements other than ``legend_*`` ones.
    """
    options = dict(options)
    theme = options.pop("theme", None)
    if theme is None:
        return options
    if not isinstance(theme, FeatureSpec) or theme.kind != "theme":
        raise SpecError(
            f"{function}(theme=...): expected theme(legend_*=...), got {type(theme).__name__}"
        )
    elements = theme.as_dict()
    if "name" in elements:
        raise SpecError(
            f"{function}(theme=...): a complete theme ({elements['name']!r}) is not "
            "supported here; pass theme(legend_*=...) options"
        )
    others = sorted(
        key for key in elements if not key.startswith("legend_") or key in _BLOCK_ELEMENTS
    )
    if others:
        raise SpecError(
            f"{function}(theme=...): a guide theme styles its own legend only; got "
            f"{', '.join(others)}. Set those in the plot's theme(), and give this guide "
            "legend_* options such as legend_text or legend_key_size"
        )
    options["theme"] = elements
    return options


def layer_key(
    label: str,
    group: str | None = None,
    *,
    index: int | None = None,
    **kwargs: Any,
) -> FeatureSpec:
    """Describe a layer's entry in a manual legend (``manual_key=`` of any geom).

    The key is drawn by the layer's geom with its constant aesthetics, then the
    values given here. Layers with the same ``group`` share a legend; a label used
    by several layers of one group becomes one key drawing all of them. The key
    shows even when the layer has ``show_legend=False``.

    Parameters
    ----------
    label : str
        Text next to the key.
    group : str, default="manual"
        Legend the key joins. The ``"manual"`` legend has no title; give one with
        ``labs(manual="Title")`` or ``guides(manual=guide_legend("Title"))``. Other
        groups are titled by their name unless ``guides(<group>=...)`` sets one.
    index : int, optional
        Position of the key in its legend; keys with an index come first, in index
        order, then the others in layer order.
    **kwargs
        Aesthetic values for the key only, e.g. ``size=3`` or ``shape=21``.

    Returns
    -------
    FeatureSpec
        The key specification, for a geom's ``manual_key``.

    Raises
    ------
    TypeError
        When ``label`` is not a string.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"day": [1, 2, 3, 4, 5], "steps": [6.1, 8.4, 7.2, 9.9, 8.8]}
    >>> plot = (
    ...     ggplot(data, aes("day", "steps"))
    ...     + geom_line(color="grey", manual_key=layer_key("trend", linetype="solid"))
    ...     + geom_point(color="firebrick", size=4, manual_key=layer_key("measured"))
    ...     + labs(manual="Steps (thousands)")
    ... )
    """
    if not isinstance(label, str):
        message = f"layer_key(): label must be a string, got {type(label).__name__}"
        raise TypeError(message)
    return FeatureSpec("layer_key", name=None, label=label, group=group, index=index, **kwargs)
