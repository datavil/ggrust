"""Other spellings of public functions, for code moving from ggplot2 or plotnine.

Each alias is the same function object as its target (``scale_colour_hue is
scale_color_hue``), so it takes the same arguments and builds the same spec.
``ALIASES`` maps every alias to its target's name; the API reference lists them from it.
lets_plot has none of these names.
"""

from __future__ import annotations

from ggrust.plot.coord import coord_fixed
from ggrust.plot.geom import geom_bin2d, geom_contourf, geom_density2d, geom_density2df
from ggrust.plot.geom_function import geom_function
from ggrust.plot.guide import guide_colorbar
from ggrust.plot.scale import scale_x_datetime, scale_x_time, scale_y_datetime, scale_y_time
from ggrust.plot.scale_color import (
    scale_color_brewer,
    scale_color_continuous,
    scale_color_discrete,
    scale_color_gradient,
    scale_color_gradient2,
    scale_color_gradientn,
    scale_color_grey,
    scale_color_hue,
    scale_color_manual,
    scale_color_viridis,
    scale_fill_grey,
)
from ggrust.plot.scale_colormap_mpl import scale_color_cmapmpl
from ggrust.plot.scale_extra import (
    scale_alpha_ordinal,
    scale_color_desaturate,
    scale_color_distiller,
    scale_color_ordinal,
    scale_size_ordinal,
)
from ggrust.plot.scale_ggsci import (
    scale_color_aaas,
    scale_color_d3,
    scale_color_igv,
    scale_color_jama,
    scale_color_jco,
    scale_color_lancet,
    scale_color_nejm,
    scale_color_npg,
)
from ggrust.plot.scale_identity import scale_color_identity
from ggrust.plot.scale_shape import scale_linetype, scale_shape
from ggrust.plot.scale_size import scale_alpha, scale_linewidth, scale_size, scale_stroke
from ggrust.plot.stat_layer import stat_bin2d, stat_density2d

__all__ = [
    "coord_equal",
    "geom_bin_2d",
    "geom_contour_filled",
    "geom_density_2d",
    "geom_density_2d_filled",
    "guide_colourbar",
    "scale_alpha_continuous",
    "scale_alpha_discrete",
    "scale_color_gray",
    "scale_colour_aaas",
    "scale_colour_brewer",
    "scale_colour_cmapmpl",
    "scale_colour_continuous",
    "scale_colour_d3",
    "scale_colour_desaturate",
    "scale_colour_discrete",
    "scale_colour_distiller",
    "scale_colour_gradient",
    "scale_colour_gradient2",
    "scale_colour_gradientn",
    "scale_colour_gray",
    "scale_colour_grey",
    "scale_colour_hue",
    "scale_colour_identity",
    "scale_colour_igv",
    "scale_colour_jama",
    "scale_colour_jco",
    "scale_colour_lancet",
    "scale_colour_manual",
    "scale_colour_nejm",
    "scale_colour_npg",
    "scale_colour_ordinal",
    "scale_colour_viridis",
    "scale_fill_gray",
    "scale_linetype_discrete",
    "scale_linewidth_continuous",
    "scale_shape_discrete",
    "scale_size_continuous",
    "scale_size_discrete",
    "scale_stroke_continuous",
    "scale_x_date",
    "scale_x_timedelta",
    "scale_y_date",
    "scale_y_timedelta",
    "stat_bin_2d",
    "stat_density_2d",
    "stat_function",
]

scale_colour_aaas = scale_color_aaas
scale_colour_brewer = scale_color_brewer
scale_colour_cmapmpl = scale_color_cmapmpl
scale_colour_continuous = scale_color_continuous
scale_colour_d3 = scale_color_d3
scale_colour_desaturate = scale_color_desaturate
scale_colour_discrete = scale_color_discrete
scale_colour_distiller = scale_color_distiller
scale_colour_gradient = scale_color_gradient
scale_colour_gradient2 = scale_color_gradient2
scale_colour_gradientn = scale_color_gradientn
scale_colour_grey = scale_color_grey
scale_colour_hue = scale_color_hue
scale_colour_identity = scale_color_identity
scale_colour_igv = scale_color_igv
scale_colour_jama = scale_color_jama
scale_colour_jco = scale_color_jco
scale_colour_lancet = scale_color_lancet
scale_colour_manual = scale_color_manual
scale_colour_nejm = scale_color_nejm
scale_colour_npg = scale_color_npg
scale_colour_ordinal = scale_color_ordinal
scale_colour_viridis = scale_color_viridis
scale_colour_gray = scale_color_grey
scale_color_gray = scale_color_grey
scale_fill_gray = scale_fill_grey
guide_colourbar = guide_colorbar
geom_bin_2d = geom_bin2d
geom_density_2d = geom_density2d
geom_density_2d_filled = geom_density2df
geom_contour_filled = geom_contourf
stat_function = geom_function
stat_bin_2d = stat_bin2d
stat_density_2d = stat_density2d
coord_equal = coord_fixed
scale_x_date = scale_x_datetime
scale_y_date = scale_y_datetime
scale_x_timedelta = scale_x_time
scale_y_timedelta = scale_y_time
scale_alpha_continuous = scale_alpha
scale_alpha_discrete = scale_alpha_ordinal
scale_size_continuous = scale_size
scale_size_discrete = scale_size_ordinal
scale_linewidth_continuous = scale_linewidth
scale_stroke_continuous = scale_stroke
scale_shape_discrete = scale_shape
scale_linetype_discrete = scale_linetype

ALIASES: dict[str, str] = {name: globals()[name].__name__ for name in __all__}
"""Alias name -> name of the function it points at."""
