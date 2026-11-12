"""``annotation_logticks()``: log tick marks along the panel edges (ggplot2).

Not in lets-plot. Drawn by the engine's ``logticks`` geom once per panel.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from ggrust.errors import SpecError
from ggrust.plot.geom import _geom

if TYPE_CHECKING:
    from ggrust._typing import Color, Number
    from ggrust.plot.core import LayerSpec

__all__ = ["annotation_logticks"]


def annotation_logticks(
    base: Number = 10,
    sides: str = "bl",
    outside: bool = False,
    scaled: bool = True,
    short: Number | None = None,
    mid: Number | None = None,
    long: Number | None = None,
    color: Color | None = None,
    size: Number | None = None,
    linetype: str | int | None = None,
    alpha: Number | None = None,
) -> LayerSpec:
    """Add log tick marks along the panel edges (ggplot2 ``annotation_logticks``).

    For every power of ``base`` in the axis range, ticks at 1, 2, ..., 9 times
    it: long at 1, mid at 5 (base 10), short at the rest. Use it with a log
    axis: ``scale_x_log10()`` (``scaled=True``, the default) or
    ``coord_trans(x="log10")`` (``scaled=False``).

    Parameters
    ----------
    base : float, default=10
        The log base.
    sides : str, default="bl"
        Edges to draw on: any of ``"t"`` (top), ``"r"`` (right), ``"b"``
        (bottom), ``"l"`` (left).
    outside : bool, default=False
        Draw the ticks outside the panel, pointing away from it.
    scaled : bool, default=True
        True when the axis is in log units (a log scale, or data logged by
        hand); False when it shows raw values (``coord_trans()``).
    short, mid, long : float, default=3.8, 7.6, 11.3
        Tick lengths in pixels (ggplot2's 0.1, 0.2 and 0.3 cm at 96 dpi).
    color : str, default="black"
        Tick color.
    size : float, default=0.5
        Tick line width, as ``size`` of ``geom_line``.
    linetype : str or int, optional
        Tick line type.
    alpha : float, optional
        Tick opacity.

    Returns
    -------
    LayerSpec
        The annotation layer.

    Raises
    ------
    SpecError
        If ``sides`` holds a letter other than t, r, b, l.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"area": [1, 3, 10, 40, 120, 900], "people": [2, 5, 9, 30, 70, 500]}
    >>> plot = ggplot(data, aes("area", "people")) + geom_point()
    >>> plot = plot + scale_x_log10() + scale_y_log10() + annotation_logticks()
    """
    if not isinstance(sides, str) or not sides or any(side not in "trbl" for side in sides):
        reason = "expected some of 't', 'r', 'b', 'l', e.g. 'bl' for bottom and left"
        raise SpecError(
            f"annotation_logticks(sides={sides!r}): {reason}", path="sides", reason=reason
        )
    return _geom(
        "logticks",
        inherit_aes=False,
        show_legend=False,
        base=base,
        sides=sides,
        outside=outside,
        scaled=scaled,
        short=short,
        mid=mid,
        long=long,
        color=color,
        size=size,
        linetype=linetype,
        alpha=alpha,
    )
