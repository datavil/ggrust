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
        Level order of the columns and rows: ascending, descending, or data order.
    x_format, y_format : str, optional
        Strip label formats: a number format (``".2f"``) or a template
        (``"{d} cylinders"``).
    x_labwidth, y_labwidth : int, optional
        Strip label wrap width in characters. Labels split at newlines first, then
        each part wraps at this width.

    Returns
    -------
    FeatureSpec
        The facet specification.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"hours": [2, 4, 6, 8] * 4, "score": [50, 60, 70, 75, 55, 62, 74, 85] * 2,
    ...         "subject": (["math"] * 4 + ["art"] * 4) * 2, "year": [1] * 8 + [2] * 8}
    >>> plot = ggplot(data, aes("hours", "score")) + geom_line()
    >>> plot = plot + facet_grid(x="subject", y="year")
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
        Strip label wrap width in characters, matched to ``facets`` by position (a
        single value applies to the first column). Labels split at newlines first,
        then each part wraps at this width.

    Returns
    -------
    FeatureSpec
        The facet specification.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight")) + geom_point()
    >>> plot = plot + facet_wrap(facets="pet", ncol=3)
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
