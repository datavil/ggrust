# CI must run exactly what `make check` runs, with the same tools

On 2026-11-14 most pushes to `main` failed CI or Docs while `make check` passed
locally. Two independent causes, and the earlier fixes only patched symptoms
(a clippy fix here, a regenerated reference page there), so the next push broke again.

1. **Unpinned Rust.** CI used `dtolnay/rust-toolchain@stable` (rustc 1.100, released
   2026-11-09); local `stable` was 1.97. Clippy runs with `-D warnings`, so each new
   Rust release adds lints that fail only in CI (`needless_late_init` in `build.rs`).
   Fix: `rust-toolchain.toml` pins one version for local, CI, docs and wheel builds.
   Workflows install it with `rustup toolchain install` (no version in the YAML).
   Bump it on purpose: edit the channel, `make check`, fix the new lints, commit.
2. **A CI-only check.** The Docs workflow failed when `docs/reference/` was stale
   (a docstring or example changed without `docs/build_reference.py`), but
   `make check` never ran that check. Fix: `make check-reference`
   (`build_reference.py --check`, writes nothing) is part of `make check`.

Rule: any check a workflow runs must also be in `make check`, and every tool version
comes from a lock or pin file (`uv.lock`, `rust-toolchain.toml`), never `@stable` or
`latest`. A remaining human failure mode: pushing a "docs only" commit without
`make check`; ruff formats Python blocks inside markdown, so docs commits can fail
lint too (see [check_before_commit.md](check_before_commit.md)).
