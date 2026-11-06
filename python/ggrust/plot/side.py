"""Side panels: ggside side layers and lets-plot's ``ggmarginal()``.

Not in lets-plot (except ``ggmarginal()``); ggplot2 needs the ggside package. A side
layer draws in a band beside the main panel that shares one of its axes:
``geom_xside*()`` layers in the x side panel (on top by default), ``geom_yside*()``
layers in the y side panel (on the right). Several layers in one side panel share it,
and discrete values across the band make tracks, e.g. the annotation rows of a
heatmap. ``ggside()`` moves the side panels, ``scale_xsidey_*()`` and
``scale_ysidex_*()`` set their own axes, and ``theme(ggside_panel_scale=...,
ggside_panel_spacing=...)`` their size and gap.

More than one band per axis (not in R ggside, which has one): every side layer takes
``side_pos=`` (``"top"``/``"bottom"`` for ``geom_xside*()``, ``"left"``/``"right"`` for
``geom_yside*()``) to override ``ggside()`` for that layer, ``side_band="name"`` to put it
in a band of its own (layers with the same side and name share a band; the bands on one
side stack outward from the panel in plot order, ``ggside_panel_spacing`` apart) and
``side_size=`` for its band's thickness as a share of the panel (else
``ggside_panel_scale_*``). ``scale_xsidey_*()`` and ``scale_ysidex_*()`` take the same
``side_pos=`` / ``side_band=`` to set one band's own axis; without them they set every band
on their axis that no such scale names.

``ggmarginal()`` is lets-plot's marginal plot: copies of a layer in margins of the
panel, without axes, background or grid. Both draw on the same engine.

The spec marks a ggside layer with ``"ggside": "x" | "y"`` and a marginal layer with
lets-plot's own keys (``marginal``, ``margin_side``, ``margin_size``).
"""

from __future__ import annotations

import math
import numbers
from typing import TYPE_CHECKING, Any, Literal

from ggrust.errors import SpecError, did_you_mean
from ggrust.plot.core import FeatureSpec, FeatureSpecArray, LayerSpec
from ggrust.plot.geom import (
    geom_bar,
    geom_boxplot,
    geom_col,
    geom_density,
    geom_histogram,
    geom_line,
    geom_point,
    geom_text,
    geom_tile,
    geom_violin,
)
from ggrust.plot.scale import (
    scale_x_continuous,
    scale_x_discrete,
    scale_y_continuous,
    scale_y_discrete,
)

if TYPE_CHECKING:
    from collections.abc import Sequence

__all__ = [
    "geom_xsidebar",
    "geom_xsideboxplot",
    "geom_xsidecol",
    "geom_xsidedensity",
    "geom_xsidehistogram",
    "geom_xsideline",
    "geom_xsidepoint",
    "geom_xsidetext",
    "geom_xsidetile",
    "geom_xsideviolin",
    "geom_ysidebar",
    "geom_ysideboxplot",
    "geom_ysidecol",
    "geom_ysidedensity",
    "geom_ysidehistogram",
    "geom_ysideline",
    "geom_ysidepoint",
    "geom_ysidetext",
    "geom_ysidetile",
    "geom_ysideviolin",
    "ggmarginal",
    "ggside",
    "scale_xsidey_continuous",
    "scale_xsidey_discrete",
    "scale_ysidex_continuous",
    "scale_ysidex_discrete",
]

SideAxis = Literal["x", "y"]
"""The main-panel axis a side layer shares."""


def _layers(feature: FeatureSpec) -> list[LayerSpec]:
    """Return the layers of ``feature``: itself, or the layers of a layer list."""
    if isinstance(feature, FeatureSpecArray):
        return [element for element in feature.elements() if isinstance(element, LayerSpec)]
    if isinstance(feature, LayerSpec):
        return [feature]
    return []


def _side(
    feature: FeatureSpec,
    axis: SideAxis,
    geom: str,
    given: dict[str, Any],
    *,
    orientation: bool,
) -> Any:  # noqa: ANN401 (returns the geom's own type)
    """Mark the layers of ``feature`` as ``axis`` side layers.

    Parameters
    ----------
    feature : FeatureSpec
        The layer, or the layer list ``geom_boxplot()`` returns.
    axis : {"x", "y"}
        The axis the side panel shares with the main panel.
    geom : str
        The geom name, for the boxplot/violin default.
    given : dict
        The keyword arguments the user gave.
    orientation : bool
        Whether ``orientation`` defaults to ``"y"`` (unless given).

    Returns
    -------
    FeatureSpec
        ``feature``, marked.
    """
    _check_band_keys(f"geom_{axis}side{geom}", axis, given, size=True)
    other = "y" if axis == "x" else "x"
    for layer in _layers(feature):
        props = layer.props()
        props["ggside"] = axis
        # Every layer of a layer list (boxplot + outliers) goes in the same band.
        for key in ("side_pos", "side_band", "side_size"):
            if given.get(key) is not None:
                props[key] = given[key]
        if orientation and props.get("orientation") is None:
            props["orientation"] = "y"
        mapping = props.get("mapping")
        own = mapping.props() if isinstance(mapping, FeatureSpec) else {}
        if geom in ("boxplot", "violin") and own.get(other) is None and given.get(other) is None:
            props[other] = 0
    return feature


