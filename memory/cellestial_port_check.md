# Run cellestial port checks in a mirrored, isolated env

How step 1.7 ran the cellestial suite on ggrust without touching the user's checkout:

- `git -C ../cellestial worktree add -b ggrust-port <scratch dir> dev`, then swap imports with
  `perl -pi -e 's/\blets_plot\b/ggrust/g'` (BSD `sed` has no `\b`: it silently changes nothing).
- New venv in the worktree; install ggrust from this repo, then **the exact package versions of the
  user's poetry env** (`uv pip freeze --python <poetry python>`, minus lets-plot and cellestial).
- Baseline: the same suite in the poetry env on real lets_plot.

**Why:** a fresh resolve pulled polars 1.44 / anndata 0.13 and produced 57 false failures
(`DuplicateError` in `unpivot`) that had nothing to do with ggrust.

**How to apply:** for every port check (2.7, 3.4, 5.4), compare against the lets_plot baseline in
the same versions, and render plots (cellestial tests mostly check `isinstance` / `as_dict`).
lets_plot itself fails some exports (e.g. on-data legend PNG: "Unsupported color value: none").

**Browser tests (5.4):** run them with `-m ""` (cellestial's addopts deselect `browser`). Its
`tests/plot_assertions.py` reads lets-plot HTML internals; on `ggrust-port` it is rewritten for
ggrust's HTML (decided with the user). Rebase that commit when the branch moves to a newer `dev`.
