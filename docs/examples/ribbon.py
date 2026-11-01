"""Daily temperature range

A ribbon from the typical daily low to high, with the mean drawn on top, one panel per city.
"""

# --8<-- [start:code]
import ggrust as gg
import polars as pl

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
cities = pl.read_csv(DATA + "cities.csv")

plot = (
    gg.ggplot(cities, gg.aes("month"))
    + gg.geom_ribbon(gg.aes(ymin="low", ymax="high", fill="city"), alpha=0.3, color="rgba(0,0,0,0)")
    + gg.geom_line(gg.aes(y="temperature", color="city"), size=1)
    + gg.facet_wrap("city", ncol=5)
    + gg.scale_x_continuous(breaks=[1, 4, 7, 10], labels=["Jan", "Apr", "Jul", "Oct"])
    + gg.labs(x="", y="Temperature (°C)")
    + gg.theme_bw()
    + gg.theme(legend_position="none")
    + gg.ggsize(760, 300)
)
plot
# --8<-- [end:code]
