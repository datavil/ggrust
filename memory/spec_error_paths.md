# serde loses error paths through tagged enums and buffering

**Broke:** `#[serde(tag = "kind")]` on `Figure` made every error read `at '.'`: serde
buffers the whole map to find the tag, and `serde_path_to_error` cannot see inside
buffered content. The same happens with `#[serde(untagged)]` and inside `flatten` maps.

**Works:** parse to `serde_json::Value`, dispatch on `kind` by hand, then run
`serde_path_to_error::deserialize` on the typed struct. Nested figures join paths
(`nest_error_path` in `spec/mod.rs`). Known fields of structs with `flatten` keep their
paths; only the flattened leftovers lose them.

**How to apply:** don't put untagged or internally tagged enums above deep structures.
Use them only for small leaves (`PositionSpec`, `GuideSpec`, `TooltipsSpec`). Add a test
asserting the path for any new nesting level.
