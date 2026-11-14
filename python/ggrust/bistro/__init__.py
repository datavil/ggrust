"""Prebuilt plots, as ``lets_plot.bistro``: ``from ggrust.bistro import corr_plot``.

``corr_plot``, ``image_matrix``, ``joint_plot``, ``qq_plot`` and ``residual_plot`` follow
lets-plot (also importable from ``ggrust.bistro.corr``, ``ggrust.bistro.im``,
``ggrust.bistro.joint``, ``ggrust.bistro.qq`` and ``ggrust.bistro.residual``).
``funky_heatmap`` ports the R package funkyheatmap (not part of lets-plot).

Not yet: ``waterfall_plot``.
"""

from ggrust.bistro.corr import corr_plot
from ggrust.bistro.funky_heatmap import PositionArguments, funky_heatmap, position_arguments
from ggrust.bistro.im import image_matrix
from ggrust.bistro.joint import joint_plot
from ggrust.bistro.qq import qq_plot
from ggrust.bistro.residual import residual_plot

__all__ = [
    "PositionArguments",
    "corr_plot",
    "funky_heatmap",
    "image_matrix",
    "joint_plot",
    "position_arguments",
    "qq_plot",
    "residual_plot",
]
