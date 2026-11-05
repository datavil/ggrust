"""Highlighted lines

Monthly mean temperature of five cities. `gghighlight` keeps the cities with a freezing
month in color and greys out the rest, as R's gghighlight does.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
cities = pl.read_csv(DATA + "cities.csv")

plot = (
    ggplot(cities, aes("month", "temperature", color="city"))
    + geom_line(size=1.5)
    + gghighlight(pl.col("temperature").min() < 0, label_key="city")
    + scale_x_continuous(breaks=list(range(1, 13)))
    + labs(y="mean temperature (°C)")
    + theme_minimal()
    + ggsize(800, 500)
)
plot
# --8<-- [end:code]
