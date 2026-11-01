"""Drinks side by side

Dodged bars compare drinks within each day; `coord_flip` turns them sideways.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
coffee = pl.read_csv(DATA + "coffee.csv").filter(pl.col("weekday").is_in(["Mon", "Wed", "Sat"]))

plot = (
    ggplot(coffee, aes("drink", "cups", fill="weekday"))
    + geom_bar(stat="identity", position="dodge", width=0.8)
    + coord_flip()
    + scale_fill_manual(values=["#264653", "#2a9d8f", "#e9c46a"])
    + labs(x="", y="Cups sold", fill="Day")
    + theme_light()
    + ggsize(600, 380)
)
plot
# --8<-- [end:code]
