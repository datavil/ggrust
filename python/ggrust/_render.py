"""Bridge from plot specs to the Rust engine: one place for spec JSON, data and output."""

from __future__ import annotations

import datetime
import json
import math
import os
import secrets
import sys
import warnings
from typing import TYPE_CHECKING, Any, cast

from ggrust._data import as_arrow
from ggrust._ggrust import inspect, referenced_columns, render, tooltip_columns
from ggrust.errors import DataError, ExportError, UnsupportedValueError, did_you_mean

if TYPE_CHECKING:
    from collections.abc import Callable, Iterable

    from ggrust._data import ArrowStreamExportable
    from ggrust._typing import Number, PanelScales, PlotBuild, Target
    from ggrust.plot.core import PlotSpec
    from ggrust.plot.subplots import SupPlotsSpec

DEFAULT_PNG_SCALE = 2.0  # lets-plot's default
SCREEN_DPI = 96.0  # layout pixels per inch (CSS), lets-plot's PNG default resolution
PRINT_DPI = 300.0  # lets-plot's resolution for sizes in physical units
MAX_UNITLESS_SIZE = 20.0  # lets-plot: larger w / h without a unit look like pixels, not inches
INCHES_PER_UNIT = {"in": 1.0, "cm": 1 / 2.54, "mm": 1 / 25.4}


class GgrustWarning(UserWarning):
    """Something in the plot was ignored or adjusted; the message says what and where.

    Examples
    --------
    >>> from ggrust import *
    >>> import warnings
    >>> data = {"weight": [2.6, 2.9, None], "mpg": [31, 28, 20]}
    >>> chart = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> with warnings.catch_warnings(record=True) as caught:
    ...     warnings.simplefilter("always")
    ...     svg = chart.to_svg()
    >>> caught[0].category is GgrustWarning
    True
    """


def render_bytes(
    plot: PlotSpec | SupPlotsSpec,
    output: str,
    scale: float = 1.0,
    size: tuple | None = None,
    dpi: float | None = None,
) -> bytes:
    """Build ``plot`` in Rust and return the encoded output.

    Parameters
    ----------
    plot : PlotSpec
        The plot.
    output : {"svg", "png", "pdf", "html", "html_page", "html_iframe"}
        Output format. HTML outputs get a fresh element id on every call, so one plot
        displayed twice on a page keeps its ids apart.
    scale : float
        PNG / PDF pixel scale.
    size : tuple of float, optional
        ``(width, height)`` overriding ``ggsize``.
    dpi : float, optional
        PNG resolution metadata and PDF page resolution (96 when None).

    Returns
    -------
    bytes
        SVG or HTML (UTF-8), or PNG / PDF bytes.
    """
    spec, frames = split_data(plot.as_dict())
    spec, functions = split_functions(spec)
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
        None if dpi is None else float(dpi),
        functions,
    )
    for message in messages:
        warnings.warn(message, GgrustWarning, stacklevel=4)
    return encoded


def build_plot(plot: PlotSpec) -> PlotBuild:
    """Run the engine's stats, positions, scales and layout on ``plot``, draw nothing.

    Parameters
    ----------
    plot : PlotSpec
        A single plot.

    Returns
    -------
    PlotBuild
        One polars frame per layer and the x and y scales of every panel. Build
        warnings are emitted as ``GgrustWarning``, as when rendering.
    """
    import polars as pl

    spec, frames = split_data(plot.as_dict())
    spec, functions = split_functions(spec)
    spec_json = to_json(spec)
    columns = set(referenced_columns(spec_json))
    exported = [_export(data, columns, path) for data, path in frames]
    tables, panels, messages = inspect(spec_json, exported, functions)
    for message in messages:
        warnings.warn(message, GgrustWarning, stacklevel=4)
    layers = [pl.DataFrame(table) for table in tables]
    return {"layers": layers, "panels": cast("list[PanelScales]", panels)}


def _display_id() -> str:
    """Return a fresh root element id for one HTML display (48 random bits)."""
    return f"ggrust-{secrets.token_hex(6)}"


