"""Significance brackets

Flipper length of three penguin species, with every pair compared by a Wilcoxon
test. `stat_compare_means` computes the p-values and draws the brackets, as R's
ggpubr does; ggrust's numbers match R's `wilcox.test`.
"""

# --8<-- [start:code]
import polars as pl
from ggrust import *

DATA = "https://raw.githubusercontent.com/datavil/ggrust/main/docs/data/"
penguins = pl.read_csv(DATA + "penguins.csv").drop_nulls("flipper_length_mm")

plot = (
    ggplot(penguins, aes("species", "flipper_length_mm"))
    + geom_boxplot(aes(fill="species"), alpha=0.6, outlier_alpha=0, show_legend=False)
    + geom_quasirandom(aes(color="species"), size=1.2, width=0.25, show_legend=False)
    + stat_compare_means(
        comparisons=[("Adelie", "Chinstrap"), ("Chinstrap", "Gentoo"), ("Adelie", "Gentoo")],
        label="p.signif",
    )
    + scale_fill_jco()
    + scale_color_jco()
    + labs(x="", y="flipper length (mm)")
    + theme_classic()
    + ggsize(800, 550)
)
plot
# --8<-- [end:code]
