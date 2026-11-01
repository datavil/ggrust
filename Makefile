# Single build path: uv builds and installs the extension (maturin backend).
# `uv run` rebuilds automatically when Rust sources change (see [tool.uv] cache-keys).
.PHONY: venv develop format lint test check docs docs-serve

# cargo (clippy/test) must use the project interpreter, not the system python3.
export PYO3_PYTHON := $(CURDIR)/.venv/bin/python

venv:
	uv sync

develop: venv  ## force a rebuild + reinstall of the extension
	uv sync --reinstall-package ggrust

format: venv
	cargo fmt --all
	uv run ruff format .
	uv run ruff check --fix .

lint: venv
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	uv run ruff format --check .
	uv run ruff check .
	uv run ty check python/

test: venv
	cargo test --workspace
	uv run pytest

check:  ## everything an agent must run before committing (sequential, -j safe)
	$(MAKE) lint
	$(MAKE) test

docs: venv  ## render the example plots and build the site into site/
	uv run --group docs python docs/build_examples.py
	uv run --group docs zensical build --strict

docs-serve: venv  ## render the example plots and serve the site with live reload
	uv run --group docs python docs/build_examples.py
	uv run --group docs zensical serve