_BAND_SIDES: dict[SideAxis, tuple[str, str]] = {"x": ("top", "bottom"), "y": ("left", "right")}


def _check_band_keys(function: str, axis: SideAxis, given: dict[str, Any], *, size: bool) -> None:
    """Check the ``side_pos=``, ``side_band=`` (and ``side_size=``) arguments of a side call.

    Parameters
    ----------
    function : str
        The call, for messages (``geom_ysidetile``).
    axis : {"x", "y"}
        The axis its band shares with the main panel.
    given : dict
        The keyword arguments the user gave.
    size : bool
        Whether the call takes ``side_size=`` (layers do, scales do not).

    Raises
    ------
    SpecError
        If a value is not accepted.
    """
    position = given.get("side_pos")
    accepted = _BAND_SIDES[axis]
    if position is not None and position not in accepted:
        reason = (
            f"expected '{accepted[0]}' or '{accepted[1]}'{did_you_mean(str(position), accepted)}"
        )
        raise SpecError(
            f"{function}(side_pos={position!r}): {reason}", path="side_pos", reason=reason
        )
    band = given.get("side_band")
    if band is not None and (not isinstance(band, str) or not band):
        reason = (
            "expected the band's name as a non-empty string (layers with the same name share "
            "a band)"
        )
        raise SpecError(
            f"{function}(side_band={band!r}): {reason}", path="side_band", reason=reason
        )
    if not size:
        if given.get("side_size") is not None:
            reason = "a scale sets no band size; give side_size= on the band's layers"
            raise SpecError(f"{function}(side_size=...): {reason}", path="side_size", reason=reason)
        return
    thickness = given.get("side_size")
    valid = (
        isinstance(thickness, numbers.Real)
        and not isinstance(thickness, bool)
        and math.isfinite(thickness)
        and thickness > 0
    )
    if thickness is not None and not valid:
        reason = (
            "expected the band's thickness as a positive share of the main panel (ggside's "
            "default is 0.1)"
        )
        raise SpecError(
            f"{function}(side_size={thickness!r}): {reason}", path="side_size", reason=reason
        )


def geom_xsidetile(mapping: FeatureSpec | None = None, **kwargs: Any) -> LayerSpec:
    """Draw tiles in the x side panel (R package ggside).

    The x side panel sits above the panel (below with ``ggside(x_pos="bottom")``) and shares
    its x axis.

    Without its own ``y`` mapping, the layer is one track labelled by its fill (or color)
    column: ``aes(fill="cell_type")`` draws the track ``cell_type``.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Along x it maps through the main
        panel's x scale; across the band through the band's own y scale
        (``scale_xsidey_*()``).
    **kwargs
        Every other parameter of ``geom_tile()``.

    Returns
    -------
    LayerSpec
        The side layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "cell": ["c1", "c2", "c3", "c4"] * 2,
    ...     "gene": ["CD3E"] * 4 + ["MS4A1"] * 4,
    ...     "expression": [2.1, 1.8, 0.2, 0.1, 0.3, 0.1, 2.4, 2.0],
    ...     "cell_type": ["T", "T", "B", "B"] * 2,
    ...     "module": ["T genes"] * 4 + ["B genes"] * 4,
    ... }
    >>> plot = (
    ...     ggplot(data, aes("cell", "gene", fill="expression"))
    ...     + geom_tile()
    ...     + new_scale_fill()
    ...     + geom_xsidetile(aes(fill="cell_type"))
    ... )
    """
    layer = geom_tile(mapping, **kwargs)
    return _side(layer, "x", "tile", kwargs, orientation=False)


def geom_ysidetile(mapping: FeatureSpec | None = None, **kwargs: Any) -> LayerSpec:
    """Draw tiles in the y side panel (R package ggside).

    The y side panel sits right of the panel (left with ``ggside(y_pos="left")``) and shares
    its y axis.

    Without its own ``x`` mapping, the layer is one track labelled by its fill (or color)
    column: ``aes(fill="cell_type")`` draws the track ``cell_type``.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Along y it maps through the main
        panel's y scale; across the band through the band's own x scale
        (``scale_ysidex_*()``).
    **kwargs
        Every other parameter of ``geom_tile()``.

    Returns
    -------
    LayerSpec
        The side layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "cell": ["c1", "c2", "c3", "c4"] * 2,
    ...     "gene": ["CD3E"] * 4 + ["MS4A1"] * 4,
    ...     "expression": [2.1, 1.8, 0.2, 0.1, 0.3, 0.1, 2.4, 2.0],
    ...     "cell_type": ["T", "T", "B", "B"] * 2,
    ...     "module": ["T genes"] * 4 + ["B genes"] * 4,
    ... }
    >>> plot = (
    ...     ggplot(data, aes("cell", "gene", fill="expression"))
    ...     + geom_tile()
    ...     + new_scale_fill()
    ...     + geom_ysidetile(aes(fill="module"))
    ... )
    """
    layer = geom_tile(mapping, **kwargs)
    return _side(layer, "y", "tile", kwargs, orientation=False)


