"""lets_plot-compatible plot specification API."""

from ggrust.plot.core import aes, layer
from ggrust.plot.geom import geom_point
from ggrust.plot.guide import guides
from ggrust.plot.label import ggtitle, labs, xlab, ylab
from ggrust.plot.plot import ggplot, ggsize
from ggrust.plot.scale import (
    lims,
    scale_x_continuous,
    scale_x_discrete,
    scale_x_discrete_reversed,
    scale_x_log2,
    scale_x_log10,
    scale_x_reverse,
    scale_y_continuous,
    scale_y_discrete,
    scale_y_discrete_reversed,
    scale_y_log2,
    scale_y_log10,
    scale_y_reverse,
    xlim,
    ylim,
)
from ggrust.plot.tooltip import layer_tooltips

__all__ = [
    "aes",
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
    "scale_x_discrete",
    "scale_x_discrete_reversed",
    "scale_x_log2",
    "scale_x_log10",
    "scale_x_reverse",
    "scale_y_continuous",
    "scale_y_discrete",
    "scale_y_discrete_reversed",
    "scale_y_log2",
    "scale_y_log10",
    "scale_y_reverse",
    "xlab",
    "xlim",
    "ylab",
    "ylim",
]
