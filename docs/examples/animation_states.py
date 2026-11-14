"""Animated states

Penguins counted per year: `transition_states` holds each year, then the bars grow and
shrink into the next one, eased by `ease_aes` as in R's gganimate.
"""

# --8<-- [start:code]
from ggrust import *
from ggrust.data import penguins

plot = (
    ggplot(penguins, aes("species", fill="species"))
    + geom_bar(width=0.7)
    + labs(title="Penguins counted in {closest_state}", x="", y="penguins")
    + transition_states("year", transition_length=2, state_length=1)
    + ease_aes("cubic-in-out")
    + theme_light()
    + ggsize(600, 420)
)
animation = animate(plot, nframes=60, fps=20)
animation
# --8<-- [end:code]
