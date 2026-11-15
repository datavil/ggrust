"""Residual plot

Highway mileage against engine size: the residuals of a straight line bend, those of
a quadratic do not. `residual_plot` (from `ggrust.bistro`, as lets-plot's) fits the
model and plots what is left over.
"""

# --8<-- [start:code]
from ggrust import *
from ggrust.bistro import residual_plot
from ggrust.data import mpg

linear = residual_plot(mpg, "displ", "hwy") + ggtitle("Straight line")
quadratic = residual_plot(mpg, "displ", "hwy", deg=2) + ggtitle("Quadratic")
plot = gggrid([linear, quadratic], ncol=2) + ggsize(900, 420)
plot
# --8<-- [end:code]
