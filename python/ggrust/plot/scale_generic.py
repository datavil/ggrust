"""Generic scales for any aesthetic: ``scale_manual("color", ...)``, ``scale_gradient(...)``.

Ported from lets-plot's ``lets_plot/plot/scale.py`` (MIT, see THIRD_PARTY_NOTICES.md).
Each function takes the aesthetic first. A list of aesthetics (``["color", "fill"]``)
sets the same options on all of them at once: it returns one scale per aesthetic, as
lets-plot does. The work is done by the same helpers as ``scale_color_*`` /
``scale_fill_*``; this module only checks ``aesthetic`` and forwards.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from ggrust.errors import SpecError, UnsupportedValueError
from ggrust.plot.new_scale import canonical_aesthetic
from ggrust.plot.scale import _scale
from ggrust.plot.scale_color import _grey_ends, _hue, _manual

if TYPE_CHECKING:
    from collections.abc import Sequence

    from ggrust._typing import (
        Breaks,
        Color,
        Guide,
        Labels,
        Limits,
        Number,
        Oob,
        PositionOob,
        Rescaler,
        Transform,
    )
    from ggrust.plot.core import FeatureSpec

__all__ = [
    "scale_brewer",
    "scale_continuous",
    "scale_discrete",
    "scale_gradient",
    "scale_gradient2",
    "scale_gradientn",
    "scale_grey",
    "scale_hue",
    "scale_identity",
    "scale_manual",
    "scale_viridis",
]

_COLOR_AESTHETICS = frozenset({"color", "fill", "paint_a", "paint_b", "paint_c"})
"""Aesthetics that ``scale_continuous`` gives a color gradient, as lets-plot does."""


def _aesthetics(aesthetic: object, function: str) -> str | list[str]:
    """Check ``aesthetic=`` of a generic scale and spell aliases as ggrust does.

    Parameters
    ----------
    aesthetic : object
        The value given: a name or a list (or tuple) of names.
    function : str
        The calling function, for error messages.

    Returns
    -------
    str or list of str
        One name, or a list of names (``"colour"`` becomes ``"color"``).

    Raises
    ------
    UnsupportedValueError
        If ``aesthetic`` is not a string or a list of strings.
    SpecError
        If ``aesthetic`` is an empty list.
    """
    if isinstance(aesthetic, str):
        return canonical_aesthetic(aesthetic)
    if isinstance(aesthetic, (list, tuple)):
        names: list[object] = list(aesthetic)
        if not names:
            reason = "expected at least one aesthetic name, e.g. ['color', 'fill']"
            raise SpecError(f"{function}(aesthetic=[]): {reason}", path="aesthetic", reason=reason)
        for name in names:
            if not isinstance(name, str):
                reason = (
                    f"expected aesthetic names, got {type(name).__name__} {name!r}; "
                    "use strings such as 'color' or ['color', 'fill']"
                )
                raise UnsupportedValueError(
                    f"{function}(aesthetic={aesthetic!r}): {reason}",
                    path="aesthetic",
                    reason=reason,
                )
        return [canonical_aesthetic(name) for name in names if isinstance(name, str)]
    reason = (
        f"expected an aesthetic name or a list of them, got {type(aesthetic).__name__}; "
        "use e.g. 'color' or ['color', 'fill']"
    )
    raise UnsupportedValueError(
        f"{function}(aesthetic={aesthetic!r}): {reason}", path="aesthetic", reason=reason
    )


def _is_color_scale(aesthetic: str | list[str]) -> bool:
    """lets_plot ``_is_color_scale``: every aesthetic is a color one."""
    names = [aesthetic] if isinstance(aesthetic, str) else aesthetic
    return set(names) <= _COLOR_AESTHETICS


def scale_manual(
    aesthetic: str | Sequence[str],
    values: Sequence[Any] | dict[Any, Any],
    *,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Your own values for any aesthetic, or for several at once.

    Parameters
    ----------
    aesthetic : str or list of str
        The aesthetic(s) the scale maps, e.g. ``"color"`` or ``["color", "fill"]``.
        A list gives each aesthetic the same scale.
    values : list or dict
        Values matched in order with the scale's categories, or a dict from
        category to value.
    name : str, optional
        Legend title.
    breaks : list or dict, optional
        Categories that get a legend key (a dict maps labels to breaks).
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The categories (and their order). Data outside the limits gets ``na_value``.
    na_value : optional
        Value drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Label format, e.g. ``".1f"``. ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale, or a ``FeatureSpecArray`` of one scale per aesthetic for a list.

    Raises
    ------
    UnsupportedValueError
        If ``aesthetic`` is not a string or a list of strings.
    SpecError
        If ``aesthetic`` is an empty list.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet", fill="pet"))
    >>> plot = plot + geom_point(shape=21, size=6, alpha=0.5)
    >>> plot = plot + scale_manual(
    ...     ["color", "fill"], values=["darkorange", "saddlebrown", "grey"]
    ... )
    """
    return _manual(
        _aesthetics(aesthetic, "scale_manual"),
        values=values,
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        na_value=na_value,
        guide=guide,
        format=format,
    )


