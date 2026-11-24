# ruff: noqa: F403, F405
"""Prototype demo: draw ggrust plots inline in Ghostty (or Kitty, WezTerm).

Run it in a Ghostty window:  uv run python examples/terminal_demo.py
"""

from ggrust import *
from ggrust._terminal import show_in_terminal
from ggrust.data import mpg, penguins

print("ggrust in the terminal: scatter of engine size vs highway mpg")
show_in_terminal(
    ggplot(mpg, aes("displ", "hwy", color="class"))
    + geom_point(size=3)
    + ggtitle("Bigger engines, fewer miles per gallon")
)

print("and a density of penguin flipper length by species")
show_in_terminal(
    ggplot(penguins, aes("flipper_length_mm", fill="species"))
    + geom_density(alpha=0.5)
    + theme_minimal()
)
