"""``geom_lollipop``: a stick from a baseline to a marker.

Ported from lets-plot's ``lets_plot/plot/geom.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Literal

from ggrust.plot.geom import _geom

if TYPE_CHECKING:
    from ggrust._typing import ManualKey, Number, PlotData, Position, Sampling, Tooltips
    from ggrust.plot.core import FeatureSpec, LayerSpec

__all__ = ["geom_lollipop"]


def geom_lollipop(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    stat: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    manual_key: ManualKey | None = None,
    sampling: Sampling | None = None,
    tooltips: Tooltips | None = None,
    orientation: Literal["x", "y"] | None = None,
    dir: Literal["v", "h", "s"] | None = None,
    fatten: Number | None = None,
    slope: Number | None = None,
    intercept: Number | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Draw a stick from a baseline to a marker at each (x, y).

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, default="identity"
        Statistical transformation, by name.
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
    orientation : {"x", "y"}, optional
        The axis the layer runs along; found from the mapping when None.
    dir : {"v", "h", "s"}, default="v"
        Direction of the sticks: vertical, horizontal, or perpendicular to the baseline.
    fatten : float, default=2.5
        Multiplier of the marker size.
    slope : float, default=0
        Slope of the baseline.
    intercept : float, default=0
        Intercept of the baseline.
    color_by : str, optional
        Aesthetic the ``color`` scale of this layer reads.
    fill_by : str, optional
        Aesthetic the ``fill`` scale of this layer reads.
    na_rm : bool, default=False
        True drops missing values without a warning.
    **other_args
        Constant aesthetics: ``x``, ``y``, ``size``, ``stroke``, ``linewidth``, ``color``,
        ``fill``, ``alpha``, ``shape``, ``linetype``.

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If ``dir`` is not one of the accepted values (when the plot is built).

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"city": ["a", "b", "c"], "rain": [12, 30, 21]}
    >>> plot = ggplot(data, aes("city", "rain")) + geom_lollipop(fatten=3)
    """
    return _geom(
        "lollipop",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        orientation=orientation,
        dir=dir,
        fatten=fatten,
        slope=slope,
        intercept=intercept,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )
