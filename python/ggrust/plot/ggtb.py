"""``ggtb()``: the interactive zoom/pan toolbar.

Ported from lets-plot's ``lets_plot/plot/ggtb_.py`` (MIT, see THIRD_PARTY_NOTICES.md).
Static output ignores it, as lets-plot's does; the toolbar arrives in plan.md step 4.3.
"""

from ggrust.plot.core import FeatureSpec

__all__ = ["ggtb"]


def ggtb(size_basis=None, size_zoomin=None) -> FeatureSpec:
    """Add the zoom/pan toolbar to an HTML plot.

    Parameters
    ----------
    size_basis : {"x", "y", "min", "max"}, optional
        Axis whose zoom scales point sizes.
    size_zoomin : int, optional
        How points grow when zooming in: 0 keeps size, -1 grows without limit,
        a positive number caps growth.

    Returns
    -------
    FeatureSpec
        The toolbar feature (kind ``"ggtoolbar"``).
    """
    return FeatureSpec("ggtoolbar", name=None, size_basis=size_basis, size_zoomin=size_zoomin)
