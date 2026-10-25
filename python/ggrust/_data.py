"""Turn user data into an Arrow stream the Rust engine can read without copying."""

from collections.abc import Mapping
from typing import Any, Protocol

import polars as pl


class ArrowStreamExportable(Protocol):
    """Any object implementing the Arrow PyCapsule stream interface."""

    def __arrow_c_stream__(self, requested_schema: object | None = None) -> object:
        """Export the data as an Arrow C stream capsule."""
        ...


def as_arrow(data: Any) -> ArrowStreamExportable:
    """Return `data` as an object exporting `__arrow_c_stream__`.

    Parameters
    ----------
    data
        A polars DataFrame, pyarrow Table, pandas DataFrame, any object exporting
        `__arrow_c_stream__`, or a mapping of column names to sequences / numpy arrays.

    Returns
    -------
    ArrowStreamExportable
        `data` itself when it already exports Arrow, otherwise a polars DataFrame.

    Raises
    ------
    TypeError
        If `data` is none of the accepted kinds.
    """
    if isinstance(data, pl.LazyFrame):
        data = data.collect()
    if isinstance(data, pl.DataFrame):
        return _without_null_dtype(data)
    if hasattr(data, "__arrow_c_stream__"):
        return data
    if isinstance(data, Mapping):
        return pl.DataFrame(dict(data))
    message = (
        f"unsupported data type {type(data).__name__}; expected a polars or pandas DataFrame, "
        "a pyarrow Table, an object exporting __arrow_c_stream__, or a dict of columns"
    )
    raise TypeError(message)


def _without_null_dtype(frame: pl.DataFrame) -> pl.DataFrame:
    """Cast all-null (`pl.Null`) columns to Float64.

    polars exports `Null` columns with a data buffer that arrow-rs rejects
    ("datatype Null doesn't expect buffer"). An all-null Float64 column means the
    same thing to the engine: every value is missing.
    """
    null_columns = [name for name, dtype in frame.schema.items() if dtype == pl.Null]
    if not null_columns:
        return frame
    return frame.with_columns(pl.col(null_columns).cast(pl.Float64))
