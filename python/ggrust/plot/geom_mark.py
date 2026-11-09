"""Marks around groups of points, with callouts (R package ggforce).

Not in lets-plot; ggplot2 needs the ggforce package. ``geom_mark_hull``,
``geom_mark_ellipse`` and ``geom_mark_rect`` follow ggforce 0.5 (MIT): each group
of points (each ``fill``, ``color`` or ``group`` value) gets an outline, grown by
``expand`` with corners rounded by ``radius``. Mapping ``label`` and/or
``description`` adds a callout box per group, placed outside every mark and
connected to its mark. Lengths are millimetres (96 px per inch), font sizes points.
"""

from __future__ import annotations

import math
from numbers import Real
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from ggrust.errors import SpecError, UnsupportedValueError
from ggrust.plot.geom import _geom
from ggrust.plot.geom_beeswarm import _check_choice, _check_positive

if TYPE_CHECKING:
    from collections.abc import Sequence

    from ggrust._typing import ManualKey, Number, PlotData, Position, Sampling, Tooltips
    from ggrust.plot.core import FeatureSpec, LayerSpec

__all__ = ["geom_mark_ellipse", "geom_mark_hull", "geom_mark_rect"]

ConnectorType: TypeAlias = Literal["elbow", "straight", "none"]
"""How a callout connects to its mark."""
ConnectorBorder: TypeAlias = Literal["one", "all", "none"]
"""Which label border the connector draws: the edge it starts from, the box, none."""

_CONNECTOR_TYPES: tuple[str, ...] = ("elbow", "straight", "none")
_CONNECTOR_BORDERS: tuple[str, ...] = ("one", "all", "none")
_LENGTHS: tuple[str, ...] = (
    "expand",
    "radius",
    "label_width",
    "label_minwidth",
    "label_buffer",
    "con_cap",
)


def _check_length(function: str, name: str, value: object) -> None:
    """Raise unless ``value`` is None or a non-negative number of millimetres."""
    if value is None:
        return
    if isinstance(value, bool) or not isinstance(value, Real):
        reason = f"expected a length in millimetres (a number), got {type(value).__name__}"
        raise UnsupportedValueError(
            f"{function}({name}={value!r}): {reason}", path=name, reason=reason
        )
    if not (math.isfinite(float(value)) and float(value) >= 0):
        reason = f"{name} must be a finite, non-negative length in millimetres, got {value}"
        raise SpecError(f"{function}({name}={value!r}): {reason}", path=name, reason=reason)


def _mark(
    function: str,
    geom: str,
    shape_options: dict[str, object],
    callout_options: dict[str, object],
    common: dict[str, Any],
    other_args: dict[str, Any],
) -> LayerSpec:
    """Validate the options shared by the marks and build the layer."""
    # lets_plot spells "color"; ggforce "colour". Both are accepted.
    for alias, name in (("label_color", "label_colour"), ("con_color", "con_colour")):
        if alias in other_args:
            value = other_args.pop(alias)
            if callout_options.get(name) is None:
                callout_options[name] = value
    options = {**shape_options, **callout_options}
    for name in _LENGTHS:
        _check_length(function, name, options.get(name))
    _check_choice(function, "con_type", options.get("con_type"), _CONNECTOR_TYPES)
    _check_choice(function, "con_border", options.get("con_border"), _CONNECTOR_BORDERS)
    return _geom(geom, **common, **options, **other_args)


