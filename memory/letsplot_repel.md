# Text/label repel follows lets-plot's LabelForceLayout (Phase 5.1)

Rust: `geom/repel_layout.rs` (pure port), `geom/repel.rs` (geoms). Python: `tests/test_repel_parity.py`.

- The layout is a line-by-line port of lets-plot `LabelForceLayout` + `TransformedRectangle`.
  Given lets-plot's own text widths it reproduces lets-plot's label positions to 1e-6
  (two Rust tests). Keep it that way: mutation order (labels move in place during a
  pass), random draws (`KotlinRandom`, only in self-repulsion and zero-length
  directions), Kotlin `sign` (0 stays 0, not `signum`), `normalize` vs `savedNormalize`.
- Rendered positions still differ from lets-plot because our embedded font measures text
  a few px narrower; the layout is chaotic, so small width changes move labels. Do not
  "fix" that in the layout. Get lets-plot widths from a `geom_label(label_padding=0,
  label_r=0)` box path when you need a new parity case.
- Points repel labels with marker radius (`point_size` 3 -> 3.3 px) + `point_padding` x 2.2;
  `box_padding` x 2.2 is kept between label boxes. Nudge moves the label start only: the
  segment starts at the real point's marker edge and ends at the nearest reachable box
  edge centre. Per label: text, then its segment. Segment paint = bracket's
  `segment_stroke` (`segment_color` sentinel -> `color`).
- No seed -> seed 0 (lets-plot uses an unseeded Random), as `position_jitter`.
- `max_time` is seconds in the spec, ms inside the layout; the time cap makes huge
  layouts (thousands of labels) run-dependent, as in lets-plot.
