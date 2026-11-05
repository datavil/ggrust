"""Figure layout

Three penguin plots placed with a layout string and tagged A, B, C, as in R's
patchwork: `design` names a cell per letter, `tag_levels` numbers the panels.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
penguins = pl.read_csv(DATA + "penguins.csv").drop_nulls("bill_length_mm")

scatter = (
    ggplot(penguins, aes("bill_length_mm", "bill_depth_mm", color="species"))
    + geom_point(show_legend=False)
    + scale_color_lancet()
    + labs(x="bill length (mm)", y="bill depth (mm)")
)
mass = (
    ggplot(penguins, aes("species", "body_mass_g", fill="species"))
    + stat_halfeye(show_legend=False)
    + scale_fill_lancet()
    + labs(x="", y="body mass (g)")
)
islands = (
    ggplot(penguins, aes("island", fill="species"))
    + geom_bar()
    + scale_fill_lancet()
    + labs(x="", y="penguins")
)

plot = gggrid(
    [scatter, mass, islands],
    design="AAB\nCCB",
    tag_levels="A",
    guides="collect",
) + ggsize(900, 600)
plot
# --8<-- [end:code]
