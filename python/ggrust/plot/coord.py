"""Coordinate systems: ``coord_cartesian``, ``coord_fixed`` and ``coord_flip``.

Ported from lets-plot's ``lets_plot/plot/coord.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from ggrust.plot.core import FeatureSpec

if TYPE_CHECKING:
    from collections.abc import Sequence

    from ggrust._typing import Number

__all__ = ["coord_cartesian", "coord_fixed", "coord_flip"]


def coord_cartesian(
    xlim: Sequence[Number | None] | None = None,
    ylim: Sequence[Number | None] | None = None,
    flip: bool = False,
) -> FeatureSpec:
    """Cartesian coordinates; limits zoom without dropping data.

    Parameters
    ----------
    xlim, ylim : list, optional
        ``[low, high]`` in data units (``None`` keeps that side); the axis shows
        exactly this range, data outside is clipped, not removed.
    flip : bool, default False
        Swap x and y.

    Returns
    -------
    FeatureSpec
        The coordinate system.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> plot = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> plot = plot + coord_cartesian(xlim=[3, 4], ylim=[15, 25])
    """
    return FeatureSpec("coord", name="cartesian", xlim=xlim, ylim=ylim, flip=flip)


def coord_fixed(
    ratio: Number = 1.0,
    xlim: Sequence[Number | None] | None = None,
    ylim: Sequence[Number | None] | None = None,
    flip: bool = False,
) -> FeatureSpec:
    """Cartesian coordinates with a fixed aspect ratio.

    Parameters
    ----------
    ratio : float, default 1.0
        Length of one y unit relative to one x unit on screen; the panel shrinks
        to keep it and is centred.
    xlim, ylim : list, optional
        As in ``coord_cartesian``.
    flip : bool, default False
        Swap x and y.

    Returns
    -------
    FeatureSpec
        The coordinate system.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [0, 1, 2, 3, 4], "y": [0, 1, 4, 9, 16]}
    >>> plot = ggplot(data, aes("x", "y")) + geom_line() + coord_fixed(ratio=0.25)
    """
    return FeatureSpec("coord", name="fixed", ratio=ratio, xlim=xlim, ylim=ylim, flip=flip)


def coord_flip(
    xlim: Sequence[Number | None] | None = None, ylim: Sequence[Number | None] | None = None
) -> FeatureSpec:
    """Swap the x and y axes.

    Parameters
    ----------
    xlim, ylim : list, optional
        As in ``coord_cartesian``.

    Returns
    -------
    FeatureSpec
        The coordinate system.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"size": ["S", "M", "L", "XL"], "sold": [40, 65, 52, 18]}
    >>> plot = ggplot(data, aes("size", "sold")) + geom_bar(stat="identity")
    >>> plot = plot + coord_flip()
    """
    return FeatureSpec("coord", name="flip", xlim=xlim, ylim=ylim, flip=True)
