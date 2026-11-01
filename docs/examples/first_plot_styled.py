"""A first plot, styled

Color mapped to a column, labels, a theme and a size.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
homes = pl.read_csv(DATA + "homes.csv")

plot = (
    ggplot(homes, aes(x="area_m2", y="price_k", color="neighbourhood"))
    + geom_point(size=2, alpha=0.6)
    + labs(
        title="Bigger homes cost more, but where matters",
        x="Floor area (m²)",
        y="Price (thousands)",
        color="Neighbourhood",
    )
    + theme_classic()
    + ggsize(640, 400)
)
plot
# --8<-- [end:code]
