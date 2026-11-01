"""Drinks side by side

Dodged bars compare drinks within each day; `coord_flip` turns them sideways.
"""

# --8<-- [start:code]
import ggrust as gg
import polars as pl

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
coffee = pl.read_csv(DATA + "coffee.csv").filter(pl.col("weekday").is_in(["Mon", "Wed", "Sat"]))

plot = (
    gg.ggplot(coffee, gg.aes("drink", "cups", fill="weekday"))
    + gg.geom_bar(stat="identity", position="dodge", width=0.8)
    + gg.coord_flip()
    + gg.scale_fill_manual(values=["#264653", "#2a9d8f", "#e9c46a"])
    + gg.labs(x="", y="Cups sold", fill="Day")
    + gg.theme_light()
    + gg.ggsize(600, 380)
)
plot
# --8<-- [end:code]
