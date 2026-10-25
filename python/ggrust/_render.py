"""Bridge from plot specs to the Rust engine: one place for spec JSON, data and output."""

from __future__ import annotations

import datetime
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
    columns = referenced_columns(spec)
    encoded, messages = render(
        to_json(spec),
        _export(plot_data, columns, "data"),
        [_export(data, columns, f"layers[{index}].data") for index, data in enumerate(layer_data)],
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


def referenced_columns(spec: dict) -> set[str]:
    """Return the column names the plot can read.

    Every mapped column (plot and layers) and ``as_discrete(order_by=...)`` column.
    Only these are exported to the engine.
    """
    columns: set[str] = set()

    def add_mapping(mapping) -> None:
        for value in (mapping or {}).values():
            if isinstance(value, str):
                columns.add(value)
            elif isinstance(value, list | tuple):
                columns.update(item for item in value if isinstance(item, str))

    def add_meta(data_meta) -> None:
        for annotation in (data_meta or {}).get("mapping_annotations", []):
            order_by = annotation.get("parameters", {}).get("order_by")
            if isinstance(order_by, str):
                columns.add(order_by)

    add_mapping(spec.get("mapping"))
    add_meta(spec.get("data_meta"))
    for layer in spec.get("layers", []):
        add_mapping(layer.get("mapping"))
        add_meta(layer.get("data_meta"))
    return columns


def _export(data: Any, columns: set[str], path: str):
    """Arrow export of the referenced columns of ``data`` (None stays None)."""
    if data is None:
        return None
    try:
        return as_arrow(_select(data, columns))
    except TypeError:
        raise
    except Exception as error:
        message = f"at '{path}': could not read the data columns {sorted(columns)}: {error}"
        raise ValueError(message) from error


def _select(data: Any, columns: set[str]) -> Any:
    """Keep only ``columns`` (in the data's own order) when the type allows it.

    A frame needs at least one column to carry its row count (the engine merges plot
    and layer data by row count), so when none is referenced its first column stays.
    """
    import polars as pl

    def keep(names) -> list:
        names = list(names)
        kept = [name for name in names if name in columns]
        return kept or names[:1]

    pandas = sys.modules.get("pandas")
    pyarrow = sys.modules.get("pyarrow")
    if isinstance(data, pl.DataFrame | pl.LazyFrame):
        return data.select(keep(data.collect_schema().names()))
    if pandas is not None and isinstance(data, pandas.DataFrame):
        kept = keep(data.columns)
        if len(kept) != len(set(kept)):
            return data  # duplicate names: let the engine report them
        return data[kept]
    if pyarrow is not None and isinstance(data, pyarrow.Table):
        return data.select(keep(data.column_names))
    if isinstance(data, dict):
        kept = set(keep(data))
        return {key: value for key, value in data.items() if key in kept}
    return data


# Deeper option values than this are certainly a mistake (and would recurse forever
# for self-referencing containers).
MAX_DEPTH = 64


def to_json(spec: dict) -> str:
    """Serialise a spec to JSON.

    NaN/inf become null, numpy values become plain values, and datetimes become epoch
    milliseconds (as lets-plot sends them).
    """
    return json.dumps(_plain(spec, "spec", 0), allow_nan=False)


def _plain(value: Any, path: str, depth: int) -> Any:
    if depth > MAX_DEPTH:
        message = f"at '{path}': value is nested more than {MAX_DEPTH} levels deep"
        raise TypeError(message)
    if value is None or isinstance(value, bool | int | str):
        return value
    if isinstance(value, float):
        return value if math.isfinite(value) else None
    if isinstance(value, dict):
        plain = {}
        for key, item in value.items():
            name = str(key)
            if name in plain:
                message = f"at '{path}': keys {key!r} and {name!r} collide as JSON keys"
                raise TypeError(message)
            plain[name] = _plain(item, f"{path}.{key}", depth + 1)
        return plain
    if isinstance(value, list | tuple):
        return [_plain(item, f"{path}[{index}]", depth + 1) for index, item in enumerate(value)]
    if isinstance(value, datetime.datetime | datetime.date | datetime.timedelta):
        return _epoch_milliseconds(value)
    numpy = sys.modules.get("numpy")
    if numpy is not None:
        if isinstance(value, numpy.ndarray):
            return _plain(value.tolist(), path, depth + 1)
        if isinstance(value, numpy.datetime64):
            return float(value.astype("datetime64[us]").astype("int64")) / 1000.0
        if isinstance(value, numpy.timedelta64):
            return float(value.astype("timedelta64[us]").astype("int64")) / 1000.0
        if isinstance(value, numpy.generic):
            return _plain(value.item(), path, depth + 1)
    message = f"at '{path}': cannot send a value of type {type(value).__name__} to the engine"
    raise TypeError(message)


def _epoch_milliseconds(value) -> float:
    """Milliseconds since the Unix epoch (naive datetimes are taken as UTC)."""
    if isinstance(value, datetime.timedelta):
        return value.total_seconds() * 1000.0
    if not isinstance(value, datetime.datetime):
        value = datetime.datetime(value.year, value.month, value.day)
    if value.tzinfo is None:
        value = value.replace(tzinfo=datetime.UTC)
    return value.timestamp() * 1000.0


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
