# polars `Null` columns break arrow-rs FFI import

**Broke:** `pl.DataFrame({"empty": [None, None]})` exported via `__arrow_c_stream__`
fails in pyo3-arrow / arrow-rs with
`C Data interface error: The datatype "Null" doesn't expect buffer at index 0`.
polars attaches a buffer that arrow-rs's validation rejects. pyarrow `pa.nulls()` is fine.

**Works:** `ggrust._data.as_arrow` casts polars `pl.Null` columns to `Float64` before
export. The engine reads an all-null Float64 column as all-missing continuous, which is
what a Null column means. Test: `tests/test_data.py::test_all_null_column_is_continuous`.

Also verified: polars Float64 columns cross with zero copies (50M rows: +2 MB RSS, 0.5 ms).
Integer columns are cast to f64 (one copy).
