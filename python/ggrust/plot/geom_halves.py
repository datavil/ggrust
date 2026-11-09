"""Half violins, half boxplots and half points (R package gghalves).

Not in lets-plot; ggplot2 needs the gghalves package (MIT). Each geom draws on one
side of its x position (``side="l"`` or ``"r"``), so layers combine into raincloud
plots: a half violin, a half boxplot and the jittered points side by side. The
geometry follows gghalves 0.1.4: spans are 0.75 x the x resolution wide and dodged
by group; ``nudge`` moves the violin and the box away from the centre.
"""

from __future__ import annotations

from numbers import Integral, Real
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from ggrust.errors import SpecError, UnsupportedValueError
from ggrust.plot.core import FeatureSpec
from ggrust.plot.geom import _geom
from ggrust.plot.geom_beeswarm import _check_choice, _check_seed
from ggrust.plot.position import position_dodge

if TYPE_CHECKING:
    from collections.abc import Sequence

    from ggrust._typing import (
        Color,
        ManualKey,
        Number,
        PlotData,
        Position,
        Sampling,
        Shape,
        Tooltips,
    )
    from ggrust.plot.core import LayerSpec

__all__ = ["geom_half_boxplot", "geom_half_point", "geom_half_violin"]

Side: TypeAlias = Literal["l", "r"]
"""The side of x a half geom draws on: ``"l"`` (left) or ``"r"`` (right)."""

_SIDES: tuple[str, ...] = ("l", "r")
_HALF_WIDTH = 0.75
"""gghalves' span width, in x resolutions."""


