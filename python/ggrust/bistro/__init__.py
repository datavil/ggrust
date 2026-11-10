"""Prebuilt plots, as ``lets_plot.bistro``: ``from ggrust.bistro import corr_plot``.

``funky_heatmap`` ports the R package funkyheatmap (not part of lets-plot).

Not yet: ``joint_plot`` (needs ``ggmarginal``), ``qq_plot`` (needs ``geom_qq``),
``image_matrix``, ``residual_plot``, ``waterfall_plot``.
"""

from ggrust.bistro.corr import corr_plot
from ggrust.bistro.funky_heatmap import PositionArguments, funky_heatmap, position_arguments

__all__ = ["PositionArguments", "corr_plot", "funky_heatmap", "position_arguments"]
