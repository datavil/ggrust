"""Animated time

Penguin bill sizes per species, averaged per year: `transition_time` spaces the
frames evenly in time and moves each species' point smoothly from one year to the
next, as in R's gganimate.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *
from ggrust.data import penguins

means = (
    penguins.drop_nulls(["bill_length_mm", "bill_depth_mm"])
    .group_by("species", "year")
    .agg(pl.col("bill_length_mm", "bill_depth_mm").mean(), pl.len().alias("penguins"))
)
plot = (
    ggplot(means, aes("bill_length_mm", "bill_depth_mm", color="species", size="penguins"))
    + geom_point(alpha=0.8)
    + labs(title="Penguin bills in {frame_time}", x="bill length (mm)", y="bill depth (mm)")
    + transition_time("year")
    + ease_aes("cubic-in-out")
    + theme_light()
    + ggsize(600, 420)
)
animation = animate(plot, nframes=40, fps=10)
animation
# --8<-- [end:code]
