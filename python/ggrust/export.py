"""``ggsave()``: write a plot to a file chosen by its extension.

Ported from lets-plot's ``lets_plot/export/ggsave_.py`` (MIT, see THIRD_PARTY_NOTICES.md).
Supported: svg, png, pdf (vector), html/htm.
"""

from __future__ import annotations

import os

from ggrust.errors import ExportError, did_you_mean
from ggrust.plot.core import PlotSpec
from ggrust.plot.subplots import SupPlotsSpec

__all__ = ["ggsave"]

_DEFAULT_EXPORT_DIRECTORY = "lets-plot-images"  # lets_plot's default, kept for compatibility


def ggsave(
    plot: PlotSpec | SupPlotsSpec,
    filename: str | os.PathLike[str],
    *,
    path: str | os.PathLike[str] | None = None,
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
        File name with extension: ``.svg``, ``.png``, ``.pdf``, ``.html`` or ``.htm``.
    path : str, optional
        Directory; ``./lets-plot-images`` when None (as lets_plot).
    iframe : bool, default True
        HTML only: write an ``<iframe srcdoc>`` sized to the plot (as lets_plot).
    scale : float, optional
        PNG / PDF pixel scale (see ``PlotSpec.to_png``).
    w, h : float, optional
        Output size (svg, png, pdf); give both.
    unit : {"in", "cm", "mm", "px"}, optional
        Unit of ``w`` and ``h``; inches when None.
    dpi : int, optional
        PNG / PDF resolution; 300 by default for physical sizes.

    Returns
    -------
    str
        Absolute path of the written file.
    """
    from ggrust._render import export_html, export_pdf, export_png, export_svg

    if not isinstance(plot, PlotSpec | SupPlotsSpec):
        message = (
            f"ggsave(): 'plot' must be a PlotSpec or SupPlotsSpec (from ggplot() or gggrid()), "
            f"got {type(plot).__name__}"
        )
        raise ExportError(message)
    filename = os.fspath(filename).strip()
    name, extension = os.path.splitext(filename)
    if not name:
        message = f"ggsave(): filename {filename!r} has no name; pass e.g. 'plot.png'"
        raise ExportError(message)
    if not extension:
        message = (
            f"ggsave(): filename {filename!r} has no extension; add one of "
            "'.svg', '.png', '.pdf', '.html' (e.g. 'plot.png')"
        )
        raise ExportError(message)
    pathname = os.path.join(path or os.path.join(os.getcwd(), _DEFAULT_EXPORT_DIRECTORY), filename)
    extension = extension[1:].lower()
    if extension == "svg":
        export_svg(plot, pathname, w=w, h=h, unit=unit)
    elif extension in ["html", "htm"]:
        export_html(plot, pathname, iframe)
    elif extension == "png":
        export_png(plot, pathname, scale=scale, w=w, h=h, unit=unit, dpi=dpi)
    elif extension == "pdf":
        export_pdf(plot, pathname, scale=scale, w=w, h=h, unit=unit, dpi=dpi)
    else:
        extensions = ["svg", "png", "pdf", "html", "htm"]
        message = (
            f"ggsave(): unsupported file extension {extension!r}; use one of 'svg', 'png', "
            f"'pdf', 'html', 'htm'{did_you_mean(extension, extensions)}"
        )
        raise ExportError(message)
    return os.path.abspath(pathname)
