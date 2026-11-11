"""Coordinate systems: cartesian, fixed, flip, polar and 3D.

Ported from lets-plot's ``lets_plot/plot/coord.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from __future__ import annotations

import math
from typing import TYPE_CHECKING, Literal

from ggrust.errors import SpecError, did_you_mean
from ggrust.plot.core import FeatureSpec

if TYPE_CHECKING:
    from collections.abc import Sequence

    from ggrust._typing import Number, Transform

__all__ = [
    "coord_3d",
    "coord_cartesian",
    "coord_fixed",
    "coord_flip",
    "coord_polar",
    "coord_trans",
    "coord_transform",
]

_COORD_TRANSFORMS: tuple[str, ...] = (
    "identity",
    "log10",
    "log2",
    "log",
    "log1p",
    "exp",
    "logit",
    "probit",
    "reciprocal",
    "pseudo_log",
    "symlog",
    "sqrt",
    "reverse",
)


def coord_cartesian(
    xlim: Sequence[Number | None] | None = None,
    ylim: Sequence[Number | None] | None = None,
    flip: bool = False,
    expand: bool | None = None,
) -> FeatureSpec:
    """Cartesian coordinates; limits zoom without dropping data.

    Parameters
    ----------
    xlim, ylim : list, optional
        ``[low, high]`` in data units (``None`` keeps that side); the axis shows
        exactly this range, data outside is clipped, not removed.
    flip : bool, default False
        Swap x and y.
    expand : bool, default=True
        False drops the padding around the data on both axes, whatever the scales
        say, as ggplot2. Not in lets_plot.

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
    return FeatureSpec("coord", name="cartesian", xlim=xlim, ylim=ylim, flip=flip, expand=expand)


