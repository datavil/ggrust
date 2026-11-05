"""Clustered heatmap

Five nutrients of 19 fruits, each scaled to mean 0 and standard deviation 1. The rows
are ordered by hierarchical clustering and the tree is drawn beside the axis
(`hclust` + `scale_y_dendro`, after R's ggh4x and legendry). A side track shows each
fruit's calories with its own fill scale (`geom_ysidetile` from ggside, and
`new_scale_fill` from ggnewscale).
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
fruit = pl.read_csv(DATA + "fruit.csv")
nutrients = ["calories", "sugar_g", "fibre_g", "vitamin_c_mg"]
scaled = fruit.with_columns((pl.col(n) - pl.col(n).mean()) / pl.col(n).std() for n in nutrients)

rows = hclust(scaled.select("fruit", *nutrients), labels="fruit", method="average")
long = scaled.unpivot(index="fruit", on=nutrients, variable_name="nutrient", value_name="z")

plot = (
    ggplot(long, aes("nutrient", "fruit"))
    + geom_tile(aes(fill="z"), color="white")
    + scale_fill_gradient2(low="#2166ac", mid="white", high="#b2182b", name="z-score")
    + scale_y_dendro(rows)
    + new_scale_fill()
    + geom_ysidetile(
        aes(x="calorie", fill="calories"), data=fruit.with_columns(calorie=pl.lit("kcal"))
    )
    + scale_fill_viridis(option="cividis", name="kcal")
    + labs(x="", y="")
    + theme_minimal()
    + ggsize(800, 650)
)
plot
# --8<-- [end:code]
