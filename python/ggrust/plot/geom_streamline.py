"""Streamline layers of a 2D vector field (R package metR's ``geom_streamline``).

Not in lets-plot; ggplot2 needs the metR package. ``stat="streamline"`` turns rows
of positions (``x``, ``y``) and vectors (``u``, ``v``) into streamlines, per group
and facet panel: rows on a regular grid are the field as given, scattered rows are
averaged onto a grid first (as ``velocity_grid()``), then the lines are traced
with matplotlib's integrator (as ``streamplot()``). ``geom_streamline`` draws them
as ``geom_path`` with one arrowhead half way along each line, where matplotlib
puts it.
"""

from __future__ import annotations

from numbers import Real
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from ggrust.errors import SpecError, UnsupportedValueError
from ggrust.plot.geom import _geom
from ggrust.plot.geom_beeswarm import _check_choice, _check_positive
from ggrust.plot.geom_extras import arrow as _arrow

if TYPE_CHECKING:
    from ggrust._typing import ManualKey, Number, PlotData, Position, Sampling, Tooltips
    from ggrust.plot.core import FeatureSpec, LayerSpec

__all__ = ["geom_streamline", "stat_streamline"]

IntegrationDirection: TypeAlias = Literal["both", "forward", "backward"]
"""Which way streamlines are integrated from their seed points."""

_DIRECTIONS: tuple[str, ...] = ("both", "forward", "backward")


class _DefaultArrow:
    """Marker for the default arrowhead (``None`` means no arrowheads)."""

    def __repr__(self) -> str:
        return "arrow(angle=26.6, length=6, type='closed')"


_DEFAULT_ARROW = _DefaultArrow()


def _default_arrow() -> FeatureSpec:
    """Matplotlib's ``'-|>'`` head at ``arrowsize=1``: 4 pt long, 2 pt half width."""
    return _arrow(angle=26.6, length=6, type="closed")


