# Colab drops script attributes: find payloads by `type`

Google Colab re-creates every `<script>` in an HTML output to run it, copying only
its `type` and text. Any `class` (or other attribute) is gone, so a runtime lookup
like `querySelector("script.ggrust-zoom")` finds nothing and the plot stays static
(no toolbar, no tooltips, no canvas points), with no error.

The renderer writes each payload as
`<script type="application/vnd.ggrust-<kind>+json" class="ggrust-<kind>">` and the
runtime selects by `type` only. The class stays for tests and readers.

Guard: `tests/test_notebook_hosts.py` strips script classes and checks the runtime
still mounts the toolbar and tooltips. Found 2026-11-04 on ggrust 0.4.0 in Colab.
