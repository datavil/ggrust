"""Ridgelines

One density ridge per transport mode.
"""

# --8<-- [start:code]
import ggrust as gg
import polars as pl

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
commute = pl.read_csv(DATA + "commute.csv")

plot = (
    gg.ggplot(commute, gg.aes("minutes", "mode", fill="mode"))
    + gg.geom_area_ridges(scale=1.4, alpha=0.8, color="white")
    + gg.scale_fill_brewer(palette="Dark2")
    + gg.labs(x="Commute (minutes)", y="")
    + gg.theme_minimal()
    + gg.theme(legend_position="none")
    + gg.ggsize(600, 400)
)
plot
# --8<-- [end:code]
