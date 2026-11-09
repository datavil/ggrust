# coord_3d: project, sort rows, reuse the 2D geoms

Static 3D (tier 1) without touching the 2D `Coord` trait or any renderer.

- **Approach that works:** project each row (`coord3d::Projector`), reorder rows back to
  front, replace x/y with screen fractions of the panel, and call the layer's own 2D
  geom with `coord3d::identity_scale()` on both axes. Styles must be reordered with
  the data (`Styles::take`), and paths need their group index renumbered in drawing
  order: `draw_paths` draws by group index, not row order.
- **z is scaled in `build/space3d.rs`**, after the stat stage (x and y are scaled
  before it): `z.positions()` then `train()` on every panel's layers, one z scale for
  the whole plot (no free z in facets).
- **The 2D layout must not reserve axis room:** `arrange` lays out with
  `space3d::layout_theme` (axis text/ticks/lines blank) and titles x/y moved into
  `Space3d.titles`; the box keeps `box3d::label_room` around itself inside the panel.
- **View convention** = matplotlib: eye `(cos e cos a, cos e sin a, sin e)`; screen
  right `(-sin a, cos a, 0)`; right x up = eye (right-handed). Far pane of an axis is
  at 0 when the eye component is >= 0. Defaults -60/30 look from +x, -y, +z.
- **Break count per edge length** (30 px per break, 3 to 5): fixed 5 crowds faceted
  boxes, 2 often leaves one label in range.
- `interaction()` returns early for 3D (hover targets would use the flat panel).

- **geom_surface draws itself** (`geom::surface::draw_3d`, called from
  `space3d::draw_tile`): cells need 3D depth, which the flat-panel 2D path loses.
  Its 2D `Geom::draw` errors. Grid = distinct x and y values (exact equality), a
  guard rejects scattered data (cells > 4 x rows). Mean-depth painter's sort leaves
  small artifacts on steep surfaces, as matplotlib's does.

Recorded deviations: deviations.md rows "3D ...". Tests: `tests/test_coord_3d.py`,
golden `coord_3d_points_path`.
