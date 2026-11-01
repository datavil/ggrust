"""One hundred thousand points

Dense layers are drawn on a canvas in HTML, so zoom and pan stay fast.
"""

# --8<-- [start:code]
import ggrust as gg
import numpy as np
import polars as pl

generator = np.random.default_rng(7)
n = 100_000
angle = generator.uniform(0, 2 * np.pi, n)
radius = generator.normal(1.0, 0.15, n) + 0.3 * np.sin(5 * angle)
frame = pl.DataFrame(
    {
        "x": radius * np.cos(angle),
        "y": radius * np.sin(angle),
        "angle": angle,
    }
)

plot = (
    gg.ggplot(frame, gg.aes("x", "y", color="angle"))
    + gg.geom_point(size=0.5, alpha=0.3, tooltips="none")
    + gg.scale_color_gradientn(colors=["#264653", "#2a9d8f", "#e9c46a", "#f4a261", "#e76f51"])
    + gg.coord_fixed()
    + gg.theme_void()
    + gg.theme(legend_position="none")
    + gg.ggtb()
    + gg.ggsize(500, 500)
)
plot
# --8<-- [end:code]
