"""Violins by city

Kernel density violins, dodged by city, with sina points inside.
"""

# --8<-- [start:code]
import ggrust as gg
import polars as pl

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
commute = pl.read_csv(DATA + "commute.csv").filter(pl.col("mode").is_in(["Bus", "Car", "Train"]))

plot = (
    gg.ggplot(commute, gg.aes("mode", "minutes", fill="city"))
    + gg.geom_violin(alpha=0.5, color="rgba(0,0,0,0)")
    + gg.geom_sina(gg.aes(color="city"), size=0.8, alpha=0.6, seed=1)
    + gg.scale_fill_brewer(palette="Set1")
    + gg.scale_color_brewer(palette="Set1")
    + gg.labs(x="", y="Commute (minutes)", fill="City", color="City")
    + gg.ggsize(680, 400)
)
plot
# --8<-- [end:code]
