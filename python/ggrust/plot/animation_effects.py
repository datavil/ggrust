"""gganimate enter/exit effects and shadows, added to a plot with ``+``.

A ggrust extension: lets-plot has no animation, ggplot2 needs the gganimate package.
Names, arguments and defaults follow R gganimate (MIT). Effects go into the plot's
``"animation"`` key (``"enter"``, ``"exit"``, ``"shadow"``); they change frames only
when the plot also has a transition.
"""

from __future__ import annotations

import math
import numbers
from collections.abc import Sequence
from typing import Literal

from ggrust.errors import SpecError, did_you_mean
from ggrust.plot.animation import EASES
from ggrust.plot.core import FeatureSpec

__all__ = [
    "enter_fade",
    "enter_grow",
    "exit_fade",
    "exit_shrink",
    "shadow_mark",
    "shadow_wake",
]

Phase = Literal["enter", "exit", "raw", "static", "transition"]
"""A row's phase in a frame (tweenr's ``.phase``)."""

_PHASES: tuple[Phase, ...] = ("enter", "exit", "raw", "static", "transition")


def _number(value: object, function: str, argument: str, path: str, upper: float | None) -> float:
    """Return ``value`` as a float in ``[0, upper]``, else raise ``SpecError``."""
    if (
        isinstance(value, numbers.Real)
        and not isinstance(value, bool)
        and math.isfinite(float(value))
        and float(value) >= 0
        and (upper is None or float(value) <= upper)
    ):
        return float(value)
    accepted = f"a number between 0 and {upper:g}" if upper is not None else "a number >= 0"
    if argument in ("size", "alpha") and function == "shadow_wake":
        accepted = f"True, False or {accepted}"
    message = f"{function}({argument}={value!r}): expected {accepted}"
    raise SpecError(message, path=path)


def _effect(kind: Literal["enter", "exit"], effect: dict[str, object]) -> FeatureSpec:
    """Wrap an enter or exit effect as an ``"animation"`` feature (effects accumulate)."""
    return FeatureSpec("animation", name=None, **{kind: [effect]})


def enter_fade(alpha: float = 0) -> FeatureSpec:
    """Fade marks in as they enter (gganimate ``enter_fade``).

    A mark that has no partner in the state before (matched by group, as the
    transition moves marks) starts at ``alpha`` and moves to its own opacity
    over the transition. Marks with an ``alpha`` get ``alpha``; marks without
    one get their color and fill at that opacity. Several enter effects added
    with ``+`` apply one after the other (``enter_fade() + enter_grow()``).
    Without any enter effect, new marks appear when their state is reached.
    Works with ``transition_states()`` and ``transition_time()``.

    Parameters
    ----------
    alpha : float, default=0
        The opacity entering marks start from, between 0 and 1.

    Returns
    -------
    FeatureSpec
        The effect (kind ``"animation"``), added to a plot with ``+``.

    Raises
    ------
    SpecError
        If ``alpha`` is not a number between 0 and 1.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"year": [2021, 2022, 2022], "fruit": ["apple", "apple", "pear"], "sold": [3, 4, 2]}
    >>> plot = (
    ...     ggplot(data, aes("fruit", "sold"))
    ...     + geom_col()
    ...     + transition_states("year")
    ...     + enter_fade()
    ... )
    >>> plot.as_dict()["animation"]["enter"]
    [{'name': 'fade', 'alpha': 0.0}]

    Bars that start half transparent:

    >>> plot = (
    ...     ggplot(data, aes("fruit", "sold"))
    ...     + geom_col()
    ...     + transition_states("year")
    ...     + enter_fade(alpha=0.5)
    ... )
    """
    checked = _number(alpha, "enter_fade", "alpha", "animation.enter.alpha", 1)
    return _effect("enter", {"name": "fade", "alpha": checked})


def exit_fade(alpha: float = 0) -> FeatureSpec:
    """Fade marks out as they exit (gganimate ``exit_fade``).

    A mark that has no partner in the next state moves from its own opacity to
    ``alpha`` over the transition (see ``enter_fade``). Without any exit effect,
    leaving marks disappear when the transition starts.

    Parameters
    ----------
    alpha : float, default=0
        The opacity exiting marks end at, between 0 and 1.

    Returns
    -------
    FeatureSpec
        The effect (kind ``"animation"``), added to a plot with ``+``.

    Raises
    ------
    SpecError
        If ``alpha`` is not a number between 0 and 1.

    Examples
    --------
    >>> from ggrust import *
    >>> exit_fade(alpha=0.2).props()["exit"]
    [{'name': 'fade', 'alpha': 0.2}]

    A bar that fades out when its fruit is no longer sold:

    >>> data = {"year": [2021, 2021, 2022], "fruit": ["apple", "pear", "apple"], "sold": [3, 2, 4]}
    >>> plot = (
    ...     ggplot(data, aes("fruit", "sold"))
    ...     + geom_col()
    ...     + transition_states("year")
    ...     + exit_fade()
    ... )
    """
    checked = _number(alpha, "exit_fade", "alpha", "animation.exit.alpha", 1)
    return _effect("exit", {"name": "fade", "alpha": checked})


