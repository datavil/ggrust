"""``corr_plot``: a correlation matrix drawn as tiles, points and/or labels.

Follows lets-plot's bistro ``corr_plot`` (``lets_plot/bistro/corr.py`` for the
builder, the Kotlin ``CorrPlotOptionsBuilder`` for the layers; MIT, see
THIRD_PARTY_NOTICES.md). lets_plot hands the builder's options to its engine;
ggrust builds the plot here, as ordinary layers, scales, theme and size, from a
correlation matrix computed in Rust (``ggrust._ggrust.correlation_matrix``).
Layer types, diagonals, label colors and the plot size reproduce lets-plot's
measured output; see deviations.md for the differences.
"""

from __future__ import annotations

import math
from dataclasses import dataclass, replace
from typing import TYPE_CHECKING, Any, Literal

import polars as pl

from ggrust._data import as_arrow
from ggrust._ggrust import correlation_matrix
from ggrust.errors import SpecError, did_you_mean
from ggrust.plot.coord import coord_fixed
from ggrust.plot.core import aes
from ggrust.plot.geom import geom_point, geom_text, geom_tile
from ggrust.plot.plot import ggplot, ggsize
from ggrust.plot.scale import scale_x_discrete, scale_y_discrete
from ggrust.plot.scale_color import (
    scale_color_brewer,
    scale_color_gradient2,
    scale_fill_brewer,
    scale_fill_gradient2,
)
from ggrust.plot.scale_identity_ import scale_size_identity
from ggrust.plot.theme import element_blank, element_line, theme
from ggrust.plot.tooltip import layer_tooltips

if TYPE_CHECKING:
    from ggrust._typing import PlotData
    from ggrust.plot.core import FeatureSpec, LayerSpec, PlotSpec

__all__ = ["corr_plot"]

#: Which cells of the matrix a layer draws.
MatrixType = Literal["upper", "lower", "full"]

_TYPES: tuple[MatrixType, ...] = ("upper", "lower", "full")
_DEFAULT_LOW = "#B3412C"
_DEFAULT_MID = "#EDEDED"
_DEFAULT_HIGH = "#326C81"
_BREAKS = [-1.0, -0.5, 0.0, 0.5, 1.0]
_BREAK_LABELS = ["-1", "-0.5", "0", "0.5", "1"]
_NA_COLOR = "rgba(0,0,0,0)"
_VALUE_FORMAT = ".2f"
#: lets-plot sizes the plot from these (pixels): a cell, the grid bounds, the
#: legend, the axis band without labels, and one label character.
_CELL = 40.0
_MIN_GRID = 150.0
_MAX_GRID = 700.0
_LEGEND_WIDTH = 70.0
_AXIS_BAND = 20.0
_CHARACTER_WIDTH = 5.7
#: Axis range past the first and last category, in categories.
_PADDING = 0.6
#: Tiles a little wider than their cell, so no seam shows between them.
_TILE_SIZE = 1.002


@dataclass(frozen=True)
class _LayerParams:
    """Options of one corr_plot layer, before and after the defaults."""

    type: MatrixType | None = None
    diag: bool | None = None
    color: str | None = None
    map_size: bool | None = None


