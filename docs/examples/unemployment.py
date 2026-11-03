"""US unemployment

Monthly US unemployment on a log scale, over bands for each presidential term. A
remake of lets-plot's demo of ggplot2's `economics` and `presidential` datasets.
"""

# --8<-- [start:code]
from datetime import datetime

import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
unemployment = pl.read_csv(DATA + "unemployment.csv", try_parse_dates=True)
presidents = pl.read_csv(DATA + "presidents.csv", try_parse_dates=True)
breaks = [2.5, 5, 10]

plot = (
    ggplot()
    + geom_rect(
        aes(xmin="start", xmax="end", fill="party"),
        data=presidents,
        ymin=2400,
        ymax=16000,
        size=0,
        alpha=0.4,
    )
    + geom_label(
        aes("start", label="name", color="party"),
        data=presidents,
        y=4000,
        alpha=0.6,
        angle=90,
        vjust=1,
        show_legend=False,
    )
    + geom_line(aes("date", "unemploy"), data=unemployment)
    + scale_y_log10(
        "unemployed (millions)", breaks=[b * 1000 for b in breaks], labels=[str(b) for b in breaks]
    )
    + scale_fill_manual(name="", values={"Republican": "#e41a1c", "Democratic": "#377eb8"})
    + scale_color_manual(values={"Republican": "#e41a1c", "Democratic": "#377eb8"})
    + coord_cartesian(xlim=[datetime(1966, 1, 1), datetime(2016, 1, 1)])
    + xlab("year")
    + ggtitle("Unemployment in the US", "1967 to 2015")
    + theme(legend_position="top", plot_title=element_text(size=18, face="bold"))
    + ggsize(1000, 400)
)
plot
# --8<-- [end:code]
