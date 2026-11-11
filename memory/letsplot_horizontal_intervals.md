# Horizontal intervals follow lets-plot (2026-11-11)

- `geom_errorbar`, `geom_linerange`, `geom_pointrange`, `geom_crossbar` are horizontal
  when `xmin`/`xmax` are set (mapped or constant) without `ymin`/`ymax`, measured against
  lets-plot 4.11. `ymin`/`ymax` win when both are set. `orientation="y"` flips any layer.
- **Orientation has to be known before the mapping loop.** `geom_uses` drops mappings the
  geom does not read, and it must look at the flipped aesthetic, or `y`, `xmin`, `xmax`
  are thrown away as unused and the layer then reports them missing. Hence
  `declared_orientation` (options and mapping) before the loop, and
  `detected_orientation` (discrete columns) after it.
- Geoms stay vertical-only: y-oriented layers see x and y swapped. `width` is a share of
  the resolution of the axis the bars sit on.
- Legend keys stay vertical, as lets-plot (deviations.md).
