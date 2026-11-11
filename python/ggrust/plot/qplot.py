"""``qplot()``: a whole plot in one call, as ggplot2's and plotnine's quick plot.

Builds an ordinary ``ggplot()`` plus layers, scales, facets and labels; it adds no
drawing of its own. lets_plot has no ``qplot``; ggplot2 deprecated it in 3.4.0 and
plotnine keeps it. ``+`` still works on the result.
"""

from __future__ import annotations

from collections.abc import Callable, Sequence
from typing import TYPE_CHECKING, Any, Literal

import polars as pl

from ggrust.errors import SpecError, did_you_mean

if TYPE_CHECKING:
    from ggrust._typing import Number, PlotData
    from ggrust.plot.core import FeatureSpec, PlotSpec

__all__ = ["qplot"]

MAPPED_AESTHETICS: frozenset[str] = frozenset(
    {
        "color",
        "colour",
        "fill",
        "alpha",
        "shape",
        "linetype",
        "size",
        "stroke",
        "linewidth",
        "group",
        "label",
        "weight",
        "z",
        "xmin",
        "xmax",
        "ymin",
        "ymax",
        "xend",
        "yend",
        "sample",
    }
)
"""``qplot`` keyword arguments mapped when their value is a string or a list."""


def qplot(
    x: str | Sequence[Any] | None = None,
    y: str | Sequence[Any] | None = None,
    data: PlotData | None = None,
    geom: str | Sequence[str] = "auto",
    *,
    facets: str | None = None,
    xlim: Sequence[Number] | None = None,
    ylim: Sequence[Number] | None = None,
    log: Literal["x", "y", "xy"] | None = None,
    main: str | None = None,
    xlab: str | None = None,
    ylab: str | None = None,
    asp: Number | None = None,
    **kwargs: Any,
) -> PlotSpec:
    """Make a quick plot in one call, as ggplot2's and plotnine's ``qplot``.

    Keyword arguments named after an aesthetic (``color``, ``fill``, ``size``,
    ``shape``, ``label``, ...) with a string or list value are mapped like ``aes()``
    arguments: ``color="origin"`` maps the column. Every other keyword argument goes
    to each geom: ``size=4`` sets a constant (ggplot2 needs ``I(4)``) and ``bins=10``
    is a ``geom_histogram`` option. Not in lets_plot.

    Parameters
    ----------
    x, y : str or list, optional
        Columns of ``data`` (or values, as in ``aes()``) mapped to x and y.
    data : DataFrame or dict, optional
        The plot data.
    geom : str or list of str, default="auto"
        Geom names, without ``geom_`` (``"point"``, ``["point", "smooth"]``).
        ``"auto"`` draws points for x and y, and for x alone a histogram, or bars
        when x is not numeric, as ggplot2.
    facets : str, optional
        A formula: ``"~ a"`` or ``"~ a + b"`` wraps panels (``facet_wrap``),
        ``"a ~ b"`` makes a grid with ``a`` in rows and ``b`` in columns
        (``facet_grid``); ``.`` leaves a side empty.
    xlim, ylim : list of float, optional
        Axis limits ``[low, high]``.
    log : {"x", "y", "xy"}, optional
        Axes with a log10 scale.
    main : str, optional
        Plot title.
    xlab, ylab : str, optional
        Axis titles.
    asp : float, optional
        Aspect ratio y/x of the panel units (``coord_fixed(ratio=asp)``).
    **kwargs
        Aesthetic mappings (names listed above) and geom options.

    Returns
    -------
    PlotSpec
        The plot. Add layers, scales and features with ``+``.

    Raises
    ------
    SpecError
        If ``geom`` is not a name or list of names or names no geom, ``y`` is
        given without ``x``, ``log`` is not ``"x"``, ``"y"`` or ``"xy"``, or
        ``facets`` is not a formula.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15],
    ...         "origin": ["EU", "EU", "US", "EU", "US", "US"]}
    >>> plot = qplot("weight", "mpg", data, color="origin", size=4, main="Cars")
    """
    import ggrust.plot as plot_api

    if y is not None and x is None:
        message = "qplot(): 'y' needs 'x'; pass x too, e.g. qplot('time', 'value', data)"
        raise SpecError(message, path="x", reason=message)
    mapping_arguments = {
        key: kwargs.pop(key)
        for key in list(kwargs)
        if key in MAPPED_AESTHETICS and _is_mappable(kwargs[key])
    }
    plot = plot_api.ggplot(data, plot_api.aes(x, y, **mapping_arguments))
    for name in _geom_names(geom, x, y, data):
        plot += _geom_function(name)(**kwargs)
    if facets is not None:
        plot += _facet(facets)
    plot = _with_scales(plot, xlim, ylim, log)
    if asp is not None:
        plot += plot_api.coord_fixed(ratio=asp)
    if main is not None:
        plot += plot_api.ggtitle(main)
    if xlab is not None:
        plot += plot_api.xlab(xlab)
    if ylab is not None:
        plot += plot_api.ylab(ylab)
    return plot


def _is_mappable(value: object) -> bool:
    """Return whether an aesthetic keyword value is mapped rather than a constant.

    Parameters
    ----------
    value : object
        The keyword value.

    Returns
    -------
    bool
        True for a column name and for values with a length (list, tuple, numpy
        array, pandas or polars Series); False for scalars.
    """
    return isinstance(value, str) or (
        hasattr(value, "__len__") and not isinstance(value, bytes | dict)
    )