def enter_grow(size: float = 0) -> FeatureSpec:
    """Grow marks from nothing as they enter (gganimate ``enter_grow``).

    Entering marks start at ``size`` times their size and grow to full size
    over the transition, per geom as gganimate: points and texts scale their
    ``size`` and ``stroke``, bars their height (``y`` and ``ymax``), lines,
    paths and polygons spread out from their group's mean point, boxplots open
    from their median. Geoms whose ``size`` is a line width (rectangles, areas)
    are not scaled, as in gganimate, where that is ggplot2's ``linewidth``.

    Parameters
    ----------
    size : float, default=0
        The share of the full size entering marks start from (0: nothing,
        1: full size).

    Returns
    -------
    FeatureSpec
        The effect (kind ``"animation"``), added to a plot with ``+``.

    Raises
    ------
    SpecError
        If ``size`` is not a number >= 0.

    Examples
    --------
    >>> from ggrust import *
    >>> enter_grow().props()["enter"]
    [{'name': 'grow', 'size': 0.0}]
    """
    checked = _number(size, "enter_grow", "size", "animation.enter.size", None)
    return _effect("enter", {"name": "grow", "size": checked})


def exit_shrink(size: float = 0) -> FeatureSpec:
    """Shrink marks to nothing as they exit (gganimate ``exit_shrink``).

    Exiting marks move from full size to ``size`` times their size, by the same
    per-geom rules as ``enter_grow``.

    Parameters
    ----------
    size : float, default=0
        The share of the full size exiting marks end at.

    Returns
    -------
    FeatureSpec
        The effect (kind ``"animation"``), added to a plot with ``+``.

    Raises
    ------
    SpecError
        If ``size`` is not a number >= 0.

    Examples
    --------
    >>> from ggrust import *
    >>> exit_shrink(0.5).props()["exit"]
    [{'name': 'shrink', 'size': 0.5}]
    """
    checked = _number(size, "exit_shrink", "size", "animation.exit.size", None)
    return _effect("exit", {"name": "shrink", "size": checked})


def _end_value(value: bool | float, argument: str, upper: float | None) -> float | bool:
    """Gganimate's ``size``/``alpha`` of ``shadow_wake``: True is 0, False is off.

    False stays False in the spec (None values are dropped from ``as_dict()``,
    and an absent value means gganimate's default, 0).
    """
    if isinstance(value, bool):
        return 0.0 if value else False
    path = f"animation.shadow.{argument}"
    return _number(value, "shadow_wake", argument, path, upper)


def _layers(value: int | Sequence[int] | None, function: str) -> list[int] | None:
    """``exclude_layer`` as a list of 1-based layer numbers."""
    if value is None:
        return None
    values: Sequence[object] = (
        ([value] if isinstance(value, numbers.Integral) else value)
        if isinstance(value, (numbers.Integral, Sequence))
        else []
    )
    layers = [
        int(layer)
        for layer in values
        if isinstance(layer, numbers.Integral) and not isinstance(layer, bool)
    ]
    if values and len(layers) == len(values) and all(layer >= 1 for layer in layers):
        return layers
    message = (
        f"{function}(exclude_layer={value!r}): expected layer numbers counted from 1 "
        "(the first layer added is 1, as in gganimate)"
    )
    raise SpecError(message, path="animation.shadow.exclude_layer")


def _color(value: str | None, argument: str) -> str | None:
    """Return a color name or hex string unchanged, or raise ``SpecError``."""
    if value is None or isinstance(value, str):
        return value
    message = f"shadow_wake({argument}={value!r}): expected a color string, e.g. 'grey'"
    raise SpecError(message, path=f"animation.shadow.{argument}")