def geom_xsidebar(mapping: FeatureSpec | None = None, **kwargs: Any) -> LayerSpec:
    """Draw bars of counts in the x side panel (R package ggside).

    The x side panel sits above the panel (below with ``ggside(x_pos="bottom")``) and shares
    its x axis.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Along x it maps through the main
        panel's x scale; across the band through the band's own y scale
        (``scale_xsidey_*()``).
    **kwargs
        Every other parameter of ``geom_bar()``.

    Returns
    -------
    LayerSpec
        The side layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "height": [160, 165, 170, 172, 175, 181, 168, 177],
    ...     "weight": [55, 61, 66, 70, 72, 80, 63, 75],
    ...     "sex": ["f", "f", "f", "m", "m", "m", "f", "m"],
    ... }
    >>> plot = ggplot(data, aes("height", "weight")) + geom_point()
    >>> plot = plot + geom_xsidebar()
    """
    layer = geom_bar(mapping, **kwargs)
    return _side(layer, "x", "bar", kwargs, orientation=False)


def geom_ysidebar(mapping: FeatureSpec | None = None, **kwargs: Any) -> LayerSpec:
    """Draw bars of counts in the y side panel (R package ggside).

    The y side panel sits right of the panel (left with ``ggside(y_pos="left")``) and shares
    its y axis.

    ``orientation`` defaults to ``"y"`` here, as in ggside.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Along y it maps through the main
        panel's y scale; across the band through the band's own x scale
        (``scale_ysidex_*()``).
    **kwargs
        Every other parameter of ``geom_bar()``.

    Returns
    -------
    LayerSpec
        The side layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "height": [160, 165, 170, 172, 175, 181, 168, 177],
    ...     "weight": [55, 61, 66, 70, 72, 80, 63, 75],
    ...     "sex": ["f", "f", "f", "m", "m", "m", "f", "m"],
    ... }
    >>> plot = ggplot(data, aes("height", "weight")) + geom_point()
    >>> plot = plot + geom_ysidebar()
    """
    layer = geom_bar(mapping, **kwargs)
    return _side(layer, "y", "bar", kwargs, orientation=True)


def geom_xsidecol(mapping: FeatureSpec | None = None, **kwargs: Any) -> LayerSpec:
    """Draw bars of given heights in the x side panel (R package ggside).

    The x side panel sits above the panel (below with ``ggside(x_pos="bottom")``) and shares
    its x axis.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Along x it maps through the main
        panel's x scale; across the band through the band's own y scale
        (``scale_xsidey_*()``).
    **kwargs
        Every other parameter of ``geom_col()``.

    Returns
    -------
    LayerSpec
        The side layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "height": [160, 165, 170, 172, 175, 181, 168, 177],
    ...     "weight": [55, 61, 66, 70, 72, 80, 63, 75],
    ...     "sex": ["f", "f", "f", "m", "m", "m", "f", "m"],
    ... }
    >>> plot = ggplot(data, aes("height", "weight")) + geom_point()
    >>> plot = plot + geom_xsidecol(aes(y="weight"))
    """
    layer = geom_col(mapping, **kwargs)
    return _side(layer, "x", "col", kwargs, orientation=False)


def geom_ysidecol(mapping: FeatureSpec | None = None, **kwargs: Any) -> LayerSpec:
    """Draw bars of given heights in the y side panel (R package ggside).

    The y side panel sits right of the panel (left with ``ggside(y_pos="left")``) and shares
    its y axis.

    ``orientation`` defaults to ``"y"`` here, as in ggside.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Along y it maps through the main
        panel's y scale; across the band through the band's own x scale
        (``scale_ysidex_*()``).
    **kwargs
        Every other parameter of ``geom_col()``.

    Returns
    -------
    LayerSpec
        The side layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "height": [160, 165, 170, 172, 175, 181, 168, 177],
    ...     "weight": [55, 61, 66, 70, 72, 80, 63, 75],
    ...     "sex": ["f", "f", "f", "m", "m", "m", "f", "m"],
    ... }
    >>> plot = ggplot(data, aes("height", "weight")) + geom_point()
    >>> plot = plot + geom_ysidecol(aes(x="height"))
    """
    layer = geom_col(mapping, **kwargs)
    return _side(layer, "y", "col", kwargs, orientation=True)


def geom_xsidehistogram(mapping: FeatureSpec | None = None, **kwargs: Any) -> LayerSpec:
    """Draw a histogram in the x side panel (R package ggside).

    The x side panel sits above the panel (below with ``ggside(x_pos="bottom")``) and shares
    its x axis.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Along x it maps through the main
        panel's x scale; across the band through the band's own y scale
        (``scale_xsidey_*()``).
    **kwargs
        Every other parameter of ``geom_histogram()``.

    Returns
    -------
    LayerSpec
        The side layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "height": [160, 165, 170, 172, 175, 181, 168, 177],
    ...     "weight": [55, 61, 66, 70, 72, 80, 63, 75],
    ...     "sex": ["f", "f", "f", "m", "m", "m", "f", "m"],
    ... }
    >>> plot = ggplot(data, aes("height", "weight")) + geom_point()
    >>> plot = plot + geom_xsidehistogram(bins=4)
    """
    layer = geom_histogram(mapping, **kwargs)
    return _side(layer, "x", "histogram", kwargs, orientation=False)


