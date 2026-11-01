"""Labelled scatter

Fruit by sugar and fibre, labels pushed apart with `geom_text_repel`.
"""

# --8<-- [start:code]
import ggrust as gg
import polars as pl

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
fruit = pl.read_csv(DATA + "fruit.csv")

plot = (
    gg.ggplot(fruit, gg.aes("sugar_g", "fibre_g"))
    + gg.geom_point(gg.aes(size="calories"), color="#e76f51", alpha=0.7)
    + gg.geom_text_repel(gg.aes(label="fruit"), size=6, seed=3)
    + gg.scale_x_log10()
    + gg.labs(x="Sugar per 100 g (log scale)", y="Fibre per 100 g", size="kcal")
    + gg.theme_bw()
    + gg.ggsize(640, 440)
)
plot
# --8<-- [end:code]
