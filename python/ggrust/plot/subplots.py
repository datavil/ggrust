"""``SupPlotsSpec``: a figure made of several plots (``gggrid``).

Ported from lets-plot's ``lets_plot/plot/subplots.py`` (MIT, see THIRD_PARTY_NOTICES.md)
with identical ``as_dict()`` output and ``+`` semantics. The engine lays grids out
(``crates/ggrust-core/src/grid.rs``).
"""

from __future__ import annotations

import copy
from types import NotImplementedType
from typing import TYPE_CHECKING, TypeAlias, cast, overload

from ggrust.errors import PlotIndexError, SpecError, did_you_mean
from ggrust.plot.core import (
    DummySpec,
    FeatureSpec,
    FeatureSpecArray,
    PlotSpec,
    _size_of,
    _specs_to_dict,
    _text_labels,
    _theme_dicts_merge,
)

if TYPE_CHECKING:
    from collections.abc import Iterator
    from typing import Any

    from ggrust._rendered import HTML, PNG, SVG
    from ggrust._typing import Number, Target

Figure: TypeAlias = "PlotSpec | SupPlotsSpec | None"
"""One grid cell: a plot, a nested grid, or None for an empty cell."""

Cell: TypeAlias = "int | tuple[int, int]"
"""A grid cell: its index in ``figures`` or its ``(row, column)``."""

Cells: TypeAlias = "slice | tuple[slice, int] | tuple[int, slice] | tuple[slice, slice]"
"""Several grid cells: a slice of ``figures``, or a ``(row, column)`` pair with a slice."""

_CELL_FORMS = (
    "a grid cell is an int or a (row, column) pair of ints, and several cells are a "
    "slice (grid[:], grid[1:3]) or a pair with a slice (grid[0, :], grid[:, 1])"
)

# ``gggrid()`` options ``with_layout`` accepts (``nrow`` follows from ``ncol``).
LAYOUT_OPTIONS = (
    "ncol",
    "sharex",
    "sharey",
    "widths",
    "heights",
    "hspace",
    "vspace",
    "fit",
    "align",
    "guides",
    "design",
    "tag_levels",
    "tag_level",
)


__all__ = ["GridCells", "SupPlotsSpec"]


class SupPlotsLayoutSpec:
    """Layout of a ``SupPlotsSpec`` (``name="grid"`` plus its options)."""

    def __init__(self, name: str, **kwargs: object) -> None:
        self.__props = dict(kwargs)
        self.__props["name"] = name

    def as_dict(self) -> dict[str, Any]:
        """Return the layout options without ``None`` values."""
        return _specs_to_dict(self.__props)

    def options(self) -> dict[str, Any]:
        """Return every layout option, unset ones as None, without ``name``."""
        return {key: value for key, value in self.__props.items() if key != "name"}


