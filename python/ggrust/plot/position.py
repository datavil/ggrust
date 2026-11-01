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
    """
    return _position(
        "jitterdodge",
        dodge_width=dodge_width,
        jitter_width=jitter_width,
        jitter_height=jitter_height,
        seed=seed,
    )


def position_stack(vjust: Number | None = None, mode: str | None = None) -> FeatureSpec:
    """Stack overlapping objects on top of each other.

    Parameters
    ----------
    vjust : float, optional
        Vertical anchor within each stacked object.
    mode : {"groups", "all"}, optional
        What gets stacked.

    Returns
    -------
    FeatureSpec
        The position adjustment.
    """
    return _position("stack", vjust=vjust, mode=mode)


def position_fill(vjust: Number | None = None, mode: str | None = None) -> FeatureSpec:
    """Stack overlapping objects and normalise each stack to height 1.

    Parameters
    ----------
    vjust : float, optional
        Vertical anchor within each stacked object.
    mode : {"groups", "all"}, optional
        What gets stacked.

    Returns
    -------
    FeatureSpec
        The position adjustment.
    """
    return _position("fill", vjust=vjust, mode=mode)


def _position(name: str, **options: Any) -> FeatureSpec:
    return FeatureSpec("pos", name=name, **options)
