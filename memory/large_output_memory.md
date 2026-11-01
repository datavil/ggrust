# Large outputs: one buffer, exact reserves, encoded tooltip texts

Step 6.1i (1M-point SVG 452 -> 141 MB, HTML 490 -> 296 MB peak RSS, bytes unchanged).

- **Exact reserve + one more byte = 2x.** `String::reserve(n)` then a closing `</g>` grows
  capacity by doubling: a 70 MB SVG held 141 MB. Reserve the sum plus slack.
- **Never `format!("{big}")` or escape-copy a payload.** HTML is written into one `Vec<u8>`;
  JSON goes through `tooltip::write_script_json` (a serde_json `Formatter` escaping string
  fragments, identical to `script_safe_json`).
- **Per-row `String`s dominate HTML memory.** `TextColumn` stores codes + concatenated
  distinct texts (its JSON was already dictionary-encoded).
- Measure RSS, not just heap: a counting allocator (needs `unsafe`, forbidden in the
  workspace, so build it as a scratch crate depending on ggrust-core by path) finds where;
  `/usr/bin/time -l` and `profiling.py` rss_mb confirm. macOS RSS ignores untouched capacity.
- Byte-identity check: build the same probe against a clean worktree of HEAD and `cmp`.
