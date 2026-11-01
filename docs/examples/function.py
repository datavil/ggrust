"""Plotting a function

`geom_function` samples any Python function over a range.
"""

# --8<-- [start:code]
import math

from ggrust import *

plot = (
    ggplot()
    + geom_function(
        fun=lambda x: math.sin(x) * math.exp(-x / 6), xlim=[0, 20], n=400, color="#2a9d8f", size=1.2
    )
    + geom_function(
        fun=lambda x: math.exp(-x / 6), xlim=[0, 20], linetype="dashed", color="#999999"
    )
    + geom_function(
        fun=lambda x: -math.exp(-x / 6), xlim=[0, 20], linetype="dashed", color="#999999"
    )
    + labs(x="t", y="sin(t) · exp(-t / 6)")
    + theme_classic()
    + ggsize(600, 340)
)
plot
# --8<-- [end:code]
