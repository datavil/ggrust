"""Colorbar

Body mass of the Palmer penguins on a colorbar with pointed caps, minor ticks, a
frame and its title on the side: `guide_colorbar` and the `legend_*` theme options
follow ggplot2 3.5, the caps matplotlib's `colorbar(extend=)`.
"""

# --8<-- [start:code]
from ggrust import *
from ggrust.data import penguins

plot = (
    ggplot(penguins, aes("flipper_length_mm", "bill_length_mm", color="body_mass_g"))
    + geom_point(size=3, na_rm=True)
    + scale_color_viridis(
        limits=[3000, 6000],
        guide=guide_colorbar(
            "body mass (g)",
            barheight=260,
            shape="triangle",
            cap_size=14,
            minor_ticks=True,
        ),
    )
    + labs(x="flipper length (mm)", y="bill length (mm)")
    + theme_light()
    + theme(
        legend_frame=element_rect(color="black", size=0.6),
        legend_ticks=element_line(color="black"),
        legend_ticks_length=[-5, 0],
    )
    + ggsize(700, 450)
)
plot
# --8<-- [end:code]
