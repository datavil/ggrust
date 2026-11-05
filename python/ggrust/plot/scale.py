"""Position scales: ``scale_x_continuous()``, ``scale_x_discrete()``, ``xlim()``, ...

Ported from lets-plot's ``lets_plot/plot/scale.py``, ``scale_position.py`` and
``scale_convenience.py`` (MIT, see THIRD_PARTY_NOTICES.md). Color scales come in a
later phase.
"""

from __future__ import annotations

from collections.abc import Callable
from types import FunctionType
from typing import TYPE_CHECKING, Any, cast

from ggrust.errors import SpecError, UnsupportedValueError, did_you_mean
from ggrust.plot.core import DummySpec, FeatureSpec, FeatureSpecArray
from ggrust.plot.sec_axis import SecondaryAxisSpec

if TYPE_CHECKING:
    from collections.abc import Sequence

    from ggrust._typing import (
        AxisPosition,
        Breaks,
        Color,
        Guide,
        Labels,
        Limits,
        Number,
        Oob,
        Transform,
    )


__all__ = [
    "lims",
    "scale_x_continuous",
    "scale_x_datetime",
    "scale_x_discrete",
    "scale_x_discrete_reversed",
    "scale_x_log2",
    "scale_x_log10",
    "scale_x_reverse",
    "scale_x_time",
    "scale_y_continuous",
    "scale_y_datetime",
    "scale_y_discrete",
    "scale_y_discrete_reversed",
    "scale_y_log2",
    "scale_y_log10",
    "scale_y_reverse",
    "scale_y_time",
    "xlim",
    "ylim",
]


_OOB_NAMES = ("censor", "squish")
_AXIS_POSITIONS: dict[str, tuple[str, ...]] = {
    "x": ("bottom", "top", "both"),
    "y": ("left", "right", "both"),
}


def _check_position(aesthetic: str | Sequence[str], position: object) -> None:
    """Check ``position=`` of an x or y scale against the sides of its axis.

    Raises
    ------
    SpecError
        If ``position`` names a side the axis does not have (``"left"`` for x).
    UnsupportedValueError
        If ``position`` is not a string.
    """
    if position is None:
        return
    axes = [aesthetic] if isinstance(aesthetic, str) else list(aesthetic)
    for axis in axes:
        accepted = _AXIS_POSITIONS.get(axis)
        if accepted is None:
            continue
        if not isinstance(position, str):
            reason = (
                f"expected a side name, got {type(position).__name__}; "
                f"use one of {', '.join(repr(name) for name in accepted)}"
            )
            raise UnsupportedValueError(
                f"scale_{axis}_*(position={position!r}): {reason}", path="position", reason=reason
            )
        if position not in accepted:
            names = ", ".join(repr(name) for name in accepted)
            reason = (
                f"unknown position for a {axis} axis; use one of {names}"
                f"{did_you_mean(position, accepted)}"
            )
            raise SpecError(
                f"scale_{axis}_*(position={position!r}): {reason}", path="position", reason=reason
            )


def _checked_sec_axis(sec_axis: object) -> SecondaryAxisSpec | None:
    """Check ``sec_axis=`` of a continuous position scale.

    Raises
    ------
    UnsupportedValueError
        If ``sec_axis`` is not ``sec_axis(...)``, ``dup_axis(...)`` or None.
    """
    if sec_axis is None or isinstance(sec_axis, SecondaryAxisSpec):
        return sec_axis
    reason = (
        f"expected sec_axis(...) or dup_axis(...), got {type(sec_axis).__name__}; a bare "
        "function goes in sec_axis(transform)"
    )
    raise UnsupportedValueError(
        f"scale_*_continuous(sec_axis={sec_axis!r}): {reason}", path="sec_axis", reason=reason
    )