def geom_ysidehistogram(mapping: FeatureSpec | None = None, **kwargs: Any) -> LayerSpec:
    """Draw a histogram in the y side panel (R package ggside).

    The y side panel sits right of the panel (left with ``ggside(y_pos="left")``) and shares
    its y axis.

    ``orientation`` defaults to ``"y"`` here, as in ggside.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Along y it maps through the main
        panel's y scale; across the band through the band's own x scale
        (``scale_ysidex_*()``).
    **kwargs
        Every other parameter of ``geom_histogram()``.

    Returns
    -------
    LayerSpec
        The side layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "height": [160, 165, 170, 172, 175, 181, 168, 177],
    ...     "weight": [55, 61, 66, 70, 72, 80, 63, 75],
    ...     "sex": ["f", "f", "f", "m", "m", "m", "f", "m"],
    ... }
    >>> plot = ggplot(data, aes("height", "weight")) + geom_point()
    >>> plot = plot + geom_ysidehistogram(bins=4)
    """
    layer = geom_histogram(mapping, **kwargs)
    return _side(layer, "y", "histogram", kwargs, orientation=True)


def geom_xsidedensity(mapping: FeatureSpec | None = None, **kwargs: Any) -> LayerSpec:
    """Draw a density curve in the x side panel (R package ggside).

    The x side panel sits above the panel (below with ``ggside(x_pos="bottom")``) and shares
    its x axis.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Along x it maps through the main
        panel's x scale; across the band through the band's own y scale
        (``scale_xsidey_*()``).
    **kwargs
        Every other parameter of ``geom_density()``.

    Returns
    -------
    LayerSpec
        The side layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "height": [160, 165, 170, 172, 175, 181, 168, 177],
    ...     "weight": [55, 61, 66, 70, 72, 80, 63, 75],
    ...     "sex": ["f", "f", "f", "m", "m", "m", "f", "m"],
    ... }
    >>> plot = ggplot(data, aes("height", "weight")) + geom_point()
    >>> plot = plot + geom_xsidedensity(fill="grey")
    """
    layer = geom_density(mapping, **kwargs)
    return _side(layer, "x", "density", kwargs, orientation=False)


def geom_ysidedensity(mapping: FeatureSpec | None = None, **kwargs: Any) -> LayerSpec:
    """Draw a density curve in the y side panel (R package ggside).

    The y side panel sits right of the panel (left with ``ggside(y_pos="left")``) and shares
    its y axis.

    ``orientation`` defaults to ``"y"`` here, as in ggside.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Along y it maps through the main
        panel's y scale; across the band through the band's own x scale
        (``scale_ysidex_*()``).
    **kwargs
        Every other parameter of ``geom_density()``.

    Returns
    -------
    LayerSpec
        The side layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "height": [160, 165, 170, 172, 175, 181, 168, 177],
    ...     "weight": [55, 61, 66, 70, 72, 80, 63, 75],
    ...     "sex": ["f", "f", "f", "m", "m", "m", "f", "m"],
    ... }
    >>> plot = ggplot(data, aes("height", "weight")) + geom_point()
    >>> plot = plot + geom_ysidedensity(fill="grey")
    """
    layer = geom_density(mapping, **kwargs)
    return _side(layer, "y", "density", kwargs, orientation=True)


def geom_xsidepoint(mapping: FeatureSpec | None = None, **kwargs: Any) -> LayerSpec:
    """Draw points in the x side panel (R package ggside).

    The x side panel sits above the panel (below with ``ggside(x_pos="bottom")``) and shares
    its x axis.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Along x it maps through the main
        panel's x scale; across the band through the band's own y scale
        (``scale_xsidey_*()``).
    **kwargs
        Every other parameter of ``geom_point()``.

    Returns
    -------
    LayerSpec
        The side layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "height": [160, 165, 170, 172, 175, 181, 168, 177],
    ...     "weight": [55, 61, 66, 70, 72, 80, 63, 75],
    ...     "sex": ["f", "f", "f", "m", "m", "m", "f", "m"],
    ... }
    >>> plot = ggplot(data, aes("height", "weight")) + geom_point()
    >>> plot = plot + geom_xsidepoint(aes(y="weight"))
    """
    layer = geom_point(mapping, **kwargs)
    return _side(layer, "x", "point", kwargs, orientation=False)


def geom_ysidepoint(mapping: FeatureSpec | None = None, **kwargs: Any) -> LayerSpec:
    """Draw points in the y side panel (R package ggside).

    The y side panel sits right of the panel (left with ``ggside(y_pos="left")``) and shares
    its y axis.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Along y it maps through the main
        panel's y scale; across the band through the band's own x scale
        (``scale_ysidex_*()``).
    **kwargs
        Every other parameter of ``geom_point()``.

    Returns
    -------
    LayerSpec
        The side layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "height": [160, 165, 170, 172, 175, 181, 168, 177],
    ...     "weight": [55, 61, 66, 70, 72, 80, 63, 75],
    ...     "sex": ["f", "f", "f", "m", "m", "m", "f", "m"],
    ... }
    >>> plot = ggplot(data, aes("height", "weight")) + geom_point()
    >>> plot = plot + geom_ysidepoint(aes(x="height"))
    """
    layer = geom_point(mapping, **kwargs)
    return _side(layer, "y", "point", kwargs, orientation=False)


