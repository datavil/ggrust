"""Titles and labels: ``labs()``, ``ggtitle()``, ``xlab()``, ``ylab()``.

Ported from lets-plot's ``lets_plot/plot/label.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from ggrust.plot.core import FeatureSpec, FeatureSpecArray
from ggrust.plot.guide import _guide, guides

__all__ = ["ggtitle", "labs", "xlab", "ylab"]


def ggtitle(label, subtitle=None) -> FeatureSpec:
    """Set the plot title and subtitle.

    Parameters
    ----------
    label : str
        Title.
    subtitle : str, optional
        Subtitle.

    Returns
    -------
    FeatureSpec
        The title feature.
    """
    return labs(title=label, subtitle=subtitle)


def xlab(label) -> FeatureSpec:
    """Set the x axis title.

    Parameters
    ----------
    label : str
        Axis title.

    Returns
    -------
    FeatureSpec
        The label feature.
    """
    return labs(x=label)


def ylab(label) -> FeatureSpec:
    """Set the y axis title.

    Parameters
    ----------
    label : str
        Axis title.

    Returns
    -------
    FeatureSpec
        The label feature.
    """
    return labs(y=label)


def labs(title=None, subtitle=None, caption=None, tag=None, **labels):
    """Set the title, subtitle, caption, tag and axis or legend titles.

    Parameters
    ----------
    title, subtitle, caption, tag : str, optional
        Plot texts.
    **labels
        Aesthetic name to title, e.g. ``x="UMAP 1"``, ``color="Cluster"``.

    Returns
    -------
    FeatureSpec or FeatureSpecArray
        One feature, or several combined.
    """
    specs = []
    if title is not None or subtitle is not None:
        specs.append(FeatureSpec("ggtitle", name=None, text=title, subtitle=subtitle))
    if caption is not None:
        specs.append(FeatureSpec("caption", name=None, text=caption))
    if tag is not None:
        specs.append(FeatureSpec("tag", name=None, text=tag))
    for key, label in labels.items():
        specs.append(guides(**{key: _guide(name=None, title=label)}))
    if len(specs) == 1:
        return specs[0]
    return FeatureSpecArray(*specs)
