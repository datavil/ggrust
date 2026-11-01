"""A small dashboard

`gggrid` lays several plots out in one figure.
"""

# --8<-- [start:code]
import ggrust as gg
import polars as pl

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
homes = pl.read_csv(DATA + "homes.csv")
coffee = pl.read_csv(DATA + "coffee.csv")
cities = pl.read_csv(DATA + "cities.csv")

prices = (
    gg.ggplot(homes, gg.aes("neighbourhood", "price_k", fill="neighbourhood"))
    + gg.geom_violin(alpha=0.6)
    + gg.labs(x="", y="Price (thousands)")
    + gg.theme(legend_position="none")
)
drinks = (
    gg.ggplot(coffee, gg.aes("drink", "cups"))
    + gg.geom_bar(stat="identity", fill="#6f4e37")
    + gg.coord_flip()
    + gg.labs(x="", y="Cups per week")
)
weather = (
    gg.ggplot(cities, gg.aes("month", "temperature", color="city"))
    + gg.geom_line(size=1)
    + gg.labs(x="Month", y="°C", color="")
)
plot = gg.gggrid([prices, drinks, weather], ncol=3) + gg.ggsize(1000, 320)
plot
# --8<-- [end:code]