class SupPlotsSpec(FeatureSpec):
    """Several plots in one figure. Build it with ``gggrid()``.

    Read it back with ``figures`` (cells row by row, None for an empty cell),
    ``layout``, ``ncol``, ``nrow``, ``size``, ``theme`` (the grid's own: figures keep
    theirs), ``labels``, ``grid[i]`` or ``grid[row, column]``, and ``plots()``
    (every plot, nested grids flattened). ``with_figure()`` and ``with_layout()``
    return changed copies; ``grid[i] = figure`` and ``grid[i] += layer`` change one
    cell in place. ``grid[:] += feature`` (or ``grid[0, :]``, ``grid[:, 1]``,
    ``grid[1:3]``) adds a feature to every plot of several cells, through a
    ``GridCells`` view. A grid has no ``build()``: build its plots one by one.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> points = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> line = ggplot(data, aes("weight", "mpg")) + geom_line()
    >>> plot = gggrid([points, line])
    >>> isinstance(plot, SupPlotsSpec)
    True
    >>> plot[1] is line, plot.plots() == (points, line)
    (True, True)
    """

    @classmethod
    def duplicate(cls, other: SupPlotsSpec) -> SupPlotsSpec:
        """Return a shallow copy of ``other``.

        Parameters
        ----------
        other : SupPlotsSpec
            The spec to copy.
        """
        duplicate = SupPlotsSpec(figures=other.__figures, layout=other.__layout)
        duplicate.props().update(other.props())
        return duplicate

    def __init__(
        self, figures: list[PlotSpec | SupPlotsSpec | None], layout: SupPlotsLayoutSpec
    ) -> None:
        super().__init__("subplots", name=None)
        self.__figures = list(figures)
        self.__layout = layout

    @property
    def figures(self) -> tuple[Figure, ...]:
        """The cells in drawing order (row by row): plots, nested grids, None if empty.

        ``gggrid`` pads the last row with None, so there are ``ncol * nrow`` cells.

        Examples
        --------
        >>> from ggrust import *
        >>> points = ggplot() + geom_point()
        >>> grid = gggrid([points, None, points], ncol=2)
        >>> [figure is points for figure in grid.figures]
        [True, False, True, False]
        """
        return tuple(self.__figures)

    @property
    def layout(self) -> dict[str, Any]:
        """Every layout option, unset ones as None (a new dict).

        ``ncol``, ``nrow``, ``sharex``, ``sharey``, ``widths``, ``heights``, ``hspace``,
        ``vspace``, ``fit``, ``align`` and ``guides``, as ``gggrid()`` stored them.

        Examples
        --------
        >>> from ggrust import *
        >>> grid = gggrid([ggplot(), ggplot()], ncol=1, guides="collect")
        >>> grid.layout["nrow"], grid.layout["guides"], grid.layout["widths"]
        (2, 'collect', None)
        """
        return copy.deepcopy(self.__layout.options())

    @property
    def ncol(self) -> int:
        """The number of columns (the figure count when ``gggrid`` got no ``ncol``).

        Examples
        --------
        >>> from ggrust import *
        >>> gggrid([ggplot(), ggplot(), ggplot()]).ncol
        3
        """
        return self.__layout.options()["ncol"]

    @property
    def nrow(self) -> int:
        """The number of rows.

        Examples
        --------
        >>> from ggrust import *
        >>> gggrid([ggplot(), ggplot(), ggplot()], ncol=2).nrow
        2
        """
        return self.__layout.options()["nrow"]

    @property
    def size(self) -> tuple[Number, Number] | None:
        """``(width, height)`` from ``ggsize()``, or None for the default size.

        Examples
        --------
        >>> from ggrust import *
        >>> (gggrid([ggplot()]) + ggsize(800, 400)).size
        (800, 400)
        """
        return _size_of(self.props())

    @property
    def theme(self) -> dict[str, Any]:
        """The grid's own theme options (a new dict).

        Figures keep the theme they were built with: ``grid.figures[i].theme`` does not
        include what they inherit from the grid when drawn.

        Examples
        --------
        >>> from ggrust import *
        >>> (gggrid([ggplot()]) + theme_minimal()).theme
        {'name': 'minimal'}
        """
        return copy.deepcopy(_specs_to_dict(dict(self.props().get("theme", {}))))

    @property
    def labels(self) -> dict[str, str]:
        """The grid's title, subtitle, caption and tag (only those set).

        Examples
        --------
        >>> from ggrust import *
        >>> (gggrid([ggplot()]) + ggtitle("Dashboard")).labels
        {'title': 'Dashboard'}
        """
        return _text_labels(self.props())

    def plots(self) -> tuple[PlotSpec, ...]:
        """Every plot, nested grids flattened in drawing order, empty cells skipped.

        Returns
        -------
        tuple of PlotSpec
            The plots (the same objects).

        Examples
        --------
        >>> from ggrust import *
        >>> data = {"weight": [2.6, 2.9, 3.2], "mpg": [31, 28, 24]}
        >>> points = ggplot(data, aes("weight", "mpg")) + geom_point()
        >>> bars = ggplot(data, aes("mpg")) + geom_histogram(bins=2)
        >>> grid = gggrid([points, None, gggrid([bars, points])], ncol=2)
        >>> [plot.layers[0].geom for plot in grid.plots()]
        ['point', 'histogram', 'point']
        """
        return tuple(self.plot_paths().values())

    def plot_paths(self) -> dict[str, PlotSpec]:
        """Every plot keyed by its spec path, as error messages name it.

        Returns
        -------
        dict of str to PlotSpec
            ``{"figures[0]": plot, "figures[2].figures[1]": plot, ...}`` in drawing
            order, empty cells skipped.

        Examples
        --------
        >>> from ggrust import *
        >>> grid = gggrid([ggplot(), None, gggrid([ggplot(), ggplot()])], ncol=2)
        >>> list(grid.plot_paths())
        ['figures[0]', 'figures[2].figures[0]', 'figures[2].figures[1]']
        """
        paths: dict[str, PlotSpec] = {}
        for index, figure in enumerate(self.__figures):
            path = f"figures[{index}]"
            if isinstance(figure, SupPlotsSpec):
                for child_path, plot in figure.plot_paths().items():
                    paths[f"{path}.{child_path}"] = plot
            elif isinstance(figure, PlotSpec):
                paths[path] = figure
        return paths

    @overload
    def __getitem__(self, cell: Cell) -> Figure: ...

    @overload
    def __getitem__(self, cell: Cells) -> GridCells: ...

    def __getitem__(self, cell: Cell | Cells) -> Figure | GridCells:
        """Return the figure of a cell, or a ``GridCells`` view of several cells.

        ``grid[i]`` and ``grid[row, column]`` return the figure. A slice
        (``grid[:]``, ``grid[1:3]``, ``grid[::2]``) selects figures by index; a pair
        with a slice part (``grid[0, :]``, ``grid[:, 1]``) selects rows and columns.
        Negative values count from the end, as for tuples.

        Parameters
        ----------
        cell : int, tuple of (int, int), slice or tuple with a slice
            The cell(s) to read.

        Returns
        -------
        PlotSpec, SupPlotsSpec, None or GridCells
            The figure (None for an empty cell), or a view of the selected cells for
            ``grid[...] += feature``.

        Raises
        ------
        PlotIndexError
            If the cell (or an int part of a pair) is outside the grid.
        TypeError
            If ``cell`` is not one of the forms above.

        Examples
        --------
        >>> from ggrust import *
        >>> grid = gggrid([ggplot(), ggplot(), ggplot(), ggplot()], ncol=2)
        >>> grid[0, :]
        GridCells(grid[0, :]: figures [0, 1] of a 2 x 2 grid)
        """
        if _is_slice_key(cell):
            return self.__cells(cell)
        return self.__figures[self.__cell_position(cell)]

    def __cells(self, key: object) -> GridCells:
        """Return the ``GridCells`` view of a slice key (see ``__getitem__``)."""
        selection = f"grid[{_describe_key(key)}]"
        count = len(self.__figures)
        if isinstance(key, slice):
            positions = list(range(count)[_checked_slice(key, selection)])
        else:
            row_part, column_part = cast("tuple[object, object]", key)
            rows = self.__axis_indices(row_part, self.nrow, "row", selection)
            columns = self.__axis_indices(column_part, self.ncol, "column", selection)
            areas = self.__layout.options().get("areas")
            positions = []
            for row in rows:
                for column in columns:
                    position = self.__area_position(areas, row, column)
                    if position is not None and position not in positions:
                        positions.append(position)
        figures = tuple(self.__figures[position] for position in positions)
        return GridCells(self, selection, tuple(positions), figures, figures)

    @staticmethod
    def __axis_indices(part: object, length: int, axis: str, selection: str) -> range:
        """Return the rows or columns one part of a ``(row, column)`` key selects."""
        if isinstance(part, slice):
            return range(length)[_checked_slice(part, selection)]
        index = cast("int", part)
        if not -length <= index < length:
            plural = "s" if length != 1 else ""
            message = (
                f"{selection}: {axis} {index} is out of range: the grid has {length} "
                f"{axis}{plural}; {axis}s 0..{length - 1} (or -{length}..-1)"
            )
            raise PlotIndexError(message, index=index, length=length)
        return range(index % length, index % length + 1)

    def __area_position(
        self, areas: list[tuple[int, int, int, int]] | None, row: int, column: int
    ) -> int | None:
        """Return the index in ``figures`` of cell ``(row, column)``; None for a ``#`` cell."""
        if areas is None:
            return row * self.ncol + column
        for index, (left, top, right, bottom) in enumerate(areas):
            if left <= column <= right and top <= row <= bottom:
                return index
        return None

    def with_figure(self, cell: Cell, figure: Figure) -> SupPlotsSpec:
        """Return a copy of the grid with one cell replaced; this grid is not changed.

        Parameters
        ----------
        cell : int or tuple of (int, int)
            The cell's index in ``figures``, or its ``(row, column)``.
        figure : PlotSpec, SupPlotsSpec or None
            The new figure; None empties the cell.

        Returns
        -------
        SupPlotsSpec
            The new grid, with the same layout, size, theme and titles.

        Raises
        ------
        PlotIndexError
            If the cell is outside the grid.
        TypeError
            If ``cell`` is malformed or ``figure`` is not a plot, a grid or None.

        Examples
        --------
        >>> from ggrust import *
        >>> points, line = ggplot() + geom_point(), ggplot() + geom_line()
        >>> grid = gggrid([points, points], ncol=2)
        >>> grid.with_figure((0, 1), line)[1] is line
        True
        """
        self.__check_figure("with_figure()", figure)
        position = self.__cell_position(cell)
        grid = SupPlotsSpec.duplicate(self)
        grid.__figures[position] = figure
        return grid

    @overload
    def __setitem__(self, cell: Cell, figure: Figure) -> None: ...

    @overload
    def __setitem__(self, cell: Cells, figure: GridCells) -> None: ...

    def __setitem__(self, cell: Cell | Cells, figure: Figure | GridCells) -> None:
        """Replace the figure of a cell in place: ``grid[i] = plot``.

        Makes ``grid[i] += layer`` add a layer (or any feature) to one cell's
        figure. With a slice, ``grid[:] += feature`` adds it to every plot of the
        selected cells: the slice takes back the ``GridCells`` view that
        ``grid[:] + feature`` returned. This grid is changed; grids copied from it
        earlier are not. Use ``with_figure`` for a changed copy instead.

        Parameters
        ----------
        cell : int, tuple of (int, int), slice or tuple with a slice
            The cell's index in ``figures``, or its ``(row, column)``; or a slice
            form as for ``grid[...]``. Negative values count from the end.
        figure : PlotSpec, SupPlotsSpec, None or GridCells
            The new figure; None empties the cell. A slice takes a ``GridCells``
            view of the same cells of this grid.

        Raises
        ------
        PlotIndexError
            If the cell is outside the grid.
        TypeError
            If ``cell`` is malformed, ``figure`` is not a plot, a grid or None, or
            a slice is given anything but a view of the same cells of this grid.

        Examples
        --------
        >>> from ggrust import *
        >>> grid = gggrid([ggplot() + geom_point(), ggplot() + geom_line()], ncol=2)
        >>> grid[0] += geom_line()
        >>> [layer.geom for layer in grid[0].layers]
        ['point', 'line']
        """
        if _is_slice_key(cell):
            self.__set_cells(cell, figure)
            return
        if isinstance(figure, GridCells):
            message = (
                f"grid[cell] = figure: one cell takes a figure, got a GridCells view "
                f"({figure._selection}); assign a view back to its own slice: "
                f"{figure._selection} = view, or use grid[i] += feature for one cell"
            )
            raise TypeError(message)
        self.__check_figure("grid[cell] = figure", figure)
        self.__figures[self.__cell_position(cell)] = figure

    def __set_cells(self, key: object, view: object) -> None:
        """Write a ``GridCells`` view back to the cells of a slice key."""
        target = self.__cells(key)
        if not isinstance(view, GridCells):
            message = (
                f"{target._selection} = value: a slice of a grid takes the GridCells view "
                f"of the same cells, got {type(view).__name__}; add a feature to every "
                f"cell with {target._selection} += feature, or set one cell with "
                "grid[i] = figure"
            )
            raise TypeError(message)
        if view._grid is not self:
            message = (
                f"{target._selection} = view: the view ({view._selection}) was taken from "
                "another grid; assign a view back to the grid it came from, e.g. "
                f"{target._selection} += feature"
            )
            raise TypeError(message)
        if view._positions != target._positions:
            message = (
                f"{target._selection} = view: the view selects figures "
                f"{list(view._positions)} ({view._selection}), not figures "
                f"{list(target._positions)}; assign it back to the same cells, as "
                f"{view._selection} += feature does"
            )
            raise TypeError(message)
        changed = [
            position
            for position, taken in zip(view._positions, view._taken, strict=True)
            if self.__figures[position] is not taken
        ]
        if changed:
            message = (
                f"{target._selection} = view: figures {changed} changed after the view "
                f"was taken ({view._selection}), so writing it back would undo that "
                f"change; take a new view instead: {view._selection} += feature"
            )
            raise TypeError(message)
        for position, figure in zip(view._positions, view._figures, strict=True):
            self.__figures[position] = figure

    @staticmethod
    def __check_figure(where: str, figure: object) -> None:
        """Raise ``TypeError`` unless ``figure`` is a plot, a grid or None."""
        if figure is not None and not isinstance(figure, PlotSpec | SupPlotsSpec):
            message = (
                f"{where}: figure must be a plot (ggplot()), a grid (gggrid()) or "
                f"None, got {type(figure).__name__}"
            )
            raise TypeError(message)

    def with_layout(self, **options: object) -> SupPlotsSpec:
        """Return a copy of the grid with new ``gggrid()`` layout options.

        Options not given keep their value. Changing ``ncol`` lays the figures out
        again row by row: empty cells at the end (``gggrid``'s padding) are dropped
        first, then the last row is padded for the new column count. Size, theme and
        titles are kept; this grid is not changed.

        Parameters
        ----------
        **options
            Any of ``gggrid()``'s options: ``ncol``, ``sharex``, ``sharey``,
            ``widths``, ``heights``, ``hspace``, ``vspace``, ``fit``, ``align``,
            ``guides``.

        Returns
        -------
        SupPlotsSpec
            The new grid.

        Raises
        ------
        TypeError
            If an option is not a ``gggrid()`` option.
        SpecError
            If ``ncol`` is not a positive int.

        Examples
        --------
        >>> from ggrust import *
        >>> grid = gggrid([ggplot(), ggplot(), ggplot()], ncol=2)
        >>> wide = grid.with_layout(ncol=3, guides="collect")
        >>> (grid.ncol, grid.nrow), (wide.ncol, wide.nrow)
        ((2, 2), (3, 1))
        """
        from ggrust.plot.gggrid import gggrid

        for name in options:
            if name not in LAYOUT_OPTIONS:
                message = (
                    f"with_layout() got an unexpected option {name!r}; accepted: "
                    f"{', '.join(LAYOUT_OPTIONS)}{did_you_mean(name, LAYOUT_OPTIONS)}"
                )
                raise TypeError(message)
        ncol = options.get("ncol", self.ncol)
        if options.get("design") is None and (not _is_index(ncol) or cast("int", ncol) < 1):
            message = (
                f"with_layout(): ncol must be a positive int, got {ncol!r}; pass the "
                "number of columns, e.g. ncol=2"
            )
            raise SpecError(message, path="layout.ncol", reason=message)
        merged = {name: self.__layout.options().get(name) for name in LAYOUT_OPTIONS}
        figures = list(self.__figures)
        if "ncol" in options and options["ncol"] != merged["ncol"]:
            while len(figures) > 1 and figures[-1] is None:
                figures.pop()
        if merged["design"] is not None:
            # The design sets the columns; a new ncol replaces the design.
            if "ncol" in options and "design" not in options:
                merged["design"] = None
            else:
                merged["ncol"] = None
        merged.update(options)
        if merged["design"] is not None:
            merged["ncol"] = None
            while len(figures) > 1 and figures[-1] is None:
                figures.pop()
        grid = gggrid(figures, **merged)
        grid.props().update(self.props())
        return grid

    def __cell_position(self, cell: object) -> int:
        """Return the index in ``figures`` of ``cell`` (an index or ``(row, column)``)."""
        count = len(self.__figures)
        ncol, nrow = self.ncol, self.nrow
        if isinstance(cell, tuple):
            if len(cell) != 2 or not all(_is_index(part) for part in cell):
                message = f"{_CELL_FORMS}, got {cell!r}"
                raise TypeError(message)
            row, column = cell
            if not (-nrow <= row < nrow and -ncol <= column < ncol):
                message = (
                    f"grid cell ({row}, {column}) is out of range: the grid has {nrow} "
                    f"row{'s' if nrow != 1 else ''} and {ncol} column"
                    f"{'s' if ncol != 1 else ''}; rows 0..{nrow - 1}, columns 0..{ncol - 1}"
                )
                raise PlotIndexError(message, index=cell, length=count)
            row, column = row % nrow, column % ncol
            areas = self.__layout.options().get("areas")
            if areas is None:
                return row * ncol + column
            for index, (left, top, right, bottom) in enumerate(areas):
                if left <= column <= right and top <= row <= bottom:
                    return index
            message = (
                f"grid cell ({row}, {column}) is an empty cell ('#') of the design; "
                "index the figures by number, or pick a cell of a design area"
            )
            raise PlotIndexError(message, index=cell, length=count)
        if not _is_index(cell):
            message = f"{_CELL_FORMS}, got {type(cell).__name__} {cell!r}"
            raise TypeError(message)
        index = cast("int", cell)
        if not -count <= index < count:
            message = (
                f"grid cell {index} is out of range: the grid has {count} "
                f"cell{'s' if count != 1 else ''} ({nrow} x {ncol}); valid indices: "
                f"0..{count - 1} (or -{count}..-1), or grid[row, column]"
            )
            raise PlotIndexError(message, index=index, length=count)
        return index % count

    def __add__(self, other: object) -> SupPlotsSpec:
        """Return a new figure with ``other`` added, or ``NotImplemented`` for other operands.

        Figure-level features (``ggsize``, ``theme``, titles, tags, insets) are added;
        anything else is left to its own ``__radd__``, and Python raises ``TypeError``
        when that side does not handle the figure either.
        """
        if isinstance(other, DummySpec):
            return self
        if isinstance(other, FeatureSpecArray):
            supplots = SupPlotsSpec.duplicate(self)
            for element in other.elements():
                added = supplots.__add__(element)
                if isinstance(added, NotImplementedType):
                    return NotImplemented
                supplots = added
            return supplots
        if isinstance(other, FeatureSpec) and other.kind in [
            "ggsize",
            "theme",
            "ggtitle",
            "caption",
            "tag",
            "ggtoolbar",
        ]:
            supplots = SupPlotsSpec.duplicate(self)
            if other.kind == "theme":
                new_options = {
                    key: value for key, value in other.props().items() if value is not None
                }
                if "name" in new_options:
                    # A named theme replaces the theme, keeping the flavors.
                    for key in ("flavor", "background_fill"):
                        flavor = supplots.props().get("theme", {}).get(key)
                        if flavor is not None:
                            new_options[key] = flavor
                    supplots.props()["theme"] = new_options
                else:
                    old_options = supplots.props().get("theme", {})
                    supplots.props()["theme"] = _theme_dicts_merge(old_options, new_options)
                return supplots
            supplots.props()[other.kind] = other
            return supplots
        if isinstance(other, FeatureSpec) and other.kind == "inset":
            supplots = SupPlotsSpec.duplicate(self)
            supplots.props()["insets"] = [*supplots.props().get("insets", []), other]
            return supplots
        return NotImplemented

    def as_dict(self) -> dict[str, Any]:
        """Return the figure as a lets_plot-compatible dictionary.

        Returns
        -------
        dict
            Keys as lets_plot's ``SupPlotsSpec.as_dict()``: ``kind``, ``layout``,
            ``figures`` (each ``as_dict()`` or None for an empty cell), ...

        Examples
        --------
        >>> from ggrust import *
        >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
        >>> points = ggplot(data, aes("weight", "mpg")) + geom_point()
        >>> grid = gggrid([points, points + geom_line()], ncol=2)
        >>> spec = grid.as_dict()
        >>> spec["kind"], spec["layout"]["ncol"], len(spec["figures"])
        ('subplots', 2, 2)
        """
        result = super().as_dict()
        result["kind"] = self.kind
        result["layout"] = self.__layout.as_dict()
        result["figures"] = [
            None if figure is None else figure.as_dict() for figure in self.__figures
        ]
        if "insets" in result:
            result["insets"] = [inset.as_dict() for inset in result["insets"]]
        return result

    def __repr__(self) -> str:
        """Return a one-line summary: ``<SupPlotsSpec: 3 figures, nrow=2, ncol=2>``."""
        count = sum(figure is not None for figure in self.figures)
        noun = "figure" if count == 1 else "figures"
        return f"<SupPlotsSpec: {count} {noun}, nrow={self.nrow}, ncol={self.ncol}>"

    def _repr_html_(self) -> str:
        from ggrust._render import render_notebook_html

        return render_notebook_html(self)

    def show(self) -> None:
        """Draw the plot: inline in Jupyter/IPython, else in the default browser.

        Outside IPython the interactive HTML page is written to a temporary file
        and opened in the browser (lets_plot prints the spec dict there).

        Examples
        --------
        >>> from ggrust import *
        >>> (ggplot() + geom_point(x=0, y=0)).show()  # doctest: +SKIP
        """
        from ggrust._render import show

        show(self)

    @property
    def svg(self) -> SVG:
        """The plot as a static SVG, drawn inline when it ends a notebook cell.

        A ``str`` with the same text as ``to_svg()``. Rendered on each access; it
        does not change the session's display mode (``ggrust.display``).

        Returns
        -------
        SVG
            The SVG document, a ``str`` subclass Jupyter shows as an image.

        Examples
        --------
        Show the figure as a static image, for example in a notebook shared on GitHub,
        where scripts do not run:

        >>> from ggrust import *
        >>> from ggrust.data import mpg
        >>> scatter = ggplot(mpg, aes("displ", "hwy", color="drv")) + geom_point(size=3)
        >>> bars = ggplot(mpg, aes("drv", fill="drv")) + geom_bar()
        >>> grid = gggrid([scatter, bars], ncol=2)
        >>> grid.svg  # doctest: +SKIP

        It is also the SVG text, ready to save:

        >>> from pathlib import Path
        >>> Path("cars.svg").write_text(grid.svg)  # doctest: +SKIP
        """
        from ggrust._render import render_bytes
        from ggrust._rendered import SVG

        return SVG(render_bytes(self, "svg").decode())

    @property
    def png(self) -> PNG:
        """The plot as a PNG image, drawn inline when it ends a notebook cell.

        ``bytes`` holding the same image ``ggrust.display("png")`` shows. Use
        ``to_png()`` for another size or resolution.

        Returns
        -------
        PNG
            The encoded PNG, a ``bytes`` subclass Jupyter shows as an image.

        Examples
        --------
        Show the figure as a picture you can drag into slides or a document:

        >>> from ggrust import *
        >>> from ggrust.data import mpg
        >>> scatter = ggplot(mpg, aes("displ", "hwy", color="drv")) + geom_point(size=3)
        >>> bars = ggplot(mpg, aes("drv", fill="drv")) + geom_bar()
        >>> grid = gggrid([scatter, bars], ncol=2)
        >>> grid.png  # doctest: +SKIP

        It is also the PNG bytes, ready to save:

        >>> from pathlib import Path
        >>> Path("cars.png").write_bytes(grid.png)  # doctest: +SKIP
        """
        from ggrust._render import render_bytes
        from ggrust._rendered import PNG

        return PNG(render_bytes(self, "png"))

    @property
    def html(self) -> HTML:
        """The plot as an interactive HTML page, shown interactively in a notebook.

        A ``str`` with the same page as ``to_html()``. When it ends a notebook cell
        it shows with tooltips and the zoom/pan toolbar, whatever ``ggrust.display``
        is set to.

        Returns
        -------
        HTML
            The standalone page, a ``str`` subclass Jupyter shows interactively.

        Examples
        --------
        Keep one figure interactive while the rest of the notebook shows PNG images:

        >>> import ggrust
        >>> from ggrust import *
        >>> from ggrust.data import mpg
        >>> scatter = ggplot(mpg, aes("displ", "hwy", color="drv")) + geom_point(size=3)
        >>> bars = ggplot(mpg, aes("drv", fill="drv")) + geom_bar()
        >>> grid = gggrid([scatter, bars], ncol=2)
        >>> ggrust.display("png")
        >>> grid.html  # doctest: +SKIP

        Every other plot now shows as a PNG; ``grid.html`` keeps its tooltips and
        zoom/pan. ``ggrust.display("html")`` goes back to the default:

        >>> ggrust.display("html")
        """
        from ggrust._render import export_html
        from ggrust._rendered import HTML

        return HTML._from_plot(cast("str", export_html(self)), self)

    def to_svg(
        self,
        path: Target | None = None,
        w: Number | None = None,
        h: Number | None = None,
        unit: str | None = None,
    ) -> str | None:
        """Render to SVG; parameters as ``PlotSpec.to_svg``.

        Parameters
        ----------
        path : str or file-like, optional
            Where to write: a file path or a binary file-like object.
        w : float, optional
            Width.
        h : float, optional
            Height.
        unit : str, optional
            Unit of ``w`` and ``h``: ``"in"``, ``"cm"``, ``"mm"`` or ``"px"``.

        Examples
        --------
        >>> from ggrust import *
        >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
        >>> points = ggplot(data, aes("weight", "mpg")) + geom_point()
        >>> grid = gggrid([points, points + geom_line()], ncol=2)
        >>> grid.to_svg()[:4]
        '<svg'
        >>> grid.to_svg(w=800, h=300, unit="px")[:51]
        '<svg xmlns="http://www.w3.org/2000/svg" width="800"'
        """
        from ggrust._render import export_svg

        return export_svg(self, path, w=w, h=h, unit=unit)

    def to_png(
        self,
        path: Target,
        scale: Number | None = None,
        w: Number | None = None,
        h: Number | None = None,
        unit: str | None = None,
        dpi: Number | None = None,
    ) -> str | None:
        """Render to PNG; parameters as ``PlotSpec.to_png``.

        Parameters
        ----------
        path : str or file-like
            Where to write: a file path or a binary file-like object.
        scale : float, optional
            Multiplier of the pixel size.
        w : float, optional
            Width.
        h : float, optional
            Height.
        unit : str, optional
            Unit of ``w`` and ``h``: ``"in"``, ``"cm"``, ``"mm"`` or ``"px"``.
        dpi : float, optional
            Resolution in dots per inch.

        Examples
        --------
        To a file, here in a temporary folder:

        >>> import os
        >>> import tempfile
        >>> from ggrust import *
        >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
        >>> points = ggplot(data, aes("weight", "mpg")) + geom_point()
        >>> grid = gggrid([points, points + geom_line()], ncol=2)
        >>> with tempfile.TemporaryDirectory() as folder:
        ...     written = grid.to_png(os.path.join(folder, "cars.png"))
        ...     os.path.basename(written), os.path.exists(written)
        ('cars.png', True)

        To bytes, through a buffer:

        >>> import io
        >>> buffer = io.BytesIO()
        >>> grid.to_png(buffer)
        >>> buffer.getvalue()[1:4]
        b'PNG'

        A physical size and a resolution, as for a journal figure (6 in x 150 dpi = 900 px):

        >>> buffer = io.BytesIO()
        >>> grid.to_png(buffer, w=6, h=4, unit="in", dpi=150)
        >>> int.from_bytes(buffer.getvalue()[16:20]), int.from_bytes(buffer.getvalue()[20:24])
        (900, 600)
        """
        from ggrust._render import export_png

        return export_png(self, path, scale=scale, w=w, h=h, unit=unit, dpi=dpi)

    def to_pdf(
        self,
        path: Target,
        scale: Number | None = None,
        w: Number | None = None,
        h: Number | None = None,
        unit: str | None = None,
        dpi: Number | None = None,
    ) -> str | None:
        """Render to PDF; parameters as ``PlotSpec.to_pdf``.

        Parameters
        ----------
        path : str or file-like
            Where to write: a file path or a binary file-like object.
        scale : float, optional
            Multiplier of the pixel size.
        w : float, optional
            Width.
        h : float, optional
            Height.
        unit : str, optional
            Unit of ``w`` and ``h``: ``"in"``, ``"cm"``, ``"mm"`` or ``"px"``.
        dpi : float, optional
            Resolution in dots per inch.

        Examples
        --------
        >>> import io
        >>> from ggrust import *
        >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
        >>> points = ggplot(data, aes("weight", "mpg")) + geom_point()
        >>> grid = gggrid([points, points + geom_line()], ncol=2)
        >>> buffer = io.BytesIO()
        >>> grid.to_pdf(buffer, w=180, h=120, unit="mm")
        >>> buffer.getvalue()[:5]
        b'%PDF-'
        """
        from ggrust._render import export_pdf

        return export_pdf(self, path, scale=scale, w=w, h=h, unit=unit, dpi=dpi)

    def to_html(self, path: Target | None = None, iframe: bool | None = None) -> str | None:
        """Render to standalone HTML; parameters as ``PlotSpec.to_html``.

        Parameters
        ----------
        path : str or file-like, optional
            Where to write: a file path or a binary file-like object.
        iframe : bool, optional
            Return an ``<iframe srcdoc>`` sized to the plot instead of a page.

        Examples
        --------
        >>> from ggrust import *
        >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
        >>> points = ggplot(data, aes("weight", "mpg")) + geom_point()
        >>> grid = gggrid([points, points + geom_line()], ncol=2)
        >>> grid.to_html()[:15]
        '<!DOCTYPE html>'
        >>> grid.to_html(iframe=True)[:7]
        '<iframe'
        """
        from ggrust._render import export_html

        return export_html(self, path, iframe)


