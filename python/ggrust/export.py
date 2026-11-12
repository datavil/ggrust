"""``ggsave()``: write a plot to a file chosen by its extension.

Ported from lets-plot's ``lets_plot/export/ggsave_.py`` (MIT, see THIRD_PARTY_NOTICES.md).
Supported: svg, png, jpg/jpeg, tif/tiff, pdf (vector), html/htm, and ggrs (a plot file
``ggload`` opens). JPEG and TIFF are ggplot2 formats lets-plot does not write.
"""

from __future__ import annotations

import os

from ggrust.errors import ExportError, did_you_mean
from ggrust.plot.core import PlotSpec
from ggrust.plot.subplots import SupPlotsSpec

__all__ = ["ggsave"]


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
    quality: int | None = None,
) -> str:
    """Save a plot to ``filename`` (inside ``path`` if given); the extension picks the format.

    Parameters
    ----------
    plot : PlotSpec or SupPlotsSpec
        The figure.
    filename : str
        File name with extension: ``.svg``, ``.png``, ``.jpg`` / ``.jpeg``, ``.tif`` /
        ``.tiff``, ``.pdf``, ``.html``, ``.htm``, or ``.ggrs`` for a plot file that
        ``ggload`` opens again: the whole plot with its data and registered fonts, never
        code (a plot holding a Python function, such as a ``sec_axis`` transform, raises
        ``ExportError``).
    path : str, optional
        Directory to join in front of ``filename``. When None, ``filename`` is used as
        given: relative to the current working directory, or absolute. lets_plot writes
        into ``./lets-plot-images`` instead.
    iframe : bool, default True
        HTML only: write an ``<iframe srcdoc>`` sized to the plot (as lets_plot).
    scale : float, optional
        PNG / JPEG / TIFF / PDF pixel scale (see ``PlotSpec.to_png``).
    w, h : float, optional
        Output size (svg, png, jpeg, tiff, pdf); give both.
    unit : {"in", "cm", "mm", "px"}, optional
        Unit of ``w`` and ``h``; inches when None.
    dpi : int, optional
        PNG / JPEG / TIFF / PDF resolution; 300 by default for physical sizes. Raster
        files record it (TIFF in ``XResolution``, JPEG in the JFIF header).
    quality : int, optional
        JPEG only: quality from 1 to 100; 75 (R's ``jpeg()`` default, as ggplot2's
        ``ggsave``) when None. Raise it to avoid blur around text.

    Raises
    ------
    ExportError
        If ``plot`` is not a plot, the extension is missing or unsupported, or an
        option does not apply to the format.

    Returns
    -------
    str
        Absolute path of the written file.

    Examples
    --------
    >>> from ggrust import *
    >>> import tempfile
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> plot = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> path = ggsave(plot, "fuel.svg", path=tempfile.mkdtemp())

    A TIFF at 600 dpi, 3 by 2 inches (1800 x 1200 pixels):

    >>> path = ggsave(plot, "fuel.tiff", path=tempfile.mkdtemp(), w=3, h=2, dpi=600)
    """
    from ggrust._render import (
        export_html,
        export_pdf,
        export_png,
        export_raster,
        export_svg,
    )

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
            "'.svg', '.png', '.jpg', '.tiff', '.pdf', '.html', '.ggrs' (e.g. 'plot.png')"
        )
        raise ExportError(message)
    pathname = filename if path is None else os.path.join(path, filename)
    extension = extension[1:].lower()
    if quality is not None and extension not in ["jpg", "jpeg"]:
        message = f"ggsave(): 'quality' applies to .jpg / .jpeg only, not .{extension}; remove it"
        raise ExportError(message)
    if extension == "svg":
        export_svg(plot, pathname, w=w, h=h, unit=unit)
    elif extension in ["html", "htm"]:
        export_html(plot, pathname, iframe)
    elif extension == "png":
        export_png(plot, pathname, scale=scale, w=w, h=h, unit=unit, dpi=dpi)
    elif extension in ["jpg", "jpeg"]:
        export_raster(
            plot, pathname, "jpeg", scale=scale, w=w, h=h, unit=unit, dpi=dpi, quality=quality
        )
    elif extension in ["tif", "tiff"]:
        export_raster(plot, pathname, "tiff", scale=scale, w=w, h=h, unit=unit, dpi=dpi)
    elif extension == "pdf":
        export_pdf(plot, pathname, scale=scale, w=w, h=h, unit=unit, dpi=dpi)
    elif extension == "ggrs":
        from ggrust.plot_file import write_plot_file

        write_plot_file(plot, pathname)
    else:
        extensions = ["svg", "png", "jpg", "jpeg", "tif", "tiff", "pdf", "html", "htm", "ggrs"]
        message = (
            f"ggsave(): unsupported file extension {extension!r}; use one of 'svg', 'png', "
            f"'jpg', 'jpeg', 'tif', 'tiff', 'pdf', 'html', 'htm', "
            f"'ggrs'{did_you_mean(extension, extensions)}"
        )
        raise ExportError(message)
    return os.path.abspath(pathname)
