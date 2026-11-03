"""Venn diagram

Three overlapping sets drawn as points sized in x units (`size_unit="x"`), with
identity fill colors and labels. A remake of lets-plot's Venn diagram demo.
"""

# --8<-- [start:code]
import math

from ggrust import *

radius = 0.5
height = radius * math.sqrt(3) / 2
sets = {
    "x": [0, radius, radius / 2],
    "y": [0, 0, height],
    "size": [2 * radius] * 3,
    "color": ["#e24462", "#306998", "#ffe873"],
}
labels = {
    "x": [
        -radius / 2,
        1.5 * radius,
        radius / 2,
        radius / 2,
        -radius / 8,
        9 * radius / 8,
        radius / 2,
    ],
    "y": [
        -radius / 4,
        -radius / 4,
        height + radius / 2,
        -3 * radius / 8,
        5 * radius / 8,
        5 * radius / 8,
        5 * radius / 16,
    ],
    "text": [
        "Rust",
        "Python",
        "Grammar\nof\nGraphics",
        "PyO3",
        "ggrust-core",
        "plotnine",
        "ggrust",
    ],
    "size": [0.4 * radius] * 7,
}

plot = (
    ggplot()
    + geom_point(
        aes("x", "y", size="size", fill="color"),
        data=sets,
        size_unit="x",
        shape=21,
        alpha=0.5,
        color="black",
        tooltips="none",
    )
    + geom_text(aes("x", "y", label="text", size="size"), data=labels, size_unit="x")
    + scale_size_identity()
    + scale_fill_identity()
    + xlim(-radius, 2 * radius)
    + ylim(-radius, height + radius)
    + coord_fixed()
    + theme_void()
    + ggsize(600, 600)
)
plot
# --8<-- [end:code]
