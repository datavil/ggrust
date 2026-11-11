"""Faceting: ``facet_wrap``, ``facet_grid`` and ``facet_null``.

Ported from lets-plot's ``lets_plot/plot/facet.py`` (MIT, see THIRD_PARTY_NOTICES.md).
The Rust engine splits the data into panels and lays them out.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from ggrust.errors import SpecError, did_you_mean
from ggrust.plot.core import FeatureSpec
from ggrust.plot.labeller import resolve_labeller

if TYPE_CHECKING:
    from collections.abc import Sequence
    from typing import Literal

    from ggrust.plot.labeller import Labeller, LabellerName


__all__ = ["facet_grid", "facet_null", "facet_wrap"]


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
    labeller: Labeller | LabellerName | None = None,
    space: Literal["fixed", "free", "free_x", "free_y"] = "fixed",
    margins: bool | str | Sequence[str] = False,
    as_table: bool = True,
    shrink: bool = True,
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
    labeller : Labeller or str, optional
        Strip labeller (ggplot2): ``label_value`` (the default), ``label_both``,
        ``label_context``, ``as_labeller({...})``, or a name such as
        ``"label_both"``. Applied after the format, before wrapping.
    space : {"fixed", "free", "free_x", "free_y"}, default "fixed"
        Whether grid columns (x) and rows (y) get widths and heights in proportion
        to their scale ranges (ggplot2). Use with free ``scales``.
    margins : bool, str or list of str, default False
        Add ``(all)`` margin panels with every row: ``True`` for both faceting
        columns, or the names of the columns that get one (ggplot2).
    as_table : bool, default True
        ``False`` puts the highest row level at the top (ggplot2 ``as.table``).
    shrink : bool, default True
        ``False`` makes the position scales also cover the data before the stats,
        not only the stat results (ggplot2).

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
        labeller=resolve_labeller(labeller),
        space=_free_option(space, "space"),
        margins=_margins(margins, x, y),
        as_table=_flag(as_table, "as_table"),
        shrink=_flag(shrink, "shrink"),
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
    labeller: Labeller | LabellerName | None = None,
    as_table: bool = True,
    shrink: bool = True,
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
    labeller : Labeller or str, optional
        Strip labeller (ggplot2): ``label_value`` (the default), ``label_both``,
        ``label_context``, ``as_labeller({...})``, or a name such as
        ``"label_both"``. Applied after the format, before wrapping.
    as_table : bool, default True
        ``False`` fills the panels from the bottom row up (``dir="h"``) or from the
        right column (``dir="v"``), as ggplot2 ``as.table``.
    shrink : bool, default True
        ``False`` makes the position scales also cover the data before the stats,
        not only the stat results (ggplot2).

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
        labeller=resolve_labeller(labeller),
        as_table=_flag(as_table, "as_table"),
        shrink=_flag(shrink, "shrink"),
    )


def facet_null() -> FeatureSpec:
    """Remove faceting: one panel (ggplot2's default facet).

    Adding it to a faceted plot takes the ``facet_wrap`` or ``facet_grid`` away.

    Returns
    -------
    FeatureSpec
        The facet specification.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 2, 4], "weight": [4, 6, 8, 12], "pet": ["cat", "cat", "dog", "dog"]}
    >>> plot = ggplot(data, aes("age", "weight")) + geom_point() + facet_wrap("pet")
    >>> plot = plot + facet_null()
    >>> plot.facet is None
    True
    """
    return FeatureSpec("facet", name="null")


def _flag(value: bool, parameter: str) -> bool | None:
    """Return ``None`` for the default ``True`` (kept out of the spec), checked."""
    if not isinstance(value, bool):
        raise TypeError(f"{parameter} must be True or False, got {value!r}")
    return None if value else False


def _free_option(value: str, parameter: str) -> str | None:
    """Return a ``space`` value, ``None`` for ``"fixed"``, checked."""
    accepted = ("fixed", "free", "free_x", "free_y")
    if value not in accepted:
        reason = (
            f"unknown value {value!r}; use one of {', '.join(accepted)}"
            f"{did_you_mean(str(value), accepted)}"
        )
        raise SpecError(f"facet.{parameter}: {reason}", path=f"facet.{parameter}", reason=reason)
    return None if value == "fixed" else value


def _margins(
    margins: bool | str | Sequence[str], x: str | None, y: str | None
) -> bool | list[str] | None:
    """Return ``facet_grid(margins=)`` for the spec, checked against ``x`` and ``y``."""
    if isinstance(margins, bool):
        return True if margins else None
    names = [margins] if isinstance(margins, str) else list(margins)
    variables = [name for name in (x, y) if name is not None]
    for name in names:
        if name not in variables:
            reason = (
                f"'{name}' is not a faceting column; use one of {', '.join(variables)}"
                f"{did_you_mean(str(name), variables)}"
            )
            raise SpecError(f"facet.margins: {reason}", path="facet.margins", reason=reason)
    return names