def shadow_wake(
    wake_length: float,
    *,
    size: bool | float = True,
    alpha: bool | float = True,
    colour: str | None = None,
    fill: str | None = None,
    falloff: str = "cubic-in",
    wrap: bool = True,
    exclude_layer: int | Sequence[int] | None = None,
    exclude_phase: Phase | Sequence[Phase] | None = ("enter", "exit"),
    color: str | None = None,
) -> FeatureSpec:
    """Leave a fading wake behind moving marks (gganimate ``shadow_wake``).

    Each frame also draws the marks of the frames just before it, oldest
    first, each step closer to the mark itself: the oldest starts from the
    wake's end values (``size``, ``alpha``, ``colour``, ``fill``) and the steps
    move toward the mark's own values, eased by ``falloff``. Only marks drawn
    in the frame keep a wake, and not while they enter or exit (``exclude_phase``).
    Each mark's wake is drawn right before the mark. Layers without the
    transition's column never get a wake.

    Parameters
    ----------
    wake_length : float
        Length of the wake as a share of the frames (``0.1``: the last tenth of
        the frames), above 0 and at most 1.
    size : bool or float, default=True
        The size the wake shrinks to: ``True`` is 0, ``False`` keeps the size,
        a number is the end size relative to nothing (``0.5`` ends at half).
        Only marks with a mark size (points, texts) change.
    alpha : bool or float, default=True
        The opacity the wake fades to: ``True`` is 0, ``False`` keeps it.
    colour : str, optional
        A color the wake's outline color moves to.
    fill : str, optional
        A color the wake's fill moves to.
    falloff : str, default="cubic-in"
        The ease of the steps (any ``ease_aes()`` name).
    wrap : bool, default=True
        Whether the first frames take their wake from the last frames (for
        animations that loop).
    exclude_layer : int or sequence of int, optional
        Layers (counted from 1, as gganimate) that get no wake.
    exclude_phase : str or sequence of str, optional, default=("enter", "exit")
        Marks in these phases get no wake: ``"enter"``, ``"exit"``, ``"raw"``
        (shown as is on a state's last frame), ``"static"`` (held) or
        ``"transition"`` (moving). ``None`` excludes none.
    color : str, optional
        Alias of ``colour``.

    Returns
    -------
    FeatureSpec
        The shadow (kind ``"animation"``), added to a plot with ``+``. A later
        shadow replaces an earlier one.

    Raises
    ------
    SpecError
        If an argument has the wrong type or is out of range, or an ease or
        phase name is unknown (with a close match when there is one).

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"time": [1, 2, 3], "x": [1.0, 2.0, 3.0], "y": [1.0, 4.0, 9.0]}
    >>> plot = (
    ...     ggplot(data, aes("x", "y"))
    ...     + geom_point(size=5)
    ...     + transition_time("time")
    ...     + shadow_wake(0.1)
    ... )
    >>> plot.as_dict()["animation"]["shadow"]["wake_length"]
    0.1

    A longer trail that keeps its size and fades evenly:

    >>> plot = (
    ...     ggplot(data, aes("x", "y"))
    ...     + geom_point(size=5)
    ...     + transition_time("time")
    ...     + shadow_wake(0.3, size=False, falloff="linear")
    ... )

    A trail behind the second layer only (layers are counted from 1):

    >>> plot = (
    ...     ggplot(data, aes("x", "y"))
    ...     + geom_point(size=9, shape=1)
    ...     + geom_point(size=4)
    ...     + transition_time("time")
    ...     + shadow_wake(0.3, exclude_layer=1)
    ... )
    """
    if (
        isinstance(wake_length, bool)
        or not isinstance(wake_length, numbers.Real)
        or not 0 < float(wake_length) <= 1
    ):
        message = (
            f"shadow_wake(wake_length={wake_length!r}): expected the wake length as a "
            "share of the frames, above 0 and at most 1 (e.g. 0.1)"
        )
        raise SpecError(message, path="animation.shadow.wake_length")
    if not isinstance(falloff, str) or falloff not in EASES:
        hint = did_you_mean(falloff, EASES) if isinstance(falloff, str) else ""
        message = (
            f"shadow_wake(falloff={falloff!r}): unknown ease{hint}; accepted: the "
            "ease_aes() names, e.g. 'cubic-in' or 'linear'"
        )
        raise SpecError(message, path="animation.shadow.falloff")
    if not isinstance(wrap, bool):
        message = f"shadow_wake(wrap={wrap!r}): expected True or False"
        raise SpecError(message, path="animation.shadow.wrap")
    if colour is not None and color is not None and colour != color:
        message = f"shadow_wake(): colour={colour!r} and color={color!r} differ; give one"
        raise SpecError(message, path="animation.shadow.colour")
    phases: list[str] = []
    given = [exclude_phase] if isinstance(exclude_phase, str) else list(exclude_phase or ())
    for phase in given:
        if phase not in _PHASES:
            hint = did_you_mean(phase, _PHASES) if isinstance(phase, str) else ""
            message = (
                f"shadow_wake(exclude_phase=...): unknown phase {phase!r}{hint}; accepted: "
                + ", ".join(_PHASES)
            )
            raise SpecError(message, path="animation.shadow.exclude_phase")
        phases.append(phase)
    shadow: dict[str, object] = {
        "name": "wake",
        "wake_length": float(wake_length),
        "size": _end_value(size, "size", None),
        "alpha": _end_value(alpha, "alpha", 1),
        "colour": _color(colour if colour is not None else color, "colour"),
        "fill": _color(fill, "fill"),
        "falloff": falloff,
        "wrap": wrap,
        "exclude_layer": _layers(exclude_layer, "shadow_wake"),
        "exclude_phase": phases,
    }
    return FeatureSpec("animation", name=None, shadow=shadow)


