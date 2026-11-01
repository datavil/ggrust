"""Monthly temperatures

Lines and points for monthly means in five cities.
"""

# --8<-- [start:code]
import ggrust as gg
import polars as pl

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
cities = pl.read_csv(DATA + "cities.csv")

plot = (
    gg.ggplot(cities, gg.aes("month", "temperature", color="city"))
    + gg.geom_line(size=1)
    + gg.geom_point(size=3)
    + gg.scale_x_continuous(breaks=list(range(1, 13)), labels=cities["month_name"][:12].to_list())
    + gg.scale_color_brewer(palette="Set1")
    + gg.labs(x="", y="Mean temperature (°C)", color="City")
    + gg.theme_classic()
    + gg.ggsize(640, 380)
)
plot
# --8<-- [end:code]
