"""Scales from ggplot2 and plotnine that lets_plot does not have.

``scale_*_distiller`` (a brewer palette as a gradient), the ``scale_*_ordinal``
scales and ``scale_size_radius``.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Literal

from ggrust.plot.scale import _scale

if TYPE_CHECKING:
    from collections.abc import Sequence

    from ggrust._typing import Breaks, Color, Guide, Labels, Limits, Number, Oob, Transform
    from ggrust.plot.core import FeatureSpec

__all__ = [
    "scale_alpha_ordinal",
    "scale_color_distiller",
    "scale_color_ordinal",
    "scale_fill_distiller",
    "scale_fill_ordinal",
    "scale_size_ordinal",
    "scale_size_radius",
]

_ColorAesthetic = Literal["color", "fill"]


def _distiller(
    aesthetic: _ColorAesthetic,
    type: Literal["seq", "div", "qual"] | None,
    palette: str | int | None,
    direction: int,
    **scale: object,
) -> FeatureSpec:
    """Build a ``color_distiller`` scale for ``aesthetic``."""
    return _scale(
        aesthetic,
        type=type,
        palette=palette,
        direction=direction,
        scale_mapper_kind="color_distiller",
        **scale,  # ty: ignore[invalid-argument-type]
    )


def scale_color_distiller(
    type: Literal["seq", "div", "qual"] | None = None,
    palette: str | int | None = None,
    direction: int = -1,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    limits: Limits | None = None,
    oob: Oob | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map continuous ``color`` with a ColorBrewer palette as a smooth gradient.

    The palette's 7-color set is interpolated (ggplot2 ``scale_colour_distiller``),
    where ``scale_color_brewer`` cuts continuous data into the palette's bins.

    Parameters
    ----------
    type : {"seq", "div", "qual"}, optional
        Palette family; needed with a palette index.
    palette : str or int, optional
        Palette name (e.g. ``"Blues"``, ``"RdBu"``) or 0-based index within
        ``type`` (as ``scale_color_brewer``). Default: ``Blues``.
    direction : {-1, 1}, default=-1
        -1 (as ggplot2) puts the palette's last color at the low end, so a
        sequential palette is darkest for the smallest values.
    name : str, optional
        Legend title.
    breaks : list, optional
        Values that get a colorbar tick.
    labels : list of str or dict, optional
        One label per break (a dict maps breaks to labels).
    limits : list, optional
        ``[low, high]`` data range. Data outside the limits gets ``na_value``.
    oob : {"censor", "squish"}, optional
        What data outside ``limits`` maps to: ``na_value`` or the end color.
    na_value : str, optional
        Color of missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"colorbar"``, ``"legend"``, ``"none"``, or a guide spec.
    trans : str, optional
        Transformation of the data (``"log10"``, ``"sqrt"``, ...), as in
        ``scale_color_continuous``.
    format : str, optional
        Label format (``".2f"``, ``"{.1f} %"``).

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3, 4], "y": [3, 1, 4, 2], "speed": [10, 20, 30, 40]}
    >>> plot = ggplot(data, aes("x", "y", color="speed")) + geom_point(size=6)
    >>> plot = plot + scale_color_distiller(palette="Spectral")
    """
    return _distiller(
        "color",
        type,
        palette,
        direction,
        name=name,
        breaks=breaks,
        labels=labels,
        limits=limits,
        oob=oob,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
    )


def scale_fill_distiller(
    type: Literal["seq", "div", "qual"] | None = None,
    palette: str | int | None = None,
    direction: int = -1,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    limits: Limits | None = None,
    oob: Oob | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map continuous ``fill`` with a ColorBrewer palette as a smooth gradient.

    Parameters
    ----------
    type : {"seq", "div", "qual"}, optional
        Palette family; needed with a palette index.
    palette : str or int, optional
        Palette name or 0-based index within ``type``. Default: ``Blues``.
    direction : {-1, 1}, default=-1
        -1 (as ggplot2) puts the palette's last color at the low end.
    name : str, optional
        Legend title.
    breaks : list, optional
        Values that get a colorbar tick.
    labels : list of str or dict, optional
        One label per break.
    limits : list, optional
        ``[low, high]`` data range.
    oob : {"censor", "squish"}, optional
        What data outside ``limits`` maps to.
    na_value : str, optional
        Color of missing data and data outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"colorbar"``, ``"legend"``, ``"none"``, or a guide spec.
    trans : str, optional
        Transformation of the data.
    format : str, optional
        Label format.

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 1, 2], "y": [1, 1, 2, 2], "rain": [3, 8, 12, 20]}
    >>> plot = ggplot(data, aes("x", "y", fill="rain")) + geom_tile()
    >>> plot = plot + scale_fill_distiller(palette="YlGnBu", direction=1)
    """
    return _distiller(
        "fill",
        type,
        palette,
        direction,
        name=name,
        breaks=breaks,
        labels=labels,
        limits=limits,
        oob=oob,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
    )


def scale_color_ordinal(
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ordered categories of ``color`` to evenly spaced viridis colors.

    As ggplot2's default for ordered factors (``scale_colour_viridis_d``).

    Parameters
    ----------
    name : str, optional
        Legend title.
    breaks : list, optional
        Levels that get a legend key.
    labels : list of str or dict, optional
        One label per break.
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The levels and their order.
    na_value : str, optional
        Color of missing data and levels outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a guide spec.
    format : str, optional
        Label format.

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3], "y": [1, 2, 3], "size": ["small", "medium", "large"]}
    >>> plot = ggplot(data, aes("x", "y", color="size")) + geom_point(size=6)
    >>> plot = plot + scale_color_ordinal(limits=["small", "medium", "large"])
    """
    return _scale("color", **locals(), discrete=True, scale_mapper_kind="color_cmap")


