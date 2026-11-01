"""Built-in themes

The same plot in four built-in themes.
"""

# --8<-- [start:code]
import ggrust as gg
import polars as pl

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
coffee = pl.read_csv(DATA + "coffee.csv")

base = gg.ggplot(coffee, gg.aes("weekday", "cups", fill="drink")) + gg.geom_bar(stat="identity")
plots = [
    base + theme() + gg.ggtitle(name) + gg.theme(legend_position="none")
    for name, theme in [
        ("theme_grey", gg.theme_grey),
        ("theme_classic", gg.theme_classic),
        ("theme_minimal", gg.theme_minimal),
        ("theme_bw", gg.theme_bw),
    ]
]
plot = gg.gggrid(plots, ncol=2) + gg.ggsize(760, 520)
plot
# --8<-- [end:code]