def geom_xsideboxplot(mapping: FeatureSpec | None = None, **kwargs: Any) -> FeatureSpec:
    """Draw boxplots in the x side panel (R package ggside).

    The x side panel sits above the panel (below with ``ggside(x_pos="bottom")``) and shares
    its x axis.

    ``orientation`` defaults to ``"y"`` here, as in ggside.

    Without its own ``y`` mapping or constant, ``y=0`` puts one boxplot in the band, as
    lets-plot's ``ggmarginal()`` does.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Along x it maps through the main
        panel's x scale; across the band through the band's own y scale
        (``scale_xsidey_*()``).
    **kwargs
        Every other parameter of ``geom_boxplot()``.

    Returns
    -------
    FeatureSpec
        The side layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "height": [160, 165, 170, 172, 175, 181, 168, 177],
    ...     "weight": [55, 61, 66, 70, 72, 80, 63, 75],
    ...     "sex": ["f", "f", "f", "m", "m", "m", "f", "m"],
    ... }
    >>> plot = ggplot(data, aes("height", "weight")) + geom_point()
    >>> plot = plot + geom_xsideboxplot()
    """
    layer = geom_boxplot(mapping, **kwargs)
    return _side(layer, "x", "boxplot", kwargs, orientation=True)


def geom_ysideboxplot(mapping: FeatureSpec | None = None, **kwargs: Any) -> FeatureSpec:
    """Draw boxplots in the y side panel (R package ggside).

    The y side panel sits right of the panel (left with ``ggside(y_pos="left")``) and shares
    its y axis.

    Without its own ``x`` mapping or constant, ``x=0`` puts one boxplot in the band, as
    lets-plot's ``ggmarginal()`` does.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Along y it maps through the main
        panel's y scale; across the band through the band's own x scale
        (``scale_ysidex_*()``).
    **kwargs
        Every other parameter of ``geom_boxplot()``.

    Returns
    -------
    FeatureSpec
        The side layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "height": [160, 165, 170, 172, 175, 181, 168, 177],
    ...     "weight": [55, 61, 66, 70, 72, 80, 63, 75],
    ...     "sex": ["f", "f", "f", "m", "m", "m", "f", "m"],
    ... }
    >>> plot = ggplot(data, aes("height", "weight")) + geom_point()
    >>> plot = plot + geom_ysideboxplot()
    """
    layer = geom_boxplot(mapping, **kwargs)
    return _side(layer, "y", "boxplot", kwargs, orientation=False)


def geom_xsideviolin(mapping: FeatureSpec | None = None, **kwargs: Any) -> LayerSpec:
    """Draw violins in the x side panel (R package ggside).

    The x side panel sits above the panel (below with ``ggside(x_pos="bottom")``) and shares
    its x axis.

    ``orientation`` defaults to ``"y"`` here, as in ggside.

    Without its own ``y`` mapping or constant, ``y=0`` puts one violin in the band, as lets-
    plot's ``ggmarginal()`` does.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Along x it maps through the main
        panel's x scale; across the band through the band's own y scale
        (``scale_xsidey_*()``).
    **kwargs
        Every other parameter of ``geom_violin()``.

    Returns
    -------
    LayerSpec
        The side layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "height": [160, 165, 170, 172, 175, 181, 168, 177],
    ...     "weight": [55, 61, 66, 70, 72, 80, 63, 75],
    ...     "sex": ["f", "f", "f", "m", "m", "m", "f", "m"],
    ... }
    >>> plot = ggplot(data, aes("height", "weight")) + geom_point()
    >>> plot = plot + geom_xsideviolin()
    """
    layer = geom_violin(mapping, **kwargs)
    return _side(layer, "x", "violin", kwargs, orientation=True)


def geom_ysideviolin(mapping: FeatureSpec | None = None, **kwargs: Any) -> LayerSpec:
    """Draw violins in the y side panel (R package ggside).

    The y side panel sits right of the panel (left with ``ggside(y_pos="left")``) and shares
    its y axis.

    Without its own ``x`` mapping or constant, ``x=0`` puts one violin in the band, as lets-
    plot's ``ggmarginal()`` does.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Along y it maps through the main
        panel's y scale; across the band through the band's own x scale
        (``scale_ysidex_*()``).
    **kwargs
        Every other parameter of ``geom_violin()``.

    Returns
    -------
    LayerSpec
        The side layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "height": [160, 165, 170, 172, 175, 181, 168, 177],
    ...     "weight": [55, 61, 66, 70, 72, 80, 63, 75],
    ...     "sex": ["f", "f", "f", "m", "m", "m", "f", "m"],
    ... }
    >>> plot = ggplot(data, aes("height", "weight")) + geom_point()
    >>> plot = plot + geom_ysideviolin()
    """
    layer = geom_violin(mapping, **kwargs)
    return _side(layer, "y", "violin", kwargs, orientation=False)


def geom_xsideline(mapping: FeatureSpec | None = None, **kwargs: Any) -> LayerSpec:
    """Draw a line in the x side panel (R package ggside).

    The x side panel sits above the panel (below with ``ggside(x_pos="bottom")``) and shares
    its x axis.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Along x it maps through the main
        panel's x scale; across the band through the band's own y scale
        (``scale_xsidey_*()``).
    **kwargs
        Every other parameter of ``geom_line()``.

    Returns
    -------
    LayerSpec
        The side layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "height": [160, 165, 170, 172, 175, 181, 168, 177],
    ...     "weight": [55, 61, 66, 70, 72, 80, 63, 75],
    ...     "sex": ["f", "f", "f", "m", "m", "m", "f", "m"],
    ... }
    >>> plot = ggplot(data, aes("height", "weight")) + geom_point()
    >>> plot = plot + geom_xsideline(aes(y="weight"))
    """
    layer = geom_line(mapping, **kwargs)
    return _side(layer, "x", "line", kwargs, orientation=False)


