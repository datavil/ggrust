"""Bridge from plot specs to the Rust engine: one place for spec JSON, data and output."""

from __future__ import annotations

import json
import math
import os
import sys
import warnings
from typing import Any

from ggrust._data import as_arrow
from ggrust._ggrust import render

DEFAULT_PNG_SCALE = 2.0  # lets-plot's default


class GgrustWarning(UserWarning):
    """Something in the plot was ignored or adjusted; the message says what and where."""


def render_bytes(plot, output: str, scale: float = 1.0, size: tuple | None = None) -> bytes:
    """Build ``plot`` in Rust and return the encoded image.

    Parameters
    ----------
    plot : PlotSpec
        The plot.
    output : {"svg", "png"}
        Output format.
    scale : float
        PNG pixel scale.
    size : tuple of float, optional
        ``(width, height)`` overriding ``ggsize``.

    Returns
    -------
    bytes
        SVG (UTF-8) or PNG bytes.
    """
    spec, plot_data, layer_data = split_data(plot.as_dict())
    if size is not None:
        spec["ggsize"] = {"width": size[0], "height": size[1]}
    encoded, messages = render(
        to_json(spec),
        None if plot_data is None else as_arrow(plot_data),
        [None if data is None else as_arrow(data) for data in layer_data],
        output,
        float(scale),
    )
    for message in messages:
        warnings.warn(message, GgrustWarning, stacklevel=4)
    return encoded


def split_data(spec: dict) -> tuple[dict, Any, list]:
    """Separate inline data (sent as Arrow) from the spec (sent as JSON).

    Returns
    -------
    tuple
        ``(spec without data, plot data or None, [layer data or None, ...])``.
    """
    spec = dict(spec)
    plot_data = spec.pop("data", None)
    layers = []
    layer_data = []
    for layer in spec.get("layers", []):
        layer = dict(layer)
        layer_data.append(layer.pop("data", None))
        layers.append(layer)
    spec["layers"] = layers
    return spec, plot_data, layer_data


def to_json(spec: dict) -> str:
    """Serialise a spec: NaN/inf become null, numpy values become plain values."""
    return json.dumps(_plain(spec, "spec"), allow_nan=False)


def _plain(value: Any, path: str) -> Any:
    if value is None or isinstance(value, bool | int | str):
        return value
    if isinstance(value, float):
        return value if math.isfinite(value) else None
    if isinstance(value, dict):
        return {str(key): _plain(item, f"{path}.{key}") for key, item in value.items()}
    if isinstance(value, list | tuple):
        return [_plain(item, f"{path}[{index}]") for index, item in enumerate(value)]
    numpy = sys.modules.get("numpy")
    if numpy is not None:
        if isinstance(value, numpy.ndarray):
            return _plain(value.tolist(), path)
        if isinstance(value, numpy.generic):
            return _plain(value.item(), path)
    message = f"at '{path}': cannot send a value of type {type(value).__name__} to the engine"
    raise TypeError(message)


def _pixels(w, h, unit) -> tuple | None:
    if w is None and h is None:
        return None
    if w is None or h is None:
        message = "give both 'w' and 'h', or neither"
        raise ValueError(message)
    if unit not in (None, "px"):
        message = f"unit {unit!r} is not supported yet; use pixels ('px')"
        raise ValueError(message)
    return float(w), float(h)


def _write(target, content: bytes | str) -> str | None:
    if isinstance(target, str | os.PathLike):
        path = os.path.abspath(target)
        directory = os.path.dirname(path)
        if directory:
            os.makedirs(directory, exist_ok=True)
        mode = "wb" if isinstance(content, bytes) else "w"
        with open(
            path, mode, **({} if isinstance(content, bytes) else {"encoding": "utf-8"})
        ) as file:
            file.write(content)
        return path
    target.write(content if isinstance(content, bytes) else content.encode())
    return None


def export_svg(plot, path=None, w=None, h=None, unit=None) -> str | None:
    """See ``PlotSpec.to_svg``."""
    svg = render_bytes(plot, "svg", size=_pixels(w, h, unit)).decode()
    return svg if path is None else _write(path, svg)


def export_png(plot, path, scale=None, w=None, h=None, unit=None, dpi=None) -> str | None:
    """See ``PlotSpec.to_png``."""
    if dpi is not None:
        message = "'dpi' is not supported yet; use 'scale'"
        raise ValueError(message)
    scale = DEFAULT_PNG_SCALE if scale is None else scale
    png = render_bytes(plot, "png", scale=scale, size=_pixels(w, h, unit))
    return _write(path, png)


def export_html(plot, path=None) -> str | None:
    """See ``PlotSpec.to_html``."""
    svg = render_bytes(plot, "svg").decode()
    page = (
        '<!DOCTYPE html>\n<html lang="en">\n<head>\n<meta charset="UTF-8">\n'
        "<title>ggrust plot</title>\n</head>\n<body>\n"
        f'<div class="ggrust-plot">{svg}</div>\n</body>\n</html>\n'
    )
    return page if path is None else _write(path, page)


def render_notebook_html(plot) -> str:
    """HTML for Jupyter's ``_repr_html_``: the SVG inline (interactivity in Phase 4)."""
    svg = render_bytes(plot, "svg").decode()
    return f'<div class="ggrust-plot">{svg}</div>'
