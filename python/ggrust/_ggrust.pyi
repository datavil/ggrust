"""Type stubs for the native extension module."""

from typing import Any, TypedDict

__all__ = [
    "describe_frame",
    "engine_version",
    "referenced_columns",
    "render",
    "tooltip_columns",
    "validate_spec",
]

class ColumnSummary(TypedDict, total=False):
    """How ggrust reads one column."""

    name: str
    kind: str
    length: int
    null_count: int
    levels: list[str]
    error: str

def engine_version() -> str:
    """Return the version of the Rust engine."""

def validate_spec(spec_json: str) -> list[str]:
    """Parse a plot spec and return the paths of unsupported options.

    Parameters
    ----------
    spec_json
        JSON of a lets_plot-style `as_dict()` spec, without inline data.

    Returns
    -------
    list[str]
        Dotted paths of options ggrust does not support, e.g. ``"layers[0].sampling"``.

    Raises
    ------
    ValueError
        If the spec is invalid; the message names the offending path.
    """

def describe_frame(data: Any) -> list[ColumnSummary]:
    """Describe how ggrust reads each column of an Arrow table (debugging aid).

    Parameters
    ----------
    data
        Any object exporting `__arrow_c_stream__` or `__arrow_c_array__`.

    Returns
    -------
    list[ColumnSummary]
        One summary per column, in column order. Columns ggrust cannot plot have
        kind ``"unsupported"`` and an ``error`` message.
    """

def referenced_columns(spec_json: str) -> list[str]:
    """Return the column names a spec can read, sorted.

    Parameters
    ----------
    spec_json
        Plot spec (lets_plot ``as_dict()`` with frame references) as JSON.

    Returns
    -------
    list[str]
        Columns referenced by mappings, ``as_discrete(order_by=...)`` and facets
        (nested figures included). Only these, plus ``tooltip_columns`` for HTML
        output, need to be exported.

    Raises
    ------
    ValueError
        If the spec is invalid; the message names the offending path.
    """

def tooltip_columns(spec_json: str) -> list[str]:
    """Return the data columns ``layer_tooltips()`` reads, sorted (HTML output only).

    Parameters
    ----------
    spec_json
        Plot spec (lets_plot ``as_dict()`` with frame references) as JSON.

    Returns
    -------
    list[str]
        Variables of tooltip lines, titles and ``variables`` lists; stat variables
        (``@..count..``) are not columns and are left out.

    Raises
    ------
    ValueError
        If the spec is invalid; the message names the offending path.
    """

def render(
    spec_json: str,
    frames: list[Any],
    output: str,
    scale: float,
    html_id: str | None = None,
    dpi: float | None = None,
) -> tuple[bytes, list[str]]:
    """Build and render a plot.

    Parameters
    ----------
    spec_json
        Plot spec (lets_plot ``as_dict()``) as JSON, each ``data`` replaced by a
        frame reference ``{"frame": i}``.
    frames
        The referenced frames, each exporting ``__arrow_c_stream__``.
    output
        ``"svg"``, ``"png"``, ``"pdf"``, ``"html"`` (notebook fragment: root ``<div>``
        and the runtime ``<script>``), ``"html_page"`` or ``"html_iframe"``.
    scale
        PNG / PDF pixel scale.
    html_id
        Id of the root element of HTML outputs; prefixes every id in the SVG.
        Must be unique on the page (one per display). Required for HTML outputs.
    dpi
        PNG resolution (``pHYs``) and PDF page resolution; 96 when None.

    Returns
    -------
    tuple[bytes, list[str]]
        Encoded output (UTF-8 for SVG and HTML; PNG or PDF bytes) and build warnings.

    Raises
    ------
    ValueError
        If the spec or data is invalid; the message names the offending path.
    """
