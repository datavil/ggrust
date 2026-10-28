# HTML output: explicit ids, inlined runtime, no baseline gap

The HTML renderer (`render/html.rs`, plan 4.1) wraps the SVG in a root `<div>` and
inlines `js/ggrust.js` in one `<script>`.

- **Ids are per display, never the SVG content hash.** Python passes a fresh
  `ggrust-<12 hex>` on every HTML call; it prefixes the root id and every clip id.
  The same plot shown twice in a notebook otherwise shares clip ids, which breaks
  once the runtime edits clips (zoom/pan). `to_svg` keeps the deterministic hash.
- **The runtime file is one function expression with no trailing semicolon.** The
  renderer appends `("<version>");`. A trailing `;` makes that a no-op expression
  and the runtime silently never runs (caught before the first browser test).
  It must not contain `</` or `<!--` (script-element parsing); a unit test guards it.
- **Root div needs `vertical-align:top`** besides `display:inline-block;line-height:0`,
  or it sits on the text baseline and adds ~4 px below the plot (iframe scrolled).
- **lets-plot parity:** `to_html(iframe=True)` returns only an `<iframe srcdoc>`
  sized to the plot; `ggsave(..html)` defaults to `iframe=True`.
- Verify in the built-in browser: open the file, check `data-ggrust-mounted`,
  duplicate ids, unresolved `clip-path` refs, console errors, document scroll size.
  Rebuilds track `js/**/*.js` via `[tool.uv] cache-keys`.
