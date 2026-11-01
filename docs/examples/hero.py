"""Monthly temperatures

The home page plot.
"""

# --8<-- [start:code]
import ggrust as gg
import polars as pl

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
cities = pl.read_csv(DATA + "cities.csv")

plot = (
    gg.ggplot(cities, gg.aes("month", "temperature", color="city"))
    + gg.geom_ribbon(
        gg.aes(ymin="low", ymax="high", fill="city"), alpha=0.12, color="rgba(0,0,0,0)"
    )
    + gg.geom_line(size=1)
    + gg.geom_point(size=2.5)
    + gg.scale_x_continuous(
        breaks=[1, 3, 5, 7, 9, 11], labels=["Jan", "Mar", "May", "Jul", "Sep", "Nov"]
    )
    + gg.labs(x="", y="Temperature (°C)", color="", fill="")
    + gg.theme_minimal()
    + gg.ggsize(480, 320)
)
plot
# --8<-- [end:code]
