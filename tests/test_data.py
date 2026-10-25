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
    error = describe(frame)["when"]["error"]
    assert error.startswith("column 'when' has unsupported type Timestamp")


def test_unsupported_input_type():
    with pytest.raises(TypeError, match="unsupported data type list"):
        as_arrow([1, 2, 3])


def test_polars_float_column_is_zero_copy():
    values = np.arange(1_000_000, dtype=np.float64)
    frame = pl.DataFrame({"x": values})
    assert describe(frame)["x"]["length"] == 1_000_000


# Regressions found by the Phase 0.3 bug hunt.


def test_string_column_over_two_gibibytes_does_not_panic():
    # Views share one buffer: tiny in memory, but > 2 GiB if cast to i32-offset Utf8.
    frame = pl.select(pl.repeat("x" * 1000, 2_200_000).alias("s"))
    summary = describe(frame)["s"]
    assert summary["levels"] == ["x" * 1000]
    assert summary["length"] == 2_200_000


def test_chunked_pandas_categoricals_with_different_categories():
    first = pd.DataFrame({"c": pd.Categorical([f"a{index}" for index in range(100)])})
    second = pd.DataFrame({"c": pd.Categorical([f"b{index}" for index in range(100)])})
    table = pa.concat_tables([pa.Table.from_pandas(first), pa.Table.from_pandas(second)])
    summary = describe(table)["c"]
    assert len(summary["levels"]) == 200
    assert summary["levels"][:2] == ["a0", "a1"]


def test_duplicate_column_names_are_reported():
    table = pa.Table.from_arrays([pa.array([1.0]), pa.array(["a"])], names=["x", "x"])
    summaries = describe_frame(table)
    assert all(summary["kind"] == "unsupported" for summary in summaries)
    assert "appears 2 times" in summaries[0]["error"]


@pytest.mark.parametrize(
    "data",
    [
        {"x": [None, None]},
        {"x": pl.Series([None, None])},
        pl.DataFrame({"x": [1, 2], "nested": pl.Series([[None], [None]])}),
        pl.DataFrame({"x": [1, 2], "record": [{"a": None}, {"a": None}]}),
    ],
    ids=["dict", "dict-series", "list-of-null", "struct-with-null"],
)
def test_null_dtypes_anywhere_do_not_break_import(data):
    summaries = describe_frame(as_arrow(data))
    assert summaries[0]["kind"] == "continuous"


def test_unplotted_nested_column_is_reported_not_raised():
    frame = pl.DataFrame({"x": [1.0], "nested": [[1, 2]]})
    summary = describe(frame)
    assert summary["x"]["kind"] == "continuous"
    assert summary["nested"]["kind"] == "unsupported"
    assert "column 'nested' has unsupported type" in summary["nested"]["error"]


@pytest.mark.parametrize("series", [pl.Series("x", [1.0]), pd.Series([1.0], name="x")])
def test_series_are_rejected_with_type_error(series):
    with pytest.raises(TypeError, match="got a Series"):
        as_arrow(series)


def test_pandas_index_is_dropped():
    frame = pd.DataFrame({"x": [1.0, 2.0], "key": ["a", "b"]}).set_index("key")
    assert list(describe(frame)) == ["x"]
    named = pd.DataFrame({"x": [1.0]}, index=pd.Index([5], name="idx"))
    assert list(describe(named)) == ["x"]
