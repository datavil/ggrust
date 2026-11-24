# ruff: noqa: F403, F405
"""Prototype demo: draw ggrust plots inline in a terminal.

Ghostty, Kitty and WezTerm get images; other terminals get braille text.
Run:  uv run python examples/terminal_demo.py [auto|kitty|braille]
"""

import sys

from ggrust import *
from ggrust._terminal import show_in_terminal
from ggrust.data import mpg, penguins

mode = sys.argv[1] if len(sys.argv) > 1 else "auto"

print("ggrust in the terminal: scatter of engine size vs highway mpg")
show_in_terminal(
    ggplot(mpg, aes("displ", "hwy", color="class"))
    + geom_point(size=3)
    + ggtitle("Bigger engines, fewer miles per gallon"),
    mode=mode,
)

print("and a density of penguin flipper length by species")
show_in_terminal(
    ggplot(penguins, aes("flipper_length_mm", fill="species"))
    + geom_density(alpha=0.5)
    + theme_minimal(),
    mode=mode,
)
