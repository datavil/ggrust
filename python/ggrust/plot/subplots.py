"""``SupPlotsSpec``: a figure made of several plots (``gggrid``).

Ported from lets-plot's ``lets_plot/plot/subplots.py`` (MIT, see THIRD_PARTY_NOTICES.md)
with identical ``as_dict()`` output and ``+`` semantics. The engine lays grids out
(``crates/ggrust-core/src/grid.rs``).
"""

from __future__ import annotations

import copy
from types import NotImplementedType
from typing import TYPE_CHECKING, TypeAlias, cast

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
    from typing import Any

    from ggrust._typing import Number, Target

Figure: TypeAlias = "PlotSpec | SupPlotsSpec | None"
"""One grid cell: a plot, a nested grid, or None for an empty cell."""

Cell: TypeAlias = "int | tuple[int, int]"
"""A grid cell: its index in ``figures`` or its ``(row, column)``."""

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


__all__ = ["SupPlotsSpec"]


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
    cell in place. A grid has no ``build()``: build its plots one by one.

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

    def __getitem__(self, cell: Cell) -> Figure:
        """Return the figure of a cell: ``grid[i]`` or ``grid[row, column]``.

        Negative values count from the end, as for tuples.

        Raises
        ------
        PlotIndexError
            If the cell is outside the grid.
        TypeError
            If ``cell`` is not an int or a pair of ints.
        """
        return self.__figures[self.__cell_position(cell)]

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

    def __setitem__(self, cell: Cell, figure: Figure) -> None:
        """Replace the figure of a cell in place: ``grid[i] = plot``.

        Makes ``grid[i] += layer`` add a layer (or any feature) to one cell's
        figure. This grid is changed; grids copied from it earlier are not.
        Use ``with_figure`` for a changed copy instead.

        Parameters
        ----------
        cell : int or tuple of (int, int)
            The cell's index in ``figures``, or its ``(row, column)``.
            Negative values count from the end.
        figure : PlotSpec, SupPlotsSpec or None
            The new figure; None empties the cell.

        Raises
        ------
        PlotIndexError
            If the cell is outside the grid.
        TypeError
            If ``cell`` is malformed or ``figure`` is not a plot, a grid or None.

        Examples
        --------
        >>> from ggrust import *
        >>> grid = gggrid([ggplot() + geom_point(), ggplot() + geom_line()], ncol=2)
        >>> grid[0] += geom_line()
        >>> [layer.geom for layer in grid[0].layers]
        ['point', 'line']
        """
        self.__check_figure("grid[cell] = figure", figure)
        self.__figures[self.__cell_position(cell)] = figure

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
                message = f"a grid cell is an int or a (row, column) pair of ints, got {cell!r}"
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
            message = (
                f"a grid cell is an int or a (row, column) pair of ints, got "
                f"{type(cell).__name__} {cell!r}"
            )
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
                    # A named theme replaces the theme, keeping the flavor.
                    flavor = supplots.props().get("theme", {}).get("flavor")
                    if flavor is not None:
                        new_options["flavor"] = flavor
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
        """
        from ggrust._render import export_html

        return export_html(self, path, iframe)


def _is_index(value: object) -> bool:
    """Whether ``value`` is an int usable as an index (bools are not)."""
    return isinstance(value, int) and not isinstance(value, bool)
