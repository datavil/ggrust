"""Commute time histogram

Counts per bin, stacked by transport mode.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
commute = pl.read_csv(DATA + "commute.csv")

plot = (
    ggplot(commute, aes("minutes", fill="mode"))
    + geom_histogram(bins=40, color="white", size=0.2)
    + scale_fill_brewer(palette="Dark2")
    + labs(x="Commute (minutes)", y="People", fill="Mode")
    + ggsize(640, 380)
)
plot
# --8<-- [end:code]
