"""``gggrid()``: arrange plots in a grid.

Ported from lets-plot's ``lets_plot/plot/gggrid_.py`` (MIT, see THIRD_PARTY_NOTICES.md).
Global themes (``theme_set``) are not ported, so figures keep their own themes.
"""

from __future__ import annotations

from typing import Literal

from ggrust.errors import SpecError, did_you_mean
from ggrust.plot.subplots import SupPlotsLayoutSpec, SupPlotsSpec

__all__ = ["gggrid"]

TAG_LEVELS = ("A", "a", "1", "i", "I")
"""Numberings ``tag_levels`` accepts: letters, lowercase letters, numbers, roman
numerals in lowercase and uppercase (patchwork's)."""

COLLECT_VALUES = ("keep", "collect", "collect_x", "collect_y")
"""Values ``axes`` and ``axis_titles`` accept (patchwork's)."""

Collect = Literal["keep", "collect", "collect_x", "collect_y"]
"""A value of ``axes`` or ``axis_titles``."""

Area = tuple[int, int, int, int]
"""Cells a figure spans in a ``design``: ``(left, top, right, bottom)``, 0-based and
inclusive."""


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
    design: str | None = None,
    tag_levels: str | list[str] | None = None,
    tag_level: Literal["keep", "new"] | None = None,
    axes: Collect | None = None,
    axis_titles: Collect | None = None,
) -> SupPlotsSpec:
    r"""Combine plots into a grid.

    Parameters
    ----------
    plots : list
        ``PlotSpec`` or ``SupPlotsSpec`` objects; None leaves a cell empty.
    ncol : int, optional
        Number of columns; all plots in one row when None.
    sharex, sharey : bool or {"all", "none", "row", "col"}, optional
        Share the horizontal (vertical) axis range between all plots
        (``True``/``"all"``), the plots of a row, or of a column. Continuous axes
        take the union of the plots' ranges; discrete axes the union of their
        categories (lets-plot shares only continuous axes). Plots with their own
        scale limits keep them; nested grids do not take part.
    widths, heights : list of float, optional
        Relative column widths and row heights.
    hspace, vspace : float, optional
        Gaps between cells in pixels.
    fit : bool, optional
        Stretch each plot to its cell.
    align : bool, optional
        Align inner panels. Nested grids take part: their outer panel edges line
        up with the other figures', and a nested grid without ``align`` follows
        this one (a ggrust extension; lets-plot leaves nested grids out).
    guides : {"auto", "collect", "keep"}, optional
        Whether identical legends are collected into one.
    design : str, optional
        A layout drawn with characters, one line per row (patchwork's
        ``plot_layout(design=)``): ``"AAB\nCCB"`` gives plot 0 the top left two
        cells, plot 1 the right column and plot 2 the bottom left. Each character
        is an area, filled by the plots in sorted character order; ``#`` is an
        empty cell. Areas must be rectangles. Fewer plots than areas leave the last
        areas empty. Replaces ``ncol``; ``widths`` and ``heights`` size its columns
        and rows. A ggrust extension.
    tag_levels : str or list of str, optional
        Tag every plot automatically: ``"A"`` (A, B, C), ``"a"``, ``"1"``, ``"i"``
        (i, ii, iii) or ``"I"``, in figure order, empty cells skipped. A list gives
        one numbering per nesting level, for nested grids with
        ``tag_level="new"`` (``["A", "1"]`` gives A1, A2). Tags replace the plots'
        own ``labs(tag=)``; the theme's ``plot_tag``, ``plot_tag_position``,
        ``plot_tag_location``, ``plot_tag_prefix`` and ``plot_tag_suffix`` style
        them. A ggrust extension (patchwork ``plot_annotation(tag_levels=)``).
    tag_level : {"keep", "new"}, optional
        For a grid nested in a tagged grid: ``"keep"`` (default) numbers its plots
        on with the outer grid's; ``"new"`` takes one outer tag and numbers its
        plots at the next level of ``tag_levels``.
    axes : {"keep", "collect", "collect_x", "collect_y"}, optional
        Collect axes that repeat (patchwork's ``plot_layout(axes=)``): plots next
        to each other whose axis on a side is identical (same scale, same axis
        theme and, for x, the same columns; for y, the same rows) show it once,
        on the outer plot of the run: the bottom-most for a bottom axis, the
        left-most for a left axis. The others drop the axis (line, ticks,
        labels) and its room. ``"collect_x"`` and ``"collect_y"`` collect one
        direction only. Use with ``sharex`` / ``sharey`` so the axes are
        identical. Empty cells, nested grids and faceted plots break runs.
        A grid that collects axes or titles keeps its panels the same size
        (relative to ``widths`` / ``heights``), as patchwork: the cells are
        resized so the room an axis frees goes to every panel, and panels are
        aligned unless ``align=False`` is given. Default ``"keep"``. A ggrust
        extension.
    axis_titles : {"keep", "collect", "collect_x", "collect_y"}, optional
        Collect axis titles that repeat (patchwork's ``plot_layout(axis_titles=)``):
        neighbouring plots with the same title on a side keep one, centred on
        their panels when the run spans several columns (x) or rows (y). Works
        without ``axes``. Default ``"keep"``. A ggrust extension.

    Returns
    -------
    SupPlotsSpec
        The grid.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> points = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> bars = ggplot(data, aes("mpg")) + geom_histogram(bins=4)
    >>> plot = gggrid([points, bars], ncol=2) + ggsize(700, 300)

    A design layout with tags A, B, C:

    >>> plot = gggrid([points, bars, points], design="AAB\nCCB", tag_levels="A")

    Shared axes shown once, with one title per axis:

    >>> plot = gggrid(
    ...     [points, points, points, points],
    ...     ncol=2,
    ...     sharex=True,
    ...     sharey=True,
    ...     axes="collect",
    ...     axis_titles="collect",
    ... )
    """
    if not len(plots):
        message = "gggrid(): 'plots' is empty; pass a list of plots (None leaves a cell empty)"
        raise SpecError(message)
    levels = _tag_levels(tag_levels)
    if tag_level is not None and tag_level not in ("keep", "new"):
        message = (
            f"gggrid(): tag_level must be 'keep' or 'new', got {tag_level!r}"
            f"{did_you_mean(str(tag_level), ['keep', 'new'])}"
        )
        raise SpecError(message, path="layout.tag_level", reason=message)
    for key, value in (("axes", axes), ("axis_titles", axis_titles)):
        _check_collect(key, value)
    areas: list[Area] | None = None
    if design is not None:
        if ncol is not None:
            message = (
                "gggrid(): pass either ncol or design, not both; the design string sets the columns"
            )
            raise SpecError(message, path="layout.design", reason=message)
        ncol, nrow, areas = _design_areas(design)
        if len(plots) > len(areas):
            message = (
                f"gggrid(): the design has {len(areas)} area"
                f"{'s' if len(areas) != 1 else ''} but {len(plots)} plots were given; "
                "add a character per extra plot, or pass fewer plots"
            )
            raise SpecError(message, path="layout.design", reason=message)
        plots = list(plots) + [None] * (len(areas) - len(plots))
    elif ncol is None:
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
        **_extension_options(design, areas, levels, tag_level),
        **{
            key: value
            for key, value in (("axes", axes), ("axis_titles", axis_titles))
            if value is not None
        },
    )
    return SupPlotsSpec(figures=plots, layout=layout)


