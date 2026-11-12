"""Binned size and alpha scales (ggplot2 only): ``scale_size_binned``, ``scale_alpha_binned``.

Values are cut into bins at the scale's breaks and take the size (or alpha) at the
middle of their bin. The default guide is ``guide_bins()``: the keys touch, with
the break values on their edges. Not in lets-plot.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from ggrust.plot.scale import _scale

if TYPE_CHECKING:
    from collections.abc import Sequence

    from ggrust._typing import Breaks, Guide, Labels, Limits, Number, Oob, Transform
    from ggrust.plot.core import FeatureSpec

__all__ = ["scale_alpha_binned", "scale_size_binned"]


def scale_size_binned(
    range: Sequence[Number] | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    limits: Limits | None = None,
    n_breaks: int | None = None,
    nice_breaks: bool = True,
    right: bool = True,
    show_limits: bool = False,
    oob: Oob = "squish",
    na_value: Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``size`` to binned sizes (ggplot2).

    Each value takes the size at the middle of its bin, so sizes step from bin to
    bin. The legend is ``guide_bins()``: one key per bin, labelled on its edges.

    Parameters
    ----------
    range : list of float, optional
        ``[smallest, largest]`` size, as in ``scale_size`` (default about 1.4..9.5).
    name : str, optional
        Legend title.
    breaks : list, optional
        Bin edges inside the data range (data units). Default: about ``n_breaks``
        generated ones.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    limits : list, optional
        ``[low, high]`` range of the bins. Without limits or ``breaks``, the ends
        move out so the end bins are as wide as the others (ggplot2).
    n_breaks : int, optional
        How many breaks to aim for (default 5).
    nice_breaks : bool, default=True
        Round break values; False spaces ``n_breaks`` breaks evenly over the range.
    right : bool, default=True
        Bins are closed on the right: a value on a break joins the bin below it.
    show_limits : bool, default=False
        Also label the two outer edges of the legend.
    oob : {"squish", "censor"}, default="squish"
        What data outside ``limits`` maps to: the end bins (``"squish"``) or
        ``na_value`` (``"censor"``).
    na_value : float, optional
        Size of missing data.
    guide : str or FeatureSpec, optional
        ``"bins"`` (default), ``"legend"`` (one key per bin labelled with its
        ends, e.g. "20 - 40"), ``"none"``, or a ``guide_bins()`` /
        ``guide_legend()`` spec.
    trans : str, optional
        Transformation of the data, as in ``scale_size``; breaks are generated in
        transformed units.
    format : str, optional
        Label format, e.g. ``".1f"`` or ``","``. ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    SpecError
        If ``oob`` is not ``"squish"`` or ``"censor"``.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"country": ["Iceland", "Norway", "Spain", "Japan", "India"],
    ...         "people": [0.38, 5.5, 48, 124, 1430], "area": [103, 385, 506, 378, 3287]}
    >>> plot = ggplot(data, aes("area", "people", size="area")) + geom_point()
    >>> plot = plot + scale_size_binned(n_breaks=3)
    """
    return _scale("size", **locals(), binned=True)


def scale_alpha_binned(
    range: Sequence[Number] | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    limits: Limits | None = None,
    n_breaks: int | None = None,
    nice_breaks: bool = True,
    right: bool = True,
    show_limits: bool = False,
    oob: Oob = "squish",
    na_value: Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``alpha`` to binned transparency (ggplot2).

    Each value takes the alpha at the middle of its bin. The legend is
    ``guide_bins()``. Options as in ``scale_size_binned``.

    Parameters
    ----------
    range : list of float, optional
        ``[lowest, highest]`` alpha (default 0.1..1).
    name : str, optional
        Legend title.
    breaks : list, optional
        Bin edges inside the data range (data units).
    labels : list of str or dict, optional
        One label per break.
    limits : list, optional
        ``[low, high]`` range of the bins.
    n_breaks : int, optional
        How many breaks to aim for (default 5).
    nice_breaks : bool, default=True
        Round break values.
    right : bool, default=True
        Bins are closed on the right.
    show_limits : bool, default=False
        Also label the two outer edges of the legend.
    oob : {"squish", "censor"}, default="squish"
        What data outside ``limits`` maps to.
    na_value : float, optional
        Alpha of missing data.
    guide : str or FeatureSpec, optional
        ``"bins"`` (default), ``"legend"``, ``"none"``, or a guide spec.
    trans : str, optional
        Transformation of the data.
    format : str, optional
        Label format. ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    SpecError
        If ``oob`` is not ``"squish"`` or ``"censor"``.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3, 4, 5], "y": [2, 4, 3, 5, 1], "score": [3, 18, 41, 66, 97]}
    >>> plot = ggplot(data, aes("x", "y", alpha="score")) + geom_point(size=8)
    >>> plot = plot + scale_alpha_binned()
    """
    return _scale("alpha", **locals(), binned=True)
