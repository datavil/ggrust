"""Beeswarm

Body mass of three penguin species, every bird one point. `geom_beeswarm` packs the
points side by side so none overlap, as R's ggbeeswarm does, and `scale_color_npg`
takes its colors from the Nature Publishing Group palette of R's ggsci.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
penguins = pl.read_csv(DATA + "penguins.csv").drop_nulls("body_mass_g")

plot = (
    ggplot(penguins, aes("species", "body_mass_g", color="species"))
    + geom_beeswarm(size=2, cex=2.5, show_legend=False)
    + scale_color_npg()
    + labs(x="", y="body mass (g)")
    + theme_light()
    + ggsize(800, 500)
)
plot
# --8<-- [end:code]
