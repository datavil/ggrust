# ggrust

Grammar-of-graphics plotting engine written in Rust, used from Python through a
lets_plot-compatible API. Built for [cellestial](https://github.com/datavil/cellestial).

Status: early development. See [plan.md](https://github.com/datavil/ggrust/blob/main/plan.md) for the roadmap and
[AGENTS.md](https://github.com/datavil/ggrust/blob/main/AGENTS.md) for contributor rules.

## Development

```bash
uv sync          # create the dev environment
make develop     # build the Rust extension into .venv
make check       # fmt, clippy, ruff, cargo test, pytest
```
