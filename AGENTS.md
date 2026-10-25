# AGENTS.md

Read this file fully before touching the code. It is the contract for every agent
(human or AI) working on **ggrust**. The roadmap lives in [plan.md](plan.md).
Lessons learned live in [memory/](memory/README.md).

## Vision

ggrust is a grammar-of-graphics plotting engine **written in Rust, used from Python**.

- It exists so [cellestial](https://github.com/datavil/cellestial) can keep its
  lets-plot based API after lets-plot stopped being maintained (Kotlin notebook sunset).
- The Python API is a **lets_plot-compatible subset**: `aes`, `ggplot`, `geom_*`,
  `scale_*`, `theme`, `gggrid`, `ggtb`, `PlotSpec.as_dict()`, `+` composition.
  Porting cellestial should be (almost) an import swap: `lets_plot` -> `ggrust`.
- Only what cellestial needs (and will plausibly need) is implemented. Not all of lets-plot.
- Rust does all the heavy work (stats, scales, layout, rendering). Python is a thin
  spec builder. Data crosses the boundary once, as Arrow, zero-copy where possible.
- Outputs: SVG, PNG, interactive HTML (notebook `_repr_html_`). Interactivity
  (tooltips, zoom/pan) is a small embedded vanilla-JS runtime over Rust-rendered output.
  Dense point layers are drawn on a canvas from binary buffers.
- Performance target: match and exceed lets-plot on equal work. Memory usage matters too.
- The library is maintained by agents. Optimise for agent-friendliness (see below).

## Golden rules

1. **Test everything before handing over. Not just smoke tests.** You have computer
   access: run `cargo test`, `pytest`, render the plots, and look at them.
2. **Verify visuals.** Render PNGs and inspect them (Read the image). Compare against
   lets-plot output for the same spec where possible. Golden-image tests guard regressions.
3. **Be surgical.** Fix or implement exactly what was requested. If you must touch
   anything outside the requested part, **ask the user first. Do not fix it directly.**
4. **Simple beats clever.** Modular, re-purposable, no overengineering. 20/80 rule:
   do the 20 percent of work that delivers 80 percent of the value first.
5. **Ask the user** when unsure or stuck on a choice. Do not guess on design decisions.
6. **Bug-hunting sub agents.** After implementing a step, spin up sub agents whose
   duty is to assume the code is faulty, find the bug, and report back. Fix what they
   confirm (surgically). Re-verify.
7. **Record critical lessons** (an approach that works, or one that breaks things) as
   a file in `memory/` and add it to the index in [memory/README.md](memory/README.md).
8. **Never cheat benchmarks.** Same data, same work, same output size on both sides.
   Note lets-plot's silent default sampling (see memory index). Report honestly.
9. **Commit each significant step on `main`.** Run `cargo fmt --all` (and `ruff format`,
   `ruff check --fix` for Python) before every commit. Tests must pass at each commit.
10. Mindset of an industry lead: elegant yet robust. Robustness over flexibility.

## Naming and style

- Polars-style naming: full words, no abbreviations (`group_index`, not `grp_idx`;
  `frame`, not `df`). Well-known terms are fine (`n`, `x`, `y`, `svg`, `rgb`, `kde`, `dpi`).
  Exception: user-facing lets_plot-compatible names stay as lets_plot spells them
  (`aes`, `geom_point`, `ggsize`), because compatibility is the point.
- Rust: `cargo fmt`, `cargo clippy -- -D warnings`. Errors via `thiserror`, no `unwrap()`
  on user-controlled input. Functions return one type (use `Result`/enums, not shape-shifting).
- Python: ruff formatting and import sorting, NumPy docstrings, type hints, polars over pandas.
  Functions return a single type.
- No em dashes in code, docs, or comments.

## Architecture (keep it this way)

```
ggrust/
  crates/ggrust-core/      pure Rust engine, no Python. Testable with `cargo test`.
    src/spec/              typed plot spec (serde) parsed from the Python dict
    src/data/              Arrow-backed columns, aesthetic mapping
    src/stat/              one file per stat     (trait Stat)
    src/geom/              one file per geom     (trait Geom)
    src/position/          one file per position (trait Position)
    src/scale/             continuous / discrete / color scales, transforms, breaks
    src/coord/             cartesian, fixed, flip
    src/theme/             theme model + defaults (classic, bw, void, grey)
    src/layout/            panels, facets, axes, legends, titles, subplot grid
    src/scene/             backend-agnostic drawing primitives in pixel space
    src/render/            scene -> svg | png (tiny-skia) | html (+ interaction index)
    src/text/              font loading + text measurement (embedded font)
  crates/ggrust-python/    PyO3 bindings only. Thin: convert, call core, return.
  python/ggrust/           lets_plot-compatible spec API (pure Python, thin)
  js/                      tiny interactive runtime, embedded with include_str!
  tests/                   pytest (API, rendering, visual golden images, cellestial port)
  benchmarks/              ggrust vs lets-plot, criterion benches for the core
  memory/                  lessons learned (indexed)
```

Pipeline (mirrors ggplot2 / lets-plot):

```
Python spec dict + Arrow tables
  -> spec::PlotSpec (typed, validated, clear errors)
  -> per layer: map aesthetics -> stat -> position -> train scales
  -> per panel: coord transform -> geom draws into Scene
  -> layout: axes, legends, titles, facets, subplot grid
  -> render backend: SVG | PNG | HTML
```

**Extension points are registries of small traits.** Adding a geom, stat, scale or
position = one new file + one registry line + tests. If adding a feature requires
editing many unrelated files, the foundation is wrong: stop and tell the user.

## Agent-friendliness

- Every module starts with a short doc comment: what it does, what it does not.
- Error messages say what was wrong, which parameter, and what is accepted.
- Small files, one concept each. No hidden global state.
- `make check` runs everything an agent needs: fmt, clippy, cargo test,
  pytest, visual tests. Keep it fast and green.
- Golden images live in `tests/visual/golden/`. Update them only on purpose, and say so
  in the commit message.

## Workflow per step

1. Read `plan.md`, pick the next unchecked step. Read relevant `memory/` entries.
2. Look at how lets-plot does it (`../lets-plot`, Kotlin in `plot-base`, `plot-builder`,
   `plot-stem`; Python API in `python-package/lets_plot`). If lets-plot lacks it, check
   other ggplot implementations (ggplot2, plotnine) for the solution.
3. Implement minimally. Write unit tests (Rust) and integration tests (Python).
4. Render and look at the output. Compare with lets-plot.
5. Spin bug-hunting sub agents. Fix confirmed bugs surgically.
6. Benchmark if the step touches a hot path.
7. `cargo fmt --all`, `ruff format`, run all tests, commit on `main`, tick the box in `plan.md`.

## Commands

```bash
uv sync                  # dev env (first time)
make develop             # force rebuild + reinstall of the extension
make format              # cargo fmt + ruff format + ruff check --fix
make check               # lint + cargo test + pytest (run before every commit)
uv run pytest            # Python tests only (rebuilds the extension if Rust changed)
```
```

## Memory index

See [memory/README.md](memory/README.md). Always read it before starting work.
