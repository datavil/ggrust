"""Color scales: ``scale_color_hue()``, ``scale_color_manual()``, ``fill`` variants.

Ported from lets-plot's ``lets_plot/plot/scale.py`` (MIT, see THIRD_PARTY_NOTICES.md).
Gradient, brewer and viridis scales come in later phases.
"""

from ggrust.plot.core import FeatureSpec
from ggrust.plot.scale import _scale

__all__ = [
    "scale_color_hue",
    "scale_color_manual",
    "scale_fill_hue",
    "scale_fill_manual",
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
