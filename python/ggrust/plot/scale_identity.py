"""Identity scales: the data are the values (``scale_color_identity``, ...).

Ported from lets-plot's ``lets_plot/plot/scale_identity_.py`` (MIT, see
THIRD_PARTY_NOTICES.md).
"""

from ggrust.plot.core import FeatureSpec
from ggrust.plot.scale import _scale

__all__ = [
    "scale_alpha_identity",
    "scale_color_identity",
    "scale_fill_identity",
    "scale_shape_identity",
    "scale_size_identity",
]


def _identity(aesthetic, **options) -> FeatureSpec:
    return _scale(aesthetic, scale_mapper_kind="identity", **options)


def scale_color_identity(
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    na_value=None,
    guide="none",
    format=None,
) -> FeatureSpec:
    """Use the data as colors (names or hex codes); no legend by default.

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _identity("color", **locals())


def scale_fill_identity(
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    na_value=None,
    guide="none",
    format=None,
) -> FeatureSpec:
    """Use the data as fill colors; no legend by default.

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _identity("fill", **locals())


def scale_size_identity(
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    na_value=None,
    guide="none",
    format=None,
) -> FeatureSpec:
    """Use the data as sizes; no legend by default.

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _identity("size", **locals())


def scale_alpha_identity(
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    na_value=None,
    guide="none",
    format=None,
) -> FeatureSpec:
    """Use the data as alphas; no legend by default.

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _identity("alpha", **locals())


def scale_shape_identity(
    name=None,
    breaks=None,
    labels=None,
    lablim=None,
    limits=None,
    na_value=None,
    guide="none",
    format=None,
) -> FeatureSpec:
    """Use the data as shape codes (0..25); no legend by default.

    Returns
    -------
    FeatureSpec
        The scale.
    """
    return _identity("shape", **locals())
