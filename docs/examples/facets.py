"""Small multiples

`facet_wrap` splits the data into one panel per neighbourhood.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
homes = pl.read_csv(DATA + "homes.csv")

plot = (
    ggplot(homes, aes("area_m2", "price_k"))
    + geom_point(aes(color="year_built"), size=1.8, alpha=0.8)
    + facet_wrap("neighbourhood", ncol=2)
    + scale_color_viridis(format="d")
    + labs(x="Floor area (m²)", y="Price (thousands)", color="Built")
    + theme_bw()
    + ggsize(640, 520)
)
plot
# --8<-- [end:code]