def _scale(
    aesthetic: str | Sequence[str],
    *,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    expand: Sequence[Number] | None = None,
    na_value: Color | Number | None = None,
    trans: Transform | None = None,
    guide: Guide | None = None,
    format: str | None = None,
    position: AxisPosition | None = None,
    oob: Oob | None = None,
    **kwargs: Any,
) -> FeatureSpec:
    """Create a scale spec for one aesthetic, or an array of specs for a list of them.

    ``breaks`` / ``labels`` given as dicts are split into lists as lets-plot does.

    Raises
    ------
    SpecError
        If ``oob`` is not ``"censor"`` or ``"squish"``, or ``position`` is not a
        side of the scale's axis.
    UnsupportedValueError
        If ``oob`` or ``position`` is not a string.
    """
    _check_position(aesthetic, position)
    if oob is not None and not isinstance(oob, str):
        reason = f"expected an oob name, got {type(oob).__name__}; use 'censor' or 'squish'"
        raise UnsupportedValueError(f"scale_*(oob={oob!r}): {reason}", path="oob", reason=reason)
    if oob is not None and oob not in _OOB_NAMES:
        reason = f"unknown oob; use one of 'censor', 'squish'{did_you_mean(str(oob), _OOB_NAMES)}"
        raise SpecError(f"scale_*(oob={oob!r}): {reason}", path="oob", reason=reason)
    args: dict[str, Any] = {
        "aesthetic": aesthetic,
        "name": name,
        "breaks": breaks,
        "labels": labels,
        "lablim": lablim,
        "limits": limits,
        "expand": expand,
        "na_value": na_value,
        "trans": trans,
        "guide": guide,
        "format": format,
        "position": position,
        "oob": oob,
    }
    if isinstance(breaks, dict):
        if labels is None:
            args["labels"] = list(breaks.keys())
        args["breaks"] = breaks = list(breaks.values())
    if isinstance(labels, dict):
        if breaks is None:
            args["breaks"] = list(labels.keys())
            args["labels"] = list(labels.values())
        else:
            label_by_break = cast("dict[Any, str]", labels)
            new_breaks = [value for value in breaks if value in label_by_break]
            args["labels"] = [label_by_break[value] for value in new_breaks]
            args["breaks"] = new_breaks + [value for value in breaks if value not in new_breaks]
    if isinstance(aesthetic, list):
        args.pop("aesthetic")
        return FeatureSpecArray(
            *[FeatureSpec("scale", aesthetic=item, **args, **kwargs) for item in aesthetic]
        )
    return FeatureSpec("scale", **args, **kwargs)


def scale_x_continuous(
    name: str | None = None,
    *,
    breaks: Breaks | None = None,
    break_width: Number | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    expand: Sequence[Number] | None = None,
    na_value: Color | Number | None = None,
    trans: Transform | None = None,
    format: str | None = None,
    position: AxisPosition | None = None,
    sec_axis: SecondaryAxisSpec | None = None,
) -> FeatureSpec:
    """Continuous position scale x.

    Parameters
    ----------
    name : str, optional
        Axis title (``labs(x=...)`` takes precedence).
    breaks : list of float, optional
        Fixed break positions in data units; breaks outside the axis are dropped.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels). Ignored without
        ``breaks``, as in lets-plot.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"e"``, ``"%"``) or a
        template (``"{.1f} %"``, ``"{d} cells"``). ``labels`` take precedence.
    break_width, lablim, na_value
        As in lets-plot; not drawn yet (ggrust warns).
    position : {'bottom', 'top', 'both'}, default='bottom'
        Side of the panel the axis (ticks, labels, title) is drawn on;
        ``'both'`` draws ticks and labels on both sides, the title at the bottom.
    limits : list, optional
        ``[low, high]`` in data units; ``None`` leaves a side free.
    expand : list, optional
        ``[multiplicative]`` or ``[multiplicative, additive]`` expansion.
    trans : {'identity', 'log10', 'log2', 'sqrt', 'reverse'}, optional
        Transformation of the data.
    sec_axis : SecondaryAxisSpec, optional
        A secondary axis on the other side: ``sec_axis(...)`` or ``dup_axis()``
        (ggplot2; lets-plot has none).

    Returns
    -------
    FeatureSpec
        Scale specification.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> plot = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> plot = plot + scale_x_continuous("Weight (1000 lb)", breaks=[3, 3.5, 4])
    """
    return _scale(
        "x",
        name=name,
        breaks=breaks,
        break_width=break_width,
        labels=labels,
        lablim=lablim,
        limits=limits,
        expand=expand,
        na_value=na_value,
        trans=trans,
        format=format,
        position=position,
        sec_axis=_checked_sec_axis(sec_axis),
    )


