# Colorbar edges: no stacked anti-aliased shapes, snap to whole units

Shapes that share an edge and are drawn on top of each other (lets-plot's colorbar
slices all run to the bar's far end) stack their anti-aliased edge pixels: on a bar
off the pixel grid, the edge column shows a blend of every earlier slice (lighter
colors bleed down) and white where few layers overlap. Seen as "white peeking out"
on the left of vertical colorbars (bar at x=550.93).

What works (`layout/colorbar.rs::draw_colorbar`):
- each slice ends one step past the next slice's start: at most two adjacent colors
  ever share an edge pixel, and seams between slices stay covered;
- the drawn bar snaps its origin to whole units (ticks, labels and NA swatch move
  with it); default bar sizes are whole units, so all four edges are crisp at 1x/2x.

ggplot2 avoids the bleed differently: one interpolated raster image for the bar.
Do not go back to non-overlapping slices (conflation seams) or slices to the far end.
