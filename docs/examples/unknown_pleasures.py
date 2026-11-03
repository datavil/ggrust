"""Pulsar ridges

Eighty successive radio pulses of pulsar CP 1919, stacked as white ridges on
black: the chart on the cover of Joy Division's *Unknown Pleasures*. A remake of
lets-plot's post-punk demo, with `stat="identity"` ridges drawn from heights.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
pulses = pl.read_csv(DATA + "pulsar.csv", has_header=False)
signal = (
    pulses.with_row_index("pulse")
    .unpivot(index="pulse", variable_name="column", value_name="height")
    .with_columns(
        time=pl.col("column").str.strip_prefix("column_").cast(pl.Int64),
        height=pl.col("height") - pl.col("height").min(),
    )
)

plot = (
    ggplot(signal)
    + geom_area_ridges(
        aes("time", "pulse", height="height"),
        stat="identity",
        scale=0.25,
        color="white",
        fill="black",
    )
    + scale_y_reverse()
    + theme_minimal()
    + theme(axis="blank", panel_grid="blank")
    + flavor_high_contrast_dark()
    + ggsize(600, 600)
)
plot
# --8<-- [end:code]
