"""Shadows

Unemployment against the savings rate in the United States, one point per year:
`shadow_wake` trails the moving point with its last frames, as gganimate's
`shadow_wake` does.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *
from ggrust.data import economics

yearly = (
    economics.with_columns(
        year=pl.col("date").dt.year(),
        unemployed=pl.col("unemploy") / pl.col("pop") * 100,
    )
    .group_by("year")
    .agg(pl.col("unemployed").mean(), pl.col("psavert").mean())
    .sort("year")
)
plot = (
    ggplot(yearly, aes("unemployed", "psavert"))
    + geom_point(size=6, color="#b7410e")
    + labs(
        title="United States, {frame_time}",
        x="unemployed (% of population)",
        y="personal savings rate (%)",
    )
    + transition_time("year")
    + shadow_wake(0.15)
    + theme_light()
    + ggsize(650, 450)
)
animation = animate(plot, nframes=100, fps=14)
animation
# --8<-- [end:code]
