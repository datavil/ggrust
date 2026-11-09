"""Identity scales: the data are the values (``scale_color_identity``, ...).

Ported from lets-plot's ``lets_plot/plot/scale_identity_.py`` (MIT, see
THIRD_PARTY_NOTICES.md).
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from ggrust.plot.core import FeatureSpec
from ggrust.plot.scale import _scale

if TYPE_CHECKING:
    from collections.abc import Sequence
    from typing import Any

    from ggrust._typing import Breaks, Color, Guide, Labels, Limits, Number


__all__ = [
    "scale_alpha_identity",
    "scale_color_identity",
    "scale_fill_identity",
    "scale_linetype_identity",
    "scale_linewidth_identity",
    "scale_shape_identity",
    "scale_size_identity",
    "scale_stroke_identity",
]


def _identity(aesthetic: str | Sequence[str], **options: Any) -> FeatureSpec:
    return _scale(aesthetic, scale_mapper_kind="identity", **options)


def scale_color_identity(
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | Number | None = None,
    guide: Guide = "none",
    format: str | None = None,
) -> FeatureSpec:
    """Use the data as colors (names or hex codes); no legend by default.

    Parameters
    ----------
    name : str, optional
        Title of the scale, shown on the axis or the legend.
    breaks : list or dict, optional
        Positions of the ticks or legend keys: a list, or a dict of labels to values.
    labels : list or dict, optional
        Texts at the breaks: a list, or a dict of break values to texts.
    lablim : int, optional
        Maximum number of characters of a label; longer labels are cut.
    limits : list, optional
        Data range of the scale: ``[low, high]`` for continuous scales (None keeps a side), or the
        levels of a discrete scale.
    na_value : str or float, optional
        Value used for missing data.
    guide : str or FeatureSpec, default='none'
        Guide: ``"legend"``, ``"colorbar"``, ``"none"``, or a ``guide_*()`` spec.
    format : str, optional
        Format pattern of the labels, e.g. ``".2f"`` or ``"{d} %"``.

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3], "y": [3, 1, 2], "paint": ["tomato", "gold", "teal"]}
    >>> plot = ggplot(data, aes("x", "y", color="paint")) + geom_point(size=8)
    >>> plot = plot + scale_color_identity()
    """
    return _identity("color", **locals())


def scale_fill_identity(
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | Number | None = None,
    guide: Guide = "none",
    format: str | None = None,
) -> FeatureSpec:
    """Use the data as fill colors; no legend by default.

    Parameters
    ----------
    name : str, optional
        Title of the scale, shown on the axis or the legend.
    breaks : list or dict, optional
        Positions of the ticks or legend keys: a list, or a dict of labels to values.
    labels : list or dict, optional
        Texts at the breaks: a list, or a dict of break values to texts.
    lablim : int, optional
        Maximum number of characters of a label; longer labels are cut.
    limits : list, optional
        Data range of the scale: ``[low, high]`` for continuous scales (None keeps a side), or the
        levels of a discrete scale.
    na_value : str or float, optional
        Value used for missing data.
    guide : str or FeatureSpec, default='none'
        Guide: ``"legend"``, ``"colorbar"``, ``"none"``, or a ``guide_*()`` spec.
    format : str, optional
        Format pattern of the labels, e.g. ``".2f"`` or ``"{d} %"``.

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": ["a", "b", "c"], "y": [3, 1, 2], "paint": ["tomato", "gold", "teal"]}
    >>> plot = ggplot(data, aes("x", "y", fill="paint")) + geom_bar(stat="identity")
    >>> plot = plot + scale_fill_identity()
    """
    return _identity("fill", **locals())


def scale_size_identity(
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | Number | None = None,
    guide: Guide = "none",
    format: str | None = None,
) -> FeatureSpec:
    """Use the data as sizes; no legend by default.

    Parameters
    ----------
    name : str, optional
        Title of the scale, shown on the axis or the legend.
    breaks : list or dict, optional
        Positions of the ticks or legend keys: a list, or a dict of labels to values.
    labels : list or dict, optional
        Texts at the breaks: a list, or a dict of break values to texts.
    lablim : int, optional
        Maximum number of characters of a label; longer labels are cut.
    limits : list, optional
        Data range of the scale: ``[low, high]`` for continuous scales (None keeps a side), or the
        levels of a discrete scale.
    na_value : str or float, optional
        Value used for missing data.
    guide : str or FeatureSpec, default='none'
        Guide: ``"legend"``, ``"colorbar"``, ``"none"``, or a ``guide_*()`` spec.
    format : str, optional
        Format pattern of the labels, e.g. ``".2f"`` or ``"{d} %"``.

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3], "y": [1, 2, 3], "radius": [3, 6, 12]}
    >>> plot = ggplot(data, aes("x", "y", size="radius")) + geom_point()
    >>> plot = plot + scale_size_identity()
    """
    return _identity("size", **locals())


def scale_linewidth_identity(
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Number | None = None,
    guide: Guide = "none",
    format: str | None = None,
) -> FeatureSpec:
    """Use the data as line widths; no legend by default.

    Parameters
    ----------
    name : str, optional
        Title of the scale, shown on the legend.
    breaks : list or dict, optional
        Values of the legend keys: a list, or a dict of labels to values.
    labels : list or dict, optional
        Texts at the breaks: a list, or a dict of break values to texts.
    lablim : int, optional
        Maximum number of characters of a label; longer labels are cut.
    limits : list, optional
        Data range of the scale: ``[low, high]`` (None keeps a side).
    na_value : float, optional
        Width used for missing data.
    guide : str or FeatureSpec, default='none'
        Guide: ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Format pattern of the labels, e.g. ``".2f"`` or ``"{d} px"``.

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": ["a", "b", "c"], "low": [0, 0, 0], "y": [3, 1, 2], "width": [0.5, 1, 3]}
    >>> plot = ggplot(data, aes("x", ymin="low", ymax="y", linewidth="width"))
    >>> plot = plot + geom_pointrange(aes(y="y"))
    >>> plot = plot + scale_linewidth_identity()
    """
    return _identity("linewidth", **locals())


def scale_stroke_identity(
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Number | None = None,
    guide: Guide = "none",
    format: str | None = None,
) -> FeatureSpec:
    """Use the data as point outline widths; no legend by default.

    Parameters
    ----------
    name : str, optional
        Title of the scale, shown on the legend.
    breaks : list or dict, optional
        Values of the legend keys: a list, or a dict of labels to values.
    labels : list or dict, optional
        Texts at the breaks: a list, or a dict of break values to texts.
    lablim : int, optional
        Maximum number of characters of a label; longer labels are cut.
    limits : list, optional
        Data range of the scale: ``[low, high]`` (None keeps a side).
    na_value : float, optional
        Outline width used for missing data.
    guide : str or FeatureSpec, default='none'
        Guide: ``"legend"``, ``"none"``, or a ``guide_legend()`` spec.
    format : str, optional
        Format pattern of the labels, e.g. ``".2f"``.

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3], "y": [1, 2, 3], "outline": [0.5, 1.5, 3]}
    >>> plot = ggplot(data, aes("x", "y", stroke="outline"))
    >>> plot = plot + geom_point(shape=21, size=5, fill="white") + scale_stroke_identity()
    """
    return _identity("stroke", **locals())


def scale_alpha_identity(
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | Number | None = None,
    guide: Guide = "none",
    format: str | None = None,
) -> FeatureSpec:
    """Use the data as alphas; no legend by default.

    Parameters
    ----------
    name : str, optional
        Title of the scale, shown on the axis or the legend.
    breaks : list or dict, optional
        Positions of the ticks or legend keys: a list, or a dict of labels to values.
    labels : list or dict, optional
        Texts at the breaks: a list, or a dict of break values to texts.
    lablim : int, optional
        Maximum number of characters of a label; longer labels are cut.
    limits : list, optional
        Data range of the scale: ``[low, high]`` for continuous scales (None keeps a side), or the
        levels of a discrete scale.
    na_value : str or float, optional
        Value used for missing data.
    guide : str or FeatureSpec, default='none'
        Guide: ``"legend"``, ``"colorbar"``, ``"none"``, or a ``guide_*()`` spec.
    format : str, optional
        Format pattern of the labels, e.g. ``".2f"`` or ``"{d} %"``.

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3], "y": [1, 2, 3], "opacity": [0.2, 0.5, 1.0]}
    >>> plot = ggplot(data, aes("x", "y", alpha="opacity")) + geom_point(size=10)
    >>> plot = plot + scale_alpha_identity()
    """
    return _identity("alpha", **locals())


def scale_shape_identity(
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | Number | None = None,
    guide: Guide = "none",
    format: str | None = None,
) -> FeatureSpec:
    """Use the data as shape codes (0..25); no legend by default.

    Parameters
    ----------
    name : str, optional
        Title of the scale, shown on the axis or the legend.
    breaks : list or dict, optional
        Positions of the ticks or legend keys: a list, or a dict of labels to values.
    labels : list or dict, optional
        Texts at the breaks: a list, or a dict of break values to texts.
    lablim : int, optional
        Maximum number of characters of a label; longer labels are cut.
    limits : list, optional
        Data range of the scale: ``[low, high]`` for continuous scales (None keeps a side), or the
        levels of a discrete scale.
    na_value : str or float, optional
        Value used for missing data.
    guide : str or FeatureSpec, default='none'
        Guide: ``"legend"``, ``"colorbar"``, ``"none"``, or a ``guide_*()`` spec.
    format : str, optional
        Format pattern of the labels, e.g. ``".2f"`` or ``"{d} %"``.

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3, 4], "y": [1, 1, 1, 1], "shape": [0, 1, 2, 5]}
    >>> plot = ggplot(data, aes("x", "y", shape="shape")) + geom_point(size=8)
    >>> plot = plot + scale_shape_identity()
    """
    return _identity("shape", **locals())


def scale_linetype_identity(
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: str | int | None = None,
    guide: Guide = "none",
    format: str | None = None,
) -> FeatureSpec:
    """Use the data as line types (codes 0..6 or names such as ``"dashed"``); no legend by default.

    Parameters
    ----------
    name : str, optional
        Title of the scale, shown on the axis or the legend.
    breaks : list or dict, optional
        Positions of the ticks or legend keys: a list, or a dict of labels to values.
    labels : list or dict, optional
        Texts at the breaks: a list, or a dict of break values to texts.
    lablim : int, optional
        Maximum number of characters of a label; longer labels are cut.
    limits : list, optional
        Data range of the scale: ``[low, high]`` for continuous scales (None keeps a side), or the
        levels of a discrete scale.
    na_value : str or int, optional
        Line type used for missing data.
    guide : str or FeatureSpec, default='none'
        Guide: ``"legend"``, ``"colorbar"``, ``"none"``, or a ``guide_*()`` spec.
    format : str, optional
        Format pattern of the labels, e.g. ``".2f"`` or ``"{d} %"``.

    Returns
    -------
    FeatureSpec
        The scale.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"y": [1, 2, 3], "style": ["solid", "dashed", "dotted"]}
    >>> plot = ggplot(data) + geom_hline(aes(yintercept="y", linetype="style"), size=1)
    >>> plot = plot + scale_linetype_identity()
    """
    return _identity("linetype", **locals())
