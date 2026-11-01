"""Cups sold per weekday

Stacked bars of pre-counted values: `geom_bar(stat="identity")`.
"""

# --8<-- [start:code]
import ggrust as gg
import polars as pl

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
coffee = pl.read_csv(DATA + "coffee.csv")

plot = (
    gg.ggplot(coffee, gg.aes("weekday", "cups", fill="drink"))
    + gg.geom_bar(stat="identity", width=0.7)
    + gg.scale_fill_brewer(palette="Set2")
    + gg.labs(x="", y="Cups sold", fill="Drink")
    + gg.ggsize(600, 380)
)
plot
# --8<-- [end:code]