def scale_y_continuous(
    name: str | None = None,
    *,
    breaks: Breaks | None = None,
    break_width: Number | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    expand: Sequence[Number] | None = None,
    na_value: Color | Number | None = None,
    trans: Transform | None = None,
    format: str | None = None,
    position: AxisPosition | None = None,
    sec_axis: SecondaryAxisSpec | None = None,
) -> FeatureSpec:
    """Continuous position scale y. See ``scale_x_continuous``.

    Parameters
    ----------
    name : str, optional
        Axis title (``labs(x=...)`` takes precedence).
    breaks : list of float, optional
        Fixed break positions in data units; breaks outside the axis are dropped.
    break_width, lablim, na_value
        As in lets-plot; not drawn yet (ggrust warns).
    position : {'left', 'right', 'both'}, default='left'
        Side of the panel the axis (ticks, labels, title) is drawn on;
        ``'both'`` draws ticks and labels on both sides, the title on the left.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels). Ignored without
        ``breaks``, as in lets-plot.
    limits : list, optional
        ``[low, high]`` in data units; ``None`` leaves a side free.
    expand : list, optional
        ``[multiplicative]`` or ``[multiplicative, additive]`` expansion.
    trans : {'identity', 'log10', 'log2', 'sqrt', 'reverse'}, optional
        Transformation of the data.
    sec_axis : SecondaryAxisSpec, optional
        A secondary axis on the other side: ``sec_axis(...)`` or ``dup_axis()``
        (ggplot2; lets-plot has none).
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"e"``, ``"%"``) or a
        template (``"{.1f} %"``, ``"{d} cells"``). ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        Scale specification.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> plot = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> plot = plot + scale_y_continuous("Miles per gallon", limits=[0, 35])
    """
    return _scale(
        "y",
        name=name,
        breaks=breaks,
        break_width=break_width,
        labels=labels,
        lablim=lablim,
        limits=limits,
        expand=expand,
        na_value=na_value,
        trans=trans,
        format=format,
        position=position,
        sec_axis=_checked_sec_axis(sec_axis),
    )


def _transformed(scale: FunctionType, trans: Transform) -> Callable[..., FeatureSpec]:
    def scale_function(
        name: str | None = None,
        *,
        breaks: Breaks | None = None,
        break_width: Number | None = None,
        labels: Labels | None = None,
        lablim: int | None = None,
        limits: Limits | None = None,
        expand: Sequence[Number] | None = None,
        na_value: Color | Number | None = None,
        format: str | None = None,
        position: AxisPosition | None = None,
        sec_axis: SecondaryAxisSpec | None = None,
    ) -> FeatureSpec:
        return scale(
            name,
            breaks=breaks,
            break_width=break_width,
            labels=labels,
            lablim=lablim,
            limits=limits,
            expand=expand,
            na_value=na_value,
            trans=trans,
            format=format,
            position=position,
            sec_axis=sec_axis,
        )

    axis = scale.__name__.split("_")[1]
    scale_function.__name__ = f"scale_{axis}_{trans}"
    scale_function.__qualname__ = scale_function.__name__
    mapping = '"people", "area"' if axis == "x" else '"area", "people"'
    scale_function.__doc__ = (
        f"Continuous position scale {axis} with the '{trans}' transformation. "
        f"See ``scale_{axis}_continuous``.\n\n"
        "Returns\n-------\nFeatureSpec\n    Scale specification.\n\n"
        "Examples\n--------\n"
        ">>> from ggrust import *\n"
        '>>> data = {"country": ["Iceland", "Norway", "Spain", "Japan", "India"],\n'
        '...         "people": [0.38, 5.5, 48, 124, 1430], "area": [103, 385, 506, 378, 3287]}\n'
        f">>> plot = ggplot(data, aes({mapping})) + geom_point() + scale_{axis}_{trans}()\n"
    )
    return scale_function


