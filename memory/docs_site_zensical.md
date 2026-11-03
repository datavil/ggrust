# Docs site: Zensical, generated live plots

The docs are built with Zensical (`zensical.toml`, `make docs`). Every plot on the site
comes from one `docs/examples/<name>.py`, rendered by `docs/build_examples.py` into
gitignored outputs (`docs/plots/`, `docs/images/gallery/`, `docs/examples/figures/`,
`docs/gallery/`). CI renders them before building.

Traps found while building it (2026-11-01):

- **Do not commit the rendered HTML.** Each `to_html()` gets a fresh random id prefix,
  so the files change on every run. Regenerate in CI instead.
- **Zensical resolves raw-HTML `src` relative to the Markdown file**, not the page URL,
  and rewrites it. From `docs/guide/x.md` write `../plots/x.html` (it becomes
  `../../plots/x.html`). Writing the URL-relative path gives a broken link under
  `/ggrust/` that the dev server happens to hide.
- **Snippet includes are not watched by `zensical serve`.** After regenerating
  `figures/*.html`, restart the server or the pages keep the old markup.
- **Ruff formats Python blocks inside Markdown** and mangles `--8<-- "file"` lines into
  `--8 < --"file"`. `ruff.toml` excludes `docs/**/*.md`.
- **Embedded plots are iframes** sized from `to_html(iframe=True)`. They need
  `box-sizing: content-box` (the border otherwise shrinks the plot) and
  `scrolling="no"`; `docs/javascripts/ggrust.js` scales them with CSS `zoom` on narrow
  screens, which keeps tooltips aligned.
- **API reference is generated and committed**: `docs/build_reference.py` (SECTIONS)
  writes the grouped overview, one page per public name, and the nav between markers in
  `zensical.toml`. It fails when a name in `ggrust.__all__` is missing; CI fails when
  the committed pages are stale. Examples use `from ggrust import *`, so never bind a
  variable to a ggrust name in them (`theme` in a loop broke the themes example).
- **Zensical caches rendered docstrings.** After editing docstrings, build with
  `--clean` (`make docs` does) or pages keep the old text.
- **Every public name has a docstring example, run as a doctest** (2026-11-04):
  `tests/test_docstring_examples.py` fails when a name in `build_reference.SECTIONS`
  has none, when printed output differs, or when it warns. An example that binds `plot`
  at top level is rendered by `build_examples.py` and embedded at the end of its
  reference page, so error and warning examples bind another name (`chart`), and
  examples of features ggrust does not draw yet bind no plot (they would show an
  unstyled picture). Factory-made functions (`_transformed`, `_discrete_reversed`) get
  their examples in the generated `__doc__`; `inspect.getsourcelines` points at the
  inner `def`, so never script docstring edits by source line for them.

