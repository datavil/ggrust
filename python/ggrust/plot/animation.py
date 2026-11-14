"""gganimate transitions, added to a plot with ``+``.

A ggrust extension: lets-plot has no animation, ggplot2 needs the gganimate package.
Names and arguments follow R gganimate (MIT). The transition goes into the plot's
``"animation"`` key; ``animate()`` and the notebook display turn it into frames.
"""

from __future__ import annotations

import datetime
import numbers
from collections.abc import Sequence
from typing import Literal, cast

from ggrust.errors import SpecError, did_you_mean
from ggrust.plot.core import FeatureSpec

__all__ = [
    "ease_aes",
    "transition_manual",
    "transition_reveal",
    "transition_states",
    "transition_time",
]

TimeValue = float | datetime.date | datetime.datetime | datetime.time
"""A value of a time column: a number, a date, a date-time or a time of day."""

_EASE_FAMILIES = (
    "quadratic",
    "cubic",
    "quartic",
    "quintic",
    "sine",
    "circular",
    "exponential",
    "elastic",
    "back",
    "bounce",
)
EASES: tuple[str, ...] = (
    "linear",
    *(f"{family}-{kind}" for family in _EASE_FAMILIES for kind in ("in", "out", "in-out")),
)
"""Every ease name ``ease_aes()`` accepts (tweenr's ``validEase``)."""


def transition_manual(frames: str, cumulative: bool = False) -> FeatureSpec:
    """Animate by showing the rows of one value of ``frames`` per frame.

    Each frame draws the rows whose ``frames`` column has one value, in the
    column's order: numbers ascending, categories in their level order (a polars
    ``Enum`` or pandas categorical keeps its order, plain strings their order of
    first appearance). Stats and positions (counts, stacks, smooths) run per value.
    Axes and legends cover every frame, so they stay still. Layers whose data has
    no ``frames`` column are drawn in every frame. As gganimate, the animation has
    one frame per value when ``nframes`` is larger, and ``fps`` shrinks to keep
    the requested duration; with more values than ``nframes``, evenly spaced values
    are shown.

    Titles, subtitles and captions can show the frame with ``{current_frame}``
    (also ``{previous_frame}``, ``{next_frame}``, ``{frame}``, ``{nframes}`` and
    ``{progress}``).

    Parameters
    ----------
    frames : str
        The column whose values are the frames.
    cumulative : bool, default=False
        Keep the rows of earlier frames: frame ``k`` draws the values up to ``k``.

    Returns
    -------
    FeatureSpec
        The transition (kind ``"animation"``), added to a plot with ``+``.

    Raises
    ------
    SpecError
        If ``frames`` is not a non-empty string or ``cumulative`` is not a bool.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "year": [2021, 2021, 2022, 2022, 2023, 2023],
    ...     "fruit": ["apple", "pear"] * 3,
    ...     "sold": [10, 4, 12, 7, 15, 9],
    ... }
    >>> plot = (
    ...     ggplot(data, aes("fruit", "sold"))
    ...     + geom_col()
    ...     + transition_manual("year")
    ...     + ggtitle("Sales in {current_frame}")
    ... )
    >>> animate(plot).nframes
    3
    """
    if not isinstance(frames, str) or not frames:
        message = (
            f"transition_manual(frames={frames!r}): expected the name of a column, "
            "e.g. transition_manual('year')"
        )
        raise SpecError(message, path="animation.transition.frames")
    if not isinstance(cumulative, bool):
        message = f"transition_manual(cumulative={cumulative!r}): expected True or False"
        raise SpecError(message, path="animation.transition.cumulative")
    transition = {"name": "manual", "frames": frames, "cumulative": cumulative}
    return FeatureSpec("animation", name=None, transition=transition)


