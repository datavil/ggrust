"""Netflix movie durations

How long Netflix movies run, one ridge per genre, colored by the genre's mean
duration. A remake of the ridgeline from lets-plot's Netflix data analysis. Hover a
ridge for its genre.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
movies = (
    pl.read_csv(DATA + "netflix.csv")
    .filter(pl.col("genres") != "Movies")
    .with_columns(genre=pl.col("genres").str.split(", "))
    .explode("genre", empty_as_null=True)
    .with_columns(mean_minutes=pl.col("minutes").mean().over("genre"))
    .sort("mean_minutes", descending=True)
)

plot = (
    ggplot(movies, aes("minutes", "genre"))
    + geom_area_ridges(
        aes(group="genre", fill="mean_minutes"),
        scale=4,
        tooltips=layer_tooltips().title("@genre").line("duration|@minutes min"),
    )
    + scale_x_log10("duration (minutes)")
    + scale_fill_viridis(name="mean duration", option="plasma")
    + ggtitle("Mean Netflix movie duration")
    + theme(axis_line_x="blank")
    + ggsize(800, 600)
)
plot
# --8<-- [end:code]