def scale_continuous(
    aesthetic: str | Sequence[str],
    *,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    expand: Sequence[Number] | None = None,
    oob: Oob | PositionOob | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
    scale_mapper_kind: str | None = None,
    **kwargs: Any,
) -> FeatureSpec:
    """Map any aesthetic, or several at once, continuously.

    Color aesthetics (``color``, ``fill``, ``paint_*``) get a gradient unless
    ``scale_mapper_kind`` says otherwise.

    Parameters
    ----------
    aesthetic : str or list of str
        The aesthetic(s) the scale maps, e.g. ``"size"`` or ``["x", "y"]``.
    name : str, optional
        Axis label or legend title.
    breaks : list or dict, optional
        Values that get a tick or a legend key (a dict maps labels to breaks).
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Ignored on continuous scales, with a warning, as in lets-plot.
    limits : list, optional
        ``[low, high]`` data range of the scale.
    expand : list of float, optional
        ``[multiplicative, additive]`` expansion of a position scale.
    oob : {"censor", "squish", "keep"}, optional
        What data outside ``limits`` maps to. ``"keep"`` is for x and y only. Not in
        lets-plot (ggplot2 ``oob``).
    na_value : optional
        Value drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"colorbar"`` (color), ``"none"``, or a guide spec.
    trans : str, optional
        Transformation of the data: ``identity``, ``log10``, ``log2``, ``log``,
        ``log1p``, ``exp``, ``logit``, ``probit``, ``reciprocal``, ``pseudo_log``,
        ``symlog``, ``sqrt``, ``reverse``, or ``boxcox_trans(p)``.
    format : str, optional
        Label format, e.g. ``".1f"``. ``labels`` take precedence.
    scale_mapper_kind : str, optional
        The color mapper, e.g. ``"color_gradient"`` or ``"color_hue"``.
    **kwargs
        Options of the mapper (``low=``, ``high=``, ...) or of a position scale
        (``position=``, ``break_width=``).

    Returns
    -------
    FeatureSpec
        The scale, or a ``FeatureSpecArray`` of one scale per aesthetic for a list.

    Raises
    ------
    UnsupportedValueError
        If ``aesthetic`` is not a string or a list of strings.
    SpecError
        If ``aesthetic`` is an empty list, or ``oob`` is not accepted.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [0, 1, 2, 3], "y": [0, 1, 4, 9]}
    >>> plot = ggplot(data, aes("x", "y")) + geom_point(size=8)
    >>> plot = plot + scale_continuous(["x", "y"], expand=[0, 0])
    """
    aesthetics = _aesthetics(aesthetic, "scale_continuous")
    if scale_mapper_kind is None and _is_color_scale(aesthetics):
        scale_mapper_kind = "color_gradient"
    return _scale(
        aesthetics,
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        expand=expand,
        oob=oob,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
        scale_mapper_kind=scale_mapper_kind,
        **kwargs,
    )


