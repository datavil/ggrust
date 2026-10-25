"""ggrust: grammar-of-graphics plotting in Rust with a lets_plot-compatible API.

Use it like lets_plot::

    from ggrust import *

    ggplot(frame, aes("UMAP1", "UMAP2")) + geom_point(size=1) + ggtitle("Cells")
"""

from importlib.metadata import version as _distribution_version

from ggrust._ggrust import engine_version
from ggrust._lets_plot import LetsPlot
from ggrust._render import GgrustWarning
from ggrust.mapping import as_discrete
from ggrust.plot import (
    aes,
    geom_point,
    ggplot,
    ggsize,
    ggtitle,
    guides,
    labs,
    layer,
    layer_tooltips,
    lims,
    scale_x_continuous,
    scale_x_log2,
    scale_x_log10,
    scale_x_reverse,
    scale_y_continuous,
    scale_y_log2,
    scale_y_log10,
    scale_y_reverse,
    xlab,
    xlim,
    ylab,
    ylim,
)

__version__ = _distribution_version("ggrust")

__all__ = [
    "GgrustWarning",
    "LetsPlot",
    "__version__",
    "aes",
    "as_discrete",
    "engine_version",
    "geom_point",
    "ggplot",
    "ggsize",
    "ggtitle",
    "guides",
    "labs",
    "layer",
    "layer_tooltips",
    "lims",
    "scale_x_continuous",
    "scale_x_log2",
    "scale_x_log10",
    "scale_x_reverse",
    "scale_y_continuous",
    "scale_y_log2",
    "scale_y_log10",
    "scale_y_reverse",
    "xlab",
    "xlim",
    "ylab",
    "ylim",
]
