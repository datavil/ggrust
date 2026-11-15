"""Joint plot

Bill length against bill depth of the Palmer penguins, with the distribution of each
on the margins: `joint_plot` is one of the prebuilt plots of `ggrust.bistro`, as
lets-plot's.
"""

# --8<-- [start:code]
from ggrust import *
from ggrust.bistro import joint_plot
from ggrust.data import penguins

plot = (
    joint_plot(
        penguins.drop_nulls("bill_length_mm"),
        "bill_length_mm",
        "bill_depth_mm",
        color_by="species",
        marginal="dens:tr",
    )
    + scale_color_jco()
    + scale_fill_jco()
    + labs(x="bill length (mm)", y="bill depth (mm)")
    + ggsize(650, 550)
)
plot
# --8<-- [end:code]