def _lengths(value: float | Sequence[float], name: str) -> float | list[float]:
    """Check a ``transition_states()`` length: a number >= 0 or a list of them."""
    path = f"animation.transition.{name}"
    text = repr(value)
    shown = text if len(text) <= 60 and "\n" not in text else type(value).__name__
    message = (
        f"transition_states({name}={shown}): expected a number >= 0 or a non-empty "
        "list of them (one per state)"
    )

    def valid(number: object) -> bool:
        return (
            isinstance(number, numbers.Real)
            and not isinstance(number, bool)
            and 0 <= float(number) < float("inf")
        )

    # numpy arrays and polars or pandas Series give their items as a list.
    tolist = getattr(value, "to_list", None) or getattr(value, "tolist", None)
    items_given = tolist() if callable(tolist) and not isinstance(value, numbers.Real) else value
    if isinstance(items_given, Sequence) and not isinstance(items_given, str):
        items = list(items_given)
        if not items or not all(valid(item) for item in items):
            raise SpecError(message, path=path)
        return [float(item) for item in items]
    if not valid(items_given):
        raise SpecError(message, path=path)
    return float(items_given)


def transition_states(
    states: str,
    transition_length: float | Sequence[float] = 1,
    state_length: float | Sequence[float] = 1,
    wrap: bool = True,
) -> FeatureSpec:
    """Animate by holding each value of ``states`` and moving smoothly to the next.

    Each state (one value of the ``states`` column) is shown for a while, then
    its marks move to where they are in the next state (gganimate
    ``transition_states``). States come in the column's order: numbers
    ascending, categories in their level order (a polars ``Enum`` or pandas
    categorical keeps its order, plain strings their order of first appearance).
    Stats and positions run per state; axes and legends cover every state, so
    they stay still. Layers without the ``states`` column are drawn in every
    frame.

    Marks are matched between states by their group (set it with
    ``aes(group=...)``; discrete mappings such as ``color`` also split groups):
    the k-th row of a group moves to the k-th row of the same group. Numbers
    and sizes move linearly by the plot's ``ease_aes()``, colors through CIE Lab,
    categories, shapes and texts switch halfway. Rows without a partner show
    only in their own state.

    Titles, subtitles and captions can show ``{closest_state}``,
    ``{previous_state}``, ``{next_state}`` and ``{transitioning}`` (``TRUE``
    while moving), plus ``{frame}``, ``{nframes}`` and ``{progress}``.

    Parameters
    ----------
    states : str
        The column whose values are the states.
    transition_length : float or sequence of float, default=1
        Relative time spent moving from one state to the next; a list gives one
        length per state (recycled).
    state_length : float or sequence of float, default=1
        Relative time each state is held still; a list gives one per state.
    wrap : bool, default=True
        Move from the last state back to the first, so the animation loops
        smoothly.

    Returns
    -------
    FeatureSpec
        The transition (kind ``"animation"``), added to a plot with ``+``.

    Raises
    ------
    SpecError
        If ``states`` is not a non-empty string, a length is negative or not a
        number, or ``wrap`` is not a bool.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "year": [2021, 2021, 2022, 2022, 2023, 2023],
    ...     "fruit": ["apple", "pear"] * 3,
    ...     "sold": [10, 4, 12, 7, 15, 9],
    ... }
    >>> plot = (
    ...     ggplot(data, aes("fruit", "sold"))
    ...     + geom_col()
    ...     + transition_states("year", transition_length=2, state_length=1)
    ...     + ease_aes("cubic-in-out")
    ...     + ggtitle("Sales in {closest_state}")
    ... )
    >>> animate(plot, nframes=30).nframes
    30
    """
    if not isinstance(states, str) or not states:
        message = (
            f"transition_states(states={states!r}): expected the name of a column, "
            "e.g. transition_states('year')"
        )
        raise SpecError(message, path="animation.transition.states")
    if not isinstance(wrap, bool):
        message = f"transition_states(wrap={wrap!r}): expected True or False"
        raise SpecError(message, path="animation.transition.wrap")
    transition = {
        "name": "states",
        "states": states,
        "transition_length": _lengths(transition_length, "transition_length"),
        "state_length": _lengths(state_length, "state_length"),
        "wrap": wrap,
    }
    return FeatureSpec("animation", name=None, transition=transition)


