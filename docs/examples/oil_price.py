"""Oil price trend

The daily WTI crude oil spot price in 2024 (U.S. Energy Information Administration),
each day's segment green when the price rose and red when it fell. A remake of
lets-plot's trading chart demo; zoom and pan with the toolbar.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
prices = pl.read_csv(DATA + "oil.csv", try_parse_dates=True).with_columns(
    trend=(pl.col("price").diff().fill_null(0) > 0).cast(pl.Int64)
)

plot = (
    ggplot(prices)
    + geom_line(aes("date", "price"), color="pen", size=1.2, tooltips="none")
    + geom_line(
        aes("date", "price", color=as_discrete("trend"), group=[]),
        size=0.6,
        tooltips=layer_tooltips().title("^x").line("price|^y").anchor("top_left").min_width(200),
    )
    + scale_x_datetime(expand=[0])
    + scale_y_continuous(format="$.2f")
    + scale_color_manual(
        name="trend", values={0: "red", 1: "green"}, labels=["downtrend", "uptrend"]
    )
    + ylab("US dollars per barrel")
    + ggtitle("WTI crude oil daily spot price, 2024")
    + theme_light()
    + flavor_darcula()
    + theme(
        plot_title=element_text(size=18, face="bold"),
        legend_position="bottom",
        axis_title_x="blank",
    )
    + ggtb()
    + ggsize(1000, 500)
)
plot
# --8<-- [end:code]
