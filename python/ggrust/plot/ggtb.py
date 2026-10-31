"""``ggtb()``: the interactive zoom/pan toolbar.

Ported from lets-plot's ``lets_plot/plot/ggtb_.py`` (MIT, see THIRD_PARTY_NOTICES.md).
HTML output gets lets-plot's toolbar (pan, rubber band zoom, centerpoint zoom,
reset); static output (SVG, PNG) ignores it, as lets-plot's does.
"""

import math
from typing import Literal

from ggrust.plot.core import FeatureSpec

__all__ = ["ggtb"]

SIZE_BASES = ("x", "y", "min", "max")


def ggtb(
    size_basis: Literal["x", "y", "min", "max"] | None = None,
    size_zoomin: int | float | None = None,
) -> FeatureSpec:
    """Add the zoom/pan toolbar to an HTML plot.

    The toolbar sits above the plot. With a tool selected, dragging in a panel pans
    or zooms it and the mouse wheel zooms around the cursor; axes and grid lines
    are redrawn for the new range. Reset restores the original view.

    Parameters
    ----------
    size_basis : {"x", "y", "min", "max"}, default="max"
        Which axis' zoom factor point sizes follow (the smaller or larger of both
        for ``"min"`` / ``"max"``).
    size_zoomin : int or float, default=0
        How much points grow when zooming in: 0 keeps their size, -1 grows them
        without limit, a positive number caps the growth factor.

    Returns
    -------
    FeatureSpec
        The toolbar feature (kind ``"ggtoolbar"``).

    Raises
    ------
    ValueError
        If ``size_basis`` is not one of the accepted names, or ``size_zoomin`` is
        not a finite number >= 0 or -1.

    Examples
    --------
    >>> import ggrust as gg
    >>> plot = gg.ggplot({"x": [1, 2, 3], "y": [3, 1, 2]}, gg.aes("x", "y")) + gg.geom_point()
    >>> html = (plot + gg.ggtb(size_zoomin=-1)).to_html()
    """
    if size_basis is not None and size_basis not in SIZE_BASES:
        raise ValueError(
            f"ggtb(size_basis={size_basis!r}): unknown size_basis; "
            "use one of 'x', 'y', 'min', 'max'"
        )
    valid_number = (
        isinstance(size_zoomin, int | float)
        and not isinstance(size_zoomin, bool)
        and math.isfinite(size_zoomin)
        and (size_zoomin >= 0 or size_zoomin == -1)
    )
    if size_zoomin is not None and not valid_number:
        raise ValueError(
            f"ggtb(size_zoomin={size_zoomin!r}): use a finite number >= 0 (largest point growth, "
            "0 keeps sizes) or -1 (no limit)"
        )
    return FeatureSpec("ggtoolbar", name=None, size_basis=size_basis, size_zoomin=size_zoomin)
