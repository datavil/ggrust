"""Built-in themes

The same plot in four built-in themes.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
coffee = pl.read_csv(DATA + "coffee.csv")

base = ggplot(coffee, aes("weekday", "cups", fill="drink")) + geom_bar(stat="identity")
plots = [
    base + complete_theme() + ggtitle(name) + theme(legend_position="none")
    for name, complete_theme in [
        ("theme_grey", theme_grey),
        ("theme_classic", theme_classic),
        ("theme_minimal", theme_minimal),
        ("theme_bw", theme_bw),
    ]
]
plot = gggrid(plots, ncol=2) + ggsize(760, 520)
plot
# --8<-- [end:code]
