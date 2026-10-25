# PNG: rasterise the scene directly, never via a big SVG

**Broke:** PNG through `to_svg` + resvg failed for 1M points:
`SVG data parsing failed cause nodes limit reached` (usvg caps node count). Parsing was
also the main cost (100k points: 377 ms).

**Works:** `render/png.rs` draws geometry straight from the `Scene` with tiny-skia and
renders only text runs through resvg, using the same SVG text markup as `to_svg`
(`svg::text_document`). Clips become tiny-skia masks (intersected for nested groups).
100k points: 185 ms; 1M points: 1.4 s.

**Guard:** `render_scene.rs::direct_png_matches_resvg_rendering_of_the_svg` compares the
direct PNG with resvg's rendering of our SVG pixel by pixel. If you change geometry in one
renderer, change it in the other.

## Parity rules live in one place (`scene/geometry.rs`)

The bug hunt found 5 SVG/PNG disagreements (odd dash patterns, zero-size rects with
strokes, negative marker sizes, huge coordinates dropping whole paths in tiny-skia,
closed paths closing only the last ring). Fix pattern: one shared rule set
(`ClipBox`, `is_drawable_*`, `Stroke::dash_pattern`) used by both renderers, plus
`tests/render_parity.rs` rendering every edge case both ways.
Text: `text::single_line` (tabs/newlines -> spaces) is used by both `measure` and the
SVG writer, and text carries `xml:space="preserve"`, so rendered width == measured width.
Debug builds: dependencies need `opt-level = 2` (`[profile.dev.package."*"]`) or
tiny-skia rendering tests take minutes.
