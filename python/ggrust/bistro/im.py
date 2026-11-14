"""``image_matrix``: a grid of images, as lets-plot's bistro ``image_matrix``.

Follows ``lets_plot/bistro/im.py`` (MIT, see THIRD_PARTY_NOTICES.md): one
``geom_imshow`` plot per cell without axes, grid or margins, cells
``scale`` x the image size (at least 50 px), ``spacer`` px apart. lets_plot places
the cells with ``ggbunch``; ggrust has no ``ggbunch`` (plan.md decisions), so the
cells are ``inset_element`` figures on an empty ``theme_void`` plot of the grid's
size, and the result is a ``PlotSpec`` instead of a ``SupPlotsSpec``.
"""

from __future__ import annotations

from numbers import Real

import numpy

from ggrust.errors import SpecError
from ggrust.plot.core import PlotSpec
from ggrust.plot.geom_imshow import geom_imshow
from ggrust.plot.inset import inset_element
from ggrust.plot.plot import MAX_HEIGHT, MAX_WIDTH, ggplot, ggsize
from ggrust.plot.scale import scale_x_continuous, scale_y_continuous
from ggrust.plot.theme import theme, theme_void

__all__ = ["image_matrix"]

MINIMUM_SIZE = 50.0
"""Smallest cell side in pixels, as lets_plot."""


def image_matrix(
    image_data_array: numpy.ndarray,
    cmap: str | list[str] | None = None,
    *,
    norm: bool | None = None,
    vmin: float | None = None,
    vmax: float | None = None,
    scale: float = 1,
    spacer: float = 1,
) -> PlotSpec:
    """Show a grid of images, one per cell of a 2D object array.

    Each cell holds a 2D (greyscale) or 3D (RGB or RGBA) image array, as
    ``geom_imshow`` takes, or None for an empty cell. Every column is as wide as
    its widest image and every row as tall as its tallest one.

    Parameters
    ----------
    image_data_array : ndarray
        2D ``numpy`` array of ``dtype=object`` whose cells are images.
    cmap : str or list of str, optional
        Greyscale only: a color map name or a list of colors (see ``geom_imshow``).
    norm : bool, default=True
        Greyscale only: stretch ``vmin..vmax`` to the full luminance range.
    vmin, vmax : float, optional
        Greyscale only: the value range ``norm`` stretches.
    scale : float, default=1
        Image size magnification; each cell is at least 50 x 50 px.
    spacer : float, default=1
        Pixels between images.

    Returns
    -------
    PlotSpec
        An empty plot of the grid's size with one inset per image (lets_plot
        returns a ``SupPlotsSpec`` built with ``ggbunch``).

    Raises
    ------
    SpecError
        When ``image_data_array`` is not a 2D array or is empty, the grid is over
        10000 px on a side, a cell is not a 2D or 3D array,
        or ``scale`` or ``spacer`` is not a finite number in range.

    Examples
    --------
    >>> import numpy as np
    >>> from ggrust.bistro import image_matrix
    >>> rng = np.random.default_rng(42)
    >>> matrix = np.empty((2, 3), dtype=object)
    >>> for row in range(2):
    ...     for column in range(3):
    ...         matrix[row, column] = rng.integers(256, size=(32, 32, 3))
    >>> plot = image_matrix(matrix, scale=2)
    """
    _check_number(scale, "scale", positive=True)
    _check_number(spacer, "spacer", positive=False)
    if not isinstance(image_data_array, numpy.ndarray) or image_data_array.ndim != 2:
        found = (
            f"a {image_data_array.ndim}D array"
            if isinstance(image_data_array, numpy.ndarray)
            else type(image_data_array).__name__
        )
        raise SpecError(
            f"image_matrix image_data_array: expected a 2D numpy array of images, got "
            f"{found}; build one with numpy.empty((rows, columns), dtype=object)"
        )
    rows, columns = image_data_array.shape
    if rows * columns == 0:
        raise SpecError(
            f"image_matrix image_data_array: the grid is empty (shape {rows} x {columns}); "
            "pass at least one row and one column"
        )
    sizes = [
        [_cell_size(image_data_array[row, column], row, column, scale) for column in range(columns)]
        for row in range(rows)
    ]
    cells = [size for line in sizes for size in line if size is not None]
    width = max((cell_width for _, cell_width in cells), default=MINIMUM_SIZE)
    height = max((cell_height for cell_height, _ in cells), default=MINIMUM_SIZE)
    total_width = columns * width + max(columns - 1, 0) * spacer
    total_height = rows * height + max(rows - 1, 0) * spacer
    if total_width > MAX_WIDTH or total_height > MAX_HEIGHT:
        raise SpecError(
            f"image_matrix: the grid would be {total_width:g} x {total_height:g} px, over the "
            f"{MAX_WIDTH} x {MAX_HEIGHT} px limit; lower scale={scale!r} or spacer={spacer!r}"
        )
    plot = ggplot() + theme_void() + ggsize(total_width, total_height)
    options = (
        scale_x_continuous(expand=[0, 0])
        + scale_y_continuous(expand=[0, 0])
        + theme(axis="blank", panel_grid="blank")
        # lets_plot also sets panel_inset=0, ggrust's panels have no inset yet.
        + theme(plot_inset=0, plot_margin=0)
    )
    for row in range(rows):
        for column in range(columns):
            size = sizes[row][column]
            if size is None:
                continue
            cell_height, cell_width = size
            image = geom_imshow(
                image_data_array[row, column],
                cmap=cmap,
                norm=norm,
                vmin=vmin,
                vmax=vmax,
                show_legend=False,
            )
            # Pixel edges first, then fractions clamped to 0..1: the last row or
            # column must not land a rounding error outside the figure.
            left = column * (width + spacer)
            top = row * (height + spacer)
            plot += inset_element(
                ggplot() + image + options,
                left=_fraction(left, total_width),
                bottom=_fraction(total_height - top - cell_height, total_height),
                right=_fraction(left + cell_width, total_width),
                top=_fraction(total_height - top, total_height),
                align_to="full",
            )
    return plot


def _fraction(pixels: float, total: float) -> float:
    """Return ``pixels / total`` clamped to 0..1."""
    return min(max(pixels / total, 0.0), 1.0)


def _cell_size(image: object, row: int, column: int, scale: float) -> tuple[float, float] | None:
    """Return the cell's (height, width) in pixels, or None for an empty cell."""
    if image is None:
        return None
    if not isinstance(image, numpy.ndarray) or image.ndim not in (2, 3):
        found = (
            f"a {image.ndim}D array" if isinstance(image, numpy.ndarray) else type(image).__name__
        )
        raise SpecError(
            f"image_matrix image_data_array[{row}, {column}]: expected a 2D (greyscale) or "
            f"3D (RGB/RGBA) numpy array, or None for an empty cell; got {found}"
        )
    height, width = image.shape[:2]
    return max(height * scale, MINIMUM_SIZE), max(width * scale, MINIMUM_SIZE)


def _check_number(value: object, name: str, *, positive: bool) -> None:
    """Raise SpecError unless ``value`` is a finite number (> 0, or >= 0)."""
    valid = (
        isinstance(value, Real)
        and not isinstance(value, bool)
        and numpy.isfinite(float(value))
        and (float(value) > 0 if positive else float(value) >= 0)
    )
    if not valid:
        bound = "> 0" if positive else ">= 0"
        raise SpecError(f"image_matrix {name}={value!r}: expected a finite number {bound}")