def geom_mark_hull(
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
    expand: Number | None = None,
    radius: Number | None = None,
    concavity: Number | None = None,
    label_margin: Number | Sequence[Number] | None = None,
    label_width: Number | None = None,
    label_minwidth: Number | None = None,
    label_hjust: Number | Sequence[Number] | None = None,
    label_fontsize: Number | Sequence[Number] | None = None,
    label_family: str | Sequence[str] | None = None,
    label_lineheight: Number | Sequence[Number] | None = None,
    label_fontface: str | Sequence[str] | None = None,
    label_fill: str | None = None,
    label_colour: str | Sequence[str] | None = None,
    label_buffer: Number | None = None,
    con_colour: str | None = None,
    con_size: Number | None = None,
    con_type: ConnectorType | None = None,
    con_linetype: str | int | None = None,
    con_border: ConnectorBorder | None = None,
    con_cap: Number | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Outline each group of points with a concave hull, optionally with a callout.

    As ``geom_mark_hull`` in the R package ggforce: for clusters on a UMAP or any
    scatter plot. Each group (each ``fill``, ``color`` or ``group`` value) gets the
    concave hull of its points (in screen space), grown by ``expand``, with corners
    rounded by ``radius``. Map ``label`` (bold) and/or ``description`` (wrapped) to
    add a callout outside the marks, joined to its mark by a connector.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Needs ``x`` and ``y``; also reads
        ``label``, ``description`` and ``filter`` (a boolean column: True rows are
        outlined, the others only keep callouts away).
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, ``"identity"`` by default.
    position : str or FeatureSpec, optional
        Position adjustment.
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
        Accepted for lets_plot compatibility; marks have no tooltips yet.
    expand : float, default=5
        Millimetres the mark extends beyond its points.
    radius : float, default=2.5
        Corner radius in millimetres (at most ``expand``).
    concavity : float, default=2
        How concave the hull may be: 1 is very concave, larger values approach the
        convex hull.
    label_margin : float or list of 4 floats, default=2
        Margin inside the callout box in millimetres (top, right, bottom, left).
    label_width : float, optional
        Fixed callout width in millimetres; the label wraps to it too.
    label_minwidth : float, default=50
        Width in millimetres the description wraps to (when wider).
    label_hjust : float or list of 2 floats, default=0
        Horizontal justification of the label and the description.
    label_fontsize : float or list of 2 floats, default=12
        Font size in points of the label and the description.
    label_family : str or list of 2 str, default="sans"
        Font family of the label and the description.
    label_lineheight : float or list of 2 floats, default=1
        Line height of the label and the description.
    label_fontface : str or list of 2 str, default=["bold", "plain"]
        Font face of the label and the description.
    label_fill : str, default="white"
        Callout background color.
    label_colour : str or list of 2 str, default="black"
        Text color of the label and the description (also ``label_color``).
    label_buffer : float, default=10
        Distance in millimetres the callouts keep from the marks.
    con_colour : str, default="black"
        Connector color (also ``con_color``).
    con_size : float, default=0.65
        Connector width, in ``size`` units (ggforce's 0.5 mm).
    con_type : {"elbow", "straight", "none"}, default="elbow"
        Connector shape.
    con_linetype : str or int, default="solid"
        Connector line type.
    con_border : {"one", "all", "none"}, default="one"
        The connector also draws the callout's near edge (``"one"``), its whole
        outline (``"all"``) or nothing.
    con_cap : float, default=3
        Gap in millimetres between the connector's end and the mark.
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics (``color``, ``fill``, ``alpha``, ``size``,
        ``linetype``, ``label``, ``description``).

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If a length is negative, ``concavity`` is not positive, or ``con_type`` /
        ``con_border`` is not one of the accepted names.
    UnsupportedValueError
        If a length or ``concavity`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> import numpy as np
    >>> generator = np.random.default_rng(3)
    >>> data = {"x": np.concatenate([generator.normal(0, 1, 150), generator.normal(5, 1, 150)]),
    ...         "y": np.concatenate([generator.normal(0, 1, 150), generator.normal(3, 1, 150)]),
    ...         "cluster": ["T cells"] * 150 + ["B cells"] * 150}
    >>> plot = (ggplot(data, aes("x", "y"))
    ...         + geom_mark_hull(aes(fill="cluster", label="cluster"))
    ...         + geom_point(aes(color="cluster"), size=1.5)
    ...         + expand_limits(x=[-6, 11], y=[-5, 9]))
    """
    _check_positive("geom_mark_hull", "concavity", concavity)
    return _mark(
        "geom_mark_hull",
        "mark_hull",
        {"expand": expand, "radius": radius, "concavity": concavity},
        _callout_options(locals()),
        _common(locals()),
        other_args,
    )


def geom_mark_ellipse(
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
    expand: Number | None = None,
    radius: Number | None = None,
    n: int | None = None,
    tol: Number | None = None,
    label_margin: Number | Sequence[Number] | None = None,
    label_width: Number | None = None,
    label_minwidth: Number | None = None,
    label_hjust: Number | Sequence[Number] | None = None,
    label_fontsize: Number | Sequence[Number] | None = None,
    label_family: str | Sequence[str] | None = None,
    label_lineheight: Number | Sequence[Number] | None = None,
    label_fontface: str | Sequence[str] | None = None,
    label_fill: str | None = None,
    label_colour: str | Sequence[str] | None = None,
    label_buffer: Number | None = None,
    con_colour: str | None = None,
    con_size: Number | None = None,
    con_type: ConnectorType | None = None,
    con_linetype: str | int | None = None,
    con_border: ConnectorBorder | None = None,
    con_cap: Number | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Outline each group of points with its smallest enclosing ellipse.

    As ``geom_mark_ellipse`` in the R package ggforce: Khachiyan's minimum
    enclosing ellipse of each group's points (in screen space), grown by
    ``expand``. Callouts as in ``geom_mark_hull``.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Needs ``x`` and ``y``; also reads
        ``label``, ``description`` and ``filter``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, ``"identity"`` by default.
    position : str or FeatureSpec, optional
        Position adjustment.
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
        Accepted for lets_plot compatibility; marks have no tooltips yet.
    expand : float, default=5
        Millimetres the mark extends beyond the ellipse.
    radius : float, optional
        Corner radius in millimetres; ``expand`` by default (a smooth outline).
    n : int, default=100
        Points on the ellipse outline.
    tol : float, default=0.01
        Convergence tolerance of Khachiyan's algorithm.
    label_margin, label_width, label_minwidth, label_hjust, label_fontsize : optional
        Callout options; see ``geom_mark_hull``.
    label_family, label_lineheight, label_fontface, label_fill, label_colour : optional
        Callout options; see ``geom_mark_hull``.
    label_buffer, con_colour, con_size, con_type, con_linetype : optional
        Callout options; see ``geom_mark_hull``.
    con_border, con_cap : optional
        Callout options; see ``geom_mark_hull``.
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics.

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If a length is negative, ``tol`` is not positive or a connector option is
        not one of the accepted names.
    UnsupportedValueError
        If a length or ``tol`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3, 2, 6, 7, 8, 7], "y": [1, 2, 1, 0, 5, 6, 5, 4],
    ...         "group": ["a"] * 4 + ["b"] * 4}
    >>> plot = (ggplot(data, aes("x", "y")) + geom_mark_ellipse(aes(fill="group"))
    ...         + geom_point())
    """
    _check_positive("geom_mark_ellipse", "tol", tol)
    return _mark(
        "geom_mark_ellipse",
        "mark_ellipse",
        {"expand": expand, "radius": radius, "n": n, "tol": tol},
        _callout_options(locals()),
        _common(locals()),
        other_args,
    )


def geom_mark_rect(
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
    expand: Number | None = None,
    radius: Number | None = None,
    label_margin: Number | Sequence[Number] | None = None,
    label_width: Number | None = None,
    label_minwidth: Number | None = None,
    label_hjust: Number | Sequence[Number] | None = None,
    label_fontsize: Number | Sequence[Number] | None = None,
    label_family: str | Sequence[str] | None = None,
    label_lineheight: Number | Sequence[Number] | None = None,
    label_fontface: str | Sequence[str] | None = None,
    label_fill: str | None = None,
    label_colour: str | Sequence[str] | None = None,
    label_buffer: Number | None = None,
    con_colour: str | None = None,
    con_size: Number | None = None,
    con_type: ConnectorType | None = None,
    con_linetype: str | int | None = None,
    con_border: ConnectorBorder | None = None,
    con_cap: Number | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Outline each group of points with its bounding rectangle.

    As ``geom_mark_rect`` in the R package ggforce: the range of each group's
    points, grown by ``expand`` with corners rounded by ``radius``. Callouts as in
    ``geom_mark_hull``.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Needs ``x`` and ``y``; also reads
        ``label``, ``description`` and ``filter``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, ``"identity"`` by default.
    position : str or FeatureSpec, optional
        Position adjustment.
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
        Accepted for lets_plot compatibility; marks have no tooltips yet.
    expand : float, default=5
        Millimetres the mark extends beyond the points' range.
    radius : float, default=2.5
        Corner radius in millimetres (at most ``expand``).
    label_margin, label_width, label_minwidth, label_hjust, label_fontsize : optional
        Callout options; see ``geom_mark_hull``.
    label_family, label_lineheight, label_fontface, label_fill, label_colour : optional
        Callout options; see ``geom_mark_hull``.
    label_buffer, con_colour, con_size, con_type, con_linetype : optional
        Callout options; see ``geom_mark_hull``.
    con_border, con_cap : optional
        Callout options; see ``geom_mark_hull``.
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics.

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If a length is negative or a connector option is not one of the accepted
        names.
    UnsupportedValueError
        If a length is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3, 6, 7, 8], "y": [1, 2, 1, 5, 6, 5],
    ...         "group": ["a"] * 3 + ["b"] * 3}
    >>> plot = (ggplot(data, aes("x", "y"))
    ...         + geom_mark_rect(aes(color="group", label="group")) + geom_point()
    ...         + expand_limits(x=[-4, 13], y=[-4, 10]))
    """
    return _mark(
        "geom_mark_rect",
        "mark_rect",
        {"expand": expand, "radius": radius},
        _callout_options(locals()),
        _common(locals()),
        other_args,
    )


_CALLOUT_NAMES: tuple[str, ...] = (
    "label_margin",
    "label_width",
    "label_minwidth",
    "label_hjust",
    "label_fontsize",
    "label_family",
    "label_lineheight",
    "label_fontface",
    "label_fill",
    "label_colour",
    "label_buffer",
    "con_colour",
    "con_size",
    "con_type",
    "con_linetype",
    "con_border",
    "con_cap",
)
_COMMON_NAMES: tuple[str, ...] = (
    "mapping",
    "data",
    "stat",
    "position",
    "show_legend",
    "inherit_aes",
    "manual_key",
    "sampling",
    "tooltips",
    "na_rm",
)


def _callout_options(arguments: dict[str, Any]) -> dict[str, object]:
    """Pick the callout options out of a mark function's arguments."""
    return {name: arguments[name] for name in _CALLOUT_NAMES}


def _common(arguments: dict[str, Any]) -> dict[str, Any]:
    """Pick the layer arguments every geom takes out of a mark function's arguments."""
    return {name: arguments[name] for name in _COMMON_NAMES}
