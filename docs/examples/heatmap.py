"""Temperature heatmap

Tiles colored by temperature, with the value printed in each tile.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
cities = pl.read_csv(DATA + "cities.csv").with_columns(
    label=pl.col("temperature").round(0).cast(pl.Int64)
)
months = cities["month_name"][:12].to_list()

plot = (
    ggplot(cities, aes("month_name", "city"))
    + geom_tile(aes(fill="temperature"), color="white", size=0.5)
    + geom_text(aes(label="label"), size=6)
    + scale_x_discrete(limits=months)
    + scale_fill_gradient2(low="#2c7bb6", mid="#ffffbf", high="#d7191c", midpoint=12)
    + labs(x="", y="", fill="°C")
    + theme_minimal()
    + ggsize(700, 300)
)
plot
# --8<-- [end:code]
