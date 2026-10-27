"""``ggsave()``: write a plot to a file chosen by its extension.

Ported from lets-plot's ``lets_plot/export/ggsave_.py`` (MIT, see THIRD_PARTY_NOTICES.md).
Supported: svg, png, html/htm. pdf arrives in plan.md step 5.3.
"""

from __future__ import annotations

import os

from ggrust.plot.core import PlotSpec
from ggrust.plot.subplots import SupPlotsSpec

__all__ = ["ggsave"]

_DEFAULT_EXPORT_DIRECTORY = "lets-plot-images"  # lets_plot's default, kept for compatibility


def ggsave(
    plot: PlotSpec | SupPlotsSpec,
    filename: str,
    *,
    path: str | None = None,
    iframe: bool = True,
    scale: float | None = None,
    w: float | None = None,
    h: float | None = None,
    unit: str | None = None,
    dpi: int | None = None,
) -> str:
    """Save a plot to ``path/filename``; the extension picks the format.

    Parameters
    ----------
    plot : PlotSpec or SupPlotsSpec
        The figure.
    filename : str
        File name with extension: ``.svg``, ``.png``, ``.html`` or ``.htm``.
    path : str, optional
        Directory; ``./lets-plot-images`` when None (as lets_plot).
    iframe : bool, default True
        Accepted for lets_plot compatibility; ignored.
    scale : float, optional
        PNG pixel scale.
    w, h : float, optional
        Override the plot size (svg, png).
    unit : {"px"}, optional
        Unit of ``w`` and ``h``.
    dpi : int, optional
        Not supported yet; raises when given (png).

    Returns
    -------
    str
        Absolute path of the written file.
    """
    from ggrust._render import export_html, export_png, export_svg

    if not isinstance(plot, PlotSpec | SupPlotsSpec):
        message = f"PlotSpec or SupPlotsSpec expected but was: {type(plot)}"
        raise ValueError(message)
    filename = filename.strip()
    name, extension = os.path.splitext(filename)
    if not name:
        message = f"Malformed filename: '{filename}'."
        raise ValueError(message)
    if not extension:
        message = f"Missing file extension: '{filename}'."
        raise ValueError(message)
    pathname = os.path.join(path or os.path.join(os.getcwd(), _DEFAULT_EXPORT_DIRECTORY), filename)
    extension = extension[1:].lower()
    if extension == "svg":
        return export_svg(plot, pathname, w=w, h=h, unit=unit)
    if extension in ["html", "htm"]:
        return export_html(plot, pathname)
    if extension == "png":
        return export_png(plot, pathname, scale=scale, w=w, h=h, unit=unit, dpi=dpi)
    message = f"Unsupported file extension: '{extension}'. Use one of: 'svg', 'png', 'html', 'htm'."
    raise ValueError(message)