def geom_ysideline(mapping: FeatureSpec | None = None, **kwargs: Any) -> LayerSpec:
    """Draw a line in the y side panel (R package ggside).

    The y side panel sits right of the panel (left with ``ggside(y_pos="left")``) and shares
    its y axis.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Along y it maps through the main
        panel's y scale; across the band through the band's own x scale
        (``scale_ysidex_*()``).
    **kwargs
        Every other parameter of ``geom_line()``.

    Returns
    -------
    LayerSpec
        The side layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "height": [160, 165, 170, 172, 175, 181, 168, 177],
    ...     "weight": [55, 61, 66, 70, 72, 80, 63, 75],
    ...     "sex": ["f", "f", "f", "m", "m", "m", "f", "m"],
    ... }
    >>> plot = ggplot(data, aes("height", "weight")) + geom_point()
    >>> plot = plot + geom_ysideline(aes(x="height"))
    """
    layer = geom_line(mapping, **kwargs)
    return _side(layer, "y", "line", kwargs, orientation=False)


def geom_xsidetext(mapping: FeatureSpec | None = None, **kwargs: Any) -> LayerSpec:
    """Draw text labels in the x side panel (R package ggside).

    The x side panel sits above the panel (below with ``ggside(x_pos="bottom")``) and shares
    its x axis.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Along x it maps through the main
        panel's x scale; across the band through the band's own y scale
        (``scale_xsidey_*()``).
    **kwargs
        Every other parameter of ``geom_text()``.

    Returns
    -------
    LayerSpec
        The side layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "height": [160, 165, 170, 172, 175, 181, 168, 177],
    ...     "weight": [55, 61, 66, 70, 72, 80, 63, 75],
    ...     "sex": ["f", "f", "f", "m", "m", "m", "f", "m"],
    ... }
    >>> plot = ggplot(data, aes("height", "weight")) + geom_point()
    >>> plot = plot + geom_xsidetext(aes(label="sex"), y=0)
    """
    layer = geom_text(mapping, **kwargs)
    return _side(layer, "x", "text", kwargs, orientation=False)


def geom_ysidetext(mapping: FeatureSpec | None = None, **kwargs: Any) -> LayerSpec:
    """Draw text labels in the y side panel (R package ggside).

    The y side panel sits right of the panel (left with ``ggside(y_pos="left")``) and shares
    its y axis.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Along y it maps through the main
        panel's y scale; across the band through the band's own x scale
        (``scale_ysidex_*()``).
    **kwargs
        Every other parameter of ``geom_text()``.

    Returns
    -------
    LayerSpec
        The side layer.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "height": [160, 165, 170, 172, 175, 181, 168, 177],
    ...     "weight": [55, 61, 66, 70, 72, 80, 63, 75],
    ...     "sex": ["f", "f", "f", "m", "m", "m", "f", "m"],
    ... }
    >>> plot = ggplot(data, aes("height", "weight")) + geom_point()
    >>> plot = plot + geom_ysidetext(aes(label="sex"), x=0)
    """
    layer = geom_text(mapping, **kwargs)
    return _side(layer, "y", "text", kwargs, orientation=False)


def _side_scale(
    scale: FeatureSpec, panel: SideAxis, function: str, given: dict[str, Any]
) -> FeatureSpec:
    """Mark ``scale`` as the own scale of the ``panel`` side panel.

    Parameters
    ----------
    scale : FeatureSpec
        The scale, built from ``given``.
    panel : {"x", "y"}
        The axis the side panel shares with the main panel.
    function : str
        The call, for messages.
    given : dict
        The keyword arguments the user gave (``side_pos=``, ``side_band=`` pick the band).

    Returns
    -------
    FeatureSpec
        ``scale``, marked.

    Raises
    ------
    SpecError
        If ``side_pos=`` or ``side_band=`` is not accepted.
    """
    _check_band_keys(function, panel, given, size=False)
    props = scale.props()
    props["ggside"] = panel
    for key in ("side_pos", "side_band"):
        if given.get(key) is not None:
            props[key] = given[key]
    return scale


def _band_keys(kwargs: dict[str, Any]) -> dict[str, Any]:
    """Remove and return the band keys (``side_pos``, ``side_band``, ``side_size``)."""
    return {key: kwargs.pop(key) for key in ("side_pos", "side_band", "side_size") if key in kwargs}


def scale_xsidey_continuous(name: str | None = None, **kwargs: Any) -> FeatureSpec:
    """Set the x side panel's own continuous y axis (R package ggside).

    Parameters
    ----------
    name : str, optional
        Axis title (not drawn: side panels show ticks and labels only).
    **kwargs
        Every other parameter of ``scale_y_continuous()``.

    Returns
    -------
    FeatureSpec
        Scale specification.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1.0, 2.0, 2.5, 3.0, 4.0], "y": [2.0, 1.0, 3.0, 2.5, 4.0]}
    >>> plot = ggplot(data, aes("x", "y")) + geom_point() + geom_xsidedensity()
    >>> plot = plot + scale_xsidey_continuous(breaks=[0.0, 0.2])
    """
    band = _band_keys(kwargs)
    return _side_scale(scale_y_continuous(name, **kwargs), "x", "scale_xsidey_continuous", band)


