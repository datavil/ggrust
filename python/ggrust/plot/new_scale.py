"""ggnewscale: ``new_scale()``, ``new_scale_color()`` and ``new_scale_fill()``.

Not in lets-plot; ggplot2 needs the ggnewscale package. Adding ``new_scale_color()``
with ``+`` starts a new color scale: layers, ``scale_color_*()``, ``guides()`` and
``labs()`` added after it belong to the new scale, which gets its own legend. Those
added before keep the earlier scale. The plot records this as a ``scale_group``
on layers and scales and as ``"color:1"`` style keys in ``guides`` (see
``deviations.md``).
"""

from __future__ import annotations

from ggrust.errors import SpecError, did_you_mean
from ggrust.plot.core import FeatureSpec

__all__ = ["new_scale", "new_scale_color", "new_scale_colour", "new_scale_fill"]

STYLE_AESTHETICS: tuple[str, ...] = (
    "color",
    "fill",
    "size",
    "alpha",
    "shape",
    "linetype",
    "paint_a",
    "paint_b",
    "paint_c",
)
"""Aesthetics with a style scale, the ones ``new_scale()`` accepts."""

ALIASES: dict[str, str] = {"colour": "color", "col": "color"}
"""Spellings of style aesthetics that ggrust reads as another name."""


def canonical_aesthetic(name: str) -> str:
    """Return the aesthetic name ggrust uses for ``name`` (``colour`` -> ``color``).

    Parameters
    ----------
    name : str
        An aesthetic name or alias.

    Returns
    -------
    str
        The canonical name; unknown names are returned unchanged.
    """
    return ALIASES.get(name, name)


def new_scale(new_aes: str) -> FeatureSpec:
    """Start a new scale for an aesthetic (R package ggnewscale).

    Layers, scales, ``guides()`` and ``labs()`` added with ``+`` after it use a new,
    separate scale for ``new_aes`` with its own legend; those added before keep the
    earlier one. Add it again to start a third scale, and so on.

    Parameters
    ----------
    new_aes : str
        The style aesthetic: ``"color"`` (or ``"colour"``), ``"fill"``, ``"size"``,
        ``"alpha"``, ``"shape"``, ``"linetype"``. Position aesthetics are not
        accepted.

    Returns
    -------
    FeatureSpec
        The marker, to add to a plot with ``+``.

    Raises
    ------
    SpecError
        When ``new_aes`` is not a style aesthetic.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3, 4], "y": [2, 4, 3, 5], "size": [1, 3, 2, 4]}
    >>> plot = ggplot(data, aes("x", "y")) + geom_point(aes(size="size"))
    >>> plot = plot + new_scale("size") + geom_point(aes(size="y"), color="red", alpha=0.4)
    """
    if not isinstance(new_aes, str):
        message = f"new_scale(new_aes=...): expected an aesthetic name, got {new_aes!r}"
        raise SpecError(message, path="new_aes", reason=message)
    aesthetic = canonical_aesthetic(new_aes)
    if aesthetic not in STYLE_AESTHETICS:
        accepted = ", ".join(STYLE_AESTHETICS)
        reason = (
            f"new_scale() works for style aesthetics with a scale, not {new_aes!r}; "
            f"accepted: {accepted}{did_you_mean(new_aes, STYLE_AESTHETICS)}"
        )
        raise SpecError(f"at 'new_aes': {reason}", path="new_aes", reason=reason)
    return FeatureSpec("new_scale", name=None, aesthetic=aesthetic)


def new_scale_color() -> FeatureSpec:
    """Start a new color scale (ggnewscale): ``new_scale("color")``.

    Returns
    -------
    FeatureSpec
        The marker, to add to a plot with ``+``.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3, 4], "y": [2, 4, 3, 5], "group": ["a", "a", "b", "b"]}
    >>> plot = (
    ...     ggplot(data, aes("x", "y"))
    ...     + geom_point(aes(color="y"), size=5)
    ...     + new_scale_color()
    ...     + geom_text(aes(label="group", color="group"), nudge_y=0.3)
    ...     + scale_color_brewer(palette="Set1")
    ... )
    """
    return new_scale("color")


def new_scale_colour() -> FeatureSpec:
    """Start a new color scale (ggnewscale): alias of ``new_scale_color()``.

    Returns
    -------
    FeatureSpec
        The marker, to add to a plot with ``+``.

    Examples
    --------
    >>> from ggrust import *
    >>> marker = new_scale_colour()
    """
    return new_scale("color")


def new_scale_fill() -> FeatureSpec:
    """Start a new fill scale (ggnewscale): ``new_scale("fill")``.

    Returns
    -------
    FeatureSpec
        The marker, to add to a plot with ``+``.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3], "y": [1, 2, 3], "kind": ["a", "b", "c"]}
    >>> plot = (
    ...     ggplot(data, aes("x", "y"))
    ...     + geom_tile(aes(fill="y"))
    ...     + new_scale_fill()
    ...     + geom_point(aes(fill="kind"), shape=21, size=6)
    ... )
    """
    return new_scale("fill")
