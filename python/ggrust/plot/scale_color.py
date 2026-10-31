"""Color scales: hue, manual, gradients, viridis, brewer, continuous; ``fill`` variants.

Ported from lets-plot's ``lets_plot/plot/scale.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, cast

from ggrust.plot.core import FeatureSpec
from ggrust.plot.scale import _scale

if TYPE_CHECKING:
    from collections.abc import Sequence

    from ggrust._typing import Breaks, Color, Guide, Labels, Limits, Number, Transform

__all__ = [
    "scale_color_brewer",
    "scale_color_continuous",
    "scale_color_gradient",
    "scale_color_gradient2",
    "scale_color_gradientn",
    "scale_color_hue",
    "scale_color_manual",
    "scale_color_viridis",
    "scale_fill_brewer",
    "scale_fill_continuous",
    "scale_fill_gradient",
    "scale_fill_gradient2",
    "scale_fill_gradientn",
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
    name, breaks, labels, lablim, limits, na_value, guide, trans, format : optional
        As in lets_plot.

    Returns
    -------
    FeatureSpec
        The scale.
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
    name, breaks, labels, lablim, limits, na_value, guide, trans, format : optional
        As in lets_plot.

    Returns
    -------
    FeatureSpec
        The scale.
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
    name, breaks, labels, lablim, limits, na_value, guide, format : optional
        As in lets_plot.

    Returns
    -------
    FeatureSpec
        The scale.
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
    name, breaks, labels, lablim, limits, na_value, guide, format : optional
        As in lets_plot.

    Returns
    -------
    FeatureSpec
        The scale.
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
    name, breaks, labels, lablim, limits, na_value, guide, trans, format : optional
        As in lets_plot.

    Returns
    -------
    FeatureSpec
        The scale.
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
    name, breaks, labels, lablim, limits, na_value, guide, trans, format : optional
        As in lets_plot.

    Returns
    -------
    FeatureSpec
        The scale.
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
    name, breaks, labels, lablim, limits, na_value, guide, trans, format : optional
        As in lets_plot.

    Returns
    -------
    FeatureSpec
        The scale.
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
    name, breaks, labels, lablim, limits, na_value, guide, trans, format : optional
        As in lets_plot.

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _scale("fill", **locals(), scale_mapper_kind="color_gradient2")


def scale_color_gradientn(
    colors: Sequence[Color] | None = None,
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
    """Map ``color`` to a gradient through several colors, evenly spaced over the data.

    Parameters
    ----------
    colors : list of str
        Two or more colors.
    name, breaks, labels, lablim, limits, na_value, guide, trans, format : optional
        As in lets_plot.

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _scale("color", **locals(), scale_mapper_kind="color_gradientn")


def scale_fill_gradientn(
    colors: Sequence[Color] | None = None,
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
    """Map ``fill`` to a gradient through several colors. See ``scale_color_gradientn``.

    Parameters
    ----------
    colors : list of str
        Two or more colors.
    name, breaks, labels, lablim, limits, na_value, guide, trans, format : optional
        As in lets_plot.

    Returns
    -------
    FeatureSpec
        The scale.
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
    name, breaks, labels, lablim, limits, na_value, guide, trans, format : optional
        As in lets_plot.

    Returns
    -------
    FeatureSpec
        The scale.
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
    name, breaks, labels, lablim, limits, na_value, guide, trans, format : optional
        As in lets_plot.

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _scale("fill", **locals(), scale_mapper_kind="color_cmap")


def scale_color_continuous(
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
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
    name, breaks, labels, lablim, limits, na_value, guide, trans, format : optional
        As in lets_plot.
    scale_mapper_kind : str, optional
        Default ``"color_gradient"``.
    **kwargs
        Options of the chosen mapper (e.g. ``low``, ``high``).

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _continuous("color", locals())


def scale_fill_continuous(
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
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
    name, breaks, labels, lablim, limits, na_value, guide, trans, format : optional
        As in lets_plot.
    scale_mapper_kind : str, optional
        Default ``"color_gradient"``.
    **kwargs
        Options of the chosen mapper (e.g. ``low``, ``high``).

    Returns
    -------
    FeatureSpec
        The scale.
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
    name, breaks, labels, lablim, limits, na_value, guide, trans, format : optional
        As in lets_plot.

    Returns
    -------
    FeatureSpec
        The scale.
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
    name, breaks, labels, lablim, limits, na_value, guide, trans, format : optional
        As in lets_plot.

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _scale("fill", **locals(), scale_mapper_kind="color_brewer")
