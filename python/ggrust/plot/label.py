"""Titles and labels: ``labs()``, ``ggtitle()``, ``xlab()``, ``ylab()``.

Ported from lets-plot's ``lets_plot/plot/label.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from __future__ import annotations

from ggrust.plot.core import FeatureSpec, FeatureSpecArray
from ggrust.plot.guide import _guide, guides

__all__ = ["ggtitle", "labs", "xlab", "ylab"]


def ggtitle(label: str, subtitle: str | None = None) -> FeatureSpec:
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

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> plot = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> plot = plot + ggtitle("Fuel use", subtitle="Heavier cars go less far")
    """
    return labs(title=label, subtitle=subtitle)


def xlab(label: str | None) -> FeatureSpec:
    """Set the x axis title.

    Parameters
    ----------
    label : str
        Axis title.

    Returns
    -------
    FeatureSpec
        The label feature.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> plot = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> plot = plot + xlab("Weight (1000 lb)")
    """
    return labs(x=label)


def ylab(label: str | None) -> FeatureSpec:
    """Set the y axis title.

    Parameters
    ----------
    label : str
        Axis title.

    Returns
    -------
    FeatureSpec
        The label feature.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> plot = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> plot = plot + ylab("Miles per gallon")
    """
    return labs(y=label)


def labs(
    title: str | None = None,
    subtitle: str | None = None,
    caption: str | None = None,
    tag: str | None = None,
    **labels: str | None,
) -> FeatureSpec:
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

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> plot = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> plot = plot + labs(
    ...     title="Fuel use", subtitle="Six cars", x="Weight (1000 lb)", y="Miles per gallon"
    ... )
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
