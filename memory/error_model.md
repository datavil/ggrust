# Error model: structured paths, one class per kind, shared suggestion rule

**Works:** `Error::InvalidSpec { path, message, notes }` with `Error::spec(path, msg)`.
Never bake `at '<path>': ` into the message: nested figures join paths with
`Error::nested(outer)` (`figures[1]` + `layers[0].size`), and Python gets `SpecError.path`.
The only place a located string is parsed is the serde boundary (`Error::spec_located`,
fed by `nest_error_path` in `spec/mod.rs`). Column errors (`MissingColumn`,
`UnsupportedColumnType`) carry `path` too; `DataFrame::with_source_columns` makes
`available` list every source column, since Python exports only referenced columns.

**Mapping:** each engine kind maps to one `ggrust.errors` class in
`crates/ggrust-python/src/lib.rs` (`to_python_error`). Every class subclasses
`GgrustError` and `ValueError` (lets_plot raised ValueError, so `except ValueError`
keeps working); `TypeError` too where the type is the problem. `+` operator errors stay
plain `TypeError`, `ggsize` keeps lets_plot's `AssertionError`.

**Suggestions:** `suggest::did_you_mean` (Rust) and `errors.did_you_mean` (Python) use
the same rule: OSA distance <= len/3 (none for 1-2 letter words, case-only always).
`difflib` misses swaps (`sgv` -> `svg`); do not use it. Put the suggestion at the end of
the message (`...; supported: a, b; did you mean 'a'?`).

**Breaks:** `uv run python script.py` after editing Rust rebuilds the extension first
(cache keys include `crates/**/*.rs`) and fails on a broken build: run one-off scripts
with plain `python3`. ty narrows `isinstance(x, dict)` on unions with `Mapping`/`Sequence`
to `Top[dict]`: use `typing.cast` at those spots (newer ty flags redundant casts).

**How to apply:** new errors use `Error::spec(path, ...)` or a column kind with a path;
messages say what, where, what is accepted and how to fix it; tests assert the class,
the attributes and the key message parts (`tests/test_errors.py`).
