"""ggrastr: ``rasterize()`` draws one layer as a bitmap inside vector output.

Not in lets-plot; ggplot2 needs the ggrastr package (scattermore is similar). In SVG
and PDF output a rasterized layer becomes one embedded PNG per panel, on the pixel
grid of the plot's PNG at that resolution, so it lines up with the vector axes.
Axes, text, legends and the other layers stay vector. PNG output is a bitmap
already and does not change; HTML output keeps the layer vector so tooltips, zoom
and the canvas for dense points keep working (see ``deviations.md``).
"""

from __future__ import annotations

from numbers import Real
from typing import overload

from ggrust.errors import SpecError, UnsupportedValueError
from ggrust.plot.core import FeatureSpec, FeatureSpecArray, LayerSpec

__all__ = ["MAX_RASTERIZE_DPI", "rasterize"]

MAX_RASTERIZE_DPI: float = 4800.0
"""Highest accepted ``dpi`` (the engine's limit, ``spec::MAX_RASTERIZE_DPI``)."""


@overload
def rasterize(layer: LayerSpec, dpi: float = 300) -> LayerSpec: ...


@overload
def rasterize(layer: FeatureSpecArray, dpi: float = 300) -> FeatureSpecArray: ...


def rasterize(
    layer: LayerSpec | FeatureSpecArray, dpi: float = 300
) -> LayerSpec | FeatureSpecArray:
    """Draw a layer as a bitmap in SVG and PDF output (R package ggrastr).

    A plot with a million points saves as a small vector file: the rasterized
    layer is one embedded image per panel, while axes, text, legends and the other
    layers stay vector. The image is clipped to the panel like the layer would be
    and its pixels line up with the axes.

    PNG output is unchanged (it is a bitmap already). HTML output (notebooks) keeps
    the layer vector, so its tooltips and zoom keep working.

    Parameters
    ----------
    layer : LayerSpec or FeatureSpecArray
        The layer, e.g. ``geom_point(...)``. A ``geom_*()`` that adds several
        layers (``geom_boxplot()`` with its outliers) is rasterized as a whole.
    dpi : float, default=300
        Resolution of the bitmap in dots per inch of the output, in
        ``(0, 4800]``. In SVG a plot pixel is 1/96 inch, so 300 dpi draws about
        3 bitmap pixels per plot pixel; in PDF the page size sets the inch. The
        bitmap of one panel may have at most 32,768 pixels per side, so very
        high values fail at render time on large panels (``RenderError``).

    Returns
    -------
    LayerSpec or FeatureSpecArray
        A copy of ``layer`` that is drawn rasterized.

    Raises
    ------
    UnsupportedValueError
        When ``layer`` is not a layer, or ``dpi`` is not a number.
    SpecError
        When ``dpi`` is outside ``(0, 4800]``.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3, 4], "y": [2, 4, 3, 5]}
    >>> plot = ggplot(data, aes("x", "y")) + rasterize(geom_point(size=4), dpi=300)
    """
    if isinstance(dpi, bool) or not isinstance(dpi, Real):
        reason = f"rasterize() dpi must be a number of dots per inch, got {type(dpi).__name__}"
        raise UnsupportedValueError(f"at 'dpi': {reason}", path="dpi", reason=reason)
    value = float(dpi)
    if not 0.0 < value <= MAX_RASTERIZE_DPI:
        reason = (
            f"rasterize() dpi must be in (0, {MAX_RASTERIZE_DPI:g}], got {dpi}; "
            "300 suits print, 96 the screen"
        )
        raise SpecError(f"at 'dpi': {reason}", path="dpi", reason=reason)
    if isinstance(layer, LayerSpec):
        return _rasterized(layer, value)
    if isinstance(layer, FeatureSpecArray) and all(
        isinstance(element, LayerSpec) for element in layer
    ):
        return FeatureSpecArray(
            *(_rasterized(element, value) for element in layer if isinstance(element, LayerSpec))
        )
    reason = (
        "rasterize() takes a layer such as geom_point(...), "
        f"got {_describe(layer)}; wrap the layer, then add it to the plot"
    )
    raise UnsupportedValueError(f"at 'layer': {reason}", path="layer", reason=reason)


def _rasterized(layer: LayerSpec, dpi: float) -> LayerSpec:
    """Return a copy of ``layer`` carrying ``rasterize={"dpi": dpi}``."""
    copy = LayerSpec.duplicate(layer)
    copy.props()["rasterize"] = {"dpi": dpi}
    return copy


def _describe(value: object) -> str:
    """Name what was passed instead of a layer (``a plot``, ``int``)."""
    if isinstance(value, FeatureSpec):
        return f"a {value.kind!r} spec"
    return type(value).__name__
