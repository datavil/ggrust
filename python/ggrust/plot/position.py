"""Position adjustments: ``position_dodge``, ``position_jitter``, ...

Ported from lets-plot's ``lets_plot/plot/pos.py`` (MIT, see THIRD_PARTY_NOTICES.md).
The specs are lets_plot-identical; the engine applies every adjustment listed here.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from ggrust.plot.core import FeatureSpec

if TYPE_CHECKING:
    from typing import Any

    from ggrust._typing import Number


__all__ = [
    "position_dodge",
    "position_fill",
    "position_jitter",
    "position_jitterdodge",
    "position_nudge",
    "position_stack",
]


def position_dodge(width: Number | None = None) -> FeatureSpec:
    """Place overlapping objects side by side.

    Parameters
    ----------
    width : float, optional
        Dodging width, as a share of the band.

    Returns
    -------
    FeatureSpec
        The position adjustment.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"month": ["Jan", "Jan", "Feb", "Feb", "Mar", "Mar"],
    ...         "shop": ["north", "south", "north", "south", "north", "south"],
    ...         "sales": [12, 9, 15, 11, 14, 16]}
    >>> plot = ggplot(data, aes("month", "sales", fill="shop")) + geom_bar(
    ...     stat="identity", position=position_dodge(width=0.8), width=0.7
    ... )
    """
    return _position("dodge", width=width)


def position_jitter(
    width: Number | None = None, height: Number | None = None, seed: int | None = None
) -> FeatureSpec:
    """Add random noise to positions.

    Parameters
    ----------
    width, height : float, optional
        Jitter range, as a share of the resolution of the data.
    seed : int, optional
        Random seed.

    Returns
    -------
    FeatureSpec
        The position adjustment.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"rating": [1, 2, 2, 3, 3, 3, 4, 4, 5] * 2, "app": ["web"] * 9 + ["mobile"] * 9}
    >>> plot = ggplot(data, aes("app", "rating")) + geom_point(
    ...     position=position_jitter(width=0.2, height=0, seed=42)
    ... )
    """
    return _position("jitter", width=width, height=height, seed=seed)


def position_nudge(
    x: Number | None = None, y: Number | None = None, unit: str | None = None
) -> FeatureSpec:
    """Shift positions by a fixed offset.

    Parameters
    ----------
    x, y : float, optional
        Offsets.
    unit : {"identity", "size", "px"}, optional
        Unit of the offsets.

    Returns
    -------
    FeatureSpec
        The position adjustment.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"size": ["S", "M", "L", "XL"], "sold": [40, 65, 52, 18]}
    >>> plot = (
    ...     ggplot(data, aes("size", "sold"))
    ...     + geom_bar(stat="identity")
    ...     + geom_text(aes(label="sold"), position=position_nudge(y=3))
    ... )
    """
    return _position("nudge", x=x, y=y, unit=unit)


def position_jitterdodge(
    dodge_width: Number | None = None,
    jitter_width: Number | None = None,
    jitter_height: Number | None = None,
    seed: int | None = None,
) -> FeatureSpec:
    """Dodge, then jitter (points over dodged boxplots).

    Parameters
    ----------
    dodge_width : float, optional
        Dodging width.
    jitter_width, jitter_height : float, optional
        Jitter range.
    seed : int, optional
        Random seed.

    Returns
    -------
    FeatureSpec
        The position adjustment.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"team": ["red"] * 6 + ["blue"] * 6, "sex": ["f", "m"] * 6,
    ...         "height": [168, 172, 175, 169, 181, 177, 160, 165, 171, 158, 166, 163]}
    >>> plot = (
    ...     ggplot(data, aes("team", "height", color="sex"))
    ...     + geom_boxplot()
    ...     + geom_point(position=position_jitterdodge(jitter_width=0.2, seed=1))
    ... )
    """
    return _position(
        "jitterdodge",
        dodge_width=dodge_width,
        jitter_width=jitter_width,
        jitter_height=jitter_height,
        seed=seed,
    )


def position_stack(
    vjust: Number | None = None, mode: str | None = None, reverse: bool | None = None
) -> FeatureSpec:
    """Stack overlapping objects on top of each other.

    Parameters
    ----------
    vjust : float, optional
        Vertical anchor within each stacked object.
    mode : {"groups", "all"}, optional
        ``"groups"`` stacks groups only: rows of one group at the same x overlap
        and the group adds its largest value. ``"all"`` stacks every row. By
        default bars with ``stat="identity"`` use ``"all"``, as ggplot2, and
        everything else ``"groups"``, as lets-plot.
    reverse : bool, default=False
        True stacks the groups in the opposite order (the first group on top), as
        ggplot2. Not in lets_plot.

    Returns
    -------
    FeatureSpec
        The position adjustment.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"month": ["Jan", "Jan", "Feb", "Feb", "Mar", "Mar"],
    ...         "shop": ["north", "south", "north", "south", "north", "south"],
    ...         "sales": [12, 9, 15, 11, 14, 16]}
    >>> plot = ggplot(data, aes("month", "sales", fill="shop")) + geom_bar(
    ...     stat="identity", position=position_stack()
    ... )
    """
    return _position("stack", vjust=vjust, mode=mode, reverse=reverse)


def position_fill(
    vjust: Number | None = None, mode: str | None = None, reverse: bool | None = None
) -> FeatureSpec:
    """Stack overlapping objects and normalise each stack to height 1.

    Parameters
    ----------
    vjust : float, optional
        Vertical anchor within each stacked object.
    mode : {"groups", "all"}, optional
        ``"groups"`` stacks groups only: rows of one group at the same x overlap
        and the group adds its largest value. ``"all"`` stacks every row. By
        default bars with ``stat="identity"`` use ``"all"``, as ggplot2, and
        everything else ``"groups"``, as lets-plot.
    reverse : bool, default=False
        True stacks the groups in the opposite order (the first group on top), as
        ggplot2. Not in lets_plot.

    Returns
    -------
    FeatureSpec
        The position adjustment.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"month": ["Jan", "Jan", "Feb", "Feb", "Mar", "Mar"],
    ...         "shop": ["north", "south", "north", "south", "north", "south"],
    ...         "sales": [12, 9, 15, 11, 14, 16]}
    >>> plot = ggplot(data, aes("month", "sales", fill="shop")) + geom_bar(
    ...     stat="identity", position=position_fill()
    ... )
    """
    return _position("fill", vjust=vjust, mode=mode, reverse=reverse)


def _position(name: str, **options: Any) -> FeatureSpec:
    return FeatureSpec("pos", name=name, **options)