def scale_fill_ordinal(
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ordered categories of ``fill`` to evenly spaced viridis colors.

    Parameters
    ----------
    name : str, optional
        Legend title.
    breaks : list, optional
        Levels that get a legend key.
    labels : list of str or dict, optional
        One label per break.
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The levels and their order.
    na_value : str, optional
        Color of missing data and levels outside ``limits``.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a guide spec.
    format : str, optional
        Label format.

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"grade": ["low", "mid", "high"], "count": [4, 7, 2]}
    >>> plot = ggplot(data, aes("grade", "count", fill="grade")) + geom_col()
    >>> plot = plot + scale_fill_ordinal(limits=["low", "mid", "high"])
    """
    return _scale("fill", **locals(), discrete=True, scale_mapper_kind="color_cmap")


def scale_alpha_ordinal(
    range: Sequence[Number] = (0.1, 1),
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Number | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ordered categories of ``alpha`` to evenly spaced opacities.

    Parameters
    ----------
    range : list of float, default=(0.1, 1)
        ``[lowest, highest]`` opacity (ggplot2 and plotnine default).
    name : str, optional
        Legend title.
    breaks : list, optional
        Levels that get a legend key.
    labels : list of str or dict, optional
        One label per break.
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The levels and their order.
    na_value : float, optional
        Opacity of missing data.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a guide spec.
    format : str, optional
        Label format.

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"grade": ["low", "mid", "high"], "count": [4, 7, 2]}
    >>> plot = ggplot(data, aes("grade", "count", alpha="grade")) + geom_col()
    >>> plot = plot + scale_alpha_ordinal(limits=["low", "mid", "high"])
    """
    return _scale(
        "alpha",
        range=list(range),
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        na_value=na_value,
        guide=guide,
        format=format,
        discrete=True,
    )


def scale_size_ordinal(
    range: Sequence[Number] = (2, 6),
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Number | None = None,
    guide: Guide | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ordered categories of ``size`` to evenly spaced sizes.

    Parameters
    ----------
    range : list of float, default=(2, 6)
        ``[smallest, largest]`` size (ggplot2 and plotnine default). The sizes
        step evenly in diameter, as every ggrust size scale (ggplot2 steps
        evenly in area).
    name : str, optional
        Legend title.
    breaks : list, optional
        Levels that get a legend key.
    labels : list of str or dict, optional
        One label per break.
    lablim : int, optional
        Maximum label length in characters; longer labels are cut.
    limits : list, optional
        The levels and their order.
    na_value : float, optional
        Size of missing data.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a guide spec.
    format : str, optional
        Label format.

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3], "y": [1, 2, 3], "shirt": ["S", "M", "L"]}
    >>> plot = ggplot(data, aes("x", "y", size="shirt")) + geom_point()
    >>> plot = plot + scale_size_ordinal(limits=["S", "M", "L"])
    """
    return _scale(
        "size",
        range=list(range),
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        na_value=na_value,
        guide=guide,
        format=format,
        discrete=True,
    )


def scale_size_radius(
    range: Sequence[Number] = (1, 6),
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    limits: Limits | None = None,
    na_value: Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map continuous ``size`` linearly onto the point radius.

    As ggplot2 ``scale_radius`` and plotnine ``scale_size_radius``: the
    diameter, not the area, grows linearly with the value. This is the mapping
    of ``scale_size`` (lets-plot), with ggplot2's default range.

    Parameters
    ----------
    range : list of float, default=(1, 6)
        ``[smallest, largest]`` size.
    name : str, optional
        Legend title.
    breaks : list, optional
        Values that get a legend key.
    labels : list of str or dict, optional
        One label per break.
    limits : list, optional
        ``[low, high]`` data range.
    na_value : float, optional
        Size of missing data.
    guide : str or FeatureSpec, optional
        ``"legend"``, ``"none"``, or a guide spec.
    trans : str, optional
        Transformation of the data.
    format : str, optional
        Label format.

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3, 4], "y": [3, 1, 4, 2], "people": [10, 200, 50, 800]}
    >>> plot = ggplot(data, aes("x", "y", size="people")) + geom_point()
    >>> plot = plot + scale_size_radius(range=[2, 10])
    """
    return _scale(
        "size",
        range=list(range),
        name=name,
        breaks=breaks,
        labels=labels,
        limits=limits,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
    )
