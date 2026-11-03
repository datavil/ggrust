"""Prebuilt plots, as ``lets_plot.bistro``: ``from ggrust.bistro.corr import corr_plot``.

Not yet: ``joint_plot`` (needs ``ggmarginal``), ``qq_plot`` (needs ``geom_qq``),
``image_matrix``, ``residual_plot``, ``waterfall_plot``.
"""

from ggrust.bistro.corr import corr_plot

__all__ = ["corr_plot"]