def _time_number(value: object) -> float | None:
    """Return a time value in the engine's units, or ``None`` if it is not one.

    Numbers stay as they are, dates and date-times become epoch milliseconds
    (naive ones as UTC), times of day milliseconds since midnight.
    """
    try:
        return _time_number_or_raise(value)
    except (ArithmeticError, TypeError, ValueError):
        return None  # pandas NaT, out-of-range dates, ...


def _time_number_or_raise(value: object) -> float | None:
    """Return :func:`_time_number` of ``value``; conversions may raise."""
    if isinstance(value, bool | datetime.timedelta):
        return None
    if type(value).__name__ in ("datetime64", "timedelta64"):
        if type(value).__name__ == "timedelta64":
            return None
        # numpy: nanoseconds would come back from item() as a plain int.
        if value != value:  # NaT
            return None
        astype = getattr(value, "astype")  # noqa: B009
        return float(astype("datetime64[us]").astype("int64")) / 1000.0
    if isinstance(value, numbers.Real):
        number = float(value)
        return number if number == number and abs(number) != float("inf") else None
    if isinstance(value, datetime.datetime):
        aware = value if value.tzinfo else value.replace(tzinfo=datetime.UTC)
        return aware.timestamp() * 1000.0
    if isinstance(value, datetime.date):
        epoch = datetime.date(1970, 1, 1)
        return float((value - epoch).days) * 86_400_000.0
    if isinstance(value, datetime.time):
        return (
            (value.hour * 60 + value.minute) * 60 + value.second
        ) * 1000.0 + value.microsecond / 1000.0
    to_python = getattr(value, "item", None)  # numpy scalars
    if callable(to_python):
        item = to_python()
        return None if item is value else _time_number_or_raise(item)
    return None


RangeKind = Literal["number", "date", "datetime", "time"]
"""The kind of a time transition's ``range``, checked against the column's type."""


def _time_kind(value: object) -> RangeKind | None:
    """Return the kind of one time value (``None`` when it is not one)."""
    if _time_number(value) is None:
        return None
    if type(value).__name__ == "datetime64":
        unit = str(getattr(value, "dtype", ""))
        return (
            "date"
            if unit in ("datetime64[D]", "datetime64[W]", "datetime64[M]", "datetime64[Y]")
            else "datetime"
        )
    if isinstance(value, datetime.datetime):
        return "datetime"
    if isinstance(value, datetime.date):
        return "date"
    if isinstance(value, datetime.time):
        return "time"
    to_python = getattr(value, "item", None)  # numpy scalars
    if not isinstance(value, numbers.Real) and callable(to_python):
        item = to_python()
        return None if item is value else _time_kind(item)
    return "number"


def _range_kind(value: Sequence[TimeValue], function: str) -> RangeKind:
    """Return the shared kind of the two ``range`` values, or raise ``SpecError``."""
    shaped = not isinstance(value, str) and isinstance(value, Sequence) and len(value) == 2
    kinds = {_time_kind(item) for item in value} if shaped else {None}
    if None in kinds:
        _time_range(value, function)  # raises the general message
    if kinds <= {"date", "datetime"} and len(kinds) == 2:
        return "datetime"
    if len(kinds) == 1 and None not in kinds:
        return cast("RangeKind", kinds.pop())
    message = (
        f"{function}(range={value!r}): give both ends as the same kind of value: "
        "numbers, or dates/date-times, or times of day"
    )
    raise SpecError(message, path="animation.transition.range")


def _time_range(value: Sequence[TimeValue] | None, function: str) -> list[float] | None:
    """Check ``range=(low, high)`` of a time transition; return it in engine units."""
    if value is None:
        return None
    path = "animation.transition.range"
    text = repr(value)
    shown = text if len(text) <= 60 and "\n" not in text else type(value).__name__
    message = (
        f"{function}(range={shown}): expected two increasing values (low, high) of the "
        "time column: numbers, dates or date-times"
    )
    if isinstance(value, str) or not isinstance(value, Sequence) or len(value) != 2:
        raise SpecError(message, path=path)
    low, high = (_time_number(item) for item in value)
    if low is None or high is None or not low < high:
        raise SpecError(message, path=path)
    return [low, high]


