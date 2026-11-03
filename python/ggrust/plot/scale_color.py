"""Color scales: hue, manual, gradients, viridis, brewer, continuous; ``fill`` variants.

Ported from lets-plot's ``lets_plot/plot/scale.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from __future__ import annotations

import warnings
from typing import TYPE_CHECKING, Any, cast

from ggrust.plot.core import FeatureSpec
from ggrust.plot.scale import _scale

if TYPE_CHECKING:
    from collections.abc import Sequence

    from ggrust._typing import Breaks, Color, Guide, Labels, Limits, Number, Oob, Transform

__all__ = [
    "scale_color_brewer",
    "scale_color_continuous",
    "scale_color_discrete",
    "scale_color_gradient",
    "scale_color_gradient2",
    "scale_color_gradientn",
    "scale_color_grey",
    "scale_color_hue",
    "scale_color_manual",
    "scale_color_viridis",
    "scale_fill_brewer",
    "scale_fill_continuous",
    "scale_fill_discrete",
    "scale_fill_gradient",
    "scale_fill_gradient2",
    "scale_fill_gradientn",
    "scale_fill_grey",
    "scale_fill_hue",
    "scale_fill_manual",
    "scale_fill_viridis",
]


def scale_color_hue(
    h: Sequence[Number] | None = None,
    c: Number | None = None,
    l: Number | None = None,  # noqa: E741 (lets_plot's name)
    h_start: Number | None = None,
    direction: int | None = None,
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
    """Qualitative colors evenly spaced around the HCL color wheel, for ``color``.

    Parameters
    ----------
    h : list of float, default=[15, 375]
        Range of hues in degrees.
    c : float, default=100
        Chroma.
    l : float, default=65
        Luminance.
    h_start : float, default=0
        Hue offset in degrees.
    direction : {1, -1}, default=1
        Direction around the color wheel.
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
    na_value : str, optional
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
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + scale_color_hue(l=40)
    """
    return _hue("color", **locals())


def scale_fill_hue(
    h: Sequence[Number] | None = None,
    c: Number | None = None,
    l: Number | None = None,  # noqa: E741 (lets_plot's name)
    h_start: Number | None = None,
    direction: int | None = None,
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
    """Qualitative colors evenly spaced around the HCL color wheel, for ``fill``.

    See ``scale_color_hue`` for the parameters.

    Parameters
    ----------
    h : list of float, default=[15, 375]
        Range of hues in degrees.
    c : float, default=100
        Chroma.
    l : float, default=65
        Luminance.
    h_start : float, default=0
        Hue offset in degrees.
    direction : {1, -1}, default=1
        Direction around the color wheel.
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
    na_value : str, optional
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
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", fill="pet")) + geom_point(shape=21, size=6)
    >>> plot = plot + scale_fill_hue(c=60)
    """
    return _hue("fill", **locals())


def scale_color_manual(
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
    """Your own colors for ``color``.

    Parameters
    ----------
    values : list of str or dict
        Colors matched in order with the scale's categories, or a dict from
        category to color.
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
    na_value : str, optional
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
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + scale_color_manual(
    ...     values={"cat": "orange", "dog": "saddlebrown", "rabbit": "grey"}
    ... )
    """
    return _manual("color", **locals())


def scale_fill_manual(
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
    """Your own colors for ``fill``.

    See ``scale_color_manual`` for the parameters.

    Parameters
    ----------
    values : list of str or dict
        Colors matched in order with the scale's categories, or a dict from
        category to color.
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
    na_value : str, optional
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
    >>> plot = ggplot(data, aes("age", "weight", fill="pet")) + geom_point(shape=21, size=6)
    >>> plot = plot + scale_fill_manual(values=["orange", "saddlebrown", "grey"])
    """
    return _manual("fill", **locals())


def _hue(
    aesthetic: str | Sequence[str],
    *,
    h: Sequence[Number] | None,
    c: Number | None,
    l: Number | None,  # noqa: E741 (lets_plot's name)
    h_start: Number | None,
    direction: int | None,
    **options: Any,
) -> FeatureSpec:
    return _scale(
        aesthetic,
        **options,
        h=h,
        c=c,
        l=l,
        h_start=h_start,
        direction=direction,
        scale_mapper_kind="color_hue",
    )


def _manual(
    aesthetic: str | Sequence[str],
    *,
    values: Sequence[Any] | dict[Any, Any] | None,
    breaks: Breaks | None,
    limits: Limits | None,
    **options: Any,
) -> FeatureSpec:
    """lets_plot ``scale_manual``: a dict of values is matched by category.

    Without breaks or limits its keys become the breaks. Otherwise the values are
    reordered by the limits (or breaks); unmatched values follow in dict order
    (lets_plot uses a set there, so its order is not deterministic).
    """
    if isinstance(values, dict):
        value_by_key = values
        if breaks is None and limits is None:
            breaks = list(value_by_key.keys())
            values = list(value_by_key.values())
        else:
            base_order = cast("Breaks | Limits", breaks if limits is None else limits)
            if isinstance(base_order, dict):
                base_order = list(cast("dict[str, Any]", base_order).values())
            matched = [value_by_key[key] for key in base_order if key in value_by_key]
            if matched:
                values = matched + [
                    value for value in value_by_key.values() if value not in matched
                ]
            else:
                values = None
    return _scale(aesthetic, breaks=breaks, limits=limits, **options, values=values)


def scale_color_gradient(
    low: Color | None = None,
    high: Color | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    oob: Oob | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``color`` to a two-color gradient (in LAB space).

    Parameters
    ----------
    low, high : str, default="#132B43", "#56B1F7"
        Colors at the low and high end of the data.
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
    oob : {"censor", "squish"}, optional
        What continuous data outside ``limits`` maps to: ``"censor"`` (the
        default) draws ``na_value``, ``"squish"`` clamps it to the nearest limit
        so it takes the end color. Missing data stays ``na_value``. Not in
        lets-plot (ggplot2 ``oob``).
    na_value : str, optional
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
    >>> data = {"x": [1, 2, 3] * 3, "y": [1] * 3 + [2] * 3 + [3] * 3,
    ...         "change": [-3, -1, 0, -2, 0, 2, 0, 1, 4]}
    >>> plot = ggplot(data, aes("x", "y", color="change")) + geom_point(size=12)
    >>> plot = plot + scale_color_gradient(low="lightyellow", high="firebrick")
    """
    return _scale("color", **locals(), scale_mapper_kind="color_gradient")


