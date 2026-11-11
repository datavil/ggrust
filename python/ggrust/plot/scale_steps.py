"""Binned color scales (ggplot2 only): ``scale_*_steps``, ``steps2``, ``stepsn``, ``binned``.

Values are cut into bins at the scale's breaks and take the color at the middle of
their bin; the colorbar shows one flat block per bin. Not in lets-plot.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from ggrust.plot.scale import _scale

if TYPE_CHECKING:
    from collections.abc import Sequence

    from ggrust._typing import Breaks, Color, Guide, Labels, Limits, Number, Oob, Transform
    from ggrust.plot.core import FeatureSpec

__all__ = [
    "scale_color_binned",
    "scale_color_steps",
    "scale_color_steps2",
    "scale_color_stepsn",
    "scale_fill_binned",
    "scale_fill_steps",
    "scale_fill_steps2",
    "scale_fill_stepsn",
]


def scale_color_steps(
    low: Color | None = None,
    high: Color | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    limits: Limits | None = None,
    n_breaks: int | None = None,
    nice_breaks: bool = True,
    right: bool = True,
    show_limits: bool = False,
    oob: Oob = "squish",
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``color`` to binned colors of a two-color gradient (ggplot2).

    Each value takes the gradient's color at the middle of its bin, so the colors
    step from bin to bin and the colorbar shows one flat block per bin.

    Parameters
    ----------
    low, high : str, default="#132B43", "#56B1F7"
        Colors of the lowest and highest bin.
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
        Also label the two ends of the colorbar.
    oob : {"squish", "censor"}, default="squish"
        What data outside ``limits`` maps to: the end bins (``"squish"``) or
        ``na_value`` (``"censor"``).
    na_value : str, optional
        Color of missing data.
    guide : str or FeatureSpec, optional
        ``"colorbar"`` (default, drawn as steps), ``"legend"``, ``"none"``, or a
        ``guide_colorbar()`` / ``guide_legend()`` spec.
    trans : str, optional
        Transformation of the data, as in ``scale_color_gradient``; breaks are
        generated in transformed units.
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
    >>> data = {"x": [1, 2, 3] * 3, "y": [1] * 3 + [2] * 3 + [3] * 3,
    ...         "change": [-3, -1, 0, -2, 0, 2, 0, 1, 4]}
    >>> plot = ggplot(data, aes("x", "y", color="change")) + geom_point(size=12)
    >>> plot = plot + scale_color_steps(low="lightyellow", high="firebrick")
    """
    return _scale(
        "color",
        name=name,
        breaks=breaks,
        labels=labels,
        limits=limits,
        n_breaks=n_breaks,
        nice_breaks=nice_breaks,
        right=right,
        show_limits=show_limits,
        oob=oob,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
        scale_mapper_kind="color_gradient",
        binned=True,
        low=low,
        high=high,
    )