def scale_xsidey_discrete(name: str | None = None, **kwargs: Any) -> FeatureSpec:
    """Set the x side panel's own discrete y axis, e.g. the track order (R package ggside).

    Parameters
    ----------
    name : str, optional
        Axis title (not drawn: side panels show ticks and labels only).
    **kwargs
        Every other parameter of ``scale_y_discrete()``, e.g. ``limits`` to order
        the tracks (the first one nearest the bottom).

    Returns
    -------
    FeatureSpec
        Scale specification.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "cell": ["c1", "c2", "c3", "c4"] * 2,
    ...     "gene": ["CD3E"] * 4 + ["MS4A1"] * 4,
    ...     "expression": [2.1, 1.8, 0.2, 0.1, 0.3, 0.1, 2.4, 2.0],
    ...     "cell_type": ["T", "T", "B", "B"] * 2,
    ...     "module": ["T genes"] * 4 + ["B genes"] * 4,
    ... }
    >>> plot = (
    ...     ggplot(data, aes("cell", "gene", fill="expression"))
    ...     + geom_tile()
    ...     + new_scale_fill()
    ...     + geom_xsidetile(aes(fill="cell_type"))
    ...     + new_scale_fill()
    ...     + geom_xsidetile(aes(fill="module"))
    ...     + scale_xsidey_discrete(limits=["module", "cell_type"])
    ... )
    """
    band = _band_keys(kwargs)
    return _side_scale(scale_y_discrete(name, **kwargs), "x", "scale_xsidey_discrete", band)


def scale_ysidex_continuous(name: str | None = None, **kwargs: Any) -> FeatureSpec:
    """Set the y side panel's own continuous x axis (R package ggside).

    Parameters
    ----------
    name : str, optional
        Axis title (not drawn: side panels show ticks and labels only).
    **kwargs
        Every other parameter of ``scale_x_continuous()``.

    Returns
    -------
    FeatureSpec
        Scale specification.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "height": [160, 165, 170, 172, 175, 181, 168, 177],
    ...     "weight": [55, 61, 66, 70, 72, 80, 63, 75],
    ... }
    >>> plot = ggplot(data, aes("height", "weight")) + geom_point() + geom_ysidedensity()
    >>> plot = plot + scale_ysidex_continuous(breaks=[0.0, 0.04])
    """
    band = _band_keys(kwargs)
    return _side_scale(scale_x_continuous(name, **kwargs), "y", "scale_ysidex_continuous", band)


def scale_ysidex_discrete(name: str | None = None, **kwargs: Any) -> FeatureSpec:
    """Set the y side panel's own discrete x axis, e.g. the track order (R package ggside).

    Parameters
    ----------
    name : str, optional
        Axis title (not drawn: side panels show ticks and labels only).
    **kwargs
        Every other parameter of ``scale_x_discrete()``, e.g. ``limits`` to order
        the tracks (the first one nearest the left).

    Returns
    -------
    FeatureSpec
        Scale specification.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {
    ...     "cell": ["c1", "c2", "c3", "c4"] * 2,
    ...     "gene": ["CD3E"] * 4 + ["MS4A1"] * 4,
    ...     "expression": [2.1, 1.8, 0.2, 0.1, 0.3, 0.1, 2.4, 2.0],
    ...     "cell_type": ["T", "T", "B", "B"] * 2,
    ...     "module": ["T genes"] * 4 + ["B genes"] * 4,
    ... }
    >>> plot = (
    ...     ggplot(data, aes("cell", "gene", fill="expression"))
    ...     + geom_tile()
    ...     + new_scale_fill()
    ...     + geom_ysidetile(aes(fill="module"))
    ...     + scale_ysidex_discrete(expand=[0, 0])
    ... )
    """
    band = _band_keys(kwargs)
    return _side_scale(scale_x_discrete(name, **kwargs), "y", "scale_ysidex_discrete", band)


_POSITIONS: dict[str, tuple[str, str]] = {"x_pos": ("top", "bottom"), "y_pos": ("right", "left")}


def ggside(
    x_pos: Literal["top", "bottom"] | None = None,
    y_pos: Literal["right", "left"] | None = None,
) -> FeatureSpec:
    """Place the side panels (R package ggside).

    Parameters
    ----------
    x_pos : {"top", "bottom"}, default="top"
        Side of the x side panel (``geom_xside*()`` layers).
    y_pos : {"right", "left"}, default="right"
        Side of the y side panel (``geom_yside*()`` layers).

    Returns
    -------
    FeatureSpec
        The options, added to a plot with ``+``.

    Raises
    ------
    SpecError
        If a position is not one of the accepted sides.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1.0, 2.0, 2.5, 3.0, 4.0], "y": [2.0, 1.0, 3.0, 2.5, 4.0]}
    >>> plot = ggplot(data, aes("x", "y")) + geom_point() + geom_xsidehistogram(bins=4)
    >>> plot = plot + ggside(x_pos="bottom")
    """
    for key, value in (("x_pos", x_pos), ("y_pos", y_pos)):
        accepted = _POSITIONS[key]
        if value is not None and value not in accepted:
            reason = (
                f"expected '{accepted[0]}' or '{accepted[1]}'{did_you_mean(str(value), accepted)}"
            )
            raise SpecError(f"ggside({key}={value!r}): {reason}", path=key, reason=reason)
    return FeatureSpec("ggside", name=None, x_pos=x_pos, y_pos=y_pos)


