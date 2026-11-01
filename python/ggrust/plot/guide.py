"""``guides()`` and the guide spec helper.

Ported from lets-plot's ``lets_plot/plot/guide.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from ggrust.errors import SpecError
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

    Returns
    -------
    FeatureSpec
        The guide.
    """
    return _guide("colorbar", **locals())