def _extension_options(
    design: str | None,
    areas: list[Area] | None,
    levels: list[str] | None,
    tag_level: str | None,
) -> dict[str, object]:
    """Return the ggrust layout options that are set (lets_plot's dict stays as is)."""
    options: dict[str, object] = {}
    if design is not None and areas is not None:
        options["design"] = design
        options["areas"] = [list(area) for area in areas]
    if levels is not None:
        options["tag_levels"] = levels
    if tag_level is not None:
        options["tag_level"] = tag_level
    return options


def _check_collect(key: str, value: object) -> None:
    """Check an ``axes`` / ``axis_titles`` value against :data:`COLLECT_VALUES`.

    Raises
    ------
    SpecError
        If ``value`` is set and not one of them.
    """
    if value is None or value in COLLECT_VALUES:
        return
    message = (
        f"gggrid(): {key} must be one of "
        f"{', '.join(repr(name) for name in COLLECT_VALUES)}, got {value!r}"
        f"{did_you_mean(str(value), COLLECT_VALUES)}"
    )
    raise SpecError(message, path=f"layout.{key}", reason=message)


def _tag_levels(tag_levels: str | list[str] | None) -> list[str] | None:
    """Return ``tag_levels`` as a list, checked against :data:`TAG_LEVELS`."""
    if tag_levels is None:
        return None
    if not isinstance(tag_levels, str | list | tuple):
        message = (
            f"gggrid(): tag_levels must be 'A', 'a', '1', 'i' or 'I', or a list of them, "
            f"got {type(tag_levels).__name__} {tag_levels!r}"
        )
        raise SpecError(message, path="layout.tag_levels", reason=message)
    levels = [tag_levels] if isinstance(tag_levels, str) else list(tag_levels)
    if not levels:
        message = "gggrid(): tag_levels is empty; pass 'A', 'a', '1', 'i' or 'I', or a list"
        raise SpecError(message, path="layout.tag_levels", reason=message)
    for level in levels:
        if level not in TAG_LEVELS:
            message = (
                f"gggrid(): unknown tag level {level!r}; expected one of "
                f"{', '.join(repr(name) for name in TAG_LEVELS)} (letters, lowercase "
                f"letters, numbers, roman numerals){did_you_mean(str(level), TAG_LEVELS)}"
            )
            raise SpecError(message, path="layout.tag_levels", reason=message)
    return levels


