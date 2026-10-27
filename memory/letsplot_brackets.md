# Brackets follow lets-plot (Phase 2.6)

Parity tests: `tests/test_bracket_parity.py`; Rust: `crates/ggrust-core/tests/build_bracket.rs`.

- lets-plot `bracketDefaults` (GeomProto.kt): `inherit_aes=False`, `show_legend=False`.
  ggrust: `Geom::inherits_aes()` / `Geom::shows_legend()`, read when the layer leaves
  them unset (`build::inherits_aes`). A bracket over `ggplot(aes(x, y, color=g))` is grey.
- Bar = 4-point path: tip, left end, right end, tip. Tips hang `lenstart`/`lenend`
  (default 5) x the unit BELOW the bar (y minus), in data space; `size` unit = 2.2 px.
- Line paint is `TextUtil.toSegmentAes`: `segment_color` unless it equals the sentinel
  `rgba(255,255,255,0.999)` (then `color`), `segment_alpha` unless NaN (then `alpha`),
  width `segment_size` (0.75) x 2.2.
- Label = geom_text at the bar middle, then lifted by HALF ITS TEXT HEIGHT
  (`lines x font size / 2`) in its own rotated frame: baseline 2.1 px above the bar at 14 px.
  Under coord_flip the angle is `angle - 90`. All bars are drawn before all labels.
- Comparing lets-plot label positions: they sit in `rotate(a cx cy)` > `translate(x y)`
  groups, one `<text>` per line with `y` offsets. Apply the rotation before comparing.
