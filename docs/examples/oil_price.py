"""Bitcoin price trend

The daily Bitcoin close price in 2024, each day's segment green when the price
rose and red when it fell. A remake of lets-plot's trading chart demo; zoom and pan
with the toolbar.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
prices = pl.read_csv(DATA + "bitcoin.csv", try_parse_dates=True).with_columns(
    trend=(pl.col("close").diff().fill_null(0) > 0).cast(pl.Int64)
)

plot = (
    ggplot(prices)
    + geom_line(aes("date", "close"), color="pen", size=1.2, tooltips="none")
    + geom_line(
        aes("date", "close", color=as_discrete("trend"), group=[]),
        size=0.6,
        tooltips=layer_tooltips()
        .title("^x")
        .line("close price|^y")
        .anchor("top_left")
        .min_width(200),
    )
    + scale_x_datetime(expand=[0])
    + scale_y_continuous(format="$.4~s")
    + scale_color_manual(
        name="trend", values={0: "red", 1: "green"}, labels=["downtrend", "uptrend"]
    )
    + ylab("close price")
    + ggtitle("Bitcoin daily close price, 2024")
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