def geom_half_violin(
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
    orientation: str | None = None,
    side: Side | None = None,
    nudge: Number | None = None,
    draw_quantiles: Sequence[float] | None = None,
    trim: bool | None = None,
    scale: str | None = None,
    bw: str | Number | None = None,
    adjust: Number | None = None,
    kernel: str | None = None,
    n: int | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Half of a violin: the density of y on one side of each x.

    As ``geom_half_violin`` in the R package gghalves. The density is
    ``stat="ydensity"`` (as ``geom_violin``); the shape is one half of the
    violin, ``width`` (default 0.75) x the x resolution wide in all, dodged by
    group.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Needs ``x`` and ``y``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name; ``"ydensity"`` by default.
    position : str or FeatureSpec, optional
        Position adjustment; ``"dodge"`` (by the span width) by default.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        This layer's key in a manual legend: a label, or ``layer_key()``.
    sampling : str, int or FeatureSpec, optional
        Rows to draw: a ``sampling_*()`` spec, an int ``n`` (``sampling_random(n)``)
        or ``"none"``. Default: every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"``.
    orientation : str, optional
        ``"y"`` draws along the y axis (``"l"`` is then below); a discrete y with a
        continuous x picks it by default.
    side : {"l", "r"}, default="l"
        The side of x the half violin is drawn on.
    nudge : float, default=0
        Distance (data units) between x and the flat side of the half violin.
    draw_quantiles : list of float, optional
        Quantiles (each in [0, 1]) to draw lines across the half violin at.
    trim : bool, default=True
        False extends the density 3 bandwidths beyond the data.
    scale : {"area", "count", "width"}, default="area"
        How the violins of the layer compare in width.
    bw : str or float, optional
        Bandwidth: ``"nrd0"`` (default), ``"nrd"`` or a number.
    adjust : float, optional
        Multiplies the bandwidth.
    kernel : str, optional
        Kernel, ``"gaussian"`` by default.
    n : int, optional
        Number of points the density is evaluated at (512).
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics (``fill``, ``color``, ``alpha``, ``size``, ...) and
        ``width``.

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If ``side`` is not ``"l"`` or ``"r"``, or a quantile is outside [0, 1].
    UnsupportedValueError
        If ``nudge`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"diet": ["veg"] * 6 + ["meat"] * 6,
    ...         "weight": [61, 58, 66, 70, 63, 59, 72, 80, 77, 69, 85, 74]}
    >>> plot = (ggplot(data, aes("diet", "weight", fill="diet"))
    ...         + geom_half_violin() + geom_half_point())
    """
    function = "geom_half_violin"
    _check_side(function, side)
    _check_number(function, "nudge", nudge)
    quantiles: list[float] | None = None
    if draw_quantiles is not None:
        quantiles = [float(value) for value in draw_quantiles]
        if any(not 0 <= value <= 1 for value in quantiles):
            reason = f"draw_quantiles must lie in [0, 1], got {quantiles}"
            raise SpecError(
                f"{function}(draw_quantiles={draw_quantiles!r}): {reason}",
                path="draw_quantiles",
                reason=reason,
            )
    return _geom(
        "half_violin",
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
        side=side,
        nudge=nudge,
        quantiles=quantiles,
        quantile_lines=True if quantiles else None,
        trim=trim,
        scale=scale,
        bw=bw,
        adjust=adjust,
        kernel=kernel,
        n=n,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_half_boxplot(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    stat: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    manual_key: ManualKey | None = None,
    tooltips: Tooltips | None = None,
    orientation: str | None = None,
    side: Side | None = None,
    nudge: Number | None = None,
    errorbar_draw: bool | None = None,
    errorbar_length: Number | None = None,
    fatten: Number | None = None,
    outlier_alpha: Number | None = None,
    outlier_color: Color | None = None,
    outlier_fill: Color | None = None,
    outlier_shape: Shape | None = None,
    outlier_size: Number | None = None,
    outlier_stroke: Number | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec | FeatureSpec:
    """Half of a box-and-whisker plot, on one side of each x.

    As ``geom_half_boxplot`` in the R package gghalves: the box runs from its
    outer edge to x (less ``nudge``), the whiskers stand at x (moved ``nudge``
    towards the box), with fences reaching outwards. With the default stat, a
    second point layer (``stat="boxplot_outlier"``) draws the outliers beside
    the whiskers, as ``geom_boxplot`` does.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Needs ``x`` and ``y``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name; ``"boxplot"`` by default.
    position : str or FeatureSpec, optional
        Position adjustment; ``"dodge"`` (by the box width) by default.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        This layer's key in a manual legend: a label, or ``layer_key()``.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"``.
    orientation : str, optional
        ``"y"`` draws along the y axis; a discrete y with a continuous x picks it.
    side : {"l", "r"}, default="l"
        The side of x the half box is drawn on.
    nudge : float, default=0
        Distance (data units) between x and the box's inner edge, and the
        whiskers' shift towards the box.
    errorbar_draw : bool, default=True
        Draw fences at the whisker ends.
    errorbar_length : float, default=0.5
        Fence length as a share of the half box, from 0 to 1.
    fatten : float, default=2
        Multiplier of the median line width.
    outlier_alpha, outlier_color, outlier_fill : optional
        Style of the outlier points (also ``outlier_shape``, ``outlier_size``,
        ``outlier_stroke``); ``color``, ``fill``, ``shape``, ``size`` and
        ``stroke`` in ``other_args`` are the fallback.
    outlier_shape : int or str, optional
        Shape of the outlier points.
    outlier_size : float, optional
        Size of the outlier points.
    outlier_stroke : float, optional
        Stroke width of the outlier points.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics, ``width`` (default 0.75) and ``coef``.

    Returns
    -------
    LayerSpec or FeatureSpecArray
        The box layer, plus the outlier layer with the default stat.

    Raises
    ------
    SpecError
        If ``side`` is not ``"l"`` or ``"r"``, or ``errorbar_length`` is outside [0, 1].
    UnsupportedValueError
        If ``nudge`` or ``errorbar_length`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"diet": ["veg"] * 6 + ["meat"] * 6,
    ...         "weight": [61, 58, 66, 70, 63, 59, 72, 80, 77, 69, 85, 74]}
    >>> plot = (ggplot(data, aes("diet", "weight"))
    ...         + geom_half_boxplot(side="r") + geom_half_point(side="l"))
    """
    function = "geom_half_boxplot"
    _check_side(function, side)
    _check_number(function, "nudge", nudge)
    _check_number(function, "errorbar_length", errorbar_length)
    if errorbar_length is not None and not 0 <= float(errorbar_length) <= 1:
        reason = f"errorbar_length must be between 0 and 1, got {errorbar_length}"
        raise SpecError(
            f"{function}(errorbar_length={errorbar_length!r}): {reason}",
            path="errorbar_length",
            reason=reason,
        )
    box_layer = _geom(
        "half_boxplot",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=None,
        tooltips=tooltips,
        orientation=orientation,
        side=side,
        nudge=nudge,
        errorbar_draw=errorbar_draw,
        errorbar_length=errorbar_length,
        fatten=fatten,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )
    if stat is not None and stat != "boxplot":
        return box_layer

    def outlier_parameter(name: str, value: Any) -> Any:  # noqa: ANN401 (user constants)
        return value if value is not None else other_args.get(name)

    size = outlier_parameter("size", outlier_size)
    if size is not None:
        size *= 4  # geom_boxplot's outlier fattening
    sign = -1.0 if (side or "l") == "l" else 1.0
    width = other_args.get("width")
    # The boxes dodge by their width; the outlier points need it spelled out.
    outlier_position = position
    if position is None or position == "dodge":
        outlier_position = position_dodge(width=width if width is not None else _HALF_WIDTH)
    return box_layer + _geom(
        "point",
        mapping=mapping,
        data=data,
        stat="boxplot_outlier",
        position=outlier_position,
        show_legend=False,
        inherit_aes=inherit_aes,
        sampling=None,
        orientation=orientation,
        alpha=outlier_alpha,
        color=outlier_parameter("color", outlier_color),
        fill=outlier_parameter("fill", outlier_fill),
        shape=outlier_parameter("shape", outlier_shape),
        size=size,
        stroke=outlier_parameter("stroke", outlier_stroke),
        coef=other_args.get("coef"),
        outlier_nudge=sign * float(nudge) if nudge else None,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=True,
    )


def geom_half_point(
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
    orientation: str | None = None,
    side: Side | None = None,
    transformation: FeatureSpec | Literal["jitter", "identity"] | None = None,
    range_scale: Number | None = None,
    color_by: str | None = None,
    fill_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Points jittered within one half of their x slot.

    As ``geom_half_point`` in the R package gghalves: each point moves to the
    middle of the ``side`` half of its slot (a quarter of the slot from x), then
    is jittered there. The slot is ``width`` (default 0.75) x the x resolution,
    dodged by group.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Needs ``x`` and ``y``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name; ``"identity"`` by default.
    position : str or FeatureSpec, optional
        Position adjustment; ``"dodge"`` (by the slot width) by default.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        This layer's key in a manual legend: a label, or ``layer_key()``.
    sampling : str, int or FeatureSpec, optional
        Rows to draw: a ``sampling_*()`` spec, an int ``n`` (``sampling_random(n)``)
        or ``"none"``. Default: every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"``.
    orientation : str, optional
        ``"y"`` draws along the y axis; a discrete y with a continuous x picks it.
    side : {"l", "r"}, default="r"
        The half of the slot the points are drawn in.
    transformation : FeatureSpec or {"jitter", "identity"}, optional
        How points spread within their half: ``position_jitter(width, height,
        seed)`` (default), with ``width`` and ``height`` in data units as in
        gghalves (defaults: ``range_scale`` x a quarter of the slot, and 0.4 x
        the resolution of the group's y), or ``"identity"`` for no jitter.
    range_scale : float, default=0.75
        Default jitter width as a share of the half slot's half width.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    fill_by : str, optional
        Aesthetic the ``fill`` scale applies to (``"fill"`` or ``"color"``).
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics (``size``, ``alpha``, ``color``, ...) and ``width``.

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If ``side`` is not ``"l"`` or ``"r"``, or ``transformation`` is not a
        jitter or identity.
    UnsupportedValueError
        If ``range_scale`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"diet": ["veg"] * 6 + ["meat"] * 6,
    ...         "weight": [61, 58, 66, 70, 63, 59, 72, 80, 77, 69, 85, 74]}
    >>> plot = (ggplot(data, aes("diet", "weight"))
    ...         + geom_half_point(transformation=position_jitter(width=0.05, seed=1)))
    """
    function = "geom_half_point"
    _check_side(function, side)
    _check_number(function, "range_scale", range_scale)
    jitter = _jitter_options(function, transformation)
    return _geom(
        "half_point",
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
        side=side,
        range_scale=range_scale,
        **jitter,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def _jitter_options(function: str, transformation: object) -> dict[str, float | int | None]:
    """Return the ``jitter_width``, ``jitter_height`` and ``seed`` options of ``transformation``."""
    if transformation is None or transformation == "jitter":
        return {}
    if transformation == "identity":
        return {"jitter_width": 0.0, "jitter_height": 0.0}
    if isinstance(transformation, FeatureSpec) and transformation.kind == "pos":
        spec = transformation.as_dict()
        if spec.get("name") == "jitter":
            _check_seed(function, spec.get("seed"))
            for name in ("width", "height"):
                value = spec.get(name)
                _check_number(function, f"transformation.{name}", value)
                if value is not None and float(value) < 0:
                    reason = f"jitter {name} must be zero or positive, got {value}"
                    raise SpecError(
                        f"{function}(transformation=...): {reason}",
                        path=f"transformation.{name}",
                        reason=reason,
                    )
            return {
                "jitter_width": spec.get("width"),
                "jitter_height": spec.get("height"),
                "seed": spec.get("seed"),
            }
    reason = (
        "transformation must be position_jitter(...), 'jitter' or 'identity', "
        f"got {transformation!r}"
    )
    raise SpecError(
        f"{function}(transformation={transformation!r}): {reason}",
        path="transformation",
        reason=reason,
    )


def _check_side(function: str, side: object) -> None:
    """Raise unless ``side`` is None, ``"l"`` or ``"r"``."""
    _check_choice(function, "side", side, _SIDES)


def _check_number(function: str, name: str, value: object) -> None:
    """Raise unless ``value`` is None or a finite real number."""
    if value is None:
        return
    if isinstance(value, bool) or not isinstance(value, Real | Integral):
        reason = f"expected a number, got {type(value).__name__}"
        raise UnsupportedValueError(
            f"{function}({name}={value!r}): {reason}", path=name, reason=reason
        )
    if float(value) != float(value) or float(value) in (float("inf"), float("-inf")):
        reason = f"{name} must be a finite number, got {value}"
        raise SpecError(f"{function}({name}={value!r}): {reason}", path=name, reason=reason)
