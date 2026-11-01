"""A first plot

Data, a mapping and one layer.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
homes = pl.read_csv(DATA + "homes.csv")

plot = ggplot(homes, aes(x="area_m2", y="price_k")) + geom_point()
plot
# --8<-- [end:code]
