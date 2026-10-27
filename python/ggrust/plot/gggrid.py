"""``gggrid()``: arrange plots in a grid.

Ported from lets-plot's ``lets_plot/plot/gggrid_.py`` (MIT, see THIRD_PARTY_NOTICES.md).
Global themes (``theme_set``) are not ported, so figures keep their own themes.
"""

from ggrust.plot.subplots import SupPlotsLayoutSpec, SupPlotsSpec

__all__ = ["gggrid"]


def gggrid(
    plots: list,
    ncol: int | None = None,
    *,
    sharex: str | bool | None = None,
    sharey: str | bool | None = None,
    widths: list | None = None,
    heights: list | None = None,
    hspace: float | None = None,
    vspace: float | None = None,
    fit: bool | None = None,
    align: bool | None = None,
    guides: str | None = None,
) -> SupPlotsSpec:
    """Combine plots into a grid.

    Parameters
    ----------
    plots : list
        ``PlotSpec`` or ``SupPlotsSpec`` objects; None leaves a cell empty.
    ncol : int, optional
        Number of columns; all plots in one row when None.
    sharex, sharey : bool or {"all", "none", "row", "col"}, optional
        Share axis limits between plots.
    widths, heights : list of float, optional
        Relative column widths and row heights.
    hspace, vspace : float, optional
        Gaps between cells in pixels.
    fit : bool, optional
        Stretch each plot to its cell.
    align : bool, optional
        Align inner panels.
    guides : {"auto", "collect", "keep"}, optional
        Whether identical legends are collected into one.

    Returns
    -------
    SupPlotsSpec
        The grid.
    """
    if not len(plots):
        message = "Supplots list is empty."
        raise ValueError(message)
    if ncol is None:
        ncol = len(plots)
        nrow = 1
    else:
        extended = plots + [None] * (ncol - 1)
        nrow = len(extended) // ncol
        plots = extended[0 : ncol * nrow]
    if sharex is not None and not isinstance(sharex, str):
        sharex = "all" if sharex else "none"
    if sharey is not None and not isinstance(sharey, str):
        sharey = "all" if sharey else "none"
    layout = SupPlotsLayoutSpec(
        name="grid",
        ncol=ncol,
        nrow=nrow,
        sharex=sharex,
        sharey=sharey,
        widths=widths,
        heights=heights,
        hspace=hspace,
        vspace=vspace,
        fit=fit,
        align=align,
        guides=guides,
    )
    return SupPlotsSpec(figures=plots, layout=layout)
