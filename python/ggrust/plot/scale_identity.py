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
    "scale_shape_identity",
    "scale_size_identity",
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
    """
    return _identity("size", **locals())


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
    """
    return _identity("linetype", **locals())