def _geom_names(
    geom: str | Sequence[str],
    x: str | Sequence[Any] | None,
    y: str | Sequence[Any] | None,
    data: PlotData | None,
) -> list[str]:
    """Return the geom names to draw, with ``"auto"`` resolved.

    Parameters
    ----------
    geom : str or list of str
        The ``qplot`` geom argument.
    x, y : str or list, optional
        The x and y arguments.
    data : DataFrame or dict, optional
        The plot data.

    Returns
    -------
    list of str
        Geom names without ``geom_``.
    """
    if not isinstance(geom, str | Sequence) or not geom:
        message = (
            f"qplot(): geom must be a geom name or a list of them, got {geom!r}; "
            "pass e.g. 'point', ['point', 'smooth'] or 'auto'"
        )
        raise SpecError(message, path="geom", reason=message)
    names = [geom] if isinstance(geom, str) else list(geom)
    resolved = []
    for name in names:
        if name != "auto":
            resolved.append(name)
        elif x is None:
            resolved.append("blank")
        elif y is not None:
            resolved.append("point")
        else:
            resolved.append("histogram" if _is_numeric(x, data) else "bar")
    return resolved


def _is_numeric(x: str | Sequence[Any], data: PlotData | None) -> bool:
    """Return whether the x values are numeric; True when they cannot be read.

    Parameters
    ----------
    x : str or list
        A column name of ``data``, or the values.
    data : DataFrame or dict, optional
        The plot data.

    Returns
    -------
    bool
        False only for values known to be non-numeric (strings, categories, booleans).
    """
    source: Any = data
    try:
        values: Any = source[x] if isinstance(x, str) else x
        series = values if isinstance(values, pl.Series) else pl.Series(values)
    except Exception:  # any data the check cannot read keeps the numeric default
        return True
    return series.dtype.is_numeric() or series.dtype.is_temporal()


def _geom_function(name: str) -> Callable[..., FeatureSpec]:
    """Return the ``geom_<name>`` function.

    Parameters
    ----------
    name : str
        A geom name without ``geom_``.

    Returns
    -------
    callable
        The geom function.

    Raises
    ------
    SpecError
        If there is no ``geom_<name>``.
    """
    import ggrust.plot as plot_api

    function = getattr(plot_api, f"geom_{name}", None)
    if callable(function):
        return function
    available = sorted(n.removeprefix("geom_") for n in plot_api.__all__ if n.startswith("geom_"))
    message = (
        f"qplot(): unknown geom '{name}'{did_you_mean(name, available)} "
        f"Use a geom name without 'geom_', e.g. 'point', 'line', 'histogram', or 'auto'."
    )
    raise SpecError(message, path="geom", reason=message)


def _facet(formula: str) -> FeatureSpec:
    """Return the facet of a ``qplot`` facets formula.

    Parameters
    ----------
    formula : str
        ``"~ a"``, ``"~ a + b"``, ``"a ~ b"``, ``"a ~ ."`` or ``". ~ b"``.

    Returns
    -------
    FeatureSpec
        A ``facet_wrap`` or ``facet_grid`` spec.

    Raises
    ------
    SpecError
        If the formula has no ``~``, or a grid side names more than one column.
    """
    import ggrust.plot as plot_api

    if formula.count("~") != 1:
        message = f"qplot(): facets {formula!r} is not a formula; pass e.g. '~ a' or 'a ~ b'"
        raise SpecError(message, path="facets", reason=message)
    left, right = (
        [term.strip() for term in side.split("+") if term.strip() not in ("", ".")]
        for side in formula.split("~")
    )
    if not left:
        if not right:
            message = f"qplot(): facets {formula!r} names no column; pass e.g. '~ a'"
            raise SpecError(message, path="facets", reason=message)
        return plot_api.facet_wrap(right)
    if len(left) > 1 or len(right) > 1:
        message = (
            f"qplot(): facets {formula!r} has more than one column on a side of '~'; a grid "
            "takes one row and one column variable ('a ~ b'); use '~ a + b' to wrap"
        )
        raise SpecError(message, path="facets", reason=message)
    return plot_api.facet_grid(x=right[0] if right else None, y=left[0])


def _with_scales(
    plot: PlotSpec,
    xlim: Sequence[Number] | None,
    ylim: Sequence[Number] | None,
    log: str | None,
) -> PlotSpec:
    """Return the plot with the axis limits and log scales of ``qplot``.

    Parameters
    ----------
    plot : PlotSpec
        The plot so far.
    xlim, ylim : list of float, optional
        Axis limits.
    log : str, optional
        ``"x"``, ``"y"`` or ``"xy"``.

    Returns
    -------
    PlotSpec
        The plot with one scale per axis that needs one.

    Raises
    ------
    SpecError
        If ``log`` is not ``"x"``, ``"y"`` or ``"xy"``.
    """
    import ggrust.plot as plot_api

    if log not in (None, "x", "y", "xy", "yx"):
        message = f"qplot(): log={log!r} is not supported; pass 'x', 'y' or 'xy'"
        raise SpecError(message, path="log", reason=message)
    for axis, limits in (("x", xlim), ("y", ylim)):
        limit_list = list(limits) if limits is not None else None
        if log is not None and axis in log:
            plot += getattr(plot_api, f"scale_{axis}_log10")(limits=limit_list)
        elif limit_list is not None:
            plot += getattr(plot_api, f"scale_{axis}_continuous")(limits=limit_list)
    return plot
