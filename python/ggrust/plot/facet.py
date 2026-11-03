"""Faceting: ``facet_wrap`` and ``facet_grid``.

Ported from lets-plot's ``lets_plot/plot/facet.py`` (MIT, see THIRD_PARTY_NOTICES.md).
The Rust engine splits the data into panels and lays them out.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from ggrust.plot.core import FeatureSpec

if TYPE_CHECKING:
    from collections.abc import Sequence
    from typing import Literal


__all__ = ["facet_grid", "facet_wrap"]


def facet_grid(
    x: str | None = None,
    y: str | None = None,
    *,
    scales: Literal["fixed", "free", "free_x", "free_y"] | None = None,
    x_order: int = 1,
    y_order: int = 1,
    x_format: str | None = None,
    y_format: str | None = None,
    x_labwidth: int | None = None,
    y_labwidth: int | None = None,
) -> FeatureSpec:
    """Lay panels out in a grid: columns by ``x``, rows by ``y``.

    Every level combination gets a panel, empty ones too. Column strips sit above
    the top row, row strips right of the right column (``strip_text_y``,
    ``strip_background_y``, ``strip_spacing_x``).

    Parameters
    ----------
    x, y : str, optional
        Faceting columns: give one or both.
    scales : {"fixed", "free", "free_x", "free_y"}, optional
        Whether panels share scales. A free x scale is shared by the panels of a
        grid column, a free y scale by the panels of a grid row.
    x_order, y_order : {1, -1, 0}, default 1
        Level order: ascending, descending, or data order. Only ascending is
        rendered yet (other values warn).
    x_format, y_format : str, optional
        Strip label formats (not rendered yet; they warn).
    x_labwidth, y_labwidth : int, optional
        Strip label wrap width in characters (not rendered yet; they warn).

    Returns
    -------
    FeatureSpec
        The facet specification.
    """
    return FeatureSpec(
        "facet",
        name="grid",
        x=x,
        y=y,
        scales=scales,
        x_order=x_order,
        y_order=y_order,
        x_format=x_format,
        y_format=y_format,
        x_labwidth=x_labwidth,
        y_labwidth=y_labwidth,
    )


def facet_wrap(
    facets: str | Sequence[str],
    ncol: int | None = None,
    nrow: int | None = None,
    *,
    scales: Literal["fixed", "free", "free_x", "free_y"] | None = None,
    order: int = 1,
    format: str | None = None,
    drop: bool | None = None,
    dir: Literal["h", "v"] = "h",
    labwidth: int | Sequence[int] | None = None,
) -> FeatureSpec:
    """Wrap one panel per level of ``facets`` into rows.

    Parameters
    ----------
    facets : str or list of str
        Faceting columns.
    ncol, nrow : int, optional
        Number of columns and rows.
    scales : {"fixed", "free", "free_x", "free_y"}, optional
        Whether panels share scales.
    order : int or list, default 1
        Level order per column: ascending (1), descending (-1), or data order (0).
    format : str or list, optional
        Strip label formats.
    drop : bool, optional
        Whether to drop empty level combinations.
    dir : {"h", "v"}, default "h"
        Fill direction.
    labwidth : int or list, optional
        Strip label wrap width in characters.

    Returns
    -------
    FeatureSpec
        The facet specification.
    """
    return FeatureSpec(
        "facet",
        name="wrap",
        facets=facets,
        ncol=ncol,
        nrow=nrow,
        scales=scales,
        order=order,
        format=format,
        drop=drop,
        dir=dir,
        labwidth=labwidth,
    )
