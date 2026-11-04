# Custom fonts: one registry, one face everywhere

**Works:** `text::registry` holds registered faces as `Arc<LoadedFont>` (leaked
`&'static` data, deduped by bytes); `text::loaded(font)` is the only way to get a face,
and `measure`, `shape`, PNG, PDF and SVG all go through it, so measuring and drawing
never disagree. `FontFamily::Registered(id)` keeps `Font` `Copy`.

**PNG:** resvg picks fonts by name, so each registered face is pushed into the
`fontdb` as a `FaceInfo` under its unique `css_name` (`ggrust-<hash>`), with normal
weight/style, and SVG text for registered fonts writes that name first and **no**
`font-weight`/`font-style` (otherwise resvg or browsers pick or synthesize another
face). The database is cached by the registry generation. resvg falls back per glyph
to any face in the database, but `measure` and PDF draw the registered font's
`.notdef`: `png::fallback_selector` returns no fallback when the base face is
registered (`ggrust-` name), so all outputs agree. Embedded fonts keep resvg's
default fallback (goldens).

**SVG/HTML:** `subsetter` output is PDF-only (no `cmap`, no `OS/2`, no layout tables):
`render/webfont.rs` adds a format 4/12 `cmap`, copies `OS/2`, and builds a `kern`
table from the pair adjustments harfrust applied to the document's text, so browser
widths match the measured ones (checked: 0.01 px in Chromium). `unicode-range` on
each rule lets several documents on one page subset the same face under the same
name. Ligatures are lost in SVG (no GSUB). The zoom character set is added only for
zoomable HTML; static SVG subsets stay small (about 13 KB per face for a plot).

**Breaks:** writing Rust string continuations (`\` + newline) through a Python
heredoc edit drops the backslash and leaves runs of spaces in messages; grep new
messages for `\s{6,}` after scripted edits.

**Registering:** faces are checked (`check_face`) before the bytes are leaked, so a
bad file costs nothing; `register_font_dir` skips unusable files (Python warns per
file) and fails only when none registers. `fsType` bits: restricted (`& 0xF == 2`),
no subsetting (`0x100`) and bitmap only (`0x200`) block PDF embedding (error) and
SVG embedding (name only).

**How to apply:** new text renderers call `text::loaded`; never read
`Font::font_data()` for registered families except through the loaded face (index
matters for `.ttc`). Tests register under unique family names (the registry is
process-wide and tests run in parallel). See [[pdf_vector_output]].