_MARGIN_SIDES = ("l", "r", "t", "b")
_MARGIN_SIZE_RANGE = (0.01, 0.95)


def _margin_size(size: float | Sequence[float] | None, index: int) -> float | None:
    """Return the size of the ``index``-th side (lets-plot ``_to_size``)."""
    if size is None:
        return None
    if isinstance(size, int | float) and not isinstance(size, bool):
        return float(size)
    if not isinstance(size, list | tuple):
        reason = f"expected a number or a list of numbers, got {type(size).__name__}"
        raise TypeError(f"ggmarginal(size=...): {reason}")
    return float(size[index]) if index < len(size) else None


def _marginal_layer(side: str, size: float | None, layer: LayerSpec) -> LayerSpec:
    """Return a marginal copy of ``layer`` on ``side`` (lets-plot ``_to_marginal``)."""
    copy = LayerSpec.duplicate(layer)
    props = copy.props()
    props.update(marginal=True, margin_side=side, margin_size=size)
    stat = props.get("stat")
    stat_name = stat.props().get("name") if isinstance(stat, FeatureSpec) else stat
    kinds = {"bin": "histogram", "ydensity": "violin"}
    kind = kinds.get(stat_name, stat_name) if stat_name is not None else props.get("geom")
    vertical_margin = side in ("l", "r")
    automatic: dict[str, Any] = {}
    if vertical_margin and kind in ("histogram", "density", "freqpoly"):
        automatic["orientation"] = "y"
    if kind in ("boxplot", "boxplot_outlier", "violin"):
        if vertical_margin:
            automatic["x"] = 0
        else:
            automatic["y"] = 0
            automatic["orientation"] = "y"
    # The user's options win over the automatic ones.
    given = {key: value for key, value in props.items() if value is not None}
    props.update({**automatic, **given})
    if kind == "histogram":
        # Densities, as the density geom draws (lets-plot).
        mapping = props.get("mapping")
        own = mapping.props() if isinstance(mapping, FeatureSpec) else {}
        added = {("x" if vertical_margin else "y"): "..density.."}
        mapped = {key: value for key, value in own.items() if value is not None}
        props["mapping"] = FeatureSpec("mapping", name=None, **{**added, **mapped})
    return copy


def ggmarginal(
    sides: str,
    *,
    size: float | Sequence[float] | None = None,
    layer: LayerSpec | FeatureSpecArray,
) -> FeatureSpec:
    """Add copies of a layer in margins of the panel: a marginal plot (as lets-plot).

    Parameters
    ----------
    sides : str
        The margins, any of ``"t"``, ``"r"``, ``"b"``, ``"l"`` (top, right, bottom,
        left), e.g. ``"tr"``.
    size : float or list of float, default=0.1
        The margin's height or width as a share of the plotting area (panel plus
        margins), in ``[0.01, 0.95]``; a list gives one per side, in order.
    layer : LayerSpec or FeatureSpecArray
        The layer to copy, e.g. ``geom_density()``; histograms and densities in the
        left and right margins turn sideways, boxplots and violins sit at 0 across.

    Returns
    -------
    FeatureSpec
        The marginal layers.

    Raises
    ------
    TypeError
        If ``sides`` is not a string or ``size`` not a number or list.
    ValueError
        If ``sides`` names an unknown margin or ``size`` is out of range.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1.0, 2.0, 2.5, 3.0, 4.0], "y": [2.0, 1.0, 3.0, 2.5, 4.0]}
    >>> plot = ggplot(data, aes("x", "y")) + geom_point()
    >>> plot = plot + ggmarginal("tr", layer=geom_density(fill="grey"))
    """
    if not isinstance(sides, str):
        raise TypeError(f"ggmarginal(sides=...): expected a string, got {type(sides).__name__}")
    if not 0 < len(sides) <= len(_MARGIN_SIDES):
        message = f"ggmarginal(sides={sides!r}): expected 1 to 4 of 'l', 'r', 't', 'b', e.g. 'tr'"
        raise ValueError(message)
    layers = _layers(layer)
    if not layers:
        raise TypeError(
            f"ggmarginal(layer=...): expected a geom_*() layer, got {type(layer).__name__}"
        )
    low, high = _MARGIN_SIZE_RANGE
    sizes: list[float | None] = []
    for index, side in enumerate(sides):
        if side not in _MARGIN_SIDES:
            message = (
                f"ggmarginal(sides={sides!r}): unknown margin {side!r}; use 'l', 'r', 't' or 'b'"
            )
            raise ValueError(message)
        margin_size = _margin_size(size, index)
        if margin_size is not None and not low <= margin_size <= high:
            message = f"ggmarginal(size={size!r}): expected sizes in [{low}, {high}]"
            raise ValueError(message)
        sizes.append(margin_size)
    # Layers outside, sides inside: lets-plot's order.
    result: FeatureSpec | None = None
    for sublayer in layers:
        for side, margin_size in zip(sides, sizes, strict=True):
            marginal = _marginal_layer(side, margin_size, sublayer)
            result = marginal if result is None else result + marginal
    assert result is not None
    return result
