"""Cups sold per weekday

Stacked bars of pre-counted values: `geom_bar(stat="identity")`.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
coffee = pl.read_csv(DATA + "coffee.csv")

plot = (
    ggplot(coffee, aes("weekday", "cups", fill="drink"))
    + geom_bar(stat="identity", width=0.7)
    + scale_fill_brewer(palette="Set2")
    + labs(x="", y="Cups sold", fill="Drink")
    + ggsize(600, 380)
)
plot
# --8<-- [end:code]
