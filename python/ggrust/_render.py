"""Bridge from plot specs to the Rust engine: one place for spec JSON, data and output."""

from __future__ import annotations

import datetime
import json
import math
import os
import secrets
import sys
import warnings
from typing import Any

from ggrust._data import as_arrow
from ggrust._ggrust import referenced_columns, render, tooltip_columns

DEFAULT_PNG_SCALE = 2.0  # lets-plot's default


class GgrustWarning(UserWarning):
    """Something in the plot was ignored or adjusted; the message says what and where."""


def render_bytes(plot, output: str, scale: float = 1.0, size: tuple | None = None) -> bytes:
    """Build ``plot`` in Rust and return the encoded output.

    Parameters
    ----------
    plot : PlotSpec
        The plot.
    output : {"svg", "png", "html", "html_page", "html_iframe"}
        Output format. HTML outputs get a fresh element id on every call, so one plot
        displayed twice on a page keeps its ids apart.
    scale : float
        PNG pixel scale.
    size : tuple of float, optional
        ``(width, height)`` overriding ``ggsize``.

    Returns
    -------
    bytes
        SVG or HTML (UTF-8), or PNG bytes.
    """
    spec, frames = split_data(plot.as_dict())
    if size is not None:
        spec["ggsize"] = {"width": size[0], "height": size[1]}
    spec_json = to_json(spec)
    columns = set(referenced_columns(spec_json))
    html = output.startswith("html")
    extra = set(tooltip_columns(spec_json)) - columns if html else set()
    exported = []
    for data, path in frames:
        arrow, dropped = _export_with_tooltips(data, columns, extra, path)
        exported.append(arrow)
        for name in dropped:
            message = f"at '{path}': the tooltip column {name!r} could not be read and is not shown"
            warnings.warn(message, GgrustWarning, stacklevel=3)
    encoded, messages = render(
        spec_json,
        exported,
        output,
        float(scale),
        _display_id() if output.startswith("html") else None,
    )
    for message in messages:
        warnings.warn(message, GgrustWarning, stacklevel=4)
    return encoded


def _display_id() -> str:
    """Return a fresh root element id for one HTML display (48 random bits)."""
    return f"ggrust-{secrets.token_hex(6)}"


def split_data(spec: dict) -> tuple[dict, list[tuple[Any, str]]]:
    """Replace inline data with frame references (sent as JSON; data goes as Arrow).

    Every ``data`` value of the figure (plot, layers, nested figures) becomes
    ``{"frame": i, "columns": [...]}``. The same object used twice (by identity) is
    one frame, so a ``gggrid`` of plots on one frame exports it once. ``columns``
    names every column of the data (only referenced ones are exported): the engine
    names list mappings (``aes(x=[1, 2])``) around them, as lets-plot does.

    Returns
    -------
    tuple
        ``(spec with references, [(data, path of its first use), ...])``.
    """
    frames: list[tuple[Any, str]] = []
    index_by_id: dict[int, int] = {}

    def reference(data: Any, path: str) -> dict:
        index = index_by_id.get(id(data))
        if index is None:
            index = index_by_id[id(data)] = len(frames)
            frames.append((data, path))
        return {"frame": index, "columns": _column_names(data)}

    def split(figure: dict, prefix: str) -> dict:
        figure = dict(figure)
        if figure.get("data") is not None:
            figure["data"] = reference(figure["data"], f"{prefix}data")
        layers = []
        for index, layer in enumerate(figure.get("layers", [])):
            layer = dict(layer)
            if layer.get("data") is not None:
                layer["data"] = reference(layer["data"], f"{prefix}layers[{index}].data")
            layers.append(layer)
        if "layers" in figure:
            figure["layers"] = layers
        if "figures" in figure:
            figure["figures"] = [
                None if child is None else split(child, f"{prefix}figures[{index}].")
                for index, child in enumerate(figure["figures"])
            ]
        return figure

    return split(spec, ""), frames


def _column_names(data: Any) -> list[str]:
    """Names of every column of ``data``, without reading any values."""
    import polars as pl

    pandas = sys.modules.get("pandas")
    pyarrow = sys.modules.get("pyarrow")
    if isinstance(data, pl.DataFrame | pl.LazyFrame):
        names = data.collect_schema().names()
    elif pandas is not None and isinstance(data, pandas.DataFrame):
        names = data.columns
    elif pyarrow is not None and isinstance(data, pyarrow.Table):
        names = data.column_names
    elif isinstance(data, dict):
        names = data
    else:
        return []
    return [str(name) for name in names]


def _export_with_tooltips(
    data: Any, columns: set[str], extra: set[str], path: str
) -> tuple[Any, list[str]]:
    """Export the referenced columns plus the tooltip columns ``extra`` (HTML only).

    A tooltip column that cannot be exported never fails the plot: it is left out
    and returned in the dropped names (the tooltip line then warns as unknown).
    """
    present = extra & set(_column_names(data))
    if not present:
        return _export(data, columns, path), []
    try:
        return as_arrow(_select(data, columns | present)), []
    except Exception:
        dropped = []
        for name in sorted(present):
            try:
                as_arrow(_select(data, {name}))
            except Exception:
                dropped.append(name)
        kept = columns | (present - set(dropped))
        return _export(data, kept, path), dropped


def _export(data: Any, columns: set[str], path: str):
    """Arrow export of the referenced columns of ``data``."""
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


def export_html(plot, path=None, iframe: bool | None = None) -> str | None:
    """See ``PlotSpec.to_html``."""
    page = render_bytes(plot, "html_iframe" if iframe else "html_page").decode()
    return page if path is None else _write(path, page)


def render_notebook_html(plot) -> str:
    """HTML for Jupyter's ``_repr_html_``: the plot's root ``<div>`` and runtime script."""
    return render_bytes(plot, "html").decode()