def _is_index(value: object) -> bool:
    """Whether ``value`` is an int usable as an index (bools are not)."""
    return isinstance(value, int) and not isinstance(value, bool)


def _is_slice_key(key: object) -> bool:
    """Whether ``key`` selects several cells: a slice, or a pair with a slice part.

    The other part of a pair must be an int or a slice; anything else is left to the
    single-cell path, which raises the shared ``TypeError``.
    """
    if isinstance(key, slice):
        return True
    return (
        isinstance(key, tuple)
        and len(key) == 2
        and any(isinstance(part, slice) for part in key)
        and all(isinstance(part, slice) or _is_index(part) for part in key)
    )


def _describe_key(key: object) -> str:
    """Write a slice key as it is typed: ``:``, ``0, :``, ``1:3``, ``::2``."""

    def describe(part: object) -> str:
        if not isinstance(part, slice):
            return repr(part)
        bounds = ["" if bound is None else repr(bound) for bound in (part.start, part.stop)]
        text = ":".join(bounds)
        return text if part.step is None else f"{text}:{part.step!r}"

    if isinstance(key, tuple):
        return ", ".join(describe(part) for part in key)
    return describe(key)


def _is_slice_bound(value: object) -> bool:
    """Whether ``value`` is a slice bound: an int or any ``__index__`` type, not a bool."""
    return hasattr(type(value), "__index__") and not isinstance(value, bool)