scale_x_log10 = _transformed(scale_x_continuous, "log10")
scale_y_log10 = _transformed(scale_y_continuous, "log10")
scale_x_log2 = _transformed(scale_x_continuous, "log2")
scale_y_log2 = _transformed(scale_y_continuous, "log2")
scale_x_reverse = _transformed(scale_x_continuous, "reverse")
scale_y_reverse = _transformed(scale_y_continuous, "reverse")


def scale_x_datetime(
    name: str | None = None,
    *,
    breaks: Breaks | None = None,
    break_width: str | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    expand: Sequence[Number] | None = None,
    na_value: Color | Number | None = None,
    format: str | None = None,
    position: AxisPosition | None = None,
) -> FeatureSpec:
    """Date-time position scale x.

    Values are date-times (``datetime``, ``date``, numpy and polars date-time
    columns) or milliseconds since the Unix epoch. Breaks fall on calendar
    boundaries (whole hours, days, Mondays, months, years) and are labelled by
    their spacing (``"%b %e"``, ``"%Y"``, ...). Axes of date-time columns get
    this scale without asking for it. Labels are in UTC.

    Parameters
    ----------
    name : str, optional
        Axis title (``labs(x=...)`` takes precedence).
    breaks : list, optional
        Fixed break positions (date-times or epoch milliseconds).
    break_width : str, optional
        Fixed distance between breaks, as ``"<count> <unit>"`` with a unit of
        ``ms``, ``sec``, ``min``, ``hour``, ``day``, ``week``, ``month`` or
        ``year`` (plurals allowed), e.g. ``"2 weeks"``.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels). Ignored without
        ``breaks``, as in lets-plot.
    format : str, optional
        Date-time label pattern (``"%d.%m.%y"``, ``"%b %Y"``, ``"{%H:%M}"``).
        ``labels`` take precedence.
    limits : list, optional
        ``[low, high]`` as date-times; ``None`` leaves a side free.
    expand : list, optional
        ``[multiplicative]`` or ``[multiplicative, additive]`` expansion; the
        additive part is in milliseconds.
    lablim, na_value
        As in lets-plot; not drawn yet (ggrust warns).
    position : {'bottom', 'top', 'both'}, default='bottom'
        Side of the panel the axis (ticks, labels, title) is drawn on;
        ``'both'`` draws ticks and labels on both sides, the title at the bottom.

    Returns
    -------
    FeatureSpec
        Scale specification.

    Examples
    --------
    >>> import datetime
    >>> from ggrust import *
    >>> days = [datetime.date(2024, 1, 1) + datetime.timedelta(days=d) for d in range(60)]
    >>> data = {"day": days, "visitors": [100 + (d % 7) * 12 for d in range(60)]}
    >>> plot = (
    ...     ggplot(data, aes("day", "visitors"))
    ...     + geom_line()
    ...     + scale_x_datetime(break_width="2 weeks", format="%d %b")
    ... )
    """
    return _scale(
        "x",
        name=name,
        breaks=breaks,
        break_width=break_width,
        labels=labels,
        lablim=lablim,
        limits=limits,
        expand=expand,
        na_value=na_value,
        format=format,
        position=position,
        datetime=True,
    )