def scale_color_steps2(
    low: Color | None = None,
    mid: Color | None = None,
    high: Color | None = None,
    midpoint: Number = 0,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    limits: Limits | None = None,
    n_breaks: int | None = None,
    nice_breaks: bool = True,
    right: bool = True,
    show_limits: bool = False,
    oob: Oob = "squish",
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``color`` to binned colors of a diverging gradient (ggplot2).

    Binned like ``scale_color_steps``, colored like ``scale_color_gradient2``.

    Parameters
    ----------
    low, mid, high : str, default="#964540", "white", "#3B3D96"
        Colors of the low end, the midpoint and the high end.
    midpoint : float, default=0
        Data value that gets ``mid``.
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
        Also label the two ends of the colorbar.
    oob : {"squish", "censor"}, default="squish"
        What data outside ``limits`` maps to: the end bins (``"squish"``) or
        ``na_value`` (``"censor"``).
    na_value : str, optional
        Color of missing data.
    guide : str or FeatureSpec, optional
        ``"colorbar"`` (default, drawn as steps), ``"legend"``, ``"none"``, or a
        ``guide_colorbar()`` / ``guide_legend()`` spec.
    trans : str, optional
        Transformation of the data, as in ``scale_color_gradient``; breaks are
        generated in transformed units.
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
    >>> data = {"x": [1, 2, 3] * 3, "y": [1] * 3 + [2] * 3 + [3] * 3,
    ...         "change": [-3, -1, 0, -2, 0, 2, 0, 1, 4]}
    >>> plot = ggplot(data, aes("x", "y", color="change")) + geom_point(size=12)
    >>> plot = plot + scale_color_steps2(midpoint=0)
    """
    return _scale(
        "color",
        name=name,
        breaks=breaks,
        labels=labels,
        limits=limits,
        n_breaks=n_breaks,
        nice_breaks=nice_breaks,
        right=right,
        show_limits=show_limits,
        oob=oob,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
        scale_mapper_kind="color_gradient2",
        binned=True,
        low=low,
        mid=mid,
        high=high,
        midpoint=midpoint,
    )


def scale_color_stepsn(
    colors: Sequence[Color] | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    limits: Limits | None = None,
    n_breaks: int | None = None,
    nice_breaks: bool = True,
    right: bool = True,
    show_limits: bool = False,
    oob: Oob = "squish",
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``color`` to binned colors of an n-color gradient (ggplot2).

    Binned like ``scale_color_steps``, colored like ``scale_color_gradientn``.

    Parameters
    ----------
    colors : list of str
        Two or more colors spread evenly over the range.
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
        Also label the two ends of the colorbar.
    oob : {"squish", "censor"}, default="squish"
        What data outside ``limits`` maps to: the end bins (``"squish"``) or
        ``na_value`` (``"censor"``).
    na_value : str, optional
        Color of missing data.
    guide : str or FeatureSpec, optional
        ``"colorbar"`` (default, drawn as steps), ``"legend"``, ``"none"``, or a
        ``guide_colorbar()`` / ``guide_legend()`` spec.
    trans : str, optional
        Transformation of the data, as in ``scale_color_gradient``; breaks are
        generated in transformed units.
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
    >>> data = {"x": [1, 2, 3] * 3, "y": [1] * 3 + [2] * 3 + [3] * 3,
    ...         "change": [-3, -1, 0, -2, 0, 2, 0, 1, 4]}
    >>> plot = ggplot(data, aes("x", "y", color="change")) + geom_point(size=12)
    >>> plot = plot + scale_color_stepsn(colors=["navy", "gold", "darkred"])
    """
    return _scale(
        "color",
        name=name,
        breaks=breaks,
        labels=labels,
        limits=limits,
        n_breaks=n_breaks,
        nice_breaks=nice_breaks,
        right=right,
        show_limits=show_limits,
        oob=oob,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
        scale_mapper_kind="color_gradientn",
        binned=True,
        colors=colors,
    )


def scale_color_binned(
    low: Color | None = None,
    high: Color | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    limits: Limits | None = None,
    n_breaks: int | None = None,
    nice_breaks: bool = True,
    right: bool = True,
    show_limits: bool = False,
    oob: Oob = "squish",
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``color`` to binned colors: ``scale_color_steps`` (ggplot2).

    ggplot2's default binned color scale, the same as ``scale_color_steps``.

    Parameters
    ----------
    low, high : str, default="#132B43", "#56B1F7"
        Colors of the lowest and highest bin.
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
        Also label the two ends of the colorbar.
    oob : {"squish", "censor"}, default="squish"
        What data outside ``limits`` maps to: the end bins (``"squish"``) or
        ``na_value`` (``"censor"``).
    na_value : str, optional
        Color of missing data.
    guide : str or FeatureSpec, optional
        ``"colorbar"`` (default, drawn as steps), ``"legend"``, ``"none"``, or a
        ``guide_colorbar()`` / ``guide_legend()`` spec.
    trans : str, optional
        Transformation of the data, as in ``scale_color_gradient``; breaks are
        generated in transformed units.
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
    >>> data = {"x": [1, 2, 3] * 3, "y": [1] * 3 + [2] * 3 + [3] * 3,
    ...         "change": [-3, -1, 0, -2, 0, 2, 0, 1, 4]}
    >>> plot = ggplot(data, aes("x", "y", color="change")) + geom_point(size=12)
    >>> plot = plot + scale_color_binned()
    """
    return _scale(
        "color",
        name=name,
        breaks=breaks,
        labels=labels,
        limits=limits,
        n_breaks=n_breaks,
        nice_breaks=nice_breaks,
        right=right,
        show_limits=show_limits,
        oob=oob,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
        scale_mapper_kind="color_gradient",
        binned=True,
        low=low,
        high=high,
    )


def scale_fill_steps(
    low: Color | None = None,
    high: Color | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    limits: Limits | None = None,
    n_breaks: int | None = None,
    nice_breaks: bool = True,
    right: bool = True,
    show_limits: bool = False,
    oob: Oob = "squish",
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``fill`` to binned colors of a two-color gradient (ggplot2).

    Each value takes the gradient's color at the middle of its bin, so the colors
    step from bin to bin and the colorbar shows one flat block per bin.

    Parameters
    ----------
    low, high : str, default="#132B43", "#56B1F7"
        Colors of the lowest and highest bin.
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
        Also label the two ends of the colorbar.
    oob : {"squish", "censor"}, default="squish"
        What data outside ``limits`` maps to: the end bins (``"squish"``) or
        ``na_value`` (``"censor"``).
    na_value : str, optional
        Color of missing data.
    guide : str or FeatureSpec, optional
        ``"colorbar"`` (default, drawn as steps), ``"legend"``, ``"none"``, or a
        ``guide_colorbar()`` / ``guide_legend()`` spec.
    trans : str, optional
        Transformation of the data, as in ``scale_color_gradient``; breaks are
        generated in transformed units.
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
    >>> data = {"x": [1, 2, 3] * 3, "y": [1] * 3 + [2] * 3 + [3] * 3,
    ...         "change": [-3, -1, 0, -2, 0, 2, 0, 1, 4]}
    >>> plot = ggplot(data, aes("x", "y", fill="change")) + geom_tile()
    >>> plot = plot + scale_fill_steps(low="lightyellow", high="firebrick")
    """
    return _scale(
        "fill",
        name=name,
        breaks=breaks,
        labels=labels,
        limits=limits,
        n_breaks=n_breaks,
        nice_breaks=nice_breaks,
        right=right,
        show_limits=show_limits,
        oob=oob,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
        scale_mapper_kind="color_gradient",
        binned=True,
        low=low,
        high=high,
    )


def scale_fill_steps2(
    low: Color | None = None,
    mid: Color | None = None,
    high: Color | None = None,
    midpoint: Number = 0,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    limits: Limits | None = None,
    n_breaks: int | None = None,
    nice_breaks: bool = True,
    right: bool = True,
    show_limits: bool = False,
    oob: Oob = "squish",
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``fill`` to binned colors of a diverging gradient (ggplot2).

    Binned like ``scale_fill_steps``, colored like ``scale_fill_gradient2``.

    Parameters
    ----------
    low, mid, high : str, default="#964540", "white", "#3B3D96"
        Colors of the low end, the midpoint and the high end.
    midpoint : float, default=0
        Data value that gets ``mid``.
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
        Also label the two ends of the colorbar.
    oob : {"squish", "censor"}, default="squish"
        What data outside ``limits`` maps to: the end bins (``"squish"``) or
        ``na_value`` (``"censor"``).
    na_value : str, optional
        Color of missing data.
    guide : str or FeatureSpec, optional
        ``"colorbar"`` (default, drawn as steps), ``"legend"``, ``"none"``, or a
        ``guide_colorbar()`` / ``guide_legend()`` spec.
    trans : str, optional
        Transformation of the data, as in ``scale_color_gradient``; breaks are
        generated in transformed units.
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
    >>> data = {"x": [1, 2, 3] * 3, "y": [1] * 3 + [2] * 3 + [3] * 3,
    ...         "change": [-3, -1, 0, -2, 0, 2, 0, 1, 4]}
    >>> plot = ggplot(data, aes("x", "y", fill="change")) + geom_tile()
    >>> plot = plot + scale_fill_steps2(midpoint=0)
    """
    return _scale(
        "fill",
        name=name,
        breaks=breaks,
        labels=labels,
        limits=limits,
        n_breaks=n_breaks,
        nice_breaks=nice_breaks,
        right=right,
        show_limits=show_limits,
        oob=oob,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
        scale_mapper_kind="color_gradient2",
        binned=True,
        low=low,
        mid=mid,
        high=high,
        midpoint=midpoint,
    )


def scale_fill_stepsn(
    colors: Sequence[Color] | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    limits: Limits | None = None,
    n_breaks: int | None = None,
    nice_breaks: bool = True,
    right: bool = True,
    show_limits: bool = False,
    oob: Oob = "squish",
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``fill`` to binned colors of an n-color gradient (ggplot2).

    Binned like ``scale_fill_steps``, colored like ``scale_fill_gradientn``.

    Parameters
    ----------
    colors : list of str
        Two or more colors spread evenly over the range.
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
        Also label the two ends of the colorbar.
    oob : {"squish", "censor"}, default="squish"
        What data outside ``limits`` maps to: the end bins (``"squish"``) or
        ``na_value`` (``"censor"``).
    na_value : str, optional
        Color of missing data.
    guide : str or FeatureSpec, optional
        ``"colorbar"`` (default, drawn as steps), ``"legend"``, ``"none"``, or a
        ``guide_colorbar()`` / ``guide_legend()`` spec.
    trans : str, optional
        Transformation of the data, as in ``scale_color_gradient``; breaks are
        generated in transformed units.
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
    >>> data = {"x": [1, 2, 3] * 3, "y": [1] * 3 + [2] * 3 + [3] * 3,
    ...         "change": [-3, -1, 0, -2, 0, 2, 0, 1, 4]}
    >>> plot = ggplot(data, aes("x", "y", fill="change")) + geom_tile()
    >>> plot = plot + scale_fill_stepsn(colors=["navy", "gold", "darkred"])
    """
    return _scale(
        "fill",
        name=name,
        breaks=breaks,
        labels=labels,
        limits=limits,
        n_breaks=n_breaks,
        nice_breaks=nice_breaks,
        right=right,
        show_limits=show_limits,
        oob=oob,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
        scale_mapper_kind="color_gradientn",
        binned=True,
        colors=colors,
    )


def scale_fill_binned(
    low: Color | None = None,
    high: Color | None = None,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    limits: Limits | None = None,
    n_breaks: int | None = None,
    nice_breaks: bool = True,
    right: bool = True,
    show_limits: bool = False,
    oob: Oob = "squish",
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``fill`` to binned colors: ``scale_fill_steps`` (ggplot2).

    ggplot2's default binned color scale, the same as ``scale_fill_steps``.

    Parameters
    ----------
    low, high : str, default="#132B43", "#56B1F7"
        Colors of the lowest and highest bin.
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
        Also label the two ends of the colorbar.
    oob : {"squish", "censor"}, default="squish"
        What data outside ``limits`` maps to: the end bins (``"squish"``) or
        ``na_value`` (``"censor"``).
    na_value : str, optional
        Color of missing data.
    guide : str or FeatureSpec, optional
        ``"colorbar"`` (default, drawn as steps), ``"legend"``, ``"none"``, or a
        ``guide_colorbar()`` / ``guide_legend()`` spec.
    trans : str, optional
        Transformation of the data, as in ``scale_color_gradient``; breaks are
        generated in transformed units.
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
    >>> data = {"x": [1, 2, 3] * 3, "y": [1] * 3 + [2] * 3 + [3] * 3,
    ...         "change": [-3, -1, 0, -2, 0, 2, 0, 1, 4]}
    >>> plot = ggplot(data, aes("x", "y", fill="change")) + geom_tile()
    >>> plot = plot + scale_fill_binned()
    """
    return _scale(
        "fill",
        name=name,
        breaks=breaks,
        labels=labels,
        limits=limits,
        n_breaks=n_breaks,
        nice_breaks=nice_breaks,
        right=right,
        show_limits=show_limits,
        oob=oob,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
        scale_mapper_kind="color_gradient",
        binned=True,
        low=low,
        high=high,
    )