def _design_key(character: str) -> tuple[str, bool]:
    """Sort key of a design character: R's collation, as patchwork (``a A b B``)."""
    return character.lower(), character.isupper()


def _design_areas(design: str) -> tuple[int, int, list[Area]]:
    """Parse a design string as patchwork's ``as_areas()``.

    Returns
    -------
    tuple
        ``(columns, rows, areas)``, the areas in sorted character order.

    Raises
    ------
    SpecError
        If the design is empty, its rows differ in length, a row holds a space, or
        an area is not a rectangle.
    """
    if not isinstance(design, str):
        message = f"gggrid(): design must be a string such as 'AAB\\nCCB', got {design!r}"
        raise SpecError(message, path="layout.design", reason=message)
    rows = [line.strip() for line in design.split("\n")]
    if rows and rows[0] == "":
        rows = rows[1:]
    if rows and rows[-1] == "":
        rows = rows[:-1]
    if not rows or not all(rows):
        message = (
            f"gggrid(): design {design!r} has an empty row; write one line of "
            "characters per row, such as 'AAB\\nCCB' ('#' for an empty cell)"
        )
        raise SpecError(message, path="layout.design", reason=message)
    columns = len(rows[0])
    for index, row in enumerate(rows):
        if len(row) != columns:
            message = (
                f"gggrid(): design rows must be equally long; row {index} {row!r} has "
                f"{len(row)} cells, row 0 has {columns}"
            )
            raise SpecError(message, path="layout.design", reason=message)
        if any(character.isspace() for character in row):
            message = (
                f"gggrid(): design row {index} {row!r} holds a space; use '#' for an empty cell"
            )
            raise SpecError(message, path="layout.design", reason=message)
    names = sorted({character for row in rows for character in row} - {"#"}, key=_design_key)
    areas: list[Area] = []
    for name in names:
        cells = [
            (column, row)
            for row, line in enumerate(rows)
            for column, character in enumerate(line)
            if character == name
        ]
        left = min(column for column, _ in cells)
        right = max(column for column, _ in cells)
        top = min(row for _, row in cells)
        bottom = max(row for _, row in cells)
        if len(cells) != (right - left + 1) * (bottom - top + 1):
            message = (
                f"gggrid(): design area {name!r} is not a rectangle; each character must "
                f"fill a rectangle of cells (rows {top}..{bottom}, columns {left}..{right} "
                "here)"
            )
            raise SpecError(message, path="layout.design", reason=message)
        areas.append((left, top, right, bottom))
    if not areas:
        message = f"gggrid(): design {design!r} has no area; use letters for the plots"
        raise SpecError(message, path="layout.design", reason=message)
    return columns, len(rows), areas