class corr_plot:  # noqa: N801  (lets_plot's name)
    """Build a correlation matrix plot of tiles, points and/or labels.

    Add layers with ``tiles()``, ``points()`` and ``labels()``, pick a palette, then
    call ``build()``. Each layer covers the ``"upper"``, ``"lower"`` or ``"full"``
    matrix; unset types and diagonals are chosen from the combination of layers,
    as lets_plot does (tiles below, points or labels above the diagonal).

    Parameters
    ----------
    data : DataFrame or dict
        Data (Pearson correlations of every numeric column pair are computed,
        over the rows where both are present) or a correlation matrix: square,
        float columns, every value in -1..1 or NaN.
    show_legend : bool, default=True
        False hides the colorbar.
    flip : bool, default=True
        True draws the first variable at the top of the y axis.
    threshold : float, optional
        Cells whose absolute correlation is below this (0..1) are left out;
        variables left without any cell are dropped.

    Examples
    --------
    >>> import numpy as np
    >>> from ggrust.bistro.corr import corr_plot
    >>> rng = np.random.default_rng(42)
    >>> data = {name: rng.normal(size=10) for name in "abcd"}
    >>> plot = corr_plot(data).tiles().labels().palette_RdBu().build()
    """

    def __init__(
        self,
        data: PlotData,
        show_legend: bool = True,
        flip: bool = True,
        threshold: float | None = None,
    ) -> None:
        if threshold is not None and not (0.0 <= threshold <= 1.0):
            message = f"corr_plot(threshold={threshold!r}): expected a number between 0 and 1"
            raise SpecError(message, path="threshold")
        self._data = data
        self._show_legend = show_legend
        self._flip = bool(flip)
        self.threshold = threshold
        self._tiles: _LayerParams | None = None
        self._points: _LayerParams | None = None
        self._labels: _LayerParams | None = None
        self._palette: str | None = None
        self._gradient: tuple[str, str, str] | None = None
        self._halo: tuple[float | None, str | None] = (None, None)

    def _duplicate(self) -> corr_plot:
        """Return a copy, so each builder call leaves ``self`` unchanged."""
        duplicate = corr_plot(self._data, self._show_legend, self._flip, self.threshold)
        duplicate._tiles = self._tiles
        duplicate._points = self._points
        duplicate._labels = self._labels
        duplicate._palette = self._palette
        duplicate._gradient = self._gradient
        duplicate._halo = self._halo
        return duplicate

    def tiles(self, type: MatrixType | None = None, diag: bool | None = None) -> corr_plot:
        """Draw the matrix as square tiles filled by correlation.

        Parameters
        ----------
        type : {"upper", "lower", "full"}, optional
            Cells to draw. Default: from the other layers.
        diag : bool, optional
            Whether the main diagonal is drawn. Default: only for ``"full"``.

        Returns
        -------
        corr_plot
            A new builder with the layer.

        Raises
        ------
        SpecError
            If ``type`` is not one of the accepted values.
        """
        duplicate = self._duplicate()
        duplicate._tiles = _LayerParams(_check_type(type, "tiles"), diag)
        return duplicate

    def points(self, type: MatrixType | None = None, diag: bool | None = None) -> corr_plot:
        """Draw the matrix as circles sized by absolute and colored by correlation.

        Parameters
        ----------
        type : {"upper", "lower", "full"}, optional
            Cells to draw. Default: from the other layers.
        diag : bool, optional
            Whether the main diagonal is drawn. Default: only for ``"full"``.

        Returns
        -------
        corr_plot
            A new builder with the layer.

        Raises
        ------
        SpecError
            If ``type`` is not one of the accepted values.
        """
        duplicate = self._duplicate()
        duplicate._points = _LayerParams(_check_type(type, "points"), diag)
        return duplicate

    def labels(
        self,
        type: MatrixType | None = None,
        diag: bool | None = None,
        map_size: bool | None = None,
        color: str | None = None,
        halo_width: float | None = None,
        halo_color: str | None = None,
    ) -> corr_plot:
        """Write the correlations (two decimals) in the cells.

        Parameters
        ----------
        type : {"upper", "lower", "full"}, optional
            Cells to label. Default: from the other layers.
        diag : bool, optional
            Whether the main diagonal is labelled. Default: only for ``"full"``.
        map_size : bool, optional
            True sizes the text by the absolute correlation. Default: True when the
            labels sit on points.
        color : str, optional
            Text color. Default: white on tiles or points, else colored by correlation.
        halo_width : float, optional
            Width of a text halo; not supported yet (ggrust warns).
        halo_color : str, optional
            Color of the text halo; not supported yet (ggrust warns).

        Returns
        -------
        corr_plot
            A new builder with the layer.

        Raises
        ------
        SpecError
            If ``type`` is not one of the accepted values.
        """
        duplicate = self._duplicate()
        duplicate._labels = _LayerParams(_check_type(type, "labels"), diag, color, map_size)
        duplicate._halo = (halo_width, halo_color)
        return duplicate

    def palette_gradient(self, low: str, mid: str, high: str) -> corr_plot:
        """Color by a diverging gradient from ``low`` (-1) through ``mid`` (0) to ``high`` (1).

        Parameters
        ----------
        low : str
            Color of correlation -1.
        mid : str
            Color of correlation 0.
        high : str
            Color of correlation 1.

        Returns
        -------
        corr_plot
            A new builder with the palette.
        """
        duplicate = self._duplicate()
        duplicate._palette = None
        duplicate._gradient = (low, mid, high)
        return duplicate

    def _brewer(self, palette: str) -> corr_plot:
        """Return a copy colored by the ColorBrewer ``palette``."""
        duplicate = self._duplicate()
        duplicate._palette = palette
        duplicate._gradient = None
        return duplicate

    def palette_BrBG(self) -> corr_plot:  # noqa: N802
        """Color by the ColorBrewer BrBG palette.

        Returns
        -------
        corr_plot
            A new builder with the palette.
        """
        return self._brewer("BrBG")

    def palette_PiYG(self) -> corr_plot:  # noqa: N802
        """Color by the ColorBrewer PiYG palette.

        Returns
        -------
        corr_plot
            A new builder with the palette.
        """
        return self._brewer("PiYG")

    def palette_PRGn(self) -> corr_plot:  # noqa: N802
        """Color by the ColorBrewer PRGn palette.

        Returns
        -------
        corr_plot
            A new builder with the palette.
        """
        return self._brewer("PRGn")

    def palette_PuOr(self) -> corr_plot:  # noqa: N802
        """Color by the ColorBrewer PuOr palette.

        Returns
        -------
        corr_plot
            A new builder with the palette.
        """
        return self._brewer("PuOr")

    def palette_RdBu(self) -> corr_plot:  # noqa: N802
        """Color by the ColorBrewer RdBu palette.

        Returns
        -------
        corr_plot
            A new builder with the palette.
        """
        return self._brewer("RdBu")

    def palette_RdGy(self) -> corr_plot:  # noqa: N802
        """Color by the ColorBrewer RdGy palette.

        Returns
        -------
        corr_plot
            A new builder with the palette.
        """
        return self._brewer("RdGy")

    def palette_RdYlBu(self) -> corr_plot:  # noqa: N802
        """Color by the ColorBrewer RdYlBu palette.

        Returns
        -------
        corr_plot
            A new builder with the palette.
        """
        return self._brewer("RdYlBu")

    def palette_RdYlGn(self) -> corr_plot:  # noqa: N802
        """Color by the ColorBrewer RdYlGn palette.

        Returns
        -------
        corr_plot
            A new builder with the palette.
        """
        return self._brewer("RdYlGn")

    def palette_Spectral(self) -> corr_plot:  # noqa: N802
        """Color by the ColorBrewer Spectral palette.

        Returns
        -------
        corr_plot
            A new builder with the palette.
        """
        return self._brewer("Spectral")

    def build(self) -> PlotSpec:
        """Create the plot.

        Returns
        -------
        PlotSpec
            An ordinary plot (layers, scales, theme, ``coord_fixed`` and
            ``ggsize``); more features can be added with ``+``.

        Raises
        ------
        SpecError
            If no layer was added (call ``tiles()``, ``points()`` or ``labels()``).
        """
        if self._tiles is None and self._points is None and self._labels is None:
            message = (
                "corr_plot: no layer to draw; add one with .tiles(), .points() or .labels() "
                "before .build()"
            )
            raise SpecError(message)
        tiles, points, labels = _configure(self._tiles, self._points, self._labels)
        names, values, _ = correlation_matrix(as_arrow(self._data))
        matrix = _Matrix(names, values, self._flip, self.threshold or 0.0)
        shown = [params for params in (tiles, points, labels) if params is not None]
        keep = matrix.kept_variables(any(params.diag for params in shown))
        x_names = [
            name
            for index, name in enumerate(names)
            if index in keep and any(matrix.covers_column(index, keep, params) for params in shown)
        ]
        y_names = [
            name
            for index, name in enumerate(names)
            if index in keep and any(matrix.covers_row(index, keep, params) for params in shown)
        ]

        tooltips = layer_tooltips().format("@corr", _VALUE_FORMAT).line("@corr")
        layers: list[LayerSpec] = []
        if tiles is not None:
            layers.append(
                geom_tile(
                    aes(x="x", y="y", fill="corr"),
                    data=matrix.cells(keep, tiles),
                    width=_TILE_SIZE,
                    height=_TILE_SIZE,
                    show_legend=self._show_legend,
                    tooltips=tooltips,
                )
            )
        if points is not None:
            layers.append(
                geom_point(
                    aes(x="x", y="y", size="corr_abs", color="corr"),
                    data=matrix.cells(keep, points),
                    size_unit="x",
                    show_legend=self._show_legend,
                    tooltips=tooltips,
                )
            )
        if labels is not None:
            mapping = aes(
                x="x",
                y="y",
                label="label",
                size="corr_abs" if labels.map_size else None,
                color=None if labels.color else "corr",
            )
            constants: dict[str, Any] = {}  # geom_text options of mixed types
            if labels.color:
                constants["color"] = labels.color
            if not labels.map_size:
                constants["size"] = 1
            halo_width, halo_color = self._halo
            if halo_width is not None:
                constants["halo_width"] = halo_width
            if halo_color is not None:
                constants["halo_color"] = halo_color
            layers.append(
                geom_text(
                    mapping,
                    data=matrix.cells(keep, labels),
                    size_unit="x",
                    show_legend=self._show_legend,
                    tooltips=tooltips,
                    **constants,
                )
            )

        plot = ggplot()
        for layer in layers:
            plot += layer
        for scale in self._color_scales():
            plot += scale
        plot += scale_size_identity(guide="none", na_value=0)
        plot += scale_x_discrete(limits=x_names)
        plot += scale_y_discrete(limits=y_names[::-1] if self._flip else y_names)
        # lets-plot's discrete range: the categories padded by 0.6 on each side.
        plot += coord_fixed(
            xlim=[-_PADDING, len(x_names) - 1 + _PADDING],
            ylim=[-_PADDING, len(y_names) - 1 + _PADDING],
        )
        plot += theme(
            axis_title=element_blank(),
            legend_title=element_blank(),
            panel_grid=element_blank(),
            axis_line=element_blank(),
            axis_ticks_y=element_line(),
        )
        plot += ggsize(*_plot_size(x_names, y_names, self._show_legend))
        return plot

    def _color_scales(self) -> list[FeatureSpec]:
        """Return the color and fill scales of the palette, over -1..1."""
        limits = [-1.0, 1.0]
        if self._palette is not None:
            palette = self._palette
            return [
                scale(
                    palette=palette,
                    breaks=_BREAKS,
                    labels=_BREAK_LABELS,
                    limits=limits,
                    na_value=_NA_COLOR,
                )
                for scale in (scale_color_brewer, scale_fill_brewer)
            ]
        low, mid, high = self._gradient or (_DEFAULT_LOW, _DEFAULT_MID, _DEFAULT_HIGH)
        return [
            scale(
                low=low,
                mid=mid,
                high=high,
                midpoint=0,
                breaks=_BREAKS,
                labels=_BREAK_LABELS,
                limits=limits,
                na_value=_NA_COLOR,
            )
            for scale in (scale_color_gradient2, scale_fill_gradient2)
        ]


