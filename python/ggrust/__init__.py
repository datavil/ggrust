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
    xlab,
    ylab,
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
    "xlab",
    "ylab",
]
