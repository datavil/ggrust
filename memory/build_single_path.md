# One build path: uv, never `--no-sync` or mixed `maturin develop`

**Broke:** `make test` ran `uv run --no-sync pytest` after `maturin develop`. Editing Rust
and running `make test` alone tested the *old* extension: false green. On a fresh clone
`--no-sync` also picked a global pytest from PATH and failed to import ggrust.

**Works:** let uv own the build. `uv run pytest` rebuilds the extension when any
`[tool.uv] cache-keys` file changes (Rust sources, Cargo.toml, Cargo.lock).
`make develop` = `uv sync --reinstall-package ggrust` to force it.

Also: cargo (clippy/test) must see the project interpreter, so the Makefile exports
`PYO3_PYTHON=.venv/bin/python` (abi3-py311 fails against an older system python3).
Verified by sabotaging `engine_version()` and confirming `make test` fails.

**Parallel worktrees (2026-11-13):** `~/.cargo/config.toml` points every checkout at one
shared target dir. Two sessions building at once can leave another worktree's cdylib in
your `python/ggrust/_ggrust.abi3.so` (ImportError for a pyfunction you just added, or
another branch's engine under test). Build with `CARGO_TARGET_DIR=$PWD/target` (ignored)
when other sessions run.
