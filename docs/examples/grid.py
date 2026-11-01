"""A small dashboard

`gggrid` lays several plots out in one figure.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
homes = pl.read_csv(DATA + "homes.csv")
coffee = pl.read_csv(DATA + "coffee.csv")
cities = pl.read_csv(DATA + "cities.csv")

prices = (
    ggplot(homes, aes("neighbourhood", "price_k", fill="neighbourhood"))
    + geom_violin(alpha=0.6)
    + labs(x="", y="Price (thousands)")
    + theme(legend_position="none")
)
drinks = (
    ggplot(coffee, aes("drink", "cups"))
    + geom_bar(stat="identity", fill="#6f4e37")
    + coord_flip()
    + labs(x="", y="Cups per week")
)
weather = (
    ggplot(cities, aes("month", "temperature", color="city"))
    + geom_line(size=1)
    + labs(x="Month", y="°C", color="")
)
plot = gggrid([prices, drinks, weather], ncol=3) + ggsize(1000, 320)
plot
# --8<-- [end:code]