def coord_fixed(
    ratio: Number = 1.0,
    xlim: Sequence[Number | None] | None = None,
    ylim: Sequence[Number | None] | None = None,
    flip: bool = False,
    expand: bool | None = None,
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
    expand : bool, default=True
        As in ``coord_cartesian``. Not in lets_plot.

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
    return FeatureSpec(
        "coord", name="fixed", ratio=ratio, xlim=xlim, ylim=ylim, flip=flip, expand=expand
    )


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


def coord_polar(
    xlim: Sequence[Number | None] | None = None,
    ylim: Sequence[Number | None] | None = None,
    theta: Literal["x", "y"] | None = None,
    start: Number | None = None,
    direction: Literal[1, -1] | None = None,
    transform_bkgr: bool | None = None,
) -> FeatureSpec:
    """Bend the panel into a circle, for pie, donut, rose and radar charts.

    Parameters
    ----------
    xlim, ylim : list, optional
        As in ``coord_cartesian``.
    theta : {"x", "y"}, default="x"
        The aesthetic that runs around the circle; the other is the radius.
    start : float, default=0
        Offset from 12 o'clock, in radians.
    direction : {1, -1}, default=1
        1 runs clockwise, -1 counter-clockwise.
    transform_bkgr : bool, default=True
        Draw the panel background and border as a circle, else as a rectangle.

    Returns
    -------
    FeatureSpec
        The coordinate system.

    Notes
    -----
    HTML output is static: no tooltips, zoom or interactive legend. ``geom_raster``,
    ``geom_imshow`` and side panels cannot be bent and raise ``SpecError``.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"part": ["a", "b", "c"], "share": [50, 30, 20], "all": ["", "", ""]}
    >>> plot = ggplot(data, aes("all", "share", fill="part")) + geom_bar(stat="identity", width=1)
    >>> plot = plot + coord_polar(theta="y")
    """
    return FeatureSpec(
        "coord",
        name="polar",
        xlim=xlim,
        ylim=ylim,
        theta=theta,
        start=start,
        direction=direction,
        transform_bkgr=transform_bkgr,
    )


def coord_3d(
    azimuth: Number = -60.0,
    elevation: Number = 30.0,
    perspective: bool = False,
) -> FeatureSpec:
    """Show x, y and z in a static 3D view (ggrust only: lets-plot and ggplot2 have no 3D).

    Map ``z`` in ``aes()``. ``geom_point`` and ``geom_path`` draw in 3D; any other
    geom raises ``SpecError``. Points (and paths, as whole lines) are drawn back
    to front. The box is 4:4:3 and the default view is matplotlib's. Legends,
    themes and facets work as in 2D; HTML output has no rotation, tooltips or zoom.

    Parameters
    ----------
    azimuth : float, default -60.0
        View angle around the z axis, in degrees (matplotlib ``azim``).
    elevation : float, default 30.0
        View angle above the x-y plane, in degrees (matplotlib ``elev``).
    perspective : bool, default False
        Perspective projection (the far side looks smaller) instead of
        orthographic.

    Returns
    -------
    FeatureSpec
        The coordinate system.

    Raises
    ------
    SpecError
        If ``azimuth`` or ``elevation`` is not a finite number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "length": [5.1, 4.9, 6.4, 6.9, 6.3, 5.8],
    ...     "width": [3.5, 3.0, 3.2, 3.1, 3.3, 2.7],
    ...     "petal": [1.4, 1.4, 4.5, 4.9, 6.0, 5.1],
    ...     "species": ["A", "A", "B", "B", "C", "C"],
    ... }
    >>> plot = ggplot(data, aes("length", "width", z="petal", color="species")) + geom_point()
    >>> plot = plot + coord_3d(azimuth=-50, elevation=20)
    """
    for key, angle in (("azimuth", azimuth), ("elevation", elevation)):
        if (
            isinstance(angle, bool)
            or not isinstance(angle, int | float)
            or not math.isfinite(angle)
        ):
            raise SpecError(
                f"coord_3d({key}=...): expected a finite angle in degrees, got {angle!r}"
            )
    return FeatureSpec(
        "coord", name="3d", azimuth=azimuth, elevation=elevation, perspective=perspective
    )


def coord_transform(
    x: Transform | None = None,
    y: Transform | None = None,
    xlim: Sequence[Number | None] | None = None,
    ylim: Sequence[Number | None] | None = None,
    expand: bool | None = None,
) -> FeatureSpec:
    """Cartesian coordinates with transformed axes (ggplot2).

    Unlike a scale transform (``scale_x_log10()``), the transform applies after
    the stats: a histogram is binned in data units, a smooth is fitted in data
    units, and the breaks stay evenly spaced in data units while the axis is
    warped. Lines are drawn straight between their warped points (ggplot2 bends
    them). HTML output does not zoom.

    Parameters
    ----------
    x, y : str, optional
        Axis transform: ``"identity"`` (default), ``"log10"``, ``"log2"``,
        ``"log"``, ``"log1p"``, ``"exp"``, ``"logit"``, ``"probit"``,
        ``"reciprocal"``, ``"pseudo_log"``, ``"symlog"``, ``"sqrt"`` or
        ``"reverse"``.
    xlim, ylim : list, optional
        ``[low, high]`` in data units, as in ``coord_cartesian``.
    expand : bool, default=True
        False drops the padding around the data on both axes.

    Returns
    -------
    FeatureSpec
        The coordinate system.

    Raises
    ------
    SpecError
        If ``x`` or ``y`` is not a transform name.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"area": [1, 3, 10, 40, 120, 900], "people": [2, 5, 9, 30, 70, 500]}
    >>> plot = ggplot(data, aes("area", "people")) + geom_point()
    >>> plot = plot + coord_transform(x="log10", y="log10")
    """
    for key, name in (("x", x), ("y", y)):
        if name is not None and (not isinstance(name, str) or name not in _COORD_TRANSFORMS):
            message = (
                f"coord_transform({key}={name!r}): expected a transform name, one of "
                f"{', '.join(_COORD_TRANSFORMS)}{did_you_mean(str(name), list(_COORD_TRANSFORMS))}"
            )
            raise SpecError(message)
    return FeatureSpec("coord", name="trans", x=x, y=y, xlim=xlim, ylim=ylim, expand=expand)


def coord_trans(
    x: Transform | None = None,
    y: Transform | None = None,
    xlim: Sequence[Number | None] | None = None,
    ylim: Sequence[Number | None] | None = None,
    expand: bool | None = None,
) -> FeatureSpec:
    """Cartesian coordinates with transformed axes: ``coord_transform`` (ggplot2's older name).

    Parameters
    ----------
    x, y : str, optional
        Axis transform, as in ``coord_transform``.
    xlim, ylim : list, optional
        ``[low, high]`` in data units.
    expand : bool, default=True
        False drops the padding around the data on both axes.

    Returns
    -------
    FeatureSpec
        The coordinate system.

    Raises
    ------
    SpecError
        If ``x`` or ``y`` is not a transform name.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"area": [1, 3, 10, 40, 120, 900], "people": [2, 5, 9, 30, 70, 500]}
    >>> plot = ggplot(data, aes("area", "people")) + geom_point() + coord_trans(y="sqrt")
    """
    return coord_transform(x=x, y=y, xlim=xlim, ylim=ylim, expand=expand)
