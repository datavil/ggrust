"""Manual legend

Three functions, each with its own key in a manual legend (`manual_key=` with
`layer_key()`), placed inside the panel on a dark flavor. A remake of lets-plot's
math manual legend demo.
"""

# --8<-- [start:code]
import numpy as np
from ggrust import *

colors = ["#FFB6C1", "#AAFFAA", "#ADD8E6"]

plot = (
    ggplot()
    + geom_vline(xintercept=0, size=0.4)
    + geom_hline(yintercept=0, size=0.4)
    + geom_function(
        fun=lambda x: np.sin(3 * x) / (x * (np.cos(x) + 2)),
        xlim=[0.1, 12],
        n=300,
        color=colors[0],
        size=2,
        manual_key=layer_key("sin(3x)\n--------\nx(cos(x) + 2)", size=3),
    )
    + geom_function(
        fun=lambda x: np.cos(x) / x,
        xlim=[0.1, 12],
        n=300,
        color=colors[1],
        na_rm=True,
        manual_key=layer_key("cos(x) / x", size=3),
    )
    + geom_function(
        fun=lambda x: np.exp(-x),
        xlim=[0.1, 12],
        n=300,
        color=colors[2],
        manual_key=layer_key("exp(-x)", size=3),
    )
    + scale_x_continuous(expand=[0, 0.2])
    + scale_y_continuous(limits=[-0.4, 1.0])
    + labs(x="x", y="f(x)", title="Mathematical functions")
    + theme_classic()
    + theme(
        text=element_text(face="italic"),
        plot_title=element_text(size=20, hjust=0.05),
        axis_title=element_text(size=18),
        legend_text=element_text(size=18),
        legend_background=element_rect(size=1),
        legend_position=[0.95, 0.9],
        legend_justification=[1, 1],
        axis_line="blank",
        axis_ticks="blank",
        panel_grid_major=element_line(linetype="longdash"),
    )
    + flavor_solarized_dark()
    + ggsize(900, 500)
)
plot
# --8<-- [end:code]
