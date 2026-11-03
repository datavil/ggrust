"""Shape scales: ``scale_shape`` and ``scale_shape_manual``.

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

    from ggrust._typing import Breaks, Color, Guide, Labels, Limits, Number

__all__ = ["scale_linetype_manual", "scale_shape", "scale_shape_manual"]


def scale_shape(
    solid: bool = True,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Point shapes for categories.

    Parameters
    ----------
    solid : bool, default True
        ``False`` uses only hollow shapes (codes 0..14).
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
    na_value : int, optional
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
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", shape="pet")) + geom_point(size=5)
    >>> plot = plot + scale_shape(solid=False)
    """
    return _scale(
        "shape",
        solid=solid,
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        na_value=na_value,
        guide=guide,
        format=format,
    )


def scale_shape_manual(
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
    """Your own point shapes (lets-plot codes 0..25) for ``shape``.

    Parameters
    ----------
    values : list of int or dict
        Shape codes matched in order with the scale's categories, or a dict from
        category to shape code.
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
    na_value : int, optional
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
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", shape="pet")) + geom_point(size=5)
    >>> plot = plot + scale_shape_manual(values=[15, 17, 8])
    """
    return _manual("shape", **locals())


def scale_linetype_manual(
    values: Sequence[Any] | dict[Any, Any],
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: str | int | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Your own line types for ``linetype``.

    Parameters
    ----------
    values : list or dict
        Line types matched in order with the scale's categories, or a dict from
        category to line type. A line type is a code 0..6, a name (``"solid"``,
        ``"dashed"``, ``"dotted"``, ``"dotdash"``, ``"longdash"``, ``"twodash"``,
        ``"blank"``) or a dash list in line widths, e.g. ``[5, 2]``.
    name : str, optional
        Legend title.
    breaks : list, optional
        Values that get a legend key.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The levels (and their order). Data outside the limits gets ``na_value``.
    na_value : str or int, optional
        Line type drawn for missing data and data outside ``limits``; solid by default.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
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
    >>> data = {"t": [1, 2, 3, 1, 2, 3], "v": [1, 3, 2, 2, 1, 3], "g": list("aaabbb")}
    >>> plot = ggplot(data, aes("t", "v", linetype="g")) + geom_line() + scale_linetype_manual(
    ...     values=["solid", "dotted"]
    ... )
    """
    return _manual("linetype", **locals())