def _checked_slice(part: slice, selection: str) -> slice:
    """Return ``part`` if its bounds and step are ints or None, else raise ``TypeError``."""
    bounds = (part.start, part.stop, part.step)
    if not all(bound is None or _is_slice_bound(bound) for bound in bounds):
        message = (
            f"{selection}: slice bounds and step must be ints or None, got "
            f"{part!r}; e.g. grid[:], grid[1:3], grid[::2]"
        )
        raise TypeError(message)
    if part.step == 0:
        message = f"{selection}: slice step cannot be zero; e.g. grid[::2] or grid[::-1]"
        raise ValueError(message)
    return part


class GridCells:
    """A view of several cells of a grid, for ``grid[:] += feature``.

    ``grid[:]``, ``grid[0, :]``, ``grid[:, 1]`` and ``grid[1:3]`` return one. Do not
    create it directly. ``view + feature`` returns a new view in which every plot of
    the selected cells has the feature added (``plot + feature``); nested grids get it
    on every plot inside them, empty cells stay empty. Assigning the view back to the
    same slice (which ``grid[:] += feature`` does) writes those figures into the grid
    in place; grids copied earlier are not changed.

    Figure-level features (``theme``, ``ggsize``, ``ggtitle``, tags) go to each plot
    too: ``grid[:] += theme(...)`` themes every plot, while ``grid + theme(...)``
    themes the grid figure. A design area spanning several selected cells counts
    once, and ``#`` cells of a design are skipped.

    The view is read-only: ``len(view)`` and ``for figure in view`` give the selected
    figures in selection order (None for an empty cell).

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4], "mpg": [31, 28, 24, 22]}
    >>> points = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> grid = gggrid([points, points, points, None], ncol=2)
    >>> grid[:] += geom_line()
    >>> [len(plot.layers) for plot in grid.plots()]
    [2, 2, 2]
    >>> grid[:, 0] += geom_smooth()
    >>> [len(plot.layers) for plot in grid.plots()]
    [3, 2, 3]
    >>> len(grid[0, :]), grid[1, :]
    (2, GridCells(grid[1, :]: figures [2, 3] of a 2 x 2 grid))
    """

    def __init__(
        self,
        grid: SupPlotsSpec,
        selection: str,
        positions: tuple[int, ...],
        figures: tuple[Figure, ...],
        taken: tuple[Figure, ...],
    ) -> None:
        self._grid = grid
        self._selection = selection
        self._positions = positions
        self._figures = figures
        # The grid's figures when the view was taken, to refuse a stale write back.
        self._taken = taken

    def __len__(self) -> int:
        """Return the number of selected cells (a spanning design area counts once)."""
        return len(self._figures)

    def __iter__(self) -> Iterator[Figure]:
        """Yield the selected figures in selection order, None for an empty cell."""
        return iter(self._figures)

    def __repr__(self) -> str:
        """Return ``GridCells(grid[0, :]: figures [0, 1] of a 2 x 2 grid)``."""
        return (
            f"GridCells({self._selection}: figures {list(self._positions)} of a "
            f"{self._grid.nrow} x {self._grid.ncol} grid)"
        )

    def __add__(self, other: object) -> GridCells:
        """Return a new view with ``other`` added to every plot of the selected cells.

        Parameters
        ----------
        other : FeatureSpec
            A layer, scale, coord, labels, theme or any other plot feature.

        Returns
        -------
        GridCells
            The changed view; assign it back (``grid[:] += feature``) to change the
            grid. The figures of this view and of the grid are not changed.

        Raises
        ------
        Exception
            Whatever ``plot + other`` raises for a plot, with a note naming that
            plot's spec path (``figures[2].figures[1]``).
        """
        if isinstance(other, DummySpec):
            return self
        if not isinstance(other, FeatureSpec) or isinstance(other, PlotSpec | SupPlotsSpec):
            return NotImplemented
        figures = tuple(
            _add_to_figure(figure, other, f"figures[{position}]", self._selection)
            for position, figure in zip(self._positions, self._figures, strict=True)
        )
        return GridCells(self._grid, self._selection, self._positions, figures, self._taken)


def _add_to_figure(figure: Figure, feature: FeatureSpec, path: str, selection: str) -> Figure:
    """Return ``figure`` with ``feature`` added to each of its plots (nested grids too)."""
    if figure is None:
        return None
    if isinstance(figure, SupPlotsSpec):
        nested = SupPlotsSpec.duplicate(figure)
        for index, child in enumerate(figure.figures):
            nested[index] = _add_to_figure(child, feature, f"{path}.figures[{index}]", selection)
        return nested
    try:
        added = figure + feature
    except Exception as error:
        error.add_note(f"in {selection} + {feature.kind}: adding to the plot at {path}")
        raise
    if not isinstance(added, PlotSpec):
        message = (
            f"{selection} + {feature.kind}: the plot at {path} did not accept it "
            f"(plot + {type(feature).__name__} gave {type(added).__name__}); add "
            "features a plot accepts, such as layers, scales, labels or a theme"
        )
        raise TypeError(message)
    return added
