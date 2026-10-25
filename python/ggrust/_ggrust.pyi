"""Type stubs for the native extension module."""

from typing import Any, TypedDict

__all__ = ["describe_frame", "engine_version", "validate_spec"]

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