def scale_y_datetime(
    name: str | None = None,
    *,
    breaks: Breaks | None = None,
    break_width: str | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    expand: Sequence[Number] | None = None,
    na_value: Color | Number | None = None,
    format: str | None = None,
    position: AxisPosition | None = None,
) -> FeatureSpec:
    """Date-time position scale y. See ``scale_x_datetime``.

    Returns
    -------
    FeatureSpec
        Scale specification.

    Examples
    --------
    >>> from ggrust import *
    >>> from datetime import datetime
    >>> data = {
    ...     "day": [datetime(2026, 3, day) for day in range(1, 8)],
    ...     "steps": [6400, 8100, 7300, 10200, 5600, 12000, 9100],
    ... }
    >>> plot = ggplot(data, aes("steps", "day")) + geom_point(size=4)
    >>> plot = plot + scale_y_datetime(format="%a %d")
    """
    return _scale(
        "y",
        name=name,
        breaks=breaks,
        break_width=break_width,
        labels=labels,
        lablim=lablim,
        limits=limits,
        expand=expand,
        na_value=na_value,
        format=format,
        position=position,
        datetime=True,
    )


def scale_x_time(
    name: str | None = None,
    *,
    breaks: Breaks | None = None,
    break_width: str | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    expand: Sequence[Number] | None = None,
    na_value: Color | Number | None = None,
    position: AxisPosition | None = None,
) -> FeatureSpec:
    """Duration position scale x.

    Values are durations (``timedelta``, polars ``Duration`` columns) or
    milliseconds. Breaks fall on nice durations (seconds, minutes, hours, days,
    weeks) and are labelled like ``"1:30"`` or ``"2d 6:00"``.

    Parameters
    ----------
    name : str, optional
        Axis title (``labs(x=...)`` takes precedence).
    breaks : list, optional
        Fixed break positions (durations or milliseconds).
    break_width : str, optional
        Fixed distance between breaks, e.g. ``"15 min"`` (see
        ``scale_x_datetime``; months count as 30 days, years as 365).
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels). Ignored without
        ``breaks``, as in lets-plot.
    limits : list, optional
        ``[low, high]`` as durations; ``None`` leaves a side free.
    expand : list, optional
        ``[multiplicative]`` or ``[multiplicative, additive]`` expansion; the
        additive part is in milliseconds.
    lablim, na_value
        As in lets-plot; not drawn yet (ggrust warns).
    position : {'bottom', 'top', 'both'}, default='bottom'
        Side of the panel the axis (ticks, labels, title) is drawn on;
        ``'both'`` draws ticks and labels on both sides, the title at the bottom.

    Returns
    -------
    FeatureSpec
        Scale specification.

    Examples
    --------
    >>> import datetime
    >>> from ggrust import *
    >>> laps = {"lap": [1, 2, 3], "time": [datetime.timedelta(minutes=m) for m in (31, 29, 33)]}
    >>> plot = ggplot(laps, aes("time", "lap")) + geom_point() + scale_x_time()
    """
    return _scale(
        "x",
        name=name,
        breaks=breaks,
        break_width=break_width,
        labels=labels,
        lablim=lablim,
        limits=limits,
        expand=expand,
        na_value=na_value,
        position=position,
        time=True,
    )


def scale_y_time(
    name: str | None = None,
    *,
    breaks: Breaks | None = None,
    break_width: str | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    expand: Sequence[Number] | None = None,
    na_value: Color | Number | None = None,
    position: AxisPosition | None = None,
) -> FeatureSpec:
    """Duration position scale y. See ``scale_x_time``.

    Returns
    -------
    FeatureSpec
        Scale specification.

    Examples
    --------
    >>> from ggrust import *
    >>> from datetime import timedelta
    >>> data = {
    ...     "runner": ["Ann", "Bo", "Cy"],
    ...     "time": [timedelta(minutes=m) for m in (48, 55, 61)],
    ... }
    >>> plot = ggplot(data, aes("runner", "time")) + geom_bar(stat="identity")
    >>> plot = plot + scale_y_time()
    """
    return _scale(
        "y",
        name=name,
        breaks=breaks,
        break_width=break_width,
        labels=labels,
        lablim=lablim,
        limits=limits,
        expand=expand,
        na_value=na_value,
        position=position,
        time=True,
    )