def scale_discrete(
    aesthetic: str | Sequence[str],
    *,
    direction: int | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    expand: Sequence[Number] | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    format: str | None = None,
    scale_mapper_kind: str | None = None,
    **kwargs: Any,
) -> FeatureSpec:
    """Map any aesthetic, or several at once, as categories.

    Numbers become categories too, in order of first appearance.

    Parameters
    ----------
    aesthetic : str or list of str
        The aesthetic(s) the scale maps, e.g. ``"shape"`` or ``["color", "fill"]``.
    direction : {1, -1}, default=1
        -1 reverses the colors of a color scale.
    name : str, optional
        Axis label or legend title.
    breaks : list or dict, optional
        Categories that get a tick or a legend key (a dict maps labels to breaks).
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The categories (and their order).
    expand : list of float, optional
        ``[multiplicative, additive]`` expansion of a position scale.
    na_value : optional
        Value drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a guide spec.
    format : str, optional
        Label format, e.g. ``".1f"``. ``labels`` take precedence.
    scale_mapper_kind : str, optional
        The color mapper, e.g. ``"color_brewer"``.
    **kwargs
        Options of the mapper (``palette=``, ...) or of a position scale
        (``position=``).

    Returns
    -------
    FeatureSpec
        The scale, or a ``FeatureSpecArray`` of one scale per aesthetic for a list.

    Raises
    ------
    UnsupportedValueError
        If ``aesthetic`` is not a string or a list of strings.
    SpecError
        If ``aesthetic`` is an empty list.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3, 4], "y": [1, 2, 3, 4], "level": [1, 2, 3, 4]}
    >>> plot = ggplot(data, aes("x", "y", color="level", fill="level"))
    >>> plot = plot + geom_point(shape=21, size=8, alpha=0.5)
    >>> plot = plot + scale_discrete(["color", "fill"], name="Level")
    """
    return _scale(
        _aesthetics(aesthetic, "scale_discrete"),
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        expand=expand,
        na_value=na_value,
        guide=guide,
        format=format,
        direction=direction,
        discrete=True,
        scale_mapper_kind=scale_mapper_kind,
        **kwargs,
    )


def scale_gradient(
    aesthetic: str | Sequence[str],
    *,
    low: Color | None = None,
    high: Color | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    oob: Oob | None = None,
    rescaler: Rescaler | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map color aesthetics to a two-color gradient (in LAB space).

    Parameters
    ----------
    aesthetic : str or list of str
        The color aesthetic(s), e.g. ``"fill"`` or ``["color", "fill"]``.
    low, high : str, default="#132B43", "#56B1F7"
        Colors at the low and high end of the data.
    name : str, optional
        Legend title.
    breaks : list or dict, optional
        Values that get a colorbar tick (a dict maps labels to breaks).
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Ignored on continuous scales, with a warning, as in lets-plot.
    limits : list, optional
        ``[low, high]`` data range. Data outside the limits gets ``na_value``.
    oob : {"censor", "squish"}, optional
        What data outside ``limits`` maps to. Not in lets-plot (ggplot2 ``oob``).
    rescaler : callable, optional
        ``rescaler(x, to, from_)`` maps the data to ``(0, 1)`` before the palette.
        Not in lets-plot (ggplot2 ``rescaler``).
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"colorbar"``, ``"legend"``, ``"none"``, or a guide spec.
    trans : str, optional
        Transformation of the data, e.g. ``"log10"`` or ``"sqrt"``.
    format : str, optional
        Label format, e.g. ``".1f"``. ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale, or a ``FeatureSpecArray`` of one scale per aesthetic for a list.

    Raises
    ------
    UnsupportedValueError
        If ``aesthetic`` is not a string or a list of strings.
    SpecError
        If ``aesthetic`` is an empty list.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3, 4, 5], "y": [3, 1, 4, 1, 5], "change": [-2, -1, 0, 1, 2]}
    >>> plot = ggplot(data, aes("x", "y", color="change", fill="change"))
    >>> plot = plot + geom_point(shape=21, size=10, alpha=0.4)
    >>> plot = plot + scale_gradient(["color", "fill"], low="gold", high="darkred")
    """
    return _scale(
        _aesthetics(aesthetic, "scale_gradient"),
        low=low,
        high=high,
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        oob=oob,
        rescaler=rescaler,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
        scale_mapper_kind="color_gradient",
    )


def scale_gradient2(
    aesthetic: str | Sequence[str],
    *,
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
    rescaler: Rescaler | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map color aesthetics to a diverging three-color gradient.

    Parameters
    ----------
    aesthetic : str or list of str
        The color aesthetic(s), e.g. ``"fill"`` or ``["color", "fill"]``.
    low, mid, high : str, default="#964540", "#FBF9F7", "#3B3D96"
        Colors at the low end, the midpoint and the high end.
    midpoint : float, default=0
        Data value that gets ``mid``.
    name : str, optional
        Legend title.
    breaks : list or dict, optional
        Values that get a colorbar tick (a dict maps labels to breaks).
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Ignored on continuous scales, with a warning, as in lets-plot.
    limits : list, optional
        ``[low, high]`` data range. Data outside the limits gets ``na_value``.
    oob : {"censor", "squish"}, optional
        What data outside ``limits`` maps to. Not in lets-plot (ggplot2 ``oob``).
    rescaler : callable, optional
        ``rescaler(x, to, from_)`` maps the data to ``(0, 1)`` before the palette.
        Not in lets-plot (ggplot2 ``rescaler``).
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"colorbar"``, ``"legend"``, ``"none"``, or a guide spec.
    trans : str, optional
        Transformation of the data, e.g. ``"log10"`` or ``"sqrt"``.
    format : str, optional
        Label format, e.g. ``".1f"``. ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale, or a ``FeatureSpecArray`` of one scale per aesthetic for a list.

    Raises
    ------
    UnsupportedValueError
        If ``aesthetic`` is not a string or a list of strings.
    SpecError
        If ``aesthetic`` is an empty list.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3, 4, 5], "y": [3, 1, 4, 1, 5], "change": [-2, -1, 0, 1, 2]}
    >>> plot = ggplot(data, aes("x", "y", color="change", fill="change"))
    >>> plot = plot + geom_point(shape=21, size=10, alpha=0.4)
    >>> plot = plot + scale_gradient2(["color", "fill"], low="blue", high="red")
    """
    return _scale(
        _aesthetics(aesthetic, "scale_gradient2"),
        low=low,
        mid=mid,
        high=high,
        midpoint=midpoint,
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        oob=oob,
        rescaler=rescaler,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
        scale_mapper_kind="color_gradient2",
    )