def _column_name(value: object, function: str, argument: str) -> str:
    """Return ``value`` if it names a column, else raise ``SpecError``."""
    if isinstance(value, str) and value:
        return value
    message = (
        f"{function}({argument}={value!r}): expected the name of a column, e.g. {function}('year')"
    )
    raise SpecError(message, path=f"animation.transition.{argument}")


def transition_time(time: str, range: Sequence[TimeValue] | None = None) -> FeatureSpec:
    """Animate over a time column: frames evenly spaced in time, marks moving between times.

    The frames cover the range of ``time`` in equal steps (gganimate
    ``transition_time``); each time in the data is drawn at its nearest frame,
    and between two times the marks move from one to the next, matched by group
    (as ``transition_states``; set groups with ``aes(group=...)``). Numbers and
    sizes move by the plot's ``ease_aes()``, colors through CIE Lab. Each layer
    uses its own times: before its first time and after its last it draws
    nothing. When two times fall on the same frame, only the earlier one is
    drawn (a warning says how many; raise ``nframes`` to draw more). Stats and
    positions run per time; axes and legends cover every frame. Layers without
    the ``time`` column are drawn in every frame.

    Titles, subtitles and captions can show ``{frame_time}``, the time the frame
    stands for (whole numbers for integer columns, ``YYYY-MM-DD`` for dates,
    date-times in UTC), plus ``{frame}``, ``{nframes}`` and ``{progress}``.

    Parameters
    ----------
    time : str
        The column of the times: numbers, dates, date-times or times of day.
    range : sequence of two values, optional
        The ``(low, high)`` times the frames cover, in the column's type (numbers,
        ``datetime.date``, ``datetime.datetime``, ``numpy.datetime64``). Default: the
        data's range. A range of another kind than the column (numbers for a date
        column) raises ``SpecError`` when the frames are planned.

    Returns
    -------
    FeatureSpec
        The transition (kind ``"animation"``), added to a plot with ``+``.

    Raises
    ------
    SpecError
        If ``time`` is not a non-empty string, or ``range`` is not two
        increasing time values.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "year": [2000, 2000, 2010, 2010, 2020, 2020],
    ...     "city": ["Oslo", "Lima"] * 3,
    ...     "income": [30, 8, 42, 11, 55, 15],
    ...     "life": [79, 72, 81, 75, 83, 77],
    ... }
    >>> plot = (
    ...     ggplot(data, aes("income", "life", color="city"))
    ...     + geom_point(size=5)
    ...     + transition_time("year")
    ...     + ggtitle("Year: {frame_time}")
    ... )
    >>> animate(plot, nframes=21).nframes
    21
    """
    transition: dict[str, object] = {
        "name": "time",
        "time": _column_name(time, "transition_time", "time"),
        "range_kind": None if range is None else _range_kind(range, "transition_time"),
        "range": _time_range(range, "transition_time"),
    }
    return FeatureSpec("animation", name=None, transition=transition)


