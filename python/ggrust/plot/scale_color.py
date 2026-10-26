"""Color scales: hue, manual, gradients, viridis, continuous; ``fill`` variants.

Ported from lets-plot's ``lets_plot/plot/scale.py`` (MIT, see THIRD_PARTY_NOTICES.md).
Brewer scales come in a later phase.
"""

from ggrust.plot.core import FeatureSpec
from ggrust.plot.scale import _scale

__all__ = [
    "scale_color_continuous",
    "scale_color_gradient",
    "scale_color_gradient2",
    "scale_color_gradientn",
    "scale_color_hue",
    "scale_color_manual",
    "scale_color_viridis",
    "scale_fill_continuous",
    "scale_fill_gradient",
    "scale_fill_gradient2",
    "scale_fill_gradientn",
    "scale_fill_hue",
    "scale_fill_manual",
    "scale_fill_viridis",
]


def scale_color_hue(
    h=None,
    c=None,
    l=None,  # noqa: E741 (lets_plot's name)
    h_start=None,
    direction=None,
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    na_value=None,
    guide=None,
    trans=None,
    format=None,
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
    h=None,
    c=None,
    l=None,  # noqa: E741 (lets_plot's name)
    h_start=None,
    direction=None,
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    na_value=None,
    guide=None,
    trans=None,
    format=None,
) -> FeatureSpec:
    """Qualitative colors evenly spaced around the HCL color wheel, for ``fill``.

    See ``scale_color_hue`` for the parameters.

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _hue("fill", **locals())


def scale_color_manual(
    values,
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    na_value=None,
    guide=None,
    format=None,
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
    values,
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    na_value=None,
    guide=None,
    format=None,
) -> FeatureSpec:
    """Your own colors for ``fill``.

    See ``scale_color_manual`` for the parameters.

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _manual("fill", **locals())


def _hue(aesthetic, *, h, c, l, h_start, direction, **options) -> FeatureSpec:  # noqa: E741
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


def _manual(aesthetic, *, values, breaks, limits, **options) -> FeatureSpec:
    """lets_plot ``scale_manual``: a dict of values is matched by category.

    Without breaks or limits its keys become the breaks. Otherwise the values are
    reordered by the limits (or breaks); unmatched values follow in dict order
    (lets_plot uses a set there, so its order is not deterministic).
    """
    if isinstance(values, dict):
        if breaks is None and limits is None:
            breaks = list(values.keys())
            values = list(values.values())
        else:
            base_order = breaks if limits is None else limits
            if isinstance(base_order, dict):
                base_order = list(base_order.values())
            matched = [values[key] for key in base_order if key in values]
            if matched:
                values = matched + [value for value in values.values() if value not in matched]
            else:
                values = None
    return _scale(aesthetic, breaks=breaks, limits=limits, **options, values=values)


def scale_color_gradient(
    low=None,
    high=None,
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    na_value=None,
    guide=None,
    trans=None,
    format=None,
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
    low=None,
    high=None,
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    na_value=None,
    guide=None,
    trans=None,
    format=None,
) -> FeatureSpec:
    """Map ``fill`` to a two-color gradient. See ``scale_color_gradient``.

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _scale("fill", **locals(), scale_mapper_kind="color_gradient")


def scale_color_gradient2(
    low=None,
    mid=None,
    high=None,
    midpoint=0,
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    na_value=None,
    guide=None,
    trans=None,
    format=None,
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
    low=None,
    mid=None,
    high=None,
    midpoint=0,
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    na_value=None,
    guide=None,
    trans=None,
    format=None,
) -> FeatureSpec:
    """Map ``fill`` to a diverging gradient. See ``scale_color_gradient2``.

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _scale("fill", **locals(), scale_mapper_kind="color_gradient2")


def scale_color_gradientn(
    colors=None,
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    na_value=None,
    guide=None,
    trans=None,
    format=None,
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
    colors=None,
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    na_value=None,
    guide=None,
    trans=None,
    format=None,
) -> FeatureSpec:
    """Map ``fill`` to a gradient through several colors. See ``scale_color_gradientn``.

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _scale("fill", **locals(), scale_mapper_kind="color_gradientn")


def scale_color_viridis(
    alpha=None,
    begin=None,
    end=None,
    direction=None,
    option=None,
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    na_value=None,
    guide=None,
    trans=None,
    format=None,
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
    alpha=None,
    begin=None,
    end=None,
    direction=None,
    option=None,
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    na_value=None,
    guide=None,
    trans=None,
    format=None,
) -> FeatureSpec:
    """Map ``fill`` with a viridis-family color map. See ``scale_color_viridis``.

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _scale("fill", **locals(), scale_mapper_kind="color_cmap")


def scale_color_continuous(
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    na_value=None,
    guide=None,
    trans=None,
    format=None,
    scale_mapper_kind=None,
    **kwargs,
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
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    na_value=None,
    guide=None,
    trans=None,
    format=None,
    scale_mapper_kind=None,
    **kwargs,
) -> FeatureSpec:
    """Map ``fill`` continuously. See ``scale_color_continuous``.

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _continuous("fill", locals())


def _continuous(aesthetic, options: dict) -> FeatureSpec:
    """lets_plot ``scale_continuous``: color scales default to ``color_gradient``."""
    options = dict(options)
    kwargs = options.pop("kwargs")
    if options["scale_mapper_kind"] is None:
        options["scale_mapper_kind"] = "color_gradient"
    return _scale(aesthetic, **options, expand=None, **kwargs)
