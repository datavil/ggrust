"""``guides()`` and the guide spec helper.

Ported from lets-plot's ``lets_plot/plot/guide.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from ggrust.plot.core import FeatureSpec

__all__ = ["guides"]


def _guide(name, **kwargs) -> FeatureSpec:
    if "title" in kwargs and isinstance(kwargs["title"], int):
        message = "Use keyword arguments for all other than 'title' parameters."
        raise ValueError(message)
    return FeatureSpec("guide", name=name, **kwargs)


def guides(**kwargs) -> FeatureSpec:
    """Set guides (legends, colorbars) per aesthetic.

    Parameters
    ----------
    **kwargs
        Aesthetic name to guide (``"none"``, ``guide_legend()``, ...).

    Returns
    -------
    FeatureSpec
        The guides feature.
    """
    return FeatureSpec("guides", name=None, **kwargs)