def scale_fill_gradient(
    low: Color | None = None,
    high: Color | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    oob: Oob | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``fill`` to a two-color gradient. See ``scale_color_gradient``.

    Parameters
    ----------
    low, high : str, default="#132B43", "#56B1F7"
        Colors at the low and high end of the data.
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
    oob : {"censor", "squish"}, optional
        What continuous data outside ``limits`` maps to: ``"censor"`` (the
        default) draws ``na_value``, ``"squish"`` clamps it to the nearest limit
        so it takes the end color. Missing data stays ``na_value``. Not in
        lets-plot (ggplot2 ``oob``).
    na_value : str, optional
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
    >>> data = {"x": [1, 2, 3] * 3, "y": [1] * 3 + [2] * 3 + [3] * 3,
    ...         "change": [-3, -1, 0, -2, 0, 2, 0, 1, 4]}
    >>> plot = ggplot(data, aes("x", "y", fill="change")) + geom_tile()
    >>> plot = plot + scale_fill_gradient(low="lightyellow", high="firebrick")
    """
    return _scale("fill", **locals(), scale_mapper_kind="color_gradient")


def scale_color_gradient2(
    low: Color | None = None,
    mid: Color | None = None,
    high: Color | None = None,
    midpoint: Number = 0,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    oob: Oob | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``color`` to a diverging gradient around ``midpoint`` (continuous data only).

    Parameters
    ----------
    low, mid, high : str, default="#964540", "white", "#3B3D96"
        Colors below, at and above the midpoint.
    midpoint : float, default=0
        Data value of the middle color.
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
    oob : {"censor", "squish"}, optional
        What continuous data outside ``limits`` maps to: ``"censor"`` (the
        default) draws ``na_value``, ``"squish"`` clamps it to the nearest limit
        so it takes the end color. Missing data stays ``na_value``. Not in
        lets-plot (ggplot2 ``oob``).
    na_value : str, optional
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
    >>> data = {"x": [1, 2, 3] * 3, "y": [1] * 3 + [2] * 3 + [3] * 3,
    ...         "change": [-3, -1, 0, -2, 0, 2, 0, 1, 4]}
    >>> plot = ggplot(data, aes("x", "y", color="change")) + geom_point(size=12)
    >>> plot = plot + scale_color_gradient2(low="blue", mid="white", high="red")
    """
    return _scale("color", **locals(), scale_mapper_kind="color_gradient2")


def scale_fill_gradient2(
    low: Color | None = None,
    mid: Color | None = None,
    high: Color | None = None,
    midpoint: Number = 0,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    oob: Oob | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``fill`` to a diverging gradient. See ``scale_color_gradient2``.

    Parameters
    ----------
    low, mid, high : str, default="#964540", "white", "#3B3D96"
        Colors below, at and above the midpoint.
    midpoint : float, default=0
        Data value of the middle color.
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
    oob : {"censor", "squish"}, optional
        What continuous data outside ``limits`` maps to: ``"censor"`` (the
        default) draws ``na_value``, ``"squish"`` clamps it to the nearest limit
        so it takes the end color. Missing data stays ``na_value``. Not in
        lets-plot (ggplot2 ``oob``).
    na_value : str, optional
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
    >>> data = {"x": [1, 2, 3] * 3, "y": [1] * 3 + [2] * 3 + [3] * 3,
    ...         "change": [-3, -1, 0, -2, 0, 2, 0, 1, 4]}
    >>> plot = ggplot(data, aes("x", "y", fill="change")) + geom_tile()
    >>> plot = plot + scale_fill_gradient2(low="blue", mid="white", high="red")
    """
    return _scale("fill", **locals(), scale_mapper_kind="color_gradient2")


def scale_color_gradientn(
    colors: Sequence[Color] | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    oob: Oob | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``color`` to a gradient through several colors, evenly spaced over the data.

    Parameters
    ----------
    colors : list of str
        Two or more colors.
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
    oob : {"censor", "squish"}, optional
        What continuous data outside ``limits`` maps to: ``"censor"`` (the
        default) draws ``na_value``, ``"squish"`` clamps it to the nearest limit
        so it takes the end color. Missing data stays ``na_value``. Not in
        lets-plot (ggplot2 ``oob``).
    na_value : str, optional
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
    >>> data = {"x": [1, 2, 3] * 3, "y": [1] * 3 + [2] * 3 + [3] * 3,
    ...         "change": [-3, -1, 0, -2, 0, 2, 0, 1, 4]}
    >>> plot = ggplot(data, aes("x", "y", color="change")) + geom_point(size=12)
    >>> plot = plot + scale_color_gradientn(colors=["navy", "gold", "darkred"])
    """
    return _scale("color", **locals(), scale_mapper_kind="color_gradientn")


def scale_fill_gradientn(
    colors: Sequence[Color] | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    oob: Oob | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``fill`` to a gradient through several colors. See ``scale_color_gradientn``.

    Parameters
    ----------
    colors : list of str
        Two or more colors.
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
    oob : {"censor", "squish"}, optional
        What continuous data outside ``limits`` maps to: ``"censor"`` (the
        default) draws ``na_value``, ``"squish"`` clamps it to the nearest limit
        so it takes the end color. Missing data stays ``na_value``. Not in
        lets-plot (ggplot2 ``oob``).
    na_value : str, optional
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
    >>> data = {"x": [1, 2, 3] * 3, "y": [1] * 3 + [2] * 3 + [3] * 3,
    ...         "change": [-3, -1, 0, -2, 0, 2, 0, 1, 4]}
    >>> plot = ggplot(data, aes("x", "y", fill="change")) + geom_tile()
    >>> plot = plot + scale_fill_gradientn(colors=["navy", "gold", "darkred"])
    """
    return _scale("fill", **locals(), scale_mapper_kind="color_gradientn")


def scale_color_viridis(
    alpha: Number | None = None,
    begin: Number | None = None,
    end: Number | None = None,
    direction: int | None = None,
    option: str | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    oob: Oob | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``color`` with a viridis-family color map.

    Parameters
    ----------
    alpha : float, default=1
        Opacity.
    begin, end : float, default=0, 1
        Part of the color map to use.
    direction : {1, -1}, default=1
        -1 reverses the colors.
    option : str, default="viridis"
        "magma" ("A"), "inferno" ("B"), "plasma" ("C"), "viridis" ("D"),
        "cividis" ("E"), "turbo" or "twilight".
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
    oob : {"censor", "squish"}, optional
        What continuous data outside ``limits`` maps to: ``"censor"`` (the
        default) draws ``na_value``, ``"squish"`` clamps it to the nearest limit
        so it takes the end color. Missing data stays ``na_value``. Not in
        lets-plot (ggplot2 ``oob``).
    na_value : str, optional
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
    >>> data = {"x": [1, 2, 3] * 3, "y": [1] * 3 + [2] * 3 + [3] * 3,
    ...         "change": [-3, -1, 0, -2, 0, 2, 0, 1, 4]}
    >>> plot = ggplot(data, aes("x", "y", color="change")) + geom_point(size=12)
    >>> plot = plot + scale_color_viridis(option="magma")
    """
    return _scale("color", **locals(), scale_mapper_kind="color_cmap")


def scale_fill_viridis(
    alpha: Number | None = None,
    begin: Number | None = None,
    end: Number | None = None,
    direction: int | None = None,
    option: str | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    oob: Oob | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``fill`` with a viridis-family color map. See ``scale_color_viridis``.

    Parameters
    ----------
    alpha : float, default=1
        Opacity.
    begin, end : float, default=0, 1
        Part of the color map to use.
    direction : {1, -1}, default=1
        -1 reverses the colors.
    option : str, default="viridis"
        "magma" ("A"), "inferno" ("B"), "plasma" ("C"), "viridis" ("D"),
        "cividis" ("E"), "turbo" or "twilight".
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
    oob : {"censor", "squish"}, optional
        What continuous data outside ``limits`` maps to: ``"censor"`` (the
        default) draws ``na_value``, ``"squish"`` clamps it to the nearest limit
        so it takes the end color. Missing data stays ``na_value``. Not in
        lets-plot (ggplot2 ``oob``).
    na_value : str, optional
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
    >>> data = {"x": [1, 2, 3] * 3, "y": [1] * 3 + [2] * 3 + [3] * 3,
    ...         "change": [-3, -1, 0, -2, 0, 2, 0, 1, 4]}
    >>> plot = ggplot(data, aes("x", "y", fill="change")) + geom_tile()
    >>> plot = plot + scale_fill_viridis(option="magma")
    """
    return _scale("fill", **locals(), scale_mapper_kind="color_cmap")


def scale_color_continuous(
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    oob: Oob | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
    scale_mapper_kind: str | None = None,
    **kwargs: object,
) -> FeatureSpec:
    """Map ``color`` continuously: a gradient unless ``scale_mapper_kind`` says otherwise.

    Parameters
    ----------
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
    oob : {"censor", "squish"}, optional
        What continuous data outside ``limits`` maps to: ``"censor"`` (the
        default) draws ``na_value``, ``"squish"`` clamps it to the nearest limit
        so it takes the end color. Missing data stays ``na_value``. Not in
        lets-plot (ggplot2 ``oob``).
    na_value : str, optional
        Value drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"colorbar"`` (continuous color), ``"none"``, or a
        ``guide_legend()`` / ``guide_colorbar()`` spec.
    trans : {"identity", "log10", "log2", "sqrt", "reverse"}, optional
        Transformation of the data.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.
    scale_mapper_kind : str, optional
        Default ``"color_gradient"``.
    **kwargs
        Options of the chosen mapper (e.g. ``low``, ``high``).

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3] * 3, "y": [1] * 3 + [2] * 3 + [3] * 3,
    ...         "change": [-3, -1, 0, -2, 0, 2, 0, 1, 4]}
    >>> plot = ggplot(data, aes("x", "y", color="change")) + geom_point(size=12)
    >>> plot = plot + scale_color_continuous(low="white", high="darkgreen")
    """
    return _continuous("color", locals())


def scale_fill_continuous(
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    oob: Oob | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
    scale_mapper_kind: str | None = None,
    **kwargs: object,
) -> FeatureSpec:
    """Map ``fill`` continuously. See ``scale_color_continuous``.

    Parameters
    ----------
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
    oob : {"censor", "squish"}, optional
        What continuous data outside ``limits`` maps to: ``"censor"`` (the
        default) draws ``na_value``, ``"squish"`` clamps it to the nearest limit
        so it takes the end color. Missing data stays ``na_value``. Not in
        lets-plot (ggplot2 ``oob``).
    na_value : str, optional
        Value drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"colorbar"`` (continuous color), ``"none"``, or a
        ``guide_legend()`` / ``guide_colorbar()`` spec.
    trans : {"identity", "log10", "log2", "sqrt", "reverse"}, optional
        Transformation of the data.
    format : str, optional
        Label format: a number format (``".2f"``, ``","``, ``"%"``) or a template
        (``"{.1f} %"``). ``labels`` take precedence.
    scale_mapper_kind : str, optional
        Default ``"color_gradient"``.
    **kwargs
        Options of the chosen mapper (e.g. ``low``, ``high``).

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3] * 3, "y": [1] * 3 + [2] * 3 + [3] * 3,
    ...         "change": [-3, -1, 0, -2, 0, 2, 0, 1, 4]}
    >>> plot = ggplot(data, aes("x", "y", fill="change")) + geom_tile()
    >>> plot = plot + scale_fill_continuous(low="white", high="darkgreen")
    """
    return _continuous("fill", locals())


def _continuous(aesthetic: str | Sequence[str], options: dict[str, Any]) -> FeatureSpec:
    """lets_plot ``scale_continuous``: color scales default to ``color_gradient``."""
    options = dict(options)
    kwargs = options.pop("kwargs")
    if options["scale_mapper_kind"] is None:
        options["scale_mapper_kind"] = "color_gradient"
    return _scale(aesthetic, **options, expand=kwargs.pop("expand", None), **kwargs)


def scale_color_brewer(
    type: str | None = None,
    palette: str | int | None = None,
    direction: int | None = None,
    overflow: str | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    oob: Oob | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``color`` with a ColorBrewer palette.

    Parameters
    ----------
    type : {"seq", "div", "qual"}, optional
        Palette family; needed with a palette index.
    palette : str or int, optional
        Palette name (e.g. ``"Set2"``, ``"RdBu"``) or index within ``type``.
        Default: ``Set1`` for categories, ``Blues`` for continuous data.
    direction : {1, -1}, default=1
        -1 reverses the colors.
    overflow : {"interpolate", "cycle", "generate"}, optional
        How to get more colors than the palette has (``"i"``, ``"c"``, ``"g"``).
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
    oob : {"censor", "squish"}, optional
        What continuous data outside ``limits`` maps to: ``"censor"`` (the
        default) draws ``na_value``, ``"squish"`` clamps it to the nearest limit
        so it takes the end color. Missing data stays ``na_value``. Not in
        lets-plot (ggplot2 ``oob``).
    na_value : str, optional
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
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + scale_color_brewer(palette="Dark2")
    """
    return _scale("color", **locals(), scale_mapper_kind="color_brewer")


