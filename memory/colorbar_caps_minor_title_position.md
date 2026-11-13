# Colorbar caps, minor ticks and legend title position

- `legend_title_position` lives in `layout/legend.rs` `arrange_title`: every guide box
  (keys, colorbar, bins) goes through it, so one function places the title. ggplot2's
  rule (`Guide$add_title`): above/below, title **and** graph are offset by
  `hjust * (box width - own width)`; left/right the same with `vjust` down the height.
  Unset hjust = 0 and vjust = 0.5 reproduce the old lets-plot layout exactly (goldens
  unchanged). Gaps stay lets-plot's (half a title line above, none to a left title).
- `layout_end_labels` widens its gap for a left title only: check `title_position`
  before assuming a horizontal guide has its title on the left.
- Caps: `make_room_for_caps` grows the box only by the part of a cap that labels do not
  already cover; end labels instead add the cap to their gap. Caps are drawn before the
  bar and tucked one unit under it (opaque bars only) so no seam shows.
- Minor tick values sit exactly on domain ends (log10 of the data range); fraction maths
  can land 1e-16 outside 0..1, so filter with a tolerance and clamp. A strict
  `(0.0..=1.0).contains` silently dropped the low end tick.
- Build with `CARGO_TARGET_DIR=$PWD/target` in parallel worktrees: a plain `uv run`
  loaded another worktree's extension and reported the new guide options as unsupported.
