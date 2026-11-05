"""Side panels

Bill length against flipper length, with the distribution of each variable drawn in a
panel along its axis. Side layers (`geom_xsidedensity`, `geom_ysidedensity`) share the
main panel's axis, as in R's ggside, so they stay aligned when you zoom.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
penguins = pl.read_csv(DATA + "penguins.csv").drop_nulls("bill_length_mm")

plot = (
    ggplot(penguins, aes("flipper_length_mm", "bill_length_mm", color="species"))
    + geom_point(size=1.8)
    + geom_xsidedensity(aes(fill="species"), alpha=0.4)
    + geom_ysidedensity(aes(fill="species"), alpha=0.4, show_legend=False)
    + scale_color_nejm()
    + scale_fill_nejm()
    + labs(x="flipper length (mm)", y="bill length (mm)")
    + theme_light()
    + ggsize(800, 600)
)
plot
# --8<-- [end:code]