def geom_streamline(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    stat: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    manual_key: ManualKey | None = None,
    sampling: Sampling | None = None,
    tooltips: Tooltips | None = None,
    arrow: FeatureSpec | _DefaultArrow | None = _DEFAULT_ARROW,
    density: Number | tuple[Number, Number] | None = None,
    minlength: Number | None = None,
    maxlength: Number | None = None,
    integration_direction: IntegrationDirection | None = None,
    broken_streamlines: bool | None = None,
    gridded: bool | None = None,
    grid_density: Number | None = None,
    smooth: Number | None = None,
    n_neighbors: int | None = None,
    min_mass: Number | None = None,
    cutoff_percentile: Number | None = None,
    color_by: str | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Streamlines of a vector field, with an arrowhead along each line.

    As ``geom_streamline`` in the R package metR, with matplotlib's algorithm:
    the lines and arrows are the ones ``matplotlib.pyplot.streamplot`` draws. The
    rows give positions (``x``, ``y``) and vectors (``u``, ``v``):

    - rows on a regular grid (every pair of the distinct ``x`` and ``y`` values
      exactly once, such as wind or flow measured on a grid) are the field as given;
    - any other rows (scattered points with a direction each, such as cells with
      RNA velocities on an embedding) are averaged onto a grid first: each grid
      point takes the Gaussian-weighted mean vector of its nearest rows (scVelo's
      ``compute_velocity_on_grid``; ``velocity_grid()`` returns that grid).

    Lines are computed per group and facet panel. Styled as ``geom_path``; a mapped
    style is constant per group (its mean or first value), so map ``color`` to a
    grouping variable, not to a value that varies along the flow. Rows count as a
    grid only when their coordinates repeat exactly; others are smoothed.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Needs ``x``, ``y``, ``u`` (x component of
        the vector) and ``v`` (y component). ``color`` or ``group`` gives each group
        its own field and lines.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, by name; ``"streamline"`` by default.
    position : str or FeatureSpec, optional
        Position adjustment, applied to the line vertices.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    manual_key : str or FeatureSpec, optional
        This layer's key in a manual legend: a label, or ``layer_key()``.
    sampling : str, int or FeatureSpec, optional
        Rows to draw: a ``sampling_*()`` spec, an int ``n`` (``sampling_random(n)``)
        or ``"none"``. Default: every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"``.
    arrow : FeatureSpec or None, default=arrow(angle=26.6, length=6, type="closed")
        The arrowhead half way along each line, from ``arrow()`` (matplotlib's
        ``'-|>'`` head by default). ``None`` draws the lines only.
    density : float or tuple of float, default=1
        Closeness of the lines. ``density=1`` divides the field into a 30 x 30
        mask, each cell crossed by at most one line. A tuple sets x and y apart.
    minlength : float, default=0.1
        Lines shorter than this (in axes coordinates, the field being 1 x 1) are
        dropped.
    maxlength : float, default=4.0
        Longest line, in axes coordinates.
    integration_direction : {"both", "forward", "backward"}, default="both"
        Integrate along the field, against it, or both (each half as long).
    broken_streamlines : bool, default=True
        Stop a line when it comes too close to another. ``False`` lets lines run on.
    gridded : bool, optional
        Whether the rows are a regular grid. By default, rows that form a complete
        grid are one, other rows are scattered. ``True`` also accepts a grid with
        missing points (lines stop there); ``False`` always averages onto a grid.
    grid_density : float, default=1
        Scattered rows only: the grid has ``int(50 * grid_density)`` points per axis.
    smooth : float, default=0.5
        Scattered rows only: width of the Gaussian kernel, in grid steps.
    n_neighbors : int, optional
        Scattered rows only: rows averaged into each grid point. By default
        ``int(n_rows / 50)``, so groups need at least 50 rows unless it is given.
    min_mass : float, default=1
        Scattered rows only: grid points with less kernel weight than
        ``10 ** (min_mass - 6)`` are left empty (from 0, all, to about 5).
    cutoff_percentile : float, default=5
        Scattered rows only: grid points whose neighbours move slower than this
        percentile of all grid points are left empty.
    color_by : str, optional
        Aesthetic the ``color`` scale applies to (``"color"`` or ``"fill"``).
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics (``color``, ``size``, ``alpha``, ``linetype``).

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If ``integration_direction`` is unknown, or a density, ``smooth`` or
        ``n_neighbors`` is not positive.
    UnsupportedValueError
        If an option has the wrong type (a string for a number, ...).

    Examples
    --------
    Wind on a grid, turning around a low:

    >>> from ggrust import *
    >>> import numpy as np
    >>> east, north = np.meshgrid(np.linspace(-3, 3, 31), np.linspace(-2, 2, 21))
    >>> wind = {
    ...     "east": east.ravel(),
    ...     "north": north.ravel(),
    ...     "u": (-north - 0.3 * east).ravel(),
    ...     "v": (east - 0.3 * north).ravel(),
    ... }
    >>> plot = (
    ...     ggplot(wind, aes("east", "north"))
    ...     + geom_streamline(aes(u="u", v="v"), density=1.2, color="#2b6cb0")
    ...     + coord_fixed()
    ... )

    Scattered points, each with a direction:

    >>> generator = np.random.default_rng(1)
    >>> x, y = generator.normal(size=(2, 2000))
    >>> drift = {"x": x, "y": y, "u": 1 - y**2, "v": x * 0.5}
    >>> plot = ggplot(drift, aes("x", "y")) + geom_point(alpha=0.2) + geom_streamline(
    ...     aes(u="u", v="v"), n_neighbors=40
    ... )
    """
    _check(
        "geom_streamline",
        density=density,
        integration_direction=integration_direction,
        gridded=gridded,
        grid_density=grid_density,
        smooth=smooth,
        n_neighbors=n_neighbors,
    )
    return _geom(
        "streamline",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        arrow=_default_arrow() if isinstance(arrow, _DefaultArrow) else arrow,
        density=list(density) if isinstance(density, tuple) else density,
        minlength=minlength,
        maxlength=maxlength,
        integration_direction=integration_direction,
        broken_streamlines=broken_streamlines,
        gridded=gridded,
        grid_density=grid_density,
        smooth=smooth,
        n_neighbors=n_neighbors,
        min_mass=min_mass,
        cutoff_percentile=cutoff_percentile,
        color_by=color_by,
        na_rm=na_rm,
        **other_args,
    )


def stat_streamline(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    sampling: Sampling | None = None,
    tooltips: Tooltips | None = None,
    density: Number | tuple[Number, Number] | None = None,
    minlength: Number | None = None,
    maxlength: Number | None = None,
    integration_direction: IntegrationDirection | None = None,
    broken_streamlines: bool | None = None,
    gridded: bool | None = None,
    grid_density: Number | None = None,
    smooth: Number | None = None,
    n_neighbors: int | None = None,
    min_mass: Number | None = None,
    cutoff_percentile: Number | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Streamlines of a vector field (``stat="streamline"``), drawn by ``geom``.

    As ``stat_streamline`` in the R package metR. Every streamline is its own
    group of ``x``, ``y`` vertices in order; see ``geom_streamline`` for how the
    field is built and the options.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Needs ``x``, ``y``, ``u`` and ``v``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    geom : str, default="streamline"
        Geom that draws the lines, e.g. ``"path"`` (with ``arrow=arrow()`` for
        heads at the line ends) or ``"point"`` for the vertices.
    position : str or FeatureSpec, optional
        Position adjustment, applied to the line vertices.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping for this layer.
    sampling : str, int or FeatureSpec, optional
        Rows to draw: a ``sampling_*()`` spec, an int ``n`` (``sampling_random(n)``)
        or ``"none"``. Default: every row.
    tooltips : FeatureSpec or list of str, optional
        Tooltip content: ``layer_tooltips()``, a list of column names, or ``"none"``.
    density : float or tuple of float, default=1
        Closeness of the lines (see ``geom_streamline``).
    minlength : float, default=0.1
        Shortest line kept, in axes coordinates.
    maxlength : float, default=4.0
        Longest line, in axes coordinates.
    integration_direction : {"both", "forward", "backward"}, default="both"
        Integrate along the field, against it, or both.
    broken_streamlines : bool, default=True
        Stop a line when it comes too close to another.
    gridded : bool, optional
        Whether the rows are a regular grid (see ``geom_streamline``).
    grid_density : float, default=1
        Scattered rows only: ``int(50 * grid_density)`` grid points per axis.
    smooth : float, default=0.5
        Scattered rows only: Gaussian kernel width in grid steps.
    n_neighbors : int, optional
        Scattered rows only: rows per grid point (default ``int(n_rows / 50)``).
    min_mass : float, default=1
        Scattered rows only: kernel weight threshold, ``10 ** (min_mass - 6)``.
    cutoff_percentile : float, default=5
        Scattered rows only: speed percentile below which grid points are empty.
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics and the geom's options (``arrow=arrow()``).

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If ``integration_direction`` is unknown, or a density, ``smooth`` or
        ``n_neighbors`` is not positive.
    UnsupportedValueError
        If an option has the wrong type.

    Examples
    --------
    >>> from ggrust import *
    >>> import numpy as np
    >>> east, north = np.meshgrid(np.linspace(-2, 2, 21), np.linspace(-2, 2, 21))
    >>> field = {"x": east.ravel(), "y": north.ravel(), "u": -north.ravel(), "v": east.ravel()}
    >>> plot = ggplot(field, aes("x", "y", u="u", v="v")) + stat_streamline(
    ...     geom="path", arrow=arrow(length=5)
    ... )
    """
    _check(
        "stat_streamline",
        density=density,
        integration_direction=integration_direction,
        gridded=gridded,
        grid_density=grid_density,
        smooth=smooth,
        n_neighbors=n_neighbors,
    )
    return _geom(
        geom if geom is not None else "streamline",
        mapping=mapping,
        data=data,
        stat="streamline",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        sampling=sampling,
        tooltips=tooltips,
        density=list(density) if isinstance(density, tuple) else density,
        minlength=minlength,
        maxlength=maxlength,
        integration_direction=integration_direction,
        broken_streamlines=broken_streamlines,
        gridded=gridded,
        grid_density=grid_density,
        smooth=smooth,
        n_neighbors=n_neighbors,
        min_mass=min_mass,
        cutoff_percentile=cutoff_percentile,
        na_rm=na_rm,
        **other_args,
    )


def _check(
    function: str,
    *,
    density: object,
    integration_direction: object,
    gridded: object,
    grid_density: object,
    smooth: object,
    n_neighbors: object,
) -> None:
    """Fail early on options that are wrong whatever the data."""
    _check_choice(function, "integration_direction", integration_direction, _DIRECTIONS)
    if isinstance(density, tuple):
        if len(density) != 2:
            reason = f"expected a number or a pair (x, y), got {len(density)} values"
            raise SpecError(
                f"{function}(density={density!r}): {reason}", path="density", reason=reason
            )
        for value in density:
            _check_positive(function, "density", value)
    else:
        _check_positive(function, "density", density)
    _check_positive(function, "grid_density", grid_density)
    _check_positive(function, "smooth", smooth)
    if gridded is not None and not isinstance(gridded, bool):
        reason = f"expected True, False or None, got {type(gridded).__name__}"
        raise UnsupportedValueError(
            f"{function}(gridded={gridded!r}): {reason}", path="gridded", reason=reason
        )
    if n_neighbors is not None:
        if isinstance(n_neighbors, bool) or not isinstance(n_neighbors, Real):
            reason = f"expected a whole number, got {type(n_neighbors).__name__}"
            raise UnsupportedValueError(
                f"{function}(n_neighbors={n_neighbors!r}): {reason}",
                path="n_neighbors",
                reason=reason,
            )
        if not (float(n_neighbors) >= 1 and float(n_neighbors).is_integer()):
            reason = f"n_neighbors must be a whole number of at least 1, got {n_neighbors}"
            raise SpecError(
                f"{function}(n_neighbors={n_neighbors!r}): {reason}",
                path="n_neighbors",
                reason=reason,
            )
