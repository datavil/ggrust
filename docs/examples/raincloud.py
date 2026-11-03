"""Raincloud plot

Bill depth of three penguin species: a half violin (the cloud), a slim boxplot and
the raw measurements as half a sina plot (the rain). A remake of lets-plot's
raincloud demo.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
penguins = pl.read_csv(DATA + "penguins.csv").drop_nulls("bill_depth_mm")

plot = (
    ggplot(penguins, aes("species", "bill_depth_mm", color="species", fill="species"))
    + geom_violin(show_half=1, alpha=0.75, trim=False, position=position_nudge(x=0.07))
    + geom_sina(
        show_half=-1,
        fill="white",
        shape=21,
        seed=42,
        show_legend=False,
        position=position_nudge(x=-0.07),
    )
    + geom_boxplot(width=0.1, outlier_alpha=0, show_legend=False)
    + scale_color_brewer(palette="Set2")
    + scale_fill_brewer(palette="Pastel2")
    + coord_flip()
    + ylab("bill depth (mm)")
    + theme_light()
    + flavor_solarized_dark()
    + ggsize(800, 600)
)
plot
# --8<-- [end:code]
