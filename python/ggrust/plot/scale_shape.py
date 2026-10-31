"""Shape scales: ``scale_shape`` and ``scale_shape_manual``.

Ported from lets-plot's ``lets_plot/plot/scale.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from ggrust.plot.core import FeatureSpec
from ggrust.plot.scale import _scale
from ggrust.plot.scale_color import _manual

if TYPE_CHECKING:
    from collections.abc import Sequence
    from typing import Any

    from ggrust._typing import Breaks, Color, Guide, Labels, Limits, Number

__all__ = ["scale_shape", "scale_shape_manual"]


def scale_shape(
    solid: bool = True,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    format: str | None = None,
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
    values: Sequence[Any] | dict[Any, Any],
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    format: str | None = None,
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
