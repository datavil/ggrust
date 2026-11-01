"""Boxplots with points

Boxplots summarise each mode; the jittered points show every commuter.
"""

# --8<-- [start:code]
import ggrust as gg
import polars as pl

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
commute = pl.read_csv(DATA + "commute.csv")

plot = (
    gg.ggplot(commute, gg.aes("mode", "minutes"))
    + gg.geom_boxplot(outlier_alpha=0, width=0.6)
    + gg.geom_jitter(gg.aes(color="mode"), width=0.25, size=1.2, alpha=0.4)
    + gg.labs(x="", y="Commute (minutes)")
    + gg.theme_classic()
    + gg.theme(legend_position="none")
    + gg.ggsize(600, 400)
)
plot
# --8<-- [end:code]
