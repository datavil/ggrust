"""Home prices by floor area

A scatter plot: one point per sale, colored by neighbourhood.
Hover a point to see its row.
"""

# --8<-- [start:code]
import ggrust as gg
import polars as pl

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
homes = pl.read_csv(DATA + "homes.csv")

plot = (
    gg.ggplot(homes, gg.aes("area_m2", "price_k", color="neighbourhood"))
    + gg.geom_point(size=2.5, alpha=0.7)
    + gg.scale_color_brewer(palette="Set2")
    + gg.labs(x="Floor area (m²)", y="Price (thousands)", color="Neighbourhood")
    + gg.ggsize(640, 400)
)
plot
# --8<-- [end:code]
