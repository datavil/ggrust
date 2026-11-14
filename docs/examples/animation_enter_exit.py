"""Enter, exit and shadows

Fruit on sale per year: bars of new fruit fade and grow in, fruit no longer sold fades
and shrinks out, as gganimate's `enter_fade`, `enter_grow`, `exit_fade` and
`exit_shrink` do.
"""

# --8<-- [start:code]
from ggrust import *

sales = {
    "year": [2021] * 3 + [2022] * 3 + [2023] * 3,
    "fruit": ["apple", "pear", "plum", "apple", "pear", "kiwi", "apple", "kiwi", "fig"],
    "sold": [10, 6, 4, 12, 8, 5, 15, 9, 7],
}
plot = (
    ggplot(sales, aes("fruit", "sold", fill="fruit"))
    + geom_col(width=0.7)
    + labs(title="Fruit sold in {closest_state}", x="", y="crates")
    + transition_states("year", transition_length=2, state_length=1)
    + enter_fade()
    + enter_grow()
    + exit_fade()
    + exit_shrink()
    + ease_aes("cubic-in-out")
    + theme_light()
    + ggsize(600, 420)
)
animation = animate(plot, nframes=60, fps=20)
animation
# --8<-- [end:code]
