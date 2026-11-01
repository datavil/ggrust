"""Violins by city

Kernel density violins, dodged by city, with sina points inside.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
commute = pl.read_csv(DATA + "commute.csv").filter(pl.col("mode").is_in(["Bus", "Car", "Train"]))

plot = (
    ggplot(commute, aes("mode", "minutes", fill="city"))
    + geom_violin(alpha=0.5, color="rgba(0,0,0,0)")
    + geom_sina(aes(color="city"), size=0.8, alpha=0.6, seed=1)
    + scale_fill_brewer(palette="Set1")
    + scale_color_brewer(palette="Set1")
    + labs(x="", y="Commute (minutes)", fill="City", color="City")
    + ggsize(680, 400)
)
plot
# --8<-- [end:code]