def scale_gradientn(
    aesthetic: str | Sequence[str],
    *,
    colors: Sequence[Color] | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    oob: Oob | None = None,
    rescaler: Rescaler | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map color aesthetics to an n-color gradient.

    Parameters
    ----------
    aesthetic : str or list of str
        The color aesthetic(s), e.g. ``"fill"`` or ``["color", "fill"]``.
    colors : list of str, optional
        Colors spread evenly over the data range.
    name : str, optional
        Legend title.
    breaks : list or dict, optional
        Values that get a colorbar tick (a dict maps labels to breaks).
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Ignored on continuous scales, with a warning, as in lets-plot.
    limits : list, optional
        ``[low, high]`` data range. Data outside the limits gets ``na_value``.
    oob : {"censor", "squish"}, optional
        What data outside ``limits`` maps to. Not in lets-plot (ggplot2 ``oob``).
    rescaler : callable, optional
        ``rescaler(x, to, from_)`` maps the data to ``(0, 1)`` before the palette.
        Not in lets-plot (ggplot2 ``rescaler``).
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"colorbar"``, ``"legend"``, ``"none"``, or a guide spec.
    trans : str, optional
        Transformation of the data, e.g. ``"log10"`` or ``"sqrt"``.
    format : str, optional
        Label format, e.g. ``".1f"``. ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale, or a ``FeatureSpecArray`` of one scale per aesthetic for a list.

    Raises
    ------
    UnsupportedValueError
        If ``aesthetic`` is not a string or a list of strings.
    SpecError
        If ``aesthetic`` is an empty list.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3, 4, 5], "y": [3, 1, 4, 1, 5], "change": [-2, -1, 0, 1, 2]}
    >>> plot = ggplot(data, aes("x", "y", color="change", fill="change"))
    >>> plot = plot + geom_point(shape=21, size=10, alpha=0.4)
    >>> plot = plot + scale_gradientn(["color", "fill"], colors=["navy", "white", "firebrick"])
    """
    return _scale(
        _aesthetics(aesthetic, "scale_gradientn"),
        colors=colors,
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        oob=oob,
        rescaler=rescaler,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
        scale_mapper_kind="color_gradientn",
    )


def scale_hue(
    aesthetic: str | Sequence[str],
    *,
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
    na_value: Color | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Qualitative colors around the HCL color wheel, for color aesthetics.

    Parameters
    ----------
    aesthetic : str or list of str
        The color aesthetic(s), e.g. ``"fill"`` or ``["color", "fill"]``.
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
    breaks : list or dict, optional
        Values that get a legend key (a dict maps labels to breaks).
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters for discrete data; longer labels are cut.
    limits : list, optional
        The categories (and their order), or a ``[low, high]`` data range.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"colorbar"``, ``"none"``, or a guide spec.
    trans : str, optional
        Transformation of continuous data, e.g. ``"log10"``.
    format : str, optional
        Label format, e.g. ``".1f"``. ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale, or a ``FeatureSpecArray`` of one scale per aesthetic for a list.

    Raises
    ------
    UnsupportedValueError
        If ``aesthetic`` is not a string or a list of strings.
    SpecError
        If ``aesthetic`` is an empty list.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet", fill="pet"))
    >>> plot = plot + geom_point(shape=21, size=6, alpha=0.5)
    >>> plot = plot + scale_hue(["color", "fill"], l=40)
    """
    return _hue(
        _aesthetics(aesthetic, "scale_hue"),
        h=h,
        c=c,
        l=l,
        h_start=h_start,
        direction=direction,
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
    )