_MARK_STYLES = (
    "alpha",
    "color",
    "colour",
    "fill",
    "size",
    "stroke",
    "shape",
    "linetype",
)


def shadow_mark(
    past: bool = True,
    future: bool = False,
    *,
    exclude_layer: int | Sequence[int] | None = None,
    **aesthetics: str | float,
) -> FeatureSpec:
    """Keep the marks of earlier (or later) states in the background (gganimate ``shadow_mark``).

    Each frame also draws the marks of every state already shown (``past``)
    and/or still to come (``future``), as they are on the last frame of their
    state, behind the frame's own marks. Constant styles such as
    ``alpha=0.3`` or ``color="grey"`` restyle those marks (where the layer has
    that style). Works with ``transition_states()`` (one set of marks per
    state) and ``transition_time()`` (one per time in the data).

    Parameters
    ----------
    past : bool, default=True
        Draw the marks of states before the frame.
    future : bool, default=False
        Draw the marks of states after the frame.
    exclude_layer : int or sequence of int, optional
        Layers (counted from 1, as gganimate) that get no marks.
    **aesthetics : str or float
        Constant styles of the marks: ``alpha``, ``color``/``colour``, ``fill``,
        ``size``, ``stroke``, ``shape`` or ``linetype``. gganimate also takes
        expressions of the data (``alpha = alpha / 2``); ggrust takes constants.

    Returns
    -------
    FeatureSpec
        The shadow (kind ``"animation"``), added to a plot with ``+``. A later
        shadow replaces an earlier one.

    Raises
    ------
    SpecError
        If ``past`` or ``future`` is not a bool, a layer number is not counted
        from 1, or a style is unknown or not a constant.

    Examples
    --------
    >>> from ggrust import *
    >>> shadow_mark(alpha=0.3, color="grey").props()["shadow"]["aesthetics"]
    {'alpha': 0.3, 'color': 'grey'}

    Earlier points stay, small and faded:

    >>> data = {"time": [1, 2, 3], "x": [1.0, 2.0, 3.0], "y": [1.0, 4.0, 9.0]}
    >>> plot = (
    ...     ggplot(data, aes("x", "y"))
    ...     + geom_point(size=5)
    ...     + transition_time("time")
    ...     + shadow_mark(alpha=0.3, size=2)
    ... )
    """
    for name, value in (("past", past), ("future", future)):
        if not isinstance(value, bool):
            message = f"shadow_mark({name}={value!r}): expected True or False"
            raise SpecError(message, path=f"animation.shadow.{name}")
    styles: dict[str, str | float] = {}
    for name, value in aesthetics.items():
        if name not in _MARK_STYLES:
            message = (
                f"shadow_mark({name}=...): unknown style{did_you_mean(name, _MARK_STYLES)}; "
                "accepted: " + ", ".join(_MARK_STYLES)
            )
            raise SpecError(message, path=f"animation.shadow.aesthetics.{name}")
        if isinstance(value, bool) or not isinstance(value, (str, numbers.Real)):
            message = (
                f"shadow_mark({name}={value!r}): expected a constant (a number or a "
                "string); expressions of the data are not supported"
            )
            raise SpecError(message, path=f"animation.shadow.aesthetics.{name}")
        if isinstance(value, str):
            if name in ("alpha", "size", "stroke"):
                message = f"shadow_mark({name}={value!r}): expected a number"
                raise SpecError(message, path=f"animation.shadow.aesthetics.{name}")
            styles[name] = value
            continue
        upper = 1.0 if name == "alpha" else None
        path = f"animation.shadow.aesthetics.{name}"
        if name in ("alpha", "size", "stroke"):
            styles[name] = _number(value, "shadow_mark", name, path, upper)
        else:
            styles[name] = float(value)
    if "color" in styles and "colour" in styles and styles["color"] != styles["colour"]:
        message = (
            f"shadow_mark(): color={styles['color']!r} and colour={styles['colour']!r} "
            "differ; give one"
        )
        raise SpecError(message, path="animation.shadow.aesthetics.colour")
    if "colour" in styles:
        styles.pop("color", None)
    shadow: dict[str, object] = {
        "name": "mark",
        "past": past,
        "future": future,
        "exclude_layer": _layers(exclude_layer, "shadow_mark"),
        "aesthetics": styles,
    }
    return FeatureSpec("animation", name=None, shadow=shadow)