def split_data(spec: dict[str, Any]) -> tuple[dict, list[tuple[Any, str]]]:
    """Replace inline data with frame references (sent as JSON; data goes as Arrow).

    Every ``data`` value of the figure (plot, layers, nested figures) becomes
    ``{"frame": i, "columns": [...]}``. The same object used twice (by identity) is
    one frame, so a ``gggrid`` of plots on one frame exports it once. ``columns``
    names every column of the data (only referenced ones are exported): the engine
    names list mappings (``aes(x=[1, 2])``) around them, as lets-plot does.

    Parameters
    ----------
    spec : dict
        The figure's ``as_dict()``.

    Returns
    -------
    tuple
        ``(spec with references, [(data, path of its first use), ...])``.
    """
    frames: list[tuple[object, str]] = []
    index_by_id: dict[int, int] = {}

    def reference(data: object, path: str) -> dict[str, Any]:
        index = index_by_id.get(id(data))
        if index is None:
            index = index_by_id[id(data)] = len(frames)
            frames.append((data, path))
        return {"frame": index, "columns": _column_names(data)}

    def split(figure: dict[str, Any], prefix: str) -> dict[str, Any]:
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
        if "insets" in figure:
            figure["insets"] = [
                {**inset, "figure": split(inset["figure"], f"{prefix}insets[{index}].figure.")}
                for index, inset in enumerate(figure["insets"])
            ]
        return figure

    return split(spec, ""), frames


def split_functions(spec: dict[str, Any]) -> tuple[dict[str, Any], list[Callable[[float], float]]]:
    """Replace secondary axis transforms (callables) with indexes into a list.

    Every ``sec_axis`` of the figure (any scale, nested figures and insets) whose
    ``transform`` is a callable gets the callable's index instead; the callables go
    to the engine beside the JSON, which calls them once per value.

    Parameters
    ----------
    spec : dict
        The figure's spec (``as_dict()``, data already split).

    Returns
    -------
    tuple
        ``(spec with indexes, [callable, ...])``. The spec is copied where changed.
    """
    functions: list[Callable[[float], float]] = []

    def scale(item: object) -> object:
        if not isinstance(item, dict):
            return item
        axis = item.get("sec_axis")
        if not isinstance(axis, dict) or not callable(axis.get("transform")):
            return item
        functions.append(axis["transform"])
        return {**item, "sec_axis": {**axis, "transform": len(functions) - 1}}

    def walk(figure: object) -> object:
        if not isinstance(figure, dict):
            return figure
        figure = dict(figure)
        if isinstance(figure.get("scales"), list):
            figure["scales"] = [scale(item) for item in figure["scales"]]
        if isinstance(figure.get("figures"), list):
            figure["figures"] = [walk(child) for child in figure["figures"]]
        if isinstance(figure.get("insets"), list):
            figure["insets"] = [
                {**inset, "figure": walk(inset.get("figure"))} if isinstance(inset, dict) else inset
                for inset in figure["insets"]
            ]
        return figure

    return cast("dict[str, Any]", walk(spec)), functions


def _column_names(data: object) -> list[str]:
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
    data: object, columns: set[str], extra: set[str], path: str
) -> tuple[ArrowStreamExportable, list[str]]:
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


def _export(data: object, columns: set[str], path: str) -> ArrowStreamExportable:
    """Arrow export of the referenced columns of ``data``."""
    try:
        return as_arrow(_select(data, columns))
    except TypeError:
        raise
    except Exception as error:
        message = f"at '{path}': could not read the data columns {sorted(columns)}: {error}"
        raise DataError(message) from error


def _select(data: object, columns: set[str]) -> object:
    """Keep only ``columns`` (in the data's own order) when the type allows it.

    A frame needs at least one column to carry its row count (the engine merges plot
    and layer data by row count), so when none is referenced its first column stays.
    """
    import polars as pl

    def keep(names: Iterable[str]) -> list[str]:
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
        columns_by_name = cast("dict[str, object]", data)
        kept = set(keep(columns_by_name))
        return {key: value for key, value in columns_by_name.items() if key in kept}
    return data