def scale_grey(
    aesthetic: str | Sequence[str],
    *,
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
    """Greys from lightness ``start`` to ``end``, for color aesthetics.

    Works for categories and continuous data (a grey gradient).

    Parameters
    ----------
    aesthetic : str or list of str
        The color aesthetic(s), e.g. ``"fill"`` or ``["color", "fill"]``.
    start : float, default=0.2
        Lightness of the low end, 0 (black) to 1 (white).
    end : float, default=0.8
        Lightness of the high end, 0 to 1.
    name : str, optional
        Legend title.
    breaks : list or dict, optional
        Values that get a legend key or a colorbar tick (a dict maps labels to breaks).
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters for discrete data; longer labels are cut.
    limits : list, optional
        The categories (and their order), or a ``[low, high]`` data range.
    oob : {"censor", "squish"}, optional
        What continuous data outside ``limits`` maps to. Not in lets-plot.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"colorbar"``, ``"none"``, or a guide spec.
    trans : str, optional
        Transformation of continuous data, e.g. ``"log10"``.
    format : str, optional
        Label format, e.g. ``".1f"``. ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale, or a ``FeatureSpecArray`` of one scale per aesthetic for a list.

    Raises
    ------
    UnsupportedValueError
        If ``aesthetic`` is not a string or a list of strings.
    SpecError
        If ``aesthetic`` is an empty list.
    ValueError
        If ``start`` or ``end`` is outside 0..1 (after lets_plot's rescaling of
        0..100 values).

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet", fill="pet"))
    >>> plot = plot + geom_point(shape=21, size=6, alpha=0.5)
    >>> plot = plot + scale_grey(["color", "fill"], start=0.1, end=0.7)
    """
    aesthetics = _aesthetics(aesthetic, "scale_grey")
    start, end = _grey_ends(start, end)
    return _scale(
        aesthetics,
        start=start,
        end=end,
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        oob=oob,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
        scale_mapper_kind="color_grey",
    )


def scale_brewer(
    aesthetic: str | Sequence[str],
    *,
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
    na_value: Color | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map color aesthetics with a ColorBrewer palette.

    Parameters
    ----------
    aesthetic : str or list of str
        The color aesthetic(s), e.g. ``"fill"`` or ``["color", "fill"]``.
    type : {"seq", "div", "qual"}, optional
        Palette family; needed with a palette index.
    palette : str or int, optional
        Palette name (e.g. ``"Set2"``, ``"RdBu"``) or index within ``type``.
        Default: ``Set1`` for categories, ``Blues`` for continuous data.
    direction : {1, -1}, default=1
        -1 reverses the colors.
    overflow : str, optional
        How to get more colors than the palette has (``"i"``, ``"c"``, ``"g"``).
    name : str, optional
        Legend title.
    breaks : list or dict, optional
        Values that get a legend key or a colorbar tick (a dict maps labels to breaks).
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters for discrete data; longer labels are cut.
    limits : list, optional
        The categories (and their order), or a ``[low, high]`` data range.
    oob : {"censor", "squish"}, optional
        What continuous data outside ``limits`` maps to. Not in lets-plot.
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"colorbar"``, ``"none"``, or a guide spec.
    trans : str, optional
        Transformation of continuous data, e.g. ``"log10"``.
    format : str, optional
        Label format, e.g. ``".1f"``. ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale, or a ``FeatureSpecArray`` of one scale per aesthetic for a list.

    Raises
    ------
    UnsupportedValueError
        If ``aesthetic`` is not a string or a list of strings.
    SpecError
        If ``aesthetic`` is an empty list.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet", fill="pet"))
    >>> plot = plot + geom_point(shape=21, size=6, alpha=0.5)
    >>> plot = plot + scale_brewer(["color", "fill"], palette="Dark2")
    """
    return _scale(
        _aesthetics(aesthetic, "scale_brewer"),
        type=type,
        palette=palette,
        direction=direction,
        overflow=overflow,
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        oob=oob,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
        scale_mapper_kind="color_brewer",
    )


def scale_viridis(
    aesthetic: str | Sequence[str],
    *,
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
    rescaler: Rescaler | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map color aesthetics with a viridis-family color map.

    Parameters
    ----------
    aesthetic : str or list of str
        The color aesthetic(s), e.g. ``"fill"`` or ``["color", "fill"]``.
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
    breaks : list or dict, optional
        Values that get a legend key or a colorbar tick (a dict maps labels to breaks).
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters for discrete data; longer labels are cut.
    limits : list, optional
        The categories (and their order), or a ``[low, high]`` data range.
    oob : {"censor", "squish"}, optional
        What continuous data outside ``limits`` maps to. Not in lets-plot.
    rescaler : callable, optional
        ``rescaler(x, to, from_)`` maps the data to ``(0, 1)`` before the palette.
        Not in lets-plot (ggplot2 ``rescaler``).
    na_value : str, optional
        Color drawn for missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"colorbar"``, ``"none"``, or a guide spec.
    trans : str, optional
        Transformation of continuous data, e.g. ``"log10"``.
    format : str, optional
        Label format, e.g. ``".1f"``. ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale, or a ``FeatureSpecArray`` of one scale per aesthetic for a list.

    Raises
    ------
    UnsupportedValueError
        If ``aesthetic`` is not a string or a list of strings.
    SpecError
        If ``aesthetic`` is an empty list.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3, 4, 5], "y": [3, 1, 4, 1, 5], "change": [-2, -1, 0, 1, 2]}
    >>> plot = ggplot(data, aes("x", "y", color="change", fill="change"))
    >>> plot = plot + geom_point(shape=21, size=10, alpha=0.4)
    >>> plot = plot + scale_viridis(["color", "fill"], option="magma", end=0.9)
    """
    return _scale(
        _aesthetics(aesthetic, "scale_viridis"),
        alpha=alpha,
        begin=begin,
        end=end,
        direction=direction,
        option=option,
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        oob=oob,
        rescaler=rescaler,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
        scale_mapper_kind="color_cmap",
    )


def scale_identity(
    aesthetic: str | Sequence[str],
    *,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | Number | None = None,
    guide: Guide = "none",
    format: str | None = None,
) -> FeatureSpec:
    """Use the data as the aesthetic values (colors, sizes, shapes, ...); no legend by default.

    Parameters
    ----------
    aesthetic : str or list of str
        The aesthetic(s), e.g. ``"fill"`` or ``["color", "fill"]``.
    name : str, optional
        Title of the scale, shown on the legend.
    breaks : list or dict, optional
        Values that get a legend key (a dict maps labels to breaks).
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The values (and their order) the scale keeps.
    na_value : str or float, optional
        Value drawn for missing data.
    guide : str or FeatureSpec, default="none"
        ``"legend"`` to show a legend (give ``breaks`` and ``labels`` too).
    format : str, optional
        Label format, e.g. ``".1f"``. ``labels`` take precedence.

    Returns
    -------
    FeatureSpec
        The scale, or a ``FeatureSpecArray`` of one scale per aesthetic for a list.

    Raises
    ------
    UnsupportedValueError
        If ``aesthetic`` is not a string or a list of strings.
    SpecError
        If ``aesthetic`` is an empty list.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3], "y": [3, 1, 2], "paint": ["tomato", "gold", "teal"]}
    >>> plot = ggplot(data, aes("x", "y", color="paint", fill="paint"))
    >>> plot = plot + geom_point(shape=21, size=10, alpha=0.5)
    >>> plot = plot + scale_identity(["color", "fill"])
    """
    return _scale(
        _aesthetics(aesthetic, "scale_identity"),
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        na_value=na_value,
        guide=guide,
        format=format,
        scale_mapper_kind="identity",
    )
