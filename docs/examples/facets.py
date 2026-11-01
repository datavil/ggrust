"""Small multiples

`facet_wrap` splits the data into one panel per neighbourhood.
"""

# --8<-- [start:code]
import ggrust as gg
import polars as pl

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
homes = pl.read_csv(DATA + "homes.csv")

plot = (
    gg.ggplot(homes, gg.aes("area_m2", "price_k"))
    + gg.geom_point(gg.aes(color="year_built"), size=1.8, alpha=0.8)
    + gg.facet_wrap("neighbourhood", ncol=2)
    + gg.scale_color_viridis(format="d")
    + gg.labs(x="Floor area (m²)", y="Price (thousands)", color="Built")
    + gg.theme_bw()
    + gg.ggsize(640, 520)
)
plot
# --8<-- [end:code]
