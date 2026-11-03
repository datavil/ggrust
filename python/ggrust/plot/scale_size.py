"""Size and alpha scales, with their ``_manual`` variants.

Ported from lets-plot's ``lets_plot/plot/scale.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from ggrust.plot.core import FeatureSpec
from ggrust.plot.scale import _scale
from ggrust.plot.scale_color import _manual

if TYPE_CHECKING:
    from collections.abc import Sequence
    from typing import Any

    from ggrust._typing import Breaks, Color, Guide, Labels, Limits, Number, Transform

__all__ = [
    "scale_alpha",
    "scale_alpha_manual",
    "scale_size",
    "scale_size_area",
    "scale_size_manual",
]


def scale_size(
    range: Sequence[Number] | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Sizes for ``size``: data mapped linearly onto ``range``.

    Parameters
    ----------
    range : list of float, optional
        ``[smallest, largest]`` size (default about 1.4..9.5, 3..21 px circles).
    trans : {'identity', 'log10', 'log2', 'sqrt', 'reverse'}, optional
        Map (and pick legend breaks) in transformed units.
    name : str, optional
        Legend title.
    breaks : list, optional
        Values that get a legend key or a colorbar tick.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        ``[low, high]`` data range of a continuous scale, or the levels (and their
        order) of a discrete one. Data outside the limits gets ``na_value``.
    na_value : float, optional
        Value drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"colorbar"`` (continuous color), ``"none"``, or a
        ``guide_legend()`` / ``guide_colorbar()`` spec.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"country": ["Iceland", "Norway", "Spain", "Japan", "India"],
    ...         "people": [0.38, 5.5, 48, 124, 1430], "area": [103, 385, 506, 378, 3287]}
    >>> plot = ggplot(data, aes("area", "people", size="people")) + geom_point()
    >>> plot = plot + scale_size(range=[2, 12])
    """
    return _scale("size", **locals())


def scale_size_area(
    max_size: Number | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Sizes whose circle area is proportional to the data.

    0 is size 0 and the largest value ``max_size``; a value twice as large draws a
    circle with twice the area (as ggplot2; lets-plot scales the diameter instead).

    Parameters
    ----------
    max_size : float, optional
        Size of the largest value (default about 9.5, a 21 px circle).
    name : str, optional
        Legend title.
    breaks : list, optional
        Values that get a legend key or a colorbar tick.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        ``[low, high]`` data range of a continuous scale, or the levels (and their
        order) of a discrete one. Data outside the limits gets ``na_value``.
    na_value : float, optional
        Value drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"colorbar"`` (continuous color), ``"none"``, or a
        ``guide_legend()`` / ``guide_colorbar()`` spec.
    trans : {"identity", "log10", "log2", "sqrt", "reverse"}, optional
        Transformation of the data.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"country": ["Iceland", "Norway", "Spain", "Japan", "India"],
    ...         "people": [0.38, 5.5, 48, 124, 1430], "area": [103, 385, 506, 378, 3287]}
    >>> plot = ggplot(data, aes("area", "people", size="people")) + geom_point()
    >>> plot = plot + scale_size_area(max_size=15)
    """
    return _scale("size", scale_mapper_kind="size_area", **locals())


def scale_alpha(
    range: Sequence[Number] | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Transparency for ``alpha``: data mapped linearly onto ``range``.

    Parameters
    ----------
    range : list of float, optional
        ``[lowest, highest]`` alpha (default 0.1..1).
    name : str, optional
        Legend title.
    breaks : list, optional
        Values that get a legend key or a colorbar tick.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        ``[low, high]`` data range of a continuous scale, or the levels (and their
        order) of a discrete one. Data outside the limits gets ``na_value``.
    na_value : float, optional
        Value drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"colorbar"`` (continuous color), ``"none"``, or a
        ``guide_legend()`` / ``guide_colorbar()`` spec.
    trans : {"identity", "log10", "log2", "sqrt", "reverse"}, optional
        Transformation of the data.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"country": ["Iceland", "Norway", "Spain", "Japan", "India"],
    ...         "people": [0.38, 5.5, 48, 124, 1430], "area": [103, 385, 506, 378, 3287]}
    >>> plot = ggplot(data, aes("area", "people", alpha="people")) + geom_point(size=8)
    >>> plot = plot + scale_alpha(range=[0.2, 1])
    """
    return _scale("alpha", **locals())


def scale_size_manual(
    values: Sequence[Any] | dict[Any, Any],
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Your own sizes for ``size``, matched in order with the categories (or a dict).

    Parameters
    ----------
    values : list of Any or dict
        Values of the levels: a list, or a dict of levels to values.
    name : str, optional
        Title of the scale, shown on the axis or the legend.
    breaks : list or dict, optional
        Positions of the ticks or legend keys: a list, or a dict of labels to values.
    labels : list or dict, optional
        Texts at the breaks: a list, or a dict of break values to texts.
    lablim : int, optional
        Maximum number of characters of a label; longer labels are cut.
    limits : list, optional
        Data range of the scale: ``[low, high]`` for continuous scales (None keeps a side), or the
        levels of a discrete scale.
    na_value : str or float, optional
        Value used for missing data.
    guide : str or FeatureSpec, optional
        Guide: ``"legend"``, ``"colorbar"``, ``"none"``, or a ``guide_*()`` spec.
    format : str, optional
        Format pattern of the labels, e.g. ``".2f"`` or ``"{d} %"``.

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", size="pet")) + geom_point()
    >>> plot = plot + scale_size_manual(values=[3, 6, 9])
    """
    return _manual("size", **locals())


def scale_alpha_manual(
    values: Sequence[Any] | dict[Any, Any],
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Your own alphas for ``alpha``, matched in order with the categories (or a dict).

    Parameters
    ----------
    values : list of Any or dict
        Values of the levels: a list, or a dict of levels to values.
    name : str, optional
        Title of the scale, shown on the axis or the legend.
    breaks : list or dict, optional
        Positions of the ticks or legend keys: a list, or a dict of labels to values.
    labels : list or dict, optional
        Texts at the breaks: a list, or a dict of break values to texts.
    lablim : int, optional
        Maximum number of characters of a label; longer labels are cut.
    limits : list, optional
        Data range of the scale: ``[low, high]`` for continuous scales (None keeps a side), or the
        levels of a discrete scale.
    na_value : str or float, optional
        Value used for missing data.
    guide : str or FeatureSpec, optional
        Guide: ``"legend"``, ``"colorbar"``, ``"none"``, or a ``guide_*()`` spec.
    format : str, optional
        Format pattern of the labels, e.g. ``".2f"`` or ``"{d} %"``.

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", alpha="pet")) + geom_point(size=6)
    >>> plot = plot + scale_alpha_manual(values=[1, 0.5, 0.2])
    """
    return _manual("alpha", **locals())
