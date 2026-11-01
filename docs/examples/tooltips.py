"""Custom tooltips

`layer_tooltips` chooses the lines a hover shows and how they are formatted.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
homes = pl.read_csv(DATA + "homes.csv")

tooltips = (
    layer_tooltips()
    .title("@neighbourhood")
    .line("Price|@{price_k}k")
    .line("Area|@{area_m2} m²")
    .line("Rooms|@rooms")
    .line("Built|@year_built")
    .format("year_built", "d")
)
plot = (
    ggplot(homes, aes("area_m2", "price_k", color="neighbourhood"))
    + geom_point(size=3, alpha=0.7, tooltips=tooltips)
    + labs(x="Floor area (m²)", y="Price (thousands)", color="")
    + theme_minimal()
    + ggtb()
    + ggsize(640, 420)
)
plot
# --8<-- [end:code]
