"""Commute time histogram

Counts per bin, stacked by transport mode.
"""

# --8<-- [start:code]
import ggrust as gg
import polars as pl

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
commute = pl.read_csv(DATA + "commute.csv")

plot = (
    gg.ggplot(commute, gg.aes("minutes", fill="mode"))
    + gg.geom_histogram(bins=40, color="white", size=0.2)
    + gg.scale_fill_brewer(palette="Dark2")
    + gg.labs(x="Commute (minutes)", y="People", fill="Mode")
    + gg.ggsize(640, 380)
)
plot
# --8<-- [end:code]