def _check_type(value: MatrixType | None, layer: str) -> MatrixType | None:
    """Return ``value`` if it is a matrix type (or None), else raise ``SpecError``."""
    if value is None or value in _TYPES:
        return value
    suggestion = did_you_mean(str(value), _TYPES)
    message = f"corr_plot.{layer}(type={value!r}): expected 'upper', 'lower' or 'full'{suggestion}"
    raise SpecError(message, path=f"{layer}.type")


def _complement(value: MatrixType) -> MatrixType:
    """Return the other triangle; ``"full"`` stays ``"full"``."""
    return {"upper": "lower", "lower": "upper", "full": "full"}[value]  # type: ignore[return-value]


def _overlap(first: MatrixType, second: MatrixType) -> bool:
    """Whether two layer types share cells (the diagonal aside)."""
    return first == second or "full" in (first, second)


def _configure(
    tiles: _LayerParams | None,
    points: _LayerParams | None,
    labels: _LayerParams | None,
) -> tuple[_LayerParams | None, _LayerParams | None, _LayerParams | None]:
    """Fill unset types, diagonals, label color and size as lets-plot does.

    Measured on lets-plot 4.11 over every combination of layers and types: a
    single layer is ``"full"``; tiles go below the diagonal, points and labels
    above it; a layer without a type takes the other triangle of the layer it
    pairs with. Diagonals are drawn for ``"full"`` layers. Labels are white over
    tiles or points (else colored by correlation) and sized by the correlation
    over points.
    """
    tile_type = tiles.type if tiles else None
    point_type = points.type if points else None
    label_type = labels.type if labels else None
    if labels and labels.color and label_type is None:
        # Labels in a color of their own cover the whole matrix (measured).
        label_type = "full"
    if tiles and points:
        if tile_type is None and point_type is None:
            tile_type, point_type = "lower", "upper"
        elif tile_type is None:
            tile_type = "upper" if point_type == "lower" else "lower"
        elif point_type is None:
            point_type = _complement(tile_type)
        if labels and label_type is None:
            label_type = _complement(tile_type)
    elif tiles and labels:
        if tile_type is None and label_type is None:
            tile_type, label_type = "lower", "upper"
        elif tile_type is None:
            tile_type = "full"
        elif label_type is None:
            label_type = _complement(tile_type)
    elif points and labels:
        if point_type is None and label_type is None:
            point_type, label_type = "upper", "lower"
        elif point_type is None:
            point_type = "full"
        elif label_type is None:
            label_type = _complement(point_type)
    else:
        tile_type, point_type, label_type = (
            tile_type or "full",
            point_type or "full",
            label_type or "full",
        )

    def done(params: _LayerParams | None, layer_type: MatrixType) -> _LayerParams | None:
        if params is None:
            return None
        diag = params.diag if params.diag is not None else layer_type == "full"
        return replace(params, type=layer_type, diag=diag)

    tiles = done(tiles, tile_type or "full")
    points = done(points, point_type or "full")
    labels = done(labels, label_type or "full")
    if labels is not None:
        assert labels.type is not None
        on_tiles = (
            tiles is not None and tiles.type is not None and _overlap(labels.type, tiles.type)
        )
        on_points = (
            points is not None and points.type is not None and _overlap(labels.type, points.type)
        )
        labels = replace(
            labels,
            color=labels.color or ("white" if on_tiles or on_points else None),
            map_size=labels.map_size if labels.map_size is not None else on_points,
        )
    return tiles, points, labels