def transition_reveal(
    along: str,
    range: Sequence[TimeValue] | None = None,
    keep_last: bool = True,
) -> FeatureSpec:
    """Animate by revealing the data along a column: lines grow, points travel.

    The frames cover the range of ``along`` in equal steps (gganimate
    ``transition_reveal``). In each group (sorted by ``along``), lines, paths,
    areas and polygons show every row up to the frame plus an end point
    interpolated at the frame, so they grow smoothly. Other geoms (points,
    texts) show only the interpolated row: a mark that travels along its group.
    Each row's ``along`` is rounded to its nearest frame first, as gganimate
    does. Axes and legends cover the whole data. Layers without the ``along``
    column are drawn in every frame.

    Titles, subtitles and captions can show ``{frame_along}``, the ``along``
    value the frame stands for, plus ``{frame}``, ``{nframes}`` and
    ``{progress}``.

    Parameters
    ----------
    along : str
        The column to reveal along: numbers, dates, date-times or times of day.
    range : sequence of two values, optional
        The ``(low, high)`` values the frames cover, in the column's type
        (numbers, dates, date-times or times of day). Default: the data's range.
        A range of another kind than the column raises ``SpecError`` when the
        frames are planned.
    keep_last : bool, default=True
        Keep a point (or another geom that does not join its rows) at the end
        of its group once the reveal has passed it. Lines always keep every
        row reached.

    Returns
    -------
    FeatureSpec
        The transition (kind ``"animation"``), added to a plot with ``+``.

    Raises
    ------
    SpecError
        If ``along`` is not a non-empty string, ``range`` is not two increasing
        values, or ``keep_last`` is not a bool.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"day": [1, 2, 3, 4, 5], "visitors": [120, 180, 150, 210, 260]}
    >>> plot = (
    ...     ggplot(data, aes("day", "visitors"))
    ...     + geom_line()
    ...     + geom_point(size=4)
    ...     + transition_reveal("day")
    ...     + ggtitle("Day {frame_along}")
    ... )
    >>> animate(plot, nframes=9).nframes
    9
    """
    if not isinstance(keep_last, bool):
        message = f"transition_reveal(keep_last={keep_last!r}): expected True or False"
        raise SpecError(message, path="animation.transition.keep_last")
    transition: dict[str, object] = {
        "name": "reveal",
        "along": _column_name(along, "transition_reveal", "along"),
        "range_kind": None if range is None else _range_kind(range, "transition_reveal"),
        "range": _time_range(range, "transition_reveal"),
        "keep_last": keep_last,
    }
    return FeatureSpec("animation", name=None, transition=transition)


def _check_ease(ease: object, path: str, argument: str) -> str:
    """Return ``ease`` if it is a known ease name, else raise ``SpecError``."""
    if isinstance(ease, str) and ease in EASES:
        return ease
    hint = did_you_mean(ease, EASES) if isinstance(ease, str) else ""
    message = (
        f"ease_aes({argument}={ease!r}): unknown ease{hint}; accepted: 'linear', or "
        "quadratic, cubic, quartic, quintic, sine, circular, exponential, elastic, "
        "back or bounce followed by -in, -out or -in-out (e.g. 'cubic-in-out')"
    )
    raise SpecError(message, path=path)


def ease_aes(default: str = "linear", **aesthetics: str) -> FeatureSpec:
    """Set how values move between states (gganimate ``ease_aes``).

    An ease maps the progress of a transition to the share of the way a value
    has moved: ``"linear"`` moves evenly, ``"cubic-in-out"`` starts and ends
    slowly, ``"bounce-out"`` bounces at the end. Names are tweenr's: ``linear``,
    or one of quadratic, cubic, quartic, quintic, sine, circular, exponential,
    elastic, back and bounce followed by ``-in``, ``-out`` or ``-in-out``.

    Parameters
    ----------
    default : str, default="linear"
        The ease of every aesthetic not named.
    **aesthetics : str
        An ease per aesthetic, e.g. ``y="bounce-out"`` or ``color="sine-in"``.
        ``x`` (``y``) also sets the x-like (y-like) aesthetics not named, such as
        ``xmin`` and ``xend``. Stat variables can be named too (``count``).
        A later ``ease_aes()`` replaces an earlier one as a whole.

    Returns
    -------
    FeatureSpec
        The easing (kind ``"animation"``), added to a plot with ``+``.

    Raises
    ------
    SpecError
        If an ease name is unknown (with a close match when there is one).

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"year": [2021, 2022, 2023], "sold": [10, 12, 15]}
    >>> plot = (
    ...     ggplot(data, aes("year", "sold"))
    ...     + geom_point()
    ...     + transition_states("year")
    ...     + ease_aes("cubic-in-out", y="bounce-out")
    ... )
    >>> plot.as_dict()["animation"]["ease"]["aesthetics"]
    {'y': 'bounce-out'}
    """
    checked_default = _check_ease(default, "animation.ease.default", "default")
    checked = {
        name: _check_ease(ease, f"animation.ease.aesthetics.{name}", name)
        for name, ease in aesthetics.items()
    }
    return FeatureSpec(
        "animation", name=None, ease={"default": checked_default, "aesthetics": checked}
    )
