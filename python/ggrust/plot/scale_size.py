"""Size and alpha scales, with their ``_manual`` variants.

Ported from lets-plot's ``lets_plot/plot/scale.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from ggrust.plot.core import FeatureSpec
from ggrust.plot.scale import _scale
from ggrust.plot.scale_color import _manual

__all__ = [
    "scale_alpha",
    "scale_alpha_manual",
    "scale_size",
    "scale_size_area",
    "scale_size_manual",
]


def scale_size(
    range=None,
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
    """Sizes for ``size``: data mapped linearly onto ``range``.

    Parameters
    ----------
    range : list of float, optional
        ``[smallest, largest]`` size (default about 1.4..9.5, 3..21 px circles).
    trans : {'identity', 'log10', 'log2', 'sqrt', 'reverse'}, optional
        Map (and pick legend breaks) in transformed units.
    name, breaks, labels, lablim, limits, na_value, guide, format : optional
        As in lets_plot.

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _scale("size", **locals())


def scale_size_area(
    max_size=None,
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
    """Sizes proportional to the data: 0 is size 0, the largest value ``max_size``.

    Parameters
    ----------
    max_size : float, optional
        Size of the largest value (default about 9.5, a 21 px circle).
    name, breaks, labels, lablim, limits, na_value, guide, trans, format : optional
        As in lets_plot.

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _scale("size", scale_mapper_kind="size_area", **locals())


def scale_alpha(
    range=None,
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
    """Transparency for ``alpha``: data mapped linearly onto ``range``.

    Parameters
    ----------
    range : list of float, optional
        ``[lowest, highest]`` alpha (default 0.1..1).
    name, breaks, labels, lablim, limits, na_value, guide, trans, format : optional
        As in lets_plot.

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _scale("alpha", **locals())


def scale_size_manual(
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
    """Your own sizes for ``size``, matched in order with the categories (or a dict).

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _manual("size", **locals())


def scale_alpha_manual(
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
    """Your own alphas for ``alpha``, matched in order with the categories (or a dict).

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _manual("alpha", **locals())
