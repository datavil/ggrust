"""Animated reveal

US unemployment revealed month by month: `transition_reveal` grows the line along
`date` with a point at its moving end, as in R's gganimate.
"""

# --8<-- [start:code]
from ggrust import *
from ggrust.data import economics

plot = (
    ggplot(economics, aes("date", "unemploy"))
    + geom_line(color="#1f77b4", size=1)
    + geom_point(color="#d62728", size=4)
    + labs(title="US unemployment, {frame_along}", x="", y="unemployed (thousands)")
    + transition_reveal("date")
    + theme_light()
    + ggsize(640, 400)
)
animation = animate(plot, nframes=100, fps=20, end_pause=20)
animation
# --8<-- [end:code]
