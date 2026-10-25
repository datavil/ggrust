"""Turn user data into an Arrow stream the Rust engine can read without copying."""

import sys
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
        A polars DataFrame or LazyFrame, pandas DataFrame, pyarrow Table, any table
        object exporting `__arrow_c_stream__`, or a mapping of column names to
        sequences / numpy arrays / Series.

    Returns
    -------
    ArrowStreamExportable
        `data` itself when it already exports Arrow, otherwise a polars DataFrame.

    Raises
    ------
    TypeError
        If `data` is a Series or none of the accepted kinds.

    Notes
    -----
    The pandas index is dropped, as lets-plot ignores it.
    """
    if isinstance(data, pl.LazyFrame):
        data = data.collect()
    if isinstance(data, Mapping):
        # strict=False: mixed ints/floats/None in one list (very common) are upcast
        # instead of rejected, as lets_plot accepts them.
        data = pl.DataFrame(dict(data), strict=False)
    if isinstance(data, pl.DataFrame):
        return _without_null_dtype(data)
    if isinstance(data, pl.Series) or _is_pandas(data, "Series"):
        message = "got a Series; pass a DataFrame instead (for example `series.to_frame()`)"
        raise TypeError(message)
    if _is_pandas(data, "DataFrame"):
        return data.reset_index(drop=True)
    if hasattr(data, "__arrow_c_stream__"):
        return data
    message = (
        f"unsupported data type {type(data).__name__}; expected a polars or pandas DataFrame, "
        "a pyarrow Table, an object exporting __arrow_c_stream__, or a dict of columns"
    )
    raise TypeError(message)


def _is_pandas(data: Any, class_name: str) -> bool:
    """Check for a pandas type without importing pandas."""
    pandas = sys.modules.get("pandas")
    return pandas is not None and isinstance(data, getattr(pandas, class_name))


def _without_null_dtype(frame: pl.DataFrame) -> pl.DataFrame:
    """Replace `pl.Null` (also inside List / Array / Struct) with Float64.

    polars exports `Null` with a data buffer that arrow-rs rejects ("datatype Null
    doesn't expect buffer"), which fails the whole frame. An all-null Float64 column
    means the same thing to the engine: every value is missing.
    """
    casts = {
        name: replaced
        for name, dtype in frame.schema.items()
        if (replaced := _replace_null(dtype)) != dtype
    }
    if not casts:
        return frame
    return frame.with_columns(pl.col(name).cast(dtype) for name, dtype in casts.items())


def _replace_null(dtype: pl.DataType) -> pl.DataType:
    """Return `dtype` with every nested `pl.Null` replaced by `pl.Float64`."""
    if dtype == pl.Null:
        return pl.Float64()
    if isinstance(dtype, pl.List):
        return pl.List(_replace_null(dtype.inner))
    if isinstance(dtype, pl.Array):
        return pl.Array(_replace_null(dtype.inner), dtype.shape)
    if isinstance(dtype, pl.Struct):
        return pl.Struct(
            [pl.Field(field.name, _replace_null(field.dtype)) for field in dtype.fields]
        )
    return dtype
