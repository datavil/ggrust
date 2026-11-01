"""Boxplots with points

Boxplots summarise each mode; the jittered points show every commuter.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
commute = pl.read_csv(DATA + "commute.csv")

plot = (
    ggplot(commute, aes("mode", "minutes"))
    + geom_boxplot(outlier_alpha=0, width=0.6)
    + geom_jitter(aes(color="mode"), width=0.25, size=1.2, alpha=0.4)
    + labs(x="", y="Commute (minutes)")
    + theme_classic()
    + theme(legend_position="none")
    + ggsize(600, 400)
)
plot
# --8<-- [end:code]
