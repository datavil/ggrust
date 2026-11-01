"""``expand_limits()``: widen scales to include given values.

Ported from lets-plot's ``lets_plot/plot/expand_limits_.py`` (MIT, see
THIRD_PARTY_NOTICES.md): a ``geom_blank`` layer whose mapping holds the values.
"""

from __future__ import annotations

from collections.abc import Iterable
from datetime import date, datetime
from typing import TYPE_CHECKING, Any

from ggrust.plot.core import aes
from ggrust.plot.geom import geom_blank

if TYPE_CHECKING:
    from ggrust.plot.core import LayerSpec

__all__ = ["expand_limits"]

#: One value an axis or legend should include.
LimitValue = float | str | bool | date | datetime | None
#: One value, or several (a list, tuple, range, array, Series or other iterable).
LimitValues = LimitValue | Iterable[LimitValue]


def expand_limits(
    *,
    x: LimitValues = None,
    y: LimitValues = None,
    size: LimitValues = None,
    color: LimitValues = None,
    fill: LimitValues = None,
    alpha: LimitValues = None,
    shape: LimitValues = None,
) -> LayerSpec:
    """Widen the scales of the plot so they include the given values.

    Each argument is one value or a list of values for that aesthetic, e.g.
    ``expand_limits(y=0)`` to show the y axis from zero. The values are drawn by an
    invisible ``geom_blank`` layer, so they train the scales and nothing else.

    Parameters
    ----------
    x : value or list, optional
        Values the x scale must include.
    y : value or list, optional
        Values the y scale must include.
    size : value or list, optional
        Values the size scale must include.
    color : value or list, optional
        Values the color scale must include.
    fill : value or list, optional
        Values the fill scale must include.
    alpha : value or list, optional
        Values the alpha scale must include.
    shape : value or list, optional
        Values the shape scale must include.

    Returns
    -------
    LayerSpec
        A ``geom_blank`` layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [-3, 0, 1], "y": [2, 3, -1]}
    >>> plot = ggplot(data, aes("x", "y")) + geom_point() + expand_limits(x=-10, y=[-5, 5])
    """
    values = {
        "x": x,
        "y": y,
        "size": size,
        "color": color,
        "fill": fill,
        "alpha": alpha,
        "shape": shape,
    }

    def as_list(value: object) -> list[Any]:
        # Array-likes (numpy, polars, pandas, sets, generators) become plain lists;
        # lets_plot keeps them as one item, which its engine cannot read.
        if isinstance(value, str | bytes) or not isinstance(value, Iterable):
            return [value]
        to_list = getattr(value, "tolist", None)
        items = to_list() if callable(to_list) else list(value)
        return items if isinstance(items, list) else [items]

    listed = {name: as_list(value) for name, value in values.items()}
    # lets_plot keeps x and y even when not given.
    kept = {
        name: items
        for name, items in listed.items()
        if name in ("x", "y") or not all(item is None for item in items)
    }
    length = max(len(items) for items in kept.values())
    padded = {name: items + [None] * (length - len(items)) for name, items in kept.items()}
    return geom_blank(mapping=aes(**padded))
