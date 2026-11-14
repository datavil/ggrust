"""Prebuilt plots, as ``lets_plot.bistro``: ``from ggrust.bistro import corr_plot``.

``corr_plot``, ``joint_plot`` and ``qq_plot`` follow lets-plot (also importable from
``ggrust.bistro.corr``, ``ggrust.bistro.joint`` and ``ggrust.bistro.qq``).
``funky_heatmap`` ports the R package funkyheatmap (not part of lets-plot).

Not yet: ``image_matrix``, ``residual_plot``, ``waterfall_plot``.
"""

from ggrust.bistro.corr import corr_plot
from ggrust.bistro.funky_heatmap import PositionArguments, funky_heatmap, position_arguments
from ggrust.bistro.joint import joint_plot
from ggrust.bistro.qq import qq_plot

__all__ = [
    "PositionArguments",
    "corr_plot",
    "funky_heatmap",
    "joint_plot",
    "position_arguments",
    "qq_plot",
]
