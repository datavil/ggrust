"""Pairwise tests within groups

Body mass of female and male penguins, compared within each species: `geom_pwc` runs
the tests and draws the brackets between the dodged boxes, as ggpubr's.
"""

# --8<-- [start:code]
from ggrust import *
from ggrust.data import penguins

known = penguins.drop_nulls(["sex", "body_mass_g"]).sort("sex")
plot = (
    ggplot(known, aes("species", "body_mass_g", fill="sex"))
    + geom_boxplot(alpha=0.7)
    + geom_pwc(method="t_test", label="p.adj.signif")
    + scale_fill_npg()
    + labs(x="", y="body mass (g)")
    + theme_classic()
    + ggsize(700, 480)
)
plot
# --8<-- [end:code]
