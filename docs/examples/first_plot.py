"""A first plot

Data, a mapping and one layer.
"""

# --8<-- [start:code]
import ggrust as gg
import polars as pl

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
homes = pl.read_csv(DATA + "homes.csv")

plot = gg.ggplot(homes, gg.aes(x="area_m2", y="price_k")) + gg.geom_point()
plot
# --8<-- [end:code]