class _Matrix:
    """The correlations and which cells each layer draws."""

    def __init__(self, names: list[str], values: list[float], flip: bool, threshold: float) -> None:
        self.names = names
        self.size = len(names)
        self.values = values
        self.flip = flip
        self.threshold = threshold

    def value(self, row: int, column: int) -> float:
        """Correlation of variable ``row`` with variable ``column``."""
        return self.values[row * self.size + column]

    def passes(self, row: int, column: int) -> bool:
        """Whether the cell clears the threshold (NaN cells are kept, as lets-plot)."""
        return not abs(self.value(row, column)) < self.threshold

    def kept_variables(self, with_diagonal: bool) -> set[int]:
        """Variables with at least one cell over the threshold."""
        return {
            row
            for row in range(self.size)
            if any(
                self.passes(row, column)
                for column in range(self.size)
                if column != row or with_diagonal
            )
        }

    def in_type(self, row: int, column: int, params: _LayerParams) -> bool:
        """Whether the layer covers the cell; "upper" is above the diagonal as drawn."""
        if row == column:
            return bool(params.diag)
        if params.type == "full":
            return True
        above = column > row if self.flip else column < row
        return above == (params.type == "upper")

    def covers_column(self, column: int, keep: set[int], params: _LayerParams) -> bool:
        """Whether the layer has a cell on x = variable ``column``."""
        return any(self.in_type(row, column, params) for row in keep)

    def covers_row(self, row: int, keep: set[int], params: _LayerParams) -> bool:
        """Whether the layer has a cell on y = variable ``row``."""
        return any(self.in_type(row, column, params) for column in keep)

    def cells(self, keep: set[int], params: _LayerParams) -> pl.DataFrame:
        """Return the layer's rows: ``x``, ``y``, ``corr``, ``corr_abs`` and the ``label`` text."""
        rows = [
            (row, column)
            for row in sorted(keep)
            for column in sorted(keep)
            if self.in_type(row, column, params) and self.passes(row, column)
        ]
        correlations = [self.value(row, column) for row, column in rows]
        return pl.DataFrame(
            {
                "x": [self.names[column] for _, column in rows],
                "y": [self.names[row] for row, _ in rows],
                "corr": correlations,
                "corr_abs": [abs(value) for value in correlations],
                "label": [_format_value(value) for value in correlations],
            },
            schema={
                "x": pl.String,
                "y": pl.String,
                "corr": pl.Float64,
                "corr_abs": pl.Float64,
                "label": pl.String,
            },
        )


def _format_value(value: float) -> str:
    """Two decimals, as lets-plot's ``.2f`` (NaN as ``NaN``)."""
    return "NaN" if math.isnan(value) else f"{value:.2f}"


def _plot_size(x_names: list[str], y_names: list[str], show_legend: bool) -> tuple[int, int]:
    """Plot width and height, as measured on lets-plot 4.11.

    40 px per variable within 150..700 px, plus 5.7 px per character of the
    longest x name and the legend's 70 px across; the y names add half their
    width down when wider than a cell, else 20 px.
    """

    def grid(count: int) -> float:
        return min(max(_CELL * count, _MIN_GRID), _MAX_GRID)

    longest_x = max((len(name) for name in x_names), default=0)
    longest_y = max((len(name) for name in y_names), default=0)
    width = (
        grid(len(x_names)) + _CHARACTER_WIDTH * longest_x + (_LEGEND_WIDTH if show_legend else 0.0)
    )
    label_width = _CHARACTER_WIDTH * longest_y
    cell = grid(len(x_names)) / max(len(x_names), 1)
    height = grid(len(y_names)) + (label_width / 2 if label_width > cell else _AXIS_BAND)
    return math.floor(width), math.floor(height)
