"""Legends on several sides

Cars by engine size and highway mileage. Each guide takes its own `position`, so the
color legend sits inside the panel and the binned size legend (`scale_size_binned`
with `guide_bins`) under it, as ggplot2 3.5.
"""

# --8<-- [start:code]
from ggrust import *
from ggrust.data import mpg

plot = (
    ggplot(mpg, aes("displ", "hwy", color="drv", size="cty"))
    + geom_point(alpha=0.6)
    + scale_color_brewer(
        palette="Dark2",
        name="drive",
        labels=["four-wheel", "front", "rear"],
        guide=guide_legend(position="inside"),
    )
    + scale_size_binned(
        range=[1, 7],
        name="city mileage (mpg)",
        guide=guide_bins(position="bottom", direction="horizontal"),
    )
    + labs(x="engine size (litres)", y="highway mileage (mpg)")
    + theme_light()
    + theme(legend_position_inside=[0.98, 0.98], legend_justification=[1, 1])
    + ggsize(700, 500)
)
plot
# --8<-- [end:code]
