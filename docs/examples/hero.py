"""Monthly temperatures

The home page plot.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
cities = pl.read_csv(DATA + "cities.csv")

plot = (
    ggplot(cities, aes("month", "temperature", color="city"))
    + geom_ribbon(aes(ymin="low", ymax="high", fill="city"), alpha=0.12, color="rgba(0,0,0,0)")
    + geom_line(size=1)
    + geom_point(size=2.5)
    + scale_x_continuous(
        breaks=[1, 3, 5, 7, 9, 11], labels=["Jan", "Mar", "May", "Jul", "Sep", "Nov"]
    )
    + labs(x="", y="Temperature (°C)", color="", fill="")
    + theme_minimal()
    + ggsize(480, 320)
)
plot
# --8<-- [end:code]