def _as_boolean(value: bool | str | None, *, default: bool) -> bool:
    """lets-plot ``as_boolean``: ``None`` -> ``default``; the string ``'False'`` is false."""
    if value is None:
        return default
    return bool(value) and value != "False"


def scale_x_discrete(
    name: str | None = None,
    *,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    expand: Sequence[Number] | None = None,
    na_value: Color | Number | None = None,
    reverse: bool | None = None,
    format: str | None = None,
    position: AxisPosition | None = None,
) -> FeatureSpec:
    """Discrete position scale x.

    Parameters
    ----------
    name : str, optional
        Axis title (``labs(x=...)`` takes precedence).
    breaks : list of str, optional
        Only these categories get a tick and a label.
    labels : list of str or dict, optional
        Labels aligned with ``breaks`` (or the categories in axis order); missing
        ones are empty. A dict maps categories to labels.
    format : str, optional
        Label template, e.g. ``"<{}>"``. ``labels`` take precedence.
    lablim, na_value
        As in lets-plot; not drawn yet (ggrust warns).
    position : {'bottom', 'top', 'both'}, default='bottom'
        Side of the panel the axis (ticks, labels, title) is drawn on;
        ``'both'`` draws ticks and labels on both sides, the title at the bottom.
    limits : list of str, optional
        The categories to show, in axis order; other categories are dropped.
    expand : list, optional
        ``[multiplicative]`` or ``[multiplicative, additive]``; the default adds 0.2
        categories on each side.
    reverse : bool, default False
        Reverse the category order.

    Returns
    -------
    FeatureSpec
        Scale specification.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"size": ["S", "M", "L", "XL"], "sold": [40, 65, 52, 18]}
    >>> plot = ggplot(data, aes("size", "sold")) + geom_bar(stat="identity")
    >>> plot = plot + scale_x_discrete(limits=["XL", "L", "M", "S"])
    """
    return _scale(
        "x",
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        expand=expand,
        na_value=na_value,
        trans=None,
        guide=None,
        format=format,
        position=position,
        discrete=True,
        reverse=_as_boolean(reverse, default=False),
    )


def scale_y_discrete(
    name: str | None = None,
    *,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    expand: Sequence[Number] | None = None,
    na_value: Color | Number | None = None,
    reverse: bool | None = None,
    format: str | None = None,
    position: AxisPosition | None = None,
) -> FeatureSpec:
    """Discrete position scale y. See ``scale_x_discrete``.

    Parameters
    ----------
    name : str, optional
        Axis title (``labs(x=...)`` takes precedence).
    breaks : list of str, optional
        Only these categories get a tick and a label.
    labels : list of str or dict, optional
        Labels aligned with ``breaks`` (or the categories in axis order); missing
        ones are empty. A dict maps categories to labels.
    lablim, na_value
        As in lets-plot; not drawn yet (ggrust warns).
    position : {'left', 'right', 'both'}, default='left'
        Side of the panel the axis (ticks, labels, title) is drawn on;
        ``'both'`` draws ticks and labels on both sides, the title on the left.
    limits : list of str, optional
        The categories to show, in axis order; other categories are dropped.
    expand : list, optional
        ``[multiplicative]`` or ``[multiplicative, additive]``; the default adds 0.2
        categories on each side.
    reverse : bool, default False
        Reverse the category order.
    format : str, optional
        Label template, e.g. ``"<{}>"``. ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        Scale specification.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"size": ["S", "M", "L", "XL"], "sold": [40, 65, 52, 18]}
    >>> plot = ggplot(data, aes("sold", "size")) + geom_point(size=5)
    >>> plot = plot + scale_y_discrete(labels=["small", "medium", "large", "extra large"])
    """
    return _scale(
        "y",
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        expand=expand,
        na_value=na_value,
        trans=None,
        guide=None,
        format=format,
        position=position,
        discrete=True,
        reverse=_as_boolean(reverse, default=False),
    )


