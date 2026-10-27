"""Position adjustments: ``position_dodge``, ``position_jitter``, ...

Ported from lets-plot's ``lets_plot/plot/pos.py`` (MIT, see THIRD_PARTY_NOTICES.md).
The engine applies ``identity`` only so far; the others build lets_plot-identical
specs and rendering reports them (plan.md Phase 2).
"""

from ggrust.plot.core import FeatureSpec

__all__ = [
    "position_dodge",
    "position_fill",
    "position_jitter",
    "position_jitterdodge",
    "position_nudge",
    "position_stack",
]


def position_dodge(width=None) -> FeatureSpec:
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


def position_jitter(width=None, height=None, seed=None) -> FeatureSpec:
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


def position_nudge(x=None, y=None, unit=None) -> FeatureSpec:
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
    dodge_width=None, jitter_width=None, jitter_height=None, seed=None
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


def position_stack(vjust=None, mode=None) -> FeatureSpec:
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


def position_fill(vjust=None, mode=None) -> FeatureSpec:
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


def _position(name, **options) -> FeatureSpec:
    return FeatureSpec("pos", name=name, **options)
