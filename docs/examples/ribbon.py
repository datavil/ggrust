"""Daily temperature range

A ribbon from the typical daily low to high, with the mean drawn on top, one panel per city.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
cities = pl.read_csv(DATA + "cities.csv")

plot = (
    ggplot(cities, aes("month"))
    + geom_ribbon(aes(ymin="low", ymax="high", fill="city"), alpha=0.3, color="rgba(0,0,0,0)")
    + geom_line(aes(y="temperature", color="city"), size=1)
    + facet_wrap("city", ncol=5)
    + scale_x_continuous(breaks=[1, 4, 7, 10], labels=["Jan", "Apr", "Jul", "Oct"])
    + labs(x="", y="Temperature (°C)")
    + theme_bw()
    + theme(legend_position="none")
    + ggsize(760, 300)
)
plot
# --8<-- [end:code]
