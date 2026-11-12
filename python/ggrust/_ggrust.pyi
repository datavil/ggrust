"""Type stubs for the native extension module."""

from __future__ import annotations

from collections.abc import Callable
from os import PathLike
from typing import Any, TypedDict

__all__ = [
    "correlation_matrix",
    "dendrogram_segments",
    "density_outline",
    "describe_frame",
    "engine_version",
    "hclust",
    "kernel_density",
    "referenced_columns",
    "register_font",
    "register_font_bytes",
    "register_font_dir",
    "registered_font_files",
    "registered_fonts",
    "render",
    "streamplot",
    "tooltip_columns",
    "validate_spec",
    "velocity_grid",
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

def register_font(path: str | PathLike[str], family: str | None = None) -> list[str]:
    """Register the faces of a font file; return the families registered.

    Raises ``FontNotFoundError`` for a missing file and ``FontError`` for a file
    that is not a usable font.
    """

def register_font_dir(path: str | PathLike[str]) -> tuple[list[str], list[tuple[str, str]]]:
    """Register every font file under a directory.

    Returns the families registered and the skipped files with their reasons.
    Raises ``FontNotFoundError`` and ``FontError`` as ``register_font``.
    """

def registered_fonts() -> list[tuple[str, list[str]]]:
    """Return every registered family with its faces, in registration order."""

def registered_font_files(family: str) -> list[tuple[bytes, bool]]:
    """Return the font files of a registered family, each once.

    Each file comes with whether its license allows embedding. Empty when the
    family is not registered.
    """

def register_font_bytes(data: bytes, family: str) -> list[str]:
    """Register the faces of font file bytes under ``family``.

    Raises ``ValueError`` with the reason if the bytes are not a usable font.
    """

def png_data_url(rgba: bytes, width: int, height: int, compression: int | None = None) -> str:
    """Encode straight-alpha RGBA rows as a ``data:image/png;base64,...`` URL.

    ``compression`` is a zlib level 0..9; ``None`` picks the fastest deflate.
    Raises ``ValueError`` if ``rgba`` does not hold ``width * height * 4`` bytes.
    """

def text_width(text: str, size: float, fontface: str = "plain") -> float:
    """Advance width in pixels of one line of ``text`` at ``size`` px in the default font."""

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
    functions: list[Callable[..., object]] | None = None,
    quality: int | None = None,
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
        ``"svg"``, ``"png"``, ``"jpeg"``, ``"tiff"``, ``"pdf"``, ``"html"`` (notebook
        fragment: root ``<div>`` and the runtime ``<script>``), ``"html_page"`` or
        ``"html_iframe"``.
    scale
        Raster / PDF pixel scale.
    html_id
        Id of the root element of HTML outputs; prefixes every id in the SVG.
        Must be unique on the page (one per display). Required for HTML outputs.
    dpi
        Raster resolution (PNG ``pHYs``, JPEG JFIF density, TIFF ``XResolution``)
        and PDF page resolution; 96 when None.
    functions
        Secondary axis transforms: ``sec_axis.transform == i`` in the spec calls
        ``functions[i]`` once per value.
    quality
        JPEG quality, 1 to 100; 75 (R's ``jpeg()`` default) when None.

    Returns
    -------
    tuple[bytes, list[str]]
        Encoded output (UTF-8 for SVG and HTML; PNG, JPEG, TIFF or PDF bytes) and build warnings.

    Raises
    ------
    ValueError
        If the spec or data is invalid; the message names the offending path.
    """

def inspect(
    spec_json: str,
    frames: list[Any],
    functions: list[Callable[..., object]] | None = None,
) -> tuple[list[Any], list[dict[str, Any]], list[dict[str, Any]], list[str]]:
    """Prepare and lay out a single plot without drawing it (``PlotSpec.build()``).

    Parameters
    ----------
    spec_json
        Plot spec as for ``render``.
    frames
        The referenced frames, each exporting ``__arrow_c_stream__``.
    functions
        Secondary axis transforms, as for ``render``.

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

def hclust(
    data: object, labels: str | None, by_columns: bool, method: str, distance: str
) -> tuple[list[str], list[tuple[int, int]], list[float], list[int]]:
    """Cluster an Arrow table's rows (or numeric columns) as R's ``hclust``.

    Returns ``(labels, merge, height, order)``: merge as R's signed 1-based pairs,
    order 0-based.

    Raises
    ------
    ValueError
        If ``method`` or ``distance`` is unknown.
    ggrust.errors.DataError
        If the data cannot be clustered (non-numeric column, missing value, ...).
    """

def dendrogram_segments(
    labels: list[str], merge: list[list[int]], height: list[float], order: list[int], kind: str
) -> tuple[list[float], list[float], list[float], list[float]]:
    """Links of a tree: ``(position, height, position_end, height_end)`` columns.

    Raises
    ------
    ggrust.errors.SpecError
        If the tree is malformed.
    ValueError
        If ``kind`` is not ``"rectangle"`` or ``"triangle"``.
    """

def streamplot(
    x: list[float],
    y: list[float],
    u: list[list[float]],
    v: list[list[float]],
    density: tuple[float, float],
    minlength: float,
    maxlength: float,
    integration_direction: str,
    broken_streamlines: bool,
    start_points: list[tuple[float, float]] | None = None,
) -> tuple[
    tuple[list[float], list[float], list[int]],
    tuple[list[float], list[float], list[float], list[float], list[int]],
]:
    """Streamlines ``(x, y, group)`` and arrows ``(x, y, xend, yend, group)``.

    Raises
    ------
    ggrust.errors.SpecError
        If the grid, field, density or start points are invalid.
    ValueError
        If ``integration_direction`` is unknown.
    """

def velocity_grid(
    points: object,
    vectors: object,
    density: float,
    smooth: float,
    n_neighbors: int | None,
    min_mass: float,
    cutoff_percentile: float,
    adjust_for_stream: bool,
) -> tuple[list[float], list[float], list[list[float]], list[list[float]]]:
    """Grid ``(x, y, u, v)`` of cell velocities, ``u[row][column]`` at ``(x[column], y[row])``.

    Raises
    ------
    ggrust.errors.SpecError
        If an option is out of range, the lengths differ or a dimension has no spread.
    TypeError
        If ``points`` or ``vectors`` is not a sequence of ``(x, y)`` number pairs.
    """

def density_outline(
    points: object,
    level: float,
    padding: float,
    grid_size: int,
    bandwidth: str | float,
) -> tuple[list[float], list[float], list[int]]:
    """Outline ``(x, y, path)`` vertex columns of a point cloud at ``level`` of its peak density.

    Raises
    ------
    ggrust.errors.SpecError
        If an option is out of range, too few finite points remain or they lie on a line.
    TypeError
        If ``points`` is not a sequence of ``(x, y)`` number pairs.
    """

def compare_means(
    partitions: list[list[list[float]]],
    levels: list[str],
    method: str,
    paired: bool,
    reference: int | None,
    reference_all: bool,
    p_adjust_method: str,
) -> tuple[
    list[int],
    list[int | None],
    list[int],
    list[float],
    list[float],
    list[str],
    list[str],
    str,
]:
    """Rows of ggpubr's ``compare_means()`` table.

    ``(partition, group1, group2, p, p_adj, p_format, p_signif, method)``;
    ``group1`` is None for ``.all.``.

    Raises
    ------
    ggrust.errors.SpecError
        If the method or adjustment is unknown or a test cannot run.
    """

def format_r_numbers(values: list[float], digits: int) -> list[str]:
    """Each number as R's ``format(x, digits)`` prints it."""

def kernel_density(
    values: object,
    at: object,
    bandwidth: str | float,
) -> tuple[list[float], list[float]]:
    """``(at, density)``: the 1D Gaussian kernel density of ``values`` at each of ``at``.

    Raises
    ------
    ggrust.errors.SpecError
        If fewer than 2 finite values remain, they are all equal or ``bandwidth`` is invalid.
    TypeError
        If ``values`` or ``at`` is not a sequence of numbers.
    """
