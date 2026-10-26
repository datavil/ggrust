"""Shape scales: ``scale_shape`` and ``scale_shape_manual``.

Ported from lets-plot's ``lets_plot/plot/scale.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from ggrust.plot.core import FeatureSpec
from ggrust.plot.scale import _scale
from ggrust.plot.scale_color import _manual

__all__ = ["scale_shape", "scale_shape_manual"]


def scale_shape(
    solid=True,
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    na_value=None,
    guide=None,
    format=None,
) -> FeatureSpec:
    """Point shapes for categories.

    Parameters
    ----------
    solid : bool, default True
        ``False`` uses only hollow shapes (codes 0..14).
    name, breaks, labels, lablim, limits, na_value, guide, format : optional
        As in lets_plot.

    Returns
    -------
    FeatureSpec
        The scale.
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
    """Your own point shapes (lets-plot codes 0..25) for ``shape``.

    Parameters
    ----------
    values : list of int or dict
        Shape codes matched in order with the scale's categories, or a dict from
        category to shape code.
    name, breaks, labels, lablim, limits, na_value, guide, format : optional
        As in lets_plot.

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _manual("shape", **locals())