def _discrete_reversed(scale: FunctionType) -> Callable[..., FeatureSpec]:
    def scale_function(
        name: str | None = None,
        *,
        breaks: Breaks | None = None,
        labels: Labels | None = None,
        lablim: int | None = None,
        limits: Limits | None = None,
        expand: Sequence[Number] | None = None,
        na_value: Color | Number | None = None,
        format: str | None = None,
        position: AxisPosition | None = None,
    ) -> FeatureSpec:
        return scale(
            name,
            breaks=breaks,
            labels=labels,
            lablim=lablim,
            limits=limits,
            expand=expand,
            na_value=na_value,
            format=format,
            position=position,
            reverse=True,
        )

    scale_function.__name__ = f"{scale.__name__}_reversed"
    scale_function.__qualname__ = scale_function.__name__
    axis = scale.__name__.split("_")[1]
    plot = (
        'ggplot(data, aes("size", "sold")) + geom_bar(stat="identity")'
        if axis == "x"
        else 'ggplot(data, aes("sold", "size")) + geom_point(size=5)'
    )
    scale_function.__doc__ = (
        f"``{scale.__name__}(reverse=True)``.\n\n"
        "Returns\n-------\nFeatureSpec\n    Scale specification.\n\n"
        "Examples\n--------\n"
        ">>> from ggrust import *\n"
        '>>> data = {"size": ["S", "M", "L", "XL"], "sold": [40, 65, 52, 18]}\n'
        f">>> plot = {plot}\n"
        f">>> plot = plot + {scale_function.__name__}()\n"
    )
    return scale_function


scale_x_discrete_reversed = _discrete_reversed(scale_x_discrete)
scale_y_discrete_reversed = _discrete_reversed(scale_y_discrete)


def lims(x: Sequence[Any] | None, y: Sequence[Any] | None) -> FeatureSpec:
    """Limits for both axes: ``xlim(*x) + ylim(*y)``.

    Parameters
    ----------
    x, y : list of float or None
        ``[low, high]``; ``None`` inside a list leaves that side free.

    Returns
    -------
    FeatureSpec
        Scale specifications.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> plot = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> plot = plot + lims(x=[2, 5], y=[0, 40])
    """
    return xlim(*list(x or [])) + ylim(*list(y or []))


def xlim(*limits: object) -> FeatureSpec:
    """Limits of the x axis, e.g. ``xlim(0, 10)``, ``xlim(None, 10)`` or ``xlim("B", "T")``.

    Parameters
    ----------
    *limits
        Data range of the scale: ``[low, high]`` for continuous scales (None keeps a side), or the
        levels of a discrete scale.

    Returns
    -------
    FeatureSpec
        Scale specification.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> plot = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> plot = plot + xlim(2, 5)
    """
    return _limits("x", *limits)


def ylim(*limits: object) -> FeatureSpec:
    """Limits of the y axis. See ``xlim``.

    Parameters
    ----------
    *limits
        Data range of the scale: ``[low, high]`` for continuous scales (None keeps a side), or the
        levels of a discrete scale.

    Returns
    -------
    FeatureSpec
        Scale specification.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> plot = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> plot = plot + ylim(0, None)
    """
    return _limits("y", *limits)


def _limits(aesthetic: str | Sequence[str], *limits: Any) -> FeatureSpec:
    if len(limits) == 0:
        return DummySpec()
    if any(isinstance(value, str | bytes) for value in limits):
        scale = scale_x_discrete if aesthetic == "x" else scale_y_discrete
    else:
        scale = scale_x_continuous if aesthetic == "x" else scale_y_continuous
    return scale(limits=list(limits))
