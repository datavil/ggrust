"""``guides()``, the guide spec helpers and ``layer_key()`` (manual legend keys).

Ported from lets-plot's ``lets_plot/plot/guide.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Literal

from ggrust.errors import SpecError, did_you_mean
from ggrust.plot.core import FeatureSpec

if TYPE_CHECKING:
    from collections.abc import Mapping
    from typing import Any

    from ggrust._typing import Number

__all__ = ["guide_colorbar", "guide_legend", "guides", "layer_key"]


def _guide(name: str | None, **kwargs: Any) -> FeatureSpec:
    if "title" in kwargs and isinstance(kwargs["title"], int):
        message = (
            "guide_*(): only 'title' may be passed by position; pass the other options by "
            "keyword (e.g. guide_legend('Title', ncol=2))"
        )
        raise SpecError(message)
    return FeatureSpec("guide", name=name, **kwargs)


def guides(**kwargs: object) -> FeatureSpec:
    """Set guides (legends, colorbars) per aesthetic.

    Parameters
    ----------
    **kwargs
        Aesthetic name to guide (``"none"``, ``guide_legend()``, ...).

    Returns
    -------
    FeatureSpec
        The guides feature.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + guides(color=guide_legend(title="Animal", ncol=3))
    """
    return FeatureSpec("guides", name=None, **kwargs)


def guide_legend(
    title: str | None = None,
    *,
    nrow: int | None = None,
    ncol: int | None = None,
    byrow: bool | None = None,
    override_aes: Mapping[str, Any] | None = None,
) -> FeatureSpec:
    """Show a key legend for an aesthetic.

    Parameters
    ----------
    title : str, optional
        Legend title.
    nrow, ncol : int, optional
        Number of rows or columns of legend keys.
    byrow : bool, optional
        True fills the keys row by row instead of column by column.
    override_aes : dict, optional
        Constant aesthetics for the legend keys only, e.g. ``{"size": 5}``.

    Returns
    -------
    FeatureSpec
        The guide.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + guides(color=guide_legend(nrow=1, override_aes={"size": 8}))
    """
    return _guide("legend", **locals())


def guide_colorbar(
    title: str | None = None,
    *,
    barwidth: Number | None = None,
    barheight: Number | None = None,
    nbin: int | None = None,
    label_position: Literal["ends"] | None = None,
) -> FeatureSpec:
    """Show a colorbar for a continuous color or fill scale.

    Parameters
    ----------
    title : str, optional
        Colorbar title.
    barwidth, barheight : float, optional
        Bar size in pixels.
    nbin : int, optional
        Number of color segments (default 20).
    label_position : {"ends"}, optional
        ``"ends"`` draws only the labels of the lowest and highest breaks, beside
        the bar's ends: left and right of a horizontal bar (``RNA  Min ▬ Max``),
        below and above a vertical one. Other breaks keep their ticks. The default
        (None) puts every label below a horizontal bar or right of a vertical one.
        A ggrust option; lets-plot and ggplot2 have no equivalent.

    Returns
    -------
    FeatureSpec
        The guide.

    Raises
    ------
    SpecError
        If ``label_position`` is not ``"ends"`` or None.

    Examples
    --------
    >>> from ggrust import *
    >>> spec = guides(fill=guide_colorbar(label_position="ends"))
    """
    if label_position is not None and label_position != "ends":
        message = (
            f"guide_colorbar(label_position={label_position!r}) is not supported; "
            f"expected 'ends' or None{did_you_mean(str(label_position), ['ends'])}"
        )
        raise SpecError(message)
    return _guide("colorbar", **locals())


def layer_key(
    label: str,
    group: str | None = None,
    *,
    index: int | None = None,
    **kwargs: Any,
) -> FeatureSpec:
    """Describe a layer's entry in a manual legend (``manual_key=`` of any geom).

    The key is drawn by the layer's geom with its constant aesthetics, then the
    values given here. Layers with the same ``group`` share a legend; a label used
    by several layers of one group becomes one key drawing all of them. The key
    shows even when the layer has ``show_legend=False``.

    Parameters
    ----------
    label : str
        Text next to the key.
    group : str, default="manual"
        Legend the key joins. The ``"manual"`` legend has no title; give one with
        ``labs(manual="Title")`` or ``guides(manual=guide_legend("Title"))``. Other
        groups are titled by their name unless ``guides(<group>=...)`` sets one.
    index : int, optional
        Position of the key in its legend; keys with an index come first, in index
        order, then the others in layer order.
    **kwargs
        Aesthetic values for the key only, e.g. ``size=3`` or ``shape=21``.

    Returns
    -------
    FeatureSpec
        The key specification, for a geom's ``manual_key``.

    Raises
    ------
    TypeError
        When ``label`` is not a string.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"day": [1, 2, 3, 4, 5], "steps": [6.1, 8.4, 7.2, 9.9, 8.8]}
    >>> plot = (
    ...     ggplot(data, aes("day", "steps"))
    ...     + geom_line(color="grey", manual_key=layer_key("trend", linetype="solid"))
    ...     + geom_point(color="firebrick", size=4, manual_key=layer_key("measured"))
    ...     + labs(manual="Steps (thousands)")
    ... )
    """
    if not isinstance(label, str):
        message = f"layer_key(): label must be a string, got {type(label).__name__}"
        raise TypeError(message)
    return FeatureSpec("layer_key", name=None, label=label, group=group, index=index, **kwargs)
