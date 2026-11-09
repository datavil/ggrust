"""3D surface

The classic matplotlib `plot_surface` demo, z = sin(sqrt(x² + y²)), drawn with
`geom_surface` under `coord_3d`: grid cells are filled by height and drawn back
to front. ggrust only: lets-plot and ggplot2 have no 3D.
"""

# --8<-- [start:code]
import numpy as np
from ggrust import *

axis = np.linspace(-10, 10, 40)
x, y = np.meshgrid(axis, axis)
z = np.sin(np.sqrt(x**2 + y**2))
data = {"x": x.ravel(), "y": y.ravel(), "z": z.ravel()}

plot = (
    ggplot(data, aes("x", "y", z="z", fill="z"))
    + geom_surface(alpha=0.8)
    + scale_fill_gradient(low="#00FFFF", high="#FF00FF")
    + coord_3d()
    + ggtitle("3D Contour Plot")
    + ggsize(700, 560)
)
plot
# --8<-- [end:code]
