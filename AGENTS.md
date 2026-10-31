# AGENTS.md

Read this file fully before touching the code. It is the contract for every agent
(human or AI) working on **ggrust**. The roadmap lives in [plan.md](plan.md).
Lessons learned live in [memory/](memory/README.md).

## Vision

ggrust is a general-purpose grammar-of-graphics plotting library **written in Rust,
used from Python**.

- [cellestial](https://github.com/datavil/cellestial) is its first user: it needs to keep
  its lets-plot based API after lets-plot stopped being maintained (Kotlin notebook sunset).
  cellestial's needs set the roadmap order, not the scope of the library.
- The Python API is a **lets_plot-compatible subset**: `aes`, `ggplot`, `geom_*`,
  `scale_*`, `theme`, `gggrid`, `ggtb`, `PlotSpec.as_dict()`, `+` composition.
  Porting cellestial should be (almost) an import swap: `lets_plot` -> `ggrust`.
- Features follow cellestial's needs first, then general plotting needs. Not all of
  lets-plot. Examples and docs use everyday data, not only single-cell data.
- Rust does all the heavy work (stats, scales, layout, rendering). Python is a thin
  spec builder. Data crosses the boundary once, as Arrow, zero-copy where possible.
- Outputs: SVG, PNG, interactive HTML (notebook `_repr_html_`). Interactivity
  (tooltips, zoom/pan) is a small embedded vanilla-JS runtime over Rust-rendered output.
  Dense point layers are drawn on a canvas from binary buffers.
- Performance target: match and exceed lets-plot on equal work. Memory usage matters too.
- The library is maintained by agents. Optimise for agent-friendliness (see below).

## Parity with lets-plot (and when not to copy it)

The default is **parity with lets-plot, visually and logically**: same defaults, same
layout, same stat results, same spec semantics. But lets-plot has bugs and unintended
behavior (for example legends or geoms overflowing the plot area, clipped labels).
Do not copy those blindly. When something in the lets-plot output looks wrong:

1. **Note it first.** Write down what looks wrong and how to reproduce it (in your
   step notes, and as a `memory/` entry if it is a lasting decision).
2. **Check ggplot2.** See how ggplot2 (the reference grammar) behaves for the same
   spec. If ggplot2 clearly does the right thing, that is the candidate behavior.
3. **Still unsure? Ask the user.** Do not decide a deviation on your own when it is
   not obvious. Recorded deviations go in the `plan.md` decisions table.

Parity covers rendering and semantics only. It does **not** cover code quality:
lets-plot's sparse type hints, docstrings and error messages are not the bar (see below).

Parity is **visual, not byte-level.** Encoded payloads inside the spec or the output
(embedded PNGs, compressed streams, number formatting of coordinates) do not have to
be byte-identical to lets-plot. When a faster implementation gives the same picture
(same pixels after decoding, same layout), prefer the faster one, and test the
decoded result rather than the bytes. Never trade speed for byte-identical output.

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
   confirm (surgically). Re-verify. Use the `bug-hunter` agent
   (`.claude/agents/bug-hunter.md`, runs on Sonnet) for these.
7. **Record critical lessons** (an approach that works, or one that breaks things) as
   a file in `memory/` and add it to the index in [memory/README.md](memory/README.md).
8. **Never cheat benchmarks.** Same data, same work, same output size on both sides.
   Note lets-plot's silent default sampling (see memory index). Report honestly.
9. **Commit each significant step on `main`, then push.** Run `cargo fmt --all` (and `ruff format`,
   `ruff check --fix` for Python) before every commit. Tests must pass at each commit.
10. Mindset of an industry lead: elegant yet robust. Robustness over flexibility.
11. **Share progress visually.** At each visual milestone, send the user a rendered PNG
    (after inspecting it yourself) so they can follow along.

## Naming and style

- Polars-style naming: full words, no abbreviations (`group_index`, not `grp_idx`;
  `frame`, not `df`). Well-known terms are fine (`n`, `x`, `y`, `svg`, `rgb`, `kde`, `dpi`).
  Exception: user-facing lets_plot-compatible names stay as lets_plot spells them
  (`aes`, `geom_point`, `ggsize`), because compatibility is the point.
- Rust: `cargo fmt`, `cargo clippy -- -D warnings`. Errors via `thiserror`, no `unwrap()`
  on user-controlled input. Functions return one type (use `Result`/enums, not shape-shifting).
- Python: ruff formatting and import sorting, polars over pandas. Functions return a
  single type.
- Python type hints are **comprehensive**: every public and private function, parameter
  and return value, using precise types (`Literal[...]`, `X | None`, typed containers),
  not `Any` unless truly unavoidable. Do not match lets-plot's sparse hints.
  Checked with `ty` (`uv run ty check python/`) and `ruff`.
- Python docstrings are **NumPy style** on every public function and class: summary,
  `Parameters` (with `type, default=...`), `Returns`, `Raises`, and `Examples` where
  useful. Follow cellestial (`../cellestial/cellestial/layers/stream.py` is a good model).
- No em dashes in code, docs, or comments.

## Errors are first-class

Errors are part of the API, Rust style: typed, informative and solution-oriented.

- Every error says **what** was wrong, **where** (which parameter, layer, column or
  spec path), **what was accepted**, and **how to fix it** (a suggestion, a close
  match for typos: `unknown geom 'piont'; did you mean 'point'?`).
- Rust: one `thiserror` enum per module boundary, variants with structured fields,
  no stringly-typed catch-alls. Never panic or `unwrap()` on user input.
- Python: specific exception classes (subclassing the matching builtin, e.g.
  `ValueError`, `TypeError`), each with a docstring, as in
  `../cellestial/cellestial/util/errors.py`. Rust errors map to these at the PyO3 boundary.
  Fail early, at spec-building time, when the problem is detectable in Python.
- Tests assert on the error type and the key parts of the message.

**Order of work:** implement and verify the feature first. Type hints, docstrings and
error polish may follow after the implementation is complete (see the polish step in
`plan.md`), but new code must not make that pass larger: write hints and clear errors
as you go where it is cheap.

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
- Golden files live next to the tests that own them (`crates/ggrust-core/tests/golden/`,
  `tests/visual/golden/`). Update them only on purpose (`GGRUST_BLESS=1`), and say so in
  the commit message. Rendered outputs for inspection go to `tests/visual/output/` (ignored).

## Cost discipline

Tokens are a budget. Spend them on design and verification, not on re-reading or noise.

- **End the session at the end of a step.** After committing a step and ticking
  plan.md, stop. Do not start the next step in the same session. Tell the user the
  step is done and the next one should start in a fresh session.
- **Read narrowly.** Search before reading. Read the line range you need, not whole
  files. Do not re-read a file you just edited.
- **Test narrowly while iterating.** Run targeted tests (`cargo test -q <name>`,
  `pytest -q -k <name>`) while working. Run the full `make check` once, before each commit.
- **Keep output short.** Pipe long command output through `tail` or `grep`. Never dump
  full SVG, HTML or JSON into the context. Inspect slices.
- **Cheap sub agents for cheap work.** Searches and mechanical checks go to sub agents
  on a cheaper model (Sonnet or Haiku). Do not spawn a sub agent for something a few
  tool calls settle.
- **No open-ended loops.** Before a performance or polish step, ask the user for a stop
  condition (a target number or a time box). If a fix fails twice the same way, stop
  and ask instead of trying a third variant.

## Workflow per step

1. Read `plan.md`, pick the next unchecked step. Read relevant `memory/` entries.
2. Look at how lets-plot does it (`../lets-plot`, Kotlin in `plot-base`, `plot-builder`,
   `plot-stem`; Python API in `python-package/lets_plot`). If lets-plot lacks it, check
   other ggplot implementations (ggplot2, plotnine) for the solution.
3. Implement minimally. Write unit tests (Rust) and integration tests (Python).
4. Render and look at the output. Compare with lets-plot. If lets-plot looks wrong,
   follow "Parity with lets-plot" above (note, check ggplot2, ask).
5. Spin bug-hunting sub agents. Fix confirmed bugs surgically.
6. Benchmark if the step touches a hot path.
7. `cargo fmt --all`, `ruff format`, run all tests, commit on `main`, push, tick the box in `plan.md`.

## Commands

```bash
uv sync                  # dev env (first time)
make develop             # force rebuild + reinstall of the extension
make format              # cargo fmt + ruff format + ruff check --fix
make check               # lint + cargo test + pytest (run before every commit)
uv run pytest            # Python tests only (rebuilds the extension if Rust changed)
uv run ty check python/  # Python type check
```
```

## Memory index

See [memory/README.md](memory/README.md). Always read it before starting work.
