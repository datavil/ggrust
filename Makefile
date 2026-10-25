.PHONY: develop format lint test check

develop:  ## build and install the extension into .venv (release mode)
	uv run maturin develop --release --uv

format:
	cargo fmt --all
	uv run ruff format .
	uv run ruff check --fix .

lint:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	uv run ruff format --check .
	uv run ruff check .

test:
	cargo test --workspace
	uv run --no-sync pytest

check: lint develop test  ## everything an agent must run before committing
