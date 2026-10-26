"""``guides()`` and the guide spec helper.

Ported from lets-plot's ``lets_plot/plot/guide.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from ggrust.plot.core import FeatureSpec

__all__ = ["guide_colorbar", "guide_legend", "guides"]


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


def guide_legend(title=None, *, nrow=None, ncol=None, byrow=None, override_aes=None) -> FeatureSpec:
    """Show a key legend for an aesthetic.

    Parameters
    ----------
    title : str, optional
        Legend title.
    nrow, ncol, byrow, override_aes : optional
        As in lets_plot (not drawn by ggrust yet; reported as warnings).

    Returns
    -------
    FeatureSpec
        The guide.
    """
    return _guide("legend", **locals())


def guide_colorbar(title=None, *, barwidth=None, barheight=None, nbin=None) -> FeatureSpec:
    """Show a colorbar for a continuous color or fill scale.

    Parameters
    ----------
    title : str, optional
        Colorbar title.
    barwidth, barheight : float, optional
        Bar size in pixels.
    nbin : int, optional
        Number of color segments (default 20).

    Returns
    -------
    FeatureSpec
        The guide.
    """
    return _guide("colorbar", **locals())
