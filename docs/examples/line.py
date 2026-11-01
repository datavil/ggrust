"""Monthly temperatures

Lines and points for monthly means in five cities.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
cities = pl.read_csv(DATA + "cities.csv")

plot = (
    ggplot(cities, aes("month", "temperature", color="city"))
    + geom_line(size=1)
    + geom_point(size=3)
    + scale_x_continuous(breaks=list(range(1, 13)), labels=cities["month_name"][:12].to_list())
    + scale_color_brewer(palette="Set1")
    + labs(x="", y="Mean temperature (°C)", color="City")
    + theme_classic()
    + ggsize(640, 380)
)
plot
# --8<-- [end:code]
