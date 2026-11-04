"""``inset_element()``: draw a plot inside another plot's area (patchwork).

A ggrust extension: lets-plot has no insets. ``plot + inset_element(other, ...)``
keeps ``plot`` a ``PlotSpec`` (or ``SupPlotsSpec``) with the inset in its spec, so
later ``+`` changes ``plot``, not the inset (patchwork changes the inset). The
engine places and draws insets (``crates/ggrust-core/src/grid/inset.rs``).
"""

from __future__ import annotations

import math
from typing import TYPE_CHECKING, Any, Literal

from ggrust.errors import SpecError, did_you_mean
from ggrust.plot.core import FeatureSpec, PlotSpec

if TYPE_CHECKING:
    from ggrust.plot.subplots import SupPlotsSpec

__all__ = ["inset_element"]

ALIGN_TO = ("panel", "plot", "full")
"""Areas ``align_to`` accepts."""


class InsetSpec(FeatureSpec):
    """An inset: a figure and where it goes in its host. Build it with ``inset_element()``."""

    def __init__(self, figure: PlotSpec | SupPlotsSpec, **options: object) -> None:
        super().__init__("inset", name=None, **options)
        self.figure = figure

    def as_dict(self) -> dict[str, Any]:
        """Return the inset options with ``figure`` as the figure's ``as_dict()``.

        Returns
        -------
        dict
            ``figure``, ``left``, ``bottom``, ``right``, ``top``, ``align_to``,
            ``on_top``, ``clip`` and ``ignore_tag``.
        """
        result = super().as_dict()
        result["figure"] = self.figure.as_dict()
        return result


def inset_element(
    plot: PlotSpec | SupPlotsSpec,
    left: float,
    bottom: float,
    right: float,
    top: float,
    align_to: Literal["panel", "plot", "full"] = "panel",
    on_top: bool = True,
    clip: bool = True,
    ignore_tag: bool = False,
) -> InsetSpec:
    """Draw a plot (or grid) inside the area of the plot it is added to.

    A ggrust extension (patchwork ``inset_element()``); add it with ``+``:
    ``main + inset_element(detail, 0.6, 0.6, 1, 1)``. Later ``+`` still changes
    ``main``. Insets of grid plots are numbered by ``gggrid(tag_levels=)`` after
    their host.

    Parameters
    ----------
    plot : PlotSpec or SupPlotsSpec
        The inset figure.
    left, bottom, right, top : float
        Edges as fractions (0..1) of the area ``align_to`` names: 0 is its left or
        bottom edge, 1 its right or top edge.
    align_to : {"panel", "plot", "full"}, default="panel"
        The area: the panels, the plot without its titles, caption, legends, tag
        and margin (panels, axes and axis titles), or the whole figure.
    on_top : bool, default=True
        Draw over the host, or under it (above its background).
    clip : bool, default=True
        Cut the inset at its area.
    ignore_tag : bool, default=False
        Leave the inset out of ``gggrid(tag_levels=)`` numbering.

    Returns
    -------
    InsetSpec
        The inset, to add to a plot or grid with ``+``.

    Raises
    ------
    TypeError
        If ``plot`` is not a plot or grid, or a flag is not a bool.
    SpecError
        If an edge is outside 0..1, the inset is empty (``left >= right`` or
        ``bottom >= top``), or ``align_to`` is unknown.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> main = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> detail = ggplot(data, aes("mpg")) + geom_histogram(bins=4)
    >>> plot = main + inset_element(detail, 0.55, 0.55, 1, 1)
    """
    from ggrust.plot.subplots import SupPlotsSpec

    if not isinstance(plot, PlotSpec | SupPlotsSpec):
        message = (
            f"inset_element(): plot must be a plot (ggplot()) or a grid (gggrid()), "
            f"got {type(plot).__name__}"
        )
        raise TypeError(message)
    edges = {"left": left, "bottom": bottom, "right": right, "top": top}
    for name, value in edges.items():
        if isinstance(value, bool) or not isinstance(value, int | float):
            message = f"inset_element(): {name} must be a number, got {type(value).__name__}"
            raise TypeError(message)
        if not (math.isfinite(value) and 0 <= value <= 1):
            message = (
                f"inset_element(): {name}={value!r} is outside 0..1; edges are fractions "
                f"of the {align_to} area, 0 at its left or bottom, 1 at its right or top"
            )
            raise SpecError(message, path=f"insets.{name}", reason=message)
    if left >= right or bottom >= top:
        message = (
            f"inset_element(): the inset is empty; expected left < right and bottom < "
            f"top, got left={left}, right={right}, bottom={bottom}, top={top}"
        )
        raise SpecError(message, path="insets", reason=message)
    if align_to not in ALIGN_TO:
        message = (
            f"inset_element(): align_to must be 'panel', 'plot' or 'full', got "
            f"{align_to!r}{did_you_mean(str(align_to), ALIGN_TO)}"
        )
        raise SpecError(message, path="insets.align_to", reason=message)
    flags = {"on_top": on_top, "clip": clip, "ignore_tag": ignore_tag}
    for name, value in flags.items():
        if not isinstance(value, bool):
            message = f"inset_element(): {name} must be True or False, got {value!r}"
            raise TypeError(message)
    return InsetSpec(plot, align_to=align_to, **edges, **flags)
