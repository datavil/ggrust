"""``geom_spoke``: a segment of given length and angle through each point.

Ported from lets-plot's ``lets_plot/plot/geom.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Literal

from ggrust.plot.geom import _geom

if TYPE_CHECKING:
    from ggrust._typing import ManualKey, PlotData, Position, Sampling, Tooltips
    from ggrust.plot.core import FeatureSpec, LayerSpec

__all__ = ["geom_spoke"]


def geom_spoke(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    manual_key: ManualKey | None = None,
    sampling: Sampling | None = None,
    tooltips: Tooltips | None = None,
    arrow: FeatureSpec | None = None,
    pivot: Literal["tail", "middle", "mid", "tip"] | None = None,
    na_rm: bool | None = None,
    color_by: str | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Draw a segment of length ``radius`` at ``angle`` radians through each point.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    position : str or FeatureSpec, default="identity"
        Position adjustment.
    show_legend : bool, default=True
        False hides this layer from the legend.
    inherit_aes : bool, default=True
        False ignores the plot's mapping.
    manual_key : str or FeatureSpec, optional
        A manual legend key for this layer.
    sampling : FeatureSpec or int, optional
        Result of a ``sampling_*()`` call.
    tooltips : FeatureSpec or str, optional
        Result of ``layer_tooltips()``, or ``"none"``.
    arrow : FeatureSpec, optional
        Result of ``arrow()``.
    pivot : {"tail", "middle", "mid", "tip"}, default="tail"
        The part of the segment that sits on the point.
    na_rm : bool, default=False
        True drops missing values without a warning.
    color_by : str, optional
        Aesthetic the ``color`` scale of this layer reads.
    **other_args
        Constant aesthetics: ``x``, ``y``, ``angle``, ``radius``, ``size``, ``linetype``,
        ``color``, ``alpha``.

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If ``pivot`` is not one of the accepted values (when the plot is built).

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [0, 1, 2], "y": [0, 0, 0], "angle": [0.5, 1.0, 1.5], "speed": [1, 2, 1]}
    >>> plot = ggplot(data, aes("x", "y")) + geom_spoke(aes(angle="angle", radius="speed"))
    """
    return _geom(
        "spoke",
        mapping=mapping,
        data=data,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        arrow=arrow,
        pivot=pivot,
        na_rm=na_rm,
        color_by=color_by,
        **other_args,
    )
