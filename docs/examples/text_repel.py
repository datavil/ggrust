"""Labelled scatter

Fruit by sugar and fibre, labels pushed apart with `geom_text_repel`.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
fruit = pl.read_csv(DATA + "fruit.csv")

plot = (
    ggplot(fruit, aes("sugar_g", "fibre_g"))
    + geom_point(aes(size="calories"), color="#e76f51", alpha=0.7)
    + geom_text_repel(aes(label="fruit"), size=6, seed=3)
    + scale_x_log10()
    + labs(x="Sugar per 100 g (log scale)", y="Fibre per 100 g", size="kcal")
    + theme_bw()
    + ggsize(640, 440)
)
plot
# --8<-- [end:code]
