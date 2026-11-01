"""Ridgelines

One density ridge per transport mode.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
commute = pl.read_csv(DATA + "commute.csv")

plot = (
    ggplot(commute, aes("minutes", "mode", fill="mode"))
    + geom_area_ridges(scale=1.4, alpha=0.8, color="white")
    + scale_fill_brewer(palette="Dark2")
    + labs(x="Commute (minutes)", y="")
    + theme_minimal()
    + theme(legend_position="none")
    + ggsize(600, 400)
)
plot
# --8<-- [end:code]