# Deeper option values than this are certainly a mistake (and would recurse forever
# for self-referencing containers).
MAX_DEPTH = 64


def to_json(spec: dict[str, Any]) -> str:
    """Serialise a spec to JSON.

    NaN/inf become null, numpy values become plain values, and datetimes become epoch
    milliseconds (as lets-plot sends them).

    Parameters
    ----------
    spec : dict
        The spec, with data replaced by frame references.

    Returns
    -------
    str
        The JSON text.

    Raises
    ------
    TypeError
        If a value cannot be sent to the engine; the message names its spec path.
    """
    return json.dumps(_plain(spec, "", 0), allow_nan=False)


def _plain(value: object, path: str, depth: int) -> object:
    if depth > MAX_DEPTH:
        reason = f"value is nested more than {MAX_DEPTH} levels deep"
        raise UnsupportedValueError(f"at '{path}': {reason}", path=path, reason=reason)
    if value is None or isinstance(value, bool | int | str):
        return value
    if isinstance(value, float):
        return value if math.isfinite(value) else None
    if isinstance(value, dict):
        plain = {}
        for key, item in value.items():
            name = str(key)
            if name in plain:
                reason = f"keys {key!r} and {name!r} collide as JSON keys; rename one"
                raise UnsupportedValueError(f"at '{path}': {reason}", path=path, reason=reason)
            plain[name] = _plain(item, f"{path}.{key}" if path else name, depth + 1)
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
    reason = (
        f"cannot send a value of type {type(value).__name__} to the engine; use numbers, "
        "texts, booleans, lists, dicts or dates"
    )
    raise UnsupportedValueError(f"at '{path}': {reason}", path=path, reason=reason)


def _epoch_milliseconds(value: datetime.date | datetime.timedelta) -> float:
    """Milliseconds since the Unix epoch (naive datetimes are taken as UTC)."""
    if isinstance(value, datetime.timedelta):
        return value.total_seconds() * 1000.0
    if not isinstance(value, datetime.datetime):
        value = datetime.datetime(value.year, value.month, value.day)
    if value.tzinfo is None:
        value = value.replace(tzinfo=datetime.UTC)
    return value.timestamp() * 1000.0


def _unit(unit: str | None) -> str | None:
    """Lower-case and check ``unit``: one of ``in``, ``cm``, ``mm``, ``px``, or None."""
    if unit is None:
        return None
    normalized = str(unit).strip().lower()
    if normalized not in (*INCHES_PER_UNIT, "px"):
        units = ["in", "cm", "mm", "px"]
        message = (
            f"unsupported unit {unit!r}; use one of 'in', 'cm', 'mm', 'px'"
            f"{did_you_mean(normalized, units)}"
        )
        raise ExportError(message)
    return normalized


def _given_size(
    w: float | None, h: float | None, unit: str | None
) -> tuple[float, float, str] | None:
    """``(w, h, unit)`` when both sides are given (a missing unit is inches), else None."""
    if w is None and h is None:
        return None
    if w is None or h is None:
        message = f"give both 'w' and 'h', or neither; got w={w}, h={h}"
        raise ExportError(message)
    w, h = float(w), float(h)
    if not (math.isfinite(w) and math.isfinite(h) and w > 0 and h > 0):
        message = f"'w' and 'h' must be positive numbers, got w={w}, h={h}"
        raise ExportError(message)
    if unit is None:
        if max(w, h) >= MAX_UNITLESS_SIZE:
            message = (
                "The image size was interpreted as inches, but it seems unusually large. "
                "Please specify the size unit explicitly (px, cm, mm, in)."
            )
            raise ExportError(message)
        unit = "in"
    return w, h, unit


