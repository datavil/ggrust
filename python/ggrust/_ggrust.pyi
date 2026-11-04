"""Type stubs for the native extension module."""

from __future__ import annotations

from typing import Any, TypedDict

__all__ = [
    "correlation_matrix",
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
    """Return the version of the Rust engine.

    Returns
    -------
    str
        The engine version, e.g. ``"0.4.0"``.

    Examples
    --------
    >>> from ggrust import engine_version
    >>> version = engine_version()
    """

def png_data_url(rgba: bytes, width: int, height: int, compression: int | None = None) -> str:
    """Encode straight-alpha RGBA rows as a ``data:image/png;base64,...`` URL.

    ``compression`` is a zlib level 0..9; ``None`` picks the fastest deflate.
    Raises ``ValueError`` if ``rgba`` does not hold ``width * height * 4`` bytes.
    """

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

def correlation_matrix(data: object) -> tuple[list[str], list[float], bool]:
    """Pearson correlations between the numeric columns of an Arrow table.

    Parameters
    ----------
    data
        Any object exporting ``__arrow_c_stream__`` or ``__arrow_c_array__``.

    Returns
    -------
    tuple[list[str], list[float], bool]
        The variable names, the row-major ``n x n`` correlations (pairwise complete
        rows; NaN with fewer than 2 rows or no variance; 1 on the diagonal), and
        whether the table already was a correlation matrix (square, float columns,
        every value in -1..1 or NaN) and was returned as it is.
    """

def describe_frame(data: object) -> list[ColumnSummary]:
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

def inspect(spec_json: str, frames: list[Any]) -> tuple[list[Any], list[dict[str, Any]], list[str]]:
    """Prepare and lay out a single plot without drawing it (``PlotSpec.build()``).

    Parameters
    ----------
    spec_json
        Plot spec as for ``render``.
    frames
        The referenced frames, each exporting ``__arrow_c_stream__``.

    Returns
    -------
    tuple[list[Any], list[dict[str, Any]], list[str]]
        One Arrow table per layer (``__arrow_c_stream__``; data after stat and
        position, with ``panel`` and ``group`` columns), one dict per panel with its
        ``x`` and ``y`` scales (``discrete``, ``domain``, ``breaks``, ``labels``), and
        the build warnings.

    Raises
    ------
    ValueError
        If the spec or data is invalid, or the spec is a grid.
    """
