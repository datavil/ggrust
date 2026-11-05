"""Point density

Price against floor area of 600 homes, each point colored by how many neighbours it
has. `geom_pointdensity` follows R's ggpointdensity.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
homes = pl.read_csv(DATA + "homes.csv")

plot = (
    ggplot(homes, aes("area_m2", "price_k"))
    + geom_pointdensity(size=2, adjust=4)
    + scale_color_viridis()
    + labs(x="floor area (m²)", y="price (k)", color="neighbours")
    + theme_light()
    + ggsize(800, 550)
)
plot
# --8<-- [end:code]
