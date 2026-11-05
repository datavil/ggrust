"""Cluster marks

Bill length against bill depth: each species outlined by a concave hull and labelled
outside its points. `geom_mark_hull` follows R's ggforce.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
penguins = pl.read_csv(DATA + "penguins.csv").drop_nulls("bill_length_mm")

plot = (
    ggplot(penguins, aes("bill_length_mm", "bill_depth_mm"))
    + geom_mark_hull(
        aes(fill="species", label="species"), concavity=8, alpha=0.15, show_legend=False
    )
    + geom_point(aes(color="species"), size=2, show_legend=False)
    + scale_color_aaas()
    + scale_fill_aaas()
    + expand_limits(x=[28, 64], y=[12, 23])
    + labs(x="bill length (mm)", y="bill depth (mm)")
    + theme_minimal()
    + ggsize(800, 600)
)
plot
# --8<-- [end:code]