def _raster_options(
    w: float | None,
    h: float | None,
    unit: str | None,
    dpi: float | None,
    scale: float | None,
) -> tuple[tuple[float, float] | None, float, float]:
    """Layout size override, pixel scale and resolution of a PNG / PDF, as lets-plot.

    - ``w`` and ``h`` in px: that many pixels times ``scale`` (default 1); ``dpi`` is
      metadata only.
    - ``w`` and ``h`` in in / cm / mm (or no unit: inches): laid out at 96 px per inch,
      rendered at ``dpi`` (300 by default) times ``scale`` (default 1).
    - Otherwise the plot's own size times ``dpi / 96`` (when ``dpi`` is given, or 300
      for a physical ``unit``) times ``scale`` (2 by default, 1 once ``dpi`` or ``unit``
      is given).
    """
    unit = _unit(unit)
    if dpi is not None and not (math.isfinite(float(dpi)) and float(dpi) > 0):
        message = f"'dpi' must be a positive number, got {dpi}"
        raise ExportError(message)
    if scale is not None and not (math.isfinite(float(scale)) and float(scale) > 0):
        message = f"'scale' must be a positive number, got {scale}"
        raise ExportError(message)
    given = _given_size(w, h, unit)
    if given is not None:
        w, h, unit = given
        factor = 1.0 if scale is None else float(scale)
        if unit == "px":
            return (w, h), factor, SCREEN_DPI if dpi is None else float(dpi)
        inches = INCHES_PER_UNIT[unit]
        resolution = PRINT_DPI if dpi is None else float(dpi)
        size = (w * inches * SCREEN_DPI, h * inches * SCREEN_DPI)
        return size, factor * resolution / SCREEN_DPI, resolution
    resolution = float(dpi) if dpi is not None else PRINT_DPI if unit in INCHES_PER_UNIT else None
    if scale is None:
        scale = DEFAULT_PNG_SCALE if dpi is None and unit is None else 1.0
    density = 1.0 if resolution is None else resolution / SCREEN_DPI
    return None, density * float(scale), SCREEN_DPI if resolution is None else resolution


def _svg_size(svg: str, w: float, h: float, unit: str) -> str:
    """Give the root ``<svg>`` a physical ``width`` / ``height`` (its viewBox keeps pixels)."""
    start = svg.index("<svg")
    end = svg.index(">", start)
    root = svg[start:end]
    for name, value in (("width", w), ("height", h)):
        prefix = f' {name}="'
        position = root.index(prefix) + len(prefix)
        close = root.index('"', position)
        root = root[:position] + f"{value}{unit}" + root[close:]
    return svg[:start] + root + svg[end:]


def _write(target: Target, content: bytes | str) -> str | None:
    if isinstance(target, str | os.PathLike):
        path = os.path.abspath(target)
        directory = os.path.dirname(path)
        if directory:
            os.makedirs(directory, exist_ok=True)
        if isinstance(content, bytes):
            with open(path, "wb") as binary_file:
                binary_file.write(content)
        else:
            with open(path, "w", encoding="utf-8") as text_file:
                text_file.write(content)
        return path
    target.write(content if isinstance(content, bytes) else content.encode())
    return None


def export_svg(
    plot: PlotSpec | SupPlotsSpec,
    path: Target | None = None,
    w: Number | None = None,
    h: Number | None = None,
    unit: str | None = None,
) -> str | None:
    """See ``PlotSpec.to_svg``.

    Parameters
    ----------
    plot : PlotSpec or SupPlotsSpec
        The plot or grid of plots.
    path : str or file-like, optional
        Where to write: a file path or a binary file-like object.
    w : float, optional
        Width.
    h : float, optional
        Height.
    unit : str, optional
        Unit of ``w`` and ``h``: ``"in"``, ``"cm"``, ``"mm"`` or ``"px"``.
    """
    given = _given_size(w, h, _unit(unit))
    if given is None or given[2] == "px":
        size = None if given is None else given[:2]
        svg = render_bytes(plot, "svg", size=size).decode()
    else:
        w, h, unit = given
        inches = INCHES_PER_UNIT[unit]
        svg = render_bytes(
            plot, "svg", size=(w * inches * SCREEN_DPI, h * inches * SCREEN_DPI)
        ).decode()
        svg = _svg_size(svg, w, h, unit)
    return svg if path is None else _write(path, svg)


