"""Column type names for ``data_meta.series_annotations``.

Ported from lets-plot's ``lets_plot/plot/series_meta.py`` (MIT, see
THIRD_PARTY_NOTICES.md). The Rust engine reads types from Arrow; these annotations
keep ``as_dict()`` identical to lets_plot.
"""

import sys
from collections.abc import Iterable
from datetime import date, datetime, time

TYPE_INTEGER = "int"
TYPE_FLOATING = "float"
TYPE_STRING = "str"
TYPE_BOOLEAN = "bool"
TYPE_DATE_TIME = "datetime"
TYPE_DATE = "date"
TYPE_TIME = "time"
TYPE_UNKNOWN = "unknown"


def is_pandas_data_frame(data) -> bool:
    """Whether ``data`` is a pandas DataFrame (without importing pandas)."""
    pandas = sys.modules.get("pandas")
    return pandas is not None and isinstance(data, pandas.DataFrame)


def is_polars_data_frame(data) -> bool:
    """Whether ``data`` is a polars DataFrame (without importing polars)."""
    polars = sys.modules.get("polars")
    return polars is not None and isinstance(data, polars.DataFrame)


def infer_types(data) -> dict:
    """Map each column of ``data`` to a lets-plot type name."""
    if isinstance(data, dict):
        return {name: _infer_dict_type(content) for name, content in data.items()}
    if is_pandas_data_frame(data):
        return {name: _infer_pandas_type(content) for name, content in data.items()}
    if is_polars_data_frame(data):
        return {name: _infer_polars_type(dtype) for name, dtype in data.schema.items()}
    return {}


def _infer_pandas_type(content) -> str:
    import numpy
    import pandas

    if content.empty or content.isna().all():
        return TYPE_UNKNOWN
    inferred = pandas.api.types.infer_dtype(content.values, skipna=True)
    if inferred == "categorical":
        dtype = content.cat.categories.dtype
        if isinstance(dtype, pandas.StringDtype):
            return TYPE_STRING
        if numpy.issubdtype(dtype, numpy.integer):
            return TYPE_INTEGER
        if numpy.issubdtype(dtype, numpy.floating):
            return TYPE_FLOATING
        if numpy.issubdtype(dtype, numpy.object_) and all(
            isinstance(category, str) for category in content.cat.categories
        ):
            return TYPE_STRING
        return TYPE_UNKNOWN
    simple = {
        "string": TYPE_STRING,
        "floating": TYPE_FLOATING,
        "integer": TYPE_INTEGER,
        "boolean": TYPE_BOOLEAN,
        "datetime64": TYPE_DATE_TIME,
        "datetime": TYPE_DATE_TIME,
        "date": TYPE_DATE,
        "time": TYPE_TIME,
        "empty": TYPE_UNKNOWN,
    }
    return simple.get(inferred, f"unknown(pandas:{inferred})")


def _infer_polars_type(dtype) -> str:
    import polars

    if isinstance(dtype, polars.Enum | polars.Categorical):
        return TYPE_STRING
    if dtype in polars.datatypes.group.FLOAT_DTYPES:
        return TYPE_FLOATING
    if dtype in polars.datatypes.group.INTEGER_DTYPES:
        return TYPE_INTEGER
    if dtype == polars.String:
        return TYPE_STRING
    if dtype == polars.Boolean:
        return TYPE_BOOLEAN
    if dtype == polars.Datetime:
        return TYPE_DATE_TIME
    if dtype == polars.Date:
        return TYPE_DATE
    if dtype == polars.Time:
        return TYPE_TIME
    return f"unknown(polars:{dtype})"


def _infer_dict_type(content) -> str:
    if not isinstance(content, Iterable) or not any(True for _ in content):
        return TYPE_UNKNOWN
    types = {type(value) for value in content}
    types.discard(type(None))
    if not types:
        return TYPE_UNKNOWN
    if len(types) > 1:
        if all(issubclass(kind, int | float) for kind in types):
            return TYPE_FLOATING
        return "unknown(mixed types)"
    kind = types.pop()
    numpy = sys.modules.get("numpy")
    if kind is bool:
        return TYPE_BOOLEAN
    if issubclass(kind, int):
        return TYPE_INTEGER
    if issubclass(kind, float):
        return TYPE_FLOATING
    if issubclass(kind, str):
        return TYPE_STRING
    if issubclass(kind, datetime):
        return TYPE_DATE_TIME
    if issubclass(kind, date):
        return TYPE_DATE
    if issubclass(kind, time):
        return TYPE_TIME
    if numpy is not None:
        if issubclass(kind, numpy.datetime64):
            return TYPE_DATE_TIME
        if issubclass(kind, numpy.timedelta64):
            return f"unknown(python:{kind})"
        if issubclass(kind, numpy.integer):
            return TYPE_INTEGER
        if issubclass(kind, numpy.floating):
            return TYPE_FLOATING
    return f"unknown(python:{kind})"


def detect_time_zone(name: str, data) -> str | None:
    """Time zone of a datetime column, if any."""
    if is_pandas_data_frame(data):
        if name in data:
            content = data[name]
            if hasattr(content, "dt") and getattr(content.dt, "tz", None) is not None:
                return str(content.dt.tz)
    elif is_polars_data_frame(data):
        if name in data.columns:
            time_zone = getattr(data[name].dtype, "time_zone", None)
            if time_zone is not None:
                return str(time_zone)
    elif isinstance(data, dict) and name in data:
        content = data[name]
        if isinstance(content, Iterable):
            for value in content:
                if isinstance(value, datetime) and value.tzinfo is not None:
                    return str(value.tzinfo)
    return None
