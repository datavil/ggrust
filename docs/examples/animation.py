"""Animation

Palmer penguins, one year per frame: `transition_manual` follows R's gganimate, and
the title shows the year on each frame.
"""

# --8<-- [start:code]
from ggrust import *
from ggrust.data import penguins

plot = (
    ggplot(penguins, aes("flipper_length_mm", "body_mass_g", color="species"))
    + geom_point(size=3, alpha=0.8, na_rm=True)
    + labs(
        title="Palmer penguins, {current_frame}",
        x="flipper length (mm)",
        y="body mass (g)",
    )
    + transition_manual("year")
    + theme_light()
    + ggsize(700, 450)
)
animation = animate(plot, nframes=3, fps=1)
animation
# --8<-- [end:code]