def export_png(
    plot: PlotSpec | SupPlotsSpec,
    path: Target,
    scale: Number | None = None,
    w: Number | None = None,
    h: Number | None = None,
    unit: str | None = None,
    dpi: Number | None = None,
) -> str | None:
    """See ``PlotSpec.to_png``.

    Parameters
    ----------
    plot : PlotSpec or SupPlotsSpec
        The plot or grid of plots.
    path : str or file-like
        Where to write: a file path or a binary file-like object.
    scale : float, optional
        Multiplier of the pixel size.
    w : float, optional
        Width.
    h : float, optional
        Height.
    unit : str, optional
        Unit of ``w`` and ``h``: ``"in"``, ``"cm"``, ``"mm"`` or ``"px"``.
    dpi : float, optional
        Resolution in dots per inch.
    """
    size, pixel_scale, resolution = _raster_options(w, h, unit, dpi, scale)
    png = render_bytes(plot, "png", scale=pixel_scale, size=size, dpi=resolution)
    return _write(path, png)


def export_pdf(
    plot: PlotSpec | SupPlotsSpec,
    path: Target,
    scale: Number | None = None,
    w: Number | None = None,
    h: Number | None = None,
    unit: str | None = None,
    dpi: Number | None = None,
) -> str | None:
    """See ``PlotSpec.to_pdf``.

    Parameters
    ----------
    plot : PlotSpec or SupPlotsSpec
        The plot or grid of plots.
    path : str or file-like
        Where to write: a file path or a binary file-like object.
    scale : float, optional
        Multiplier of the pixel size.
    w : float, optional
        Width.
    h : float, optional
        Height.
    unit : str, optional
        Unit of ``w`` and ``h``: ``"in"``, ``"cm"``, ``"mm"`` or ``"px"``.
    dpi : float, optional
        Resolution in dots per inch.
    """
    size, pixel_scale, resolution = _raster_options(w, h, unit, dpi, scale)
    pdf = render_bytes(plot, "pdf", scale=pixel_scale, size=size, dpi=resolution)
    return _write(path, pdf)


def export_html(
    plot: PlotSpec | SupPlotsSpec, path: Target | None = None, iframe: bool | None = None
) -> str | None:
    """See ``PlotSpec.to_html``.

    Parameters
    ----------
    plot : PlotSpec or SupPlotsSpec
        The plot or grid of plots.
    path : str or file-like, optional
        Where to write: a file path or a binary file-like object.
    iframe : bool, optional
        Return an ``<iframe srcdoc>`` sized to the plot instead of a page.
    """
    page = render_bytes(plot, "html_iframe" if iframe else "html_page").decode()
    return page if path is None else _write(path, page)


def render_notebook_html(plot: PlotSpec | SupPlotsSpec) -> str:
    """HTML for Jupyter's ``_repr_html_``, in the mode set by ``ggrust.display``.

    Parameters
    ----------
    plot : PlotSpec or SupPlotsSpec
        The plot or grid of plots.
    """
    from ggrust._display import current_display

    mode = current_display()
    if mode == "svg":
        return render_bytes(plot, "svg").decode()
    if mode == "png":
        import base64

        encoded = base64.b64encode(render_bytes(plot, "png")).decode()
        return f'<img src="data:image/png;base64,{encoded}">'
    return render_bytes(plot, "html").decode()


def show(plot: PlotSpec | SupPlotsSpec) -> None:
    """Show a plot: inline in a running IPython/Jupyter session, else in the browser.

    lets_plot prints the spec dict outside IPython; ggrust opens the interactive
    HTML page in the default browser instead (as lets_plot's ``setup_show_ext``).

    Parameters
    ----------
    plot : PlotSpec or SupPlotsSpec
        The plot or grid of plots.
    """
    try:
        from IPython import get_ipython
        from IPython.display import display
    except ImportError:
        get_ipython = None
    if get_ipython is not None and get_ipython() is not None:
        display(plot)
        return
    import tempfile
    import webbrowser

    with tempfile.NamedTemporaryFile(
        "w", suffix=".html", prefix="ggrust-", delete=False, encoding="utf-8"
    ) as page:
        page.write(cast(str, export_html(plot, None, False)))
    webbrowser.open(f"file://{page.name}")
