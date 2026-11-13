"""Polygons with rounded corners (R package ggforce's ``geom_shape``).

Not in lets-plot or ggplot2. ``geom_shape`` draws each group as ``geom_polygon``
does, then rounds its convex corners by ``radius`` and grows it by ``expand``, both in
millimetres of screen space (96 px per inch), so corners stay round under any
aspect ratio. A radius too large for a corner shrinks to fit, so a rectangle with a
radius of at least half its height is a stadium.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from ggrust.plot.geom import _geom
from ggrust.plot.geom_mark import _check_length

if TYPE_CHECKING:
    from ggrust._typing import ManualKey, Number, PlotData, Position, Sampling, Tooltips
    from ggrust.plot.core import FeatureSpec, LayerSpec

__all__ = ["geom_shape"]


def geom_shape(
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
    expand: Number | None = None,
    radius: Number | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Draw polygons with rounded corners, optionally grown outwards.

    As ``geom_shape`` in the R package ggforce: ``geom_polygon`` (one closed shape
    per group, vertices in data order) whose convex corners become circular arcs of
    ``radius`` and whose outline is grown by ``expand``. Concave corners stay sharp.
    Both lengths are in millimetres on the screen, applied after the coordinate
    system, so a rounded corner is round with or without ``coord_fixed()``. A radius
    larger than a corner's edges allow is reduced to fit them: a rectangle with a
    ``radius`` of at least half its height is drawn as a stadium (a pill), for
    example a cell outline.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``. Needs ``x`` and ``y``; ``group`` (or a
        discrete ``fill`` / ``color``) separates the shapes.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Statistical transformation, ``"identity"`` by default.
    position : str or FeatureSpec, optional
        Position adjustment.
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
        Accepted for lets_plot compatibility; shapes have no tooltips yet.
    expand : float, default=0
        Millimetres the shape grows beyond its vertices (non-negative).
    radius : float, default=0
        Corner radius in millimetres (non-negative). Larger than half the shortest
        side rounds that side fully.
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics (``color``, ``fill``, ``alpha``, ``size``,
        ``linetype``).

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If ``expand`` or ``radius`` is negative or not finite.
    UnsupportedValueError
        If ``expand`` or ``radius`` is not a number.

    Examples
    --------
    >>> from ggrust import *
    >>> import numpy as np
    >>> generator = np.random.default_rng(1)
    >>> cell = {"x": [0, 10, 10, 0], "y": [0, 0, 2, 2]}
    >>> spots = {"x": generator.uniform(1, 9, 40), "y": generator.uniform(0.4, 1.6, 40)}
    >>> plot = (ggplot()
    ...         + geom_shape(aes("x", "y"), data=cell, radius=1000, fill="#f2f2f2",
    ...                      color="black")
    ...         + geom_point(aes("x", "y"), data=spots, color="#d62728")
    ...         + coord_fixed())
    """
    for name, value in (("expand", expand), ("radius", radius)):
        _check_length("geom_shape", name, value)
    return _geom(
        "shape",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        expand=expand,
        radius=radius,
        na_rm=na_rm,
        **other_args,
    )
