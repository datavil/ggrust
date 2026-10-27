"""Faceting: ``facet_wrap`` and ``facet_grid``.

Ported from lets-plot's ``lets_plot/plot/facet.py`` (MIT, see THIRD_PARTY_NOTICES.md).
The engine does not split panels yet (plan.md step 3.2); rendering reports the facet.
"""

from ggrust.plot.core import FeatureSpec

__all__ = ["facet_grid", "facet_wrap"]


def facet_grid(
    x=None,
    y=None,
    *,
    scales=None,
    x_order=1,
    y_order=1,
    x_format=None,
    y_format=None,
    x_labwidth=None,
    y_labwidth=None,
) -> FeatureSpec:
    """Lay panels out in a grid: columns by ``x``, rows by ``y``.

    Parameters
    ----------
    x, y : str, optional
        Faceting columns.
    scales : {"fixed", "free", "free_x", "free_y"}, optional
        Whether panels share scales.
    x_order, y_order : {1, -1, 0}, default 1
        Level order: ascending, descending, or data order.
    x_format, y_format : str, optional
        Strip label formats.
    x_labwidth, y_labwidth : int, optional
        Strip label wrap width in characters.

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
    facets,
    ncol=None,
    nrow=None,
    *,
    scales=None,
    order=1,
    format=None,
    drop=None,
    dir="h",
    labwidth=None,
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
