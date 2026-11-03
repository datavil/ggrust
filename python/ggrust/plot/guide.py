"""``guides()`` and the guide spec helper.

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

__all__ = ["guide_colorbar", "guide_legend", "guides"]


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
    >>> from ggrust import guide_colorbar, guides
    >>> spec = guides(fill=guide_colorbar(label_position="ends"))
    """
    if label_position is not None and label_position != "ends":
        message = (
            f"guide_colorbar(label_position={label_position!r}) is not supported; "
            f"expected 'ends' or None{did_you_mean(str(label_position), ['ends'])}"
        )
        raise SpecError(message)
    return _guide("colorbar", **locals())
