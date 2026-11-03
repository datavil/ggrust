# facet_wrap follows lets-plot

Source: lets-plot `FacetConfig.createWrap`, `FacetWrap`, `PlotFacets`,
`FacetedPlotLayout(Util)`, `Fixed/FreeScalesTilesLayouter`, `PlotConfigBackend.applyLayerStatistic`.

- **Levels:** first appearance over every layer's data, then `order` (lets_plot's Python
  default is `order=1`: sorted, so an Enum's order shows only with `order=0`). A single
  `order`/`format` value applies to the first faceting column only. Null/NaN values are
  in no panel. Combinations without rows are dropped (`drop`, default true).
- **Stats and positions run per panel; the stat context (bin ranges) is the whole plot**,
  except on a free axis: the panel's range there, as ggplot2 (deviation, lets-plot #1443).
  Style scales train on all panels. A layer without the faceting column (or with no
  mapped columns: `geom_hline(yintercept=..)`) shows all its rows in every panel.
- **Group order trap:** stats number output groups by first appearance *in the panel*.
  lets-plot regroups the merged stat output, so the order is plot-wide. ggrust keeps the
  layer's groups in panels and renumbers a panel's stat output by each group's first
  appearance in the whole layer (`split_layer`, `regroup_by_rank`). Without it, stacked
  bars flip colors between panels.
- **Level order trap:** panel slices keep every level (`Column::take`), and discrete style
  levels are reordered to the unsplit data order after training (`order_levels_by`):
  per-panel stats otherwise reorder legend keys.
- **Layout:** one panel size for all; a grid column is as wide as its widest shown y
  axis, a row's x band as thick as its thickest; the last panel of an unfinished column
  draws its x axis in the empty space and takes no row band. Shared scales choose breaks
  once (for the first panel). Default size `600*(0.5+0.5/ncol)*ncol` x
  `400*(0.5+0.5/nrow)*nrow`. Strips: label line box + `strip_text` margins (3), stacked
  per variable, `strip_spacing` 0, `panel_spacing` 10.

- **facet_grid** (lets-plot `FacetGrid`): panels for every (x, y) level pair, empty
  ones too; x strips on the top row only, y strips right of the last column (19 px
  at 13 px text, `strip_spacing_x` 0, text `rotate(90)` in SVG, centred); axes only on
  the bottom row and left column even with free scales. Free x is shared per grid
  column, free y per row: build uses one scale per `Tile::x_scale_group` /
  `y_scale_group` (stat range, discrete levels, training), never per panel.

**How to apply:** compare faceted output with `tests/test_facet_parity.py` (marks as
fractions of their panel). Break-count differences of a fraction of a pixel are the known
near-tie (see letsplot_reference_svg.md), not a facet bug.
