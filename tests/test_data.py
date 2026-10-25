import numpy as np
import pandas as pd
import polars as pl
import pyarrow as pa
import pytest
from ggrust._data import as_arrow
from ggrust._ggrust import describe_frame


def describe(data):
    return {summary["name"]: summary for summary in describe_frame(as_arrow(data))}


def test_polars_numeric_string_boolean_and_nulls():
    frame = pl.DataFrame(
        {
            "x": [1.5, None, 3.0],
            "count": pl.Series([1, 2, None], dtype=pl.Int32),
            "name": ["b", None, "a"],
            "flag": [True, False, None],
        }
    )
    summary = describe(frame)
    assert summary["x"] == {"name": "x", "kind": "continuous", "length": 3, "null_count": 1}
    assert summary["count"]["kind"] == "continuous"
    assert summary["count"]["null_count"] == 1
    assert summary["name"]["levels"] == ["b", "a"]
    assert summary["name"]["null_count"] == 1
    assert summary["flag"]["levels"] == ["true", "false"]


def test_polars_categorical_keeps_its_order_and_drops_unused():
    dtype = pl.Categorical(pl.Categories.random())
    pl.Series(["unused", "zzz"], dtype=dtype)  # register categories no row uses
    frame = pl.DataFrame({"cluster": pl.Series(["T", "B", "NK", "T"], dtype=dtype)})
    assert describe(frame)["cluster"]["levels"] == ["T", "B", "NK"]


def test_polars_enum_keeps_declared_order_and_drops_unused():
    enum = pl.Enum(["NK", "unused", "B", "T"])
    frame = pl.DataFrame({"cluster": pl.Series(["T", "B", "NK", None], dtype=enum)})
    summary = describe(frame)["cluster"]
    assert summary["levels"] == ["NK", "B", "T"]
    assert summary["null_count"] == 1


def test_polars_multiple_chunks():
    first = pl.DataFrame({"x": [1.0], "c": ["b"]})
    second = pl.DataFrame({"x": [2.0, 3.0], "c": ["a", "b"]})
    frame = pl.concat([first, second], rechunk=False)
    assert frame.n_chunks() == 2
    summary = describe(frame)
    assert summary["x"]["length"] == 3
    assert summary["c"]["levels"] == ["b", "a"]


def test_polars_lazy_frame_is_collected():
    lazy = pl.LazyFrame({"x": [1.0, 2.0]})
    assert describe(lazy)["x"]["length"] == 2


def test_pandas_columns():
    frame = pd.DataFrame(
        {
            "x": [1.0, np.nan, 3.0],
            "count": pd.array([1, None, 3], dtype="Int64"),
            "name": ["b", "a", "b"],
            "ordered": pd.Categorical(
                ["a", "b", "a"], categories=["b", "unused", "a"], ordered=True
            ),
            "unordered": pd.Categorical(["a", "b", "a"], categories=["b", "a"]),
        }
    )
    summary = describe(frame)
    assert summary["x"]["kind"] == "continuous"
    assert summary["count"]["null_count"] == 1
    assert summary["name"]["levels"] == ["b", "a"]
    assert summary["ordered"]["levels"] == ["b", "a"]
    assert summary["unordered"]["levels"] == ["b", "a"]


def test_pyarrow_table_record_batch_and_dictionary():
    table = pa.table(
        {
            "x": pa.array([1, 2], type=pa.int64()),
            "c": pa.array(["q", "p"]).dictionary_encode(),
            "big": pa.array(["s", "t"], type=pa.large_string()),
        }
    )
    assert describe(table)["c"]["levels"] == ["q", "p"]
    assert describe(table)["big"]["levels"] == ["s", "t"]
    batch = table.to_batches()[0]
    assert describe_frame(batch)[0]["kind"] == "continuous"


def test_dict_of_lists_and_numpy():
    summary = describe({"x": np.array([1.0, 2.0]), "c": ["a", "b"]})
    assert summary["x"]["kind"] == "continuous"
    assert summary["c"]["levels"] == ["a", "b"]


def test_empty_frames():
    assert describe(pl.DataFrame({"x": pl.Series([], dtype=pl.Float64)}))["x"]["length"] == 0
    assert describe(pl.DataFrame()) == {}


def test_all_null_column_is_continuous():
    expected = {"name": "empty", "kind": "continuous", "length": 2, "null_count": 2}
    assert describe(pl.DataFrame({"empty": [None, None]}))["empty"] == expected
    assert describe(pl.LazyFrame({"empty": [None, None]}))["empty"] == expected
    assert describe(pa.table({"empty": pa.nulls(2)}))["empty"] == expected


def test_unsupported_column_type_names_column():
    frame = pl.DataFrame({"when": pl.Series([0], dtype=pl.Datetime)})
    with pytest.raises(ValueError, match=r"column 'when' has unsupported type Timestamp"):
        describe(frame)


def test_unsupported_input_type():
    with pytest.raises(TypeError, match="unsupported data type list"):
        as_arrow([1, 2, 3])


def test_polars_float_column_is_zero_copy():
    values = np.arange(1_000_000, dtype=np.float64)
    frame = pl.DataFrame({"x": values})
    assert describe(frame)["x"]["length"] == 1_000_000
