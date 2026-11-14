"""gganimate transitions, added to a plot with ``+``.

A ggrust extension: lets-plot has no animation, ggplot2 needs the gganimate package.
Names and arguments follow R gganimate (MIT). The transition goes into the plot's
``"animation"`` key; ``animate()`` and the notebook display turn it into frames.
"""

from __future__ import annotations

import numbers
from collections.abc import Sequence

from ggrust.errors import SpecError, did_you_mean
from ggrust.plot.core import FeatureSpec

__all__ = ["ease_aes", "transition_manual", "transition_states"]

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