def scale_fill_brewer(
    type: str | None = None,
    palette: str | int | None = None,
    direction: int | None = None,
    overflow: str | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    oob: Oob | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``fill`` with a ColorBrewer palette. See ``scale_color_brewer``.

    Parameters
    ----------
    type : {"seq", "div", "qual"}, optional
        Palette family; needed with a palette index.
    palette : str or int, optional
        Palette name (e.g. ``"Set2"``, ``"RdBu"``) or index within ``type``.
        Default: ``Set1`` for categories, ``Blues`` for continuous data.
    direction : {1, -1}, default=1
        -1 reverses the colors.
    overflow : {"interpolate", "cycle", "generate"}, optional
        How to get more colors than the palette has (``"i"``, ``"c"``, ``"g"``).
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
    oob : {"censor", "squish"}, optional
        What continuous data outside ``limits`` maps to: ``"censor"`` (the
        default) draws ``na_value``, ``"squish"`` clamps it to the nearest limit
        so it takes the end color. Missing data stays ``na_value``. Not in
        lets-plot (ggplot2 ``oob``).
    na_value : str, optional
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
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", fill="pet")) + geom_point(shape=21, size=6)
    >>> plot = plot + scale_fill_brewer(palette="Pastel1")
    """
    return _scale("fill", **locals(), scale_mapper_kind="color_brewer")


def scale_color_discrete(
    direction: int | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    format: str | None = None,
    scale_mapper_kind: str | None = None,
) -> FeatureSpec:
    """Map ``color`` as categories with the default discrete palette.

    Numbers become categories too, in order of first appearance.

    Parameters
    ----------
    direction : {1, -1}, default=1
        -1 reverses the colors.
    name : str, optional
        Legend title.
    breaks : list, optional
        Categories that get a legend key.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The categories and their order. Data outside the limits gets ``na_value``.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Label format, e.g. ``".1f"``. ``labels`` take precedence.
    scale_mapper_kind : str, optional
        Mapper that builds the colors, e.g. ``"color_brewer"`` or ``"color_hue"``.

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
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + scale_color_discrete(name="Animal")
    """
    return _scale("color", **locals(), discrete=True)


def scale_fill_discrete(
    direction: int | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    format: str | None = None,
    scale_mapper_kind: str | None = None,
) -> FeatureSpec:
    """Map ``fill`` as categories with the default discrete palette.

    Numbers become categories too, in order of first appearance.

    Parameters
    ----------
    direction : {1, -1}, default=1
        -1 reverses the colors.
    name : str, optional
        Legend title.
    breaks : list, optional
        Categories that get a legend key.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The categories and their order. Data outside the limits gets ``na_value``.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Label format, e.g. ``".1f"``. ``labels`` take precedence.
    scale_mapper_kind : str, optional
        Mapper that builds the colors, e.g. ``"color_brewer"`` or ``"color_hue"``.

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
    >>> plot = ggplot(data, aes("age", "weight", fill="pet")) + geom_point(shape=21, size=6)
    >>> plot = plot + scale_fill_discrete(name="Animal")
    """
    return _scale("fill", **locals(), discrete=True)


def scale_color_grey(
    start: Number | None = None,
    end: Number | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    oob: Oob | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``color`` to greys, from lightness ``start`` to ``end``.

    Works for categories and continuous data (a grey gradient).

    Parameters
    ----------
    start : float, default=0.2
        Lightness of the low end, 0 (black) to 1 (white).
    end : float, default=0.8
        Lightness of the high end, 0 to 1.
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
    oob : {"censor", "squish"}, optional
        What continuous data outside ``limits`` maps to: ``"censor"`` (the
        default) draws ``na_value``, ``"squish"`` clamps it to the nearest limit
        so it takes the end color. Missing data stays ``na_value``. Not in
        lets-plot (ggplot2 ``oob``).
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"colorbar"``, ``"none"``, or a guide spec.
    trans : {"identity", "log10", "log2", "sqrt", "reverse"}, optional
        Transformation of the data.
    format : str, optional
        Label format, e.g. ``".1f"``. ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    ValueError
        ``start`` or ``end`` outside 0..1 (after lets_plot's rescaling of 0..100 values).

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + scale_color_grey(start=0.1, end=0.7)
    """
    start, end = _grey_ends(start, end)
    return _scale("color", **locals(), scale_mapper_kind="color_grey")


def scale_fill_grey(
    start: Number | None = None,
    end: Number | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    oob: Oob | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``fill`` to greys, from lightness ``start`` to ``end``.

    Works for categories and continuous data (a grey gradient).

    Parameters
    ----------
    start : float, default=0.2
        Lightness of the low end, 0 (black) to 1 (white).
    end : float, default=0.8
        Lightness of the high end, 0 to 1.
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
    oob : {"censor", "squish"}, optional
        What continuous data outside ``limits`` maps to: ``"censor"`` (the
        default) draws ``na_value``, ``"squish"`` clamps it to the nearest limit
        so it takes the end color. Missing data stays ``na_value``. Not in
        lets-plot (ggplot2 ``oob``).
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"colorbar"``, ``"none"``, or a guide spec.
    trans : {"identity", "log10", "log2", "sqrt", "reverse"}, optional
        Transformation of the data.
    format : str, optional
        Label format, e.g. ``".1f"``. ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    ValueError
        ``start`` or ``end`` outside 0..1 (after lets_plot's rescaling of 0..100 values).

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", fill="pet")) + geom_point(shape=21, size=6)
    >>> plot = plot + scale_fill_grey(start=0.1, end=0.7)
    """
    start, end = _grey_ends(start, end)
    return _scale("fill", **locals(), scale_mapper_kind="color_grey")


def _grey_ends(start: Number | None, end: Number | None) -> tuple[Number | None, Number | None]:
    """lets_plot ``_greyscale_check_parameters``: 0..100 values are rescaled to 0..1.

    Raises
    ------
    ValueError
        A value still outside 0..1.
    """
    checked: list[Number | None] = []
    for label, value in (("start", start), ("end", end)):
        if value is not None and not 0 <= value <= 1:
            rescaled = value / 100
            warnings.warn(
                f"scale_*_grey(): '{label}' {value} is outside 0..1; read as {rescaled} "
                "(lets_plot's old 0..100 range)",
                stacklevel=3,
            )
            value = rescaled
        if value is not None and not 0 <= value <= 1:
            message = (
                f"scale_*_grey(): '{label}' must be a lightness between 0 (black) and 1 "
                f"(white), got {value * 100}"
            )
            raise ValueError(message)
        checked.append(value)
    return checked[0], checked[1]
