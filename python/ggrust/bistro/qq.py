"""``qq_plot``: a quantile-quantile plot with a fitted line and marginal densities.

Follows lets-plot's bistro ``qq_plot`` (``lets_plot/bistro/qq.py`` for the builder,
the Kotlin ``QQPlotOptionsBuilder`` for the layers; MIT, see THIRD_PARTY_NOTICES.md).
lets_plot hands the options to its engine; ggrust builds the plot here from
``geom_qq``/``geom_qq_line`` (or ``geom_qq2``/``geom_qq2_line``), ``theme_light`` and
``ggmarginal`` layers. The layer rules reproduce lets-plot 4.11's measured output;
the margins show the computed quantiles, so they are drawn from the qq layer's
``layer_data`` (the stat runs once in Rust when the plot is built).
"""

from __future__ import annotations

import warnings
from typing import TYPE_CHECKING, Any, cast

import polars as pl

from ggrust._render import GgrustWarning
from ggrust.bistro._marginal import MarginalItem, parse_marginal
from ggrust.errors import SpecError
from ggrust.mapping import as_discrete
from ggrust.plot.core import DummySpec, aes
from ggrust.plot.geom import geom_area, geom_boxplot, geom_histogram
from ggrust.plot.geom_qq import geom_qq, geom_qq2, geom_qq2_line, geom_qq_line
from ggrust.plot.label import xlab, ylab
from ggrust.plot.plot import ggplot
from ggrust.plot.side import ggmarginal
from ggrust.plot.theme import theme_light

if TYPE_CHECKING:
    from collections.abc import Sequence

    from ggrust._typing import PlotData
    from ggrust.plot.core import FeatureSpec, FeatureSpecArray, LayerSpec, PlotSpec
    from ggrust.plot.geom_qq import Distribution

__all__ = ["qq_plot"]

_DISTRIBUTION_DEFAULT = "norm"
_MARGINAL_DEFAULT = "dens:tr"
_ALPHA_DEFAULT = 0.5
_SIZE_DEFAULT = 3.0
_LINE_COLOR_DEFAULT = "#FF0000"
_LINE_SIZE_DEFAULT = 0.75
#: Column names of the marginal layers' data (the computed quantiles).
_MARGIN_X = "__qq_x"
_MARGIN_Y = "__qq_y"


def qq_plot(
    data: PlotData | None = None,
    sample: str | None = None,
    *,
    x: str | None = None,
    y: str | None = None,
    distribution: Distribution | None = None,
    dparams: Sequence[float] | None = None,
    quantiles: Sequence[float] | None = None,
    group: str | None = None,
    show_legend: bool | None = None,
    marginal: str | None = None,
    color: str | None = None,
    fill: str | None = None,
    alpha: float | None = None,
    size: float | None = None,
    shape: int | str | None = None,
    line_color: str | None = None,
    line_size: float | None = None,
    linetype: int | str | Sequence[Any] | None = None,
) -> PlotSpec:
    """Draw a Q-Q plot: sample quantiles against theoretical (or other sample) quantiles.

    As lets_plot's ``qq_plot``. Give ``sample`` to compare a column with a
    theoretical ``distribution``, or ``x`` and ``y`` to compare two columns. Points
    show the quantile pairs, a line goes through the pair of ``quantiles``, and the
    margins show the distributions of the quantiles on each axis.

    Parameters
    ----------
    data : DataFrame or dict
        The data.
    sample : str, optional
        Column of observations to compare with ``distribution``.
    x, y : str, optional
        Columns of observations to compare with each other (both, without ``sample``).
    distribution : {"norm", "uniform", "t", "gamma", "exp", "chi2"}, default="norm"
        Theoretical distribution, with ``sample``.
    dparams : list of float, optional
        Parameters of ``distribution`` (see ``geom_qq``), with ``sample``.
    quantiles : list of float, default=[0.25, 0.75]
        The two quantiles the line goes through.
    group : str, optional
        Column grouping the observations: quantiles, lines and margins per group,
        colored by group unless ``color`` / ``line_color`` is given.
    show_legend : bool, default=True
        False hides the legend.
    marginal : str, default="dens:tr"
        The margins as ``"geom:sides[:size]"`` items separated by commas: geom one of
        ``"dens"``/``"density"``, ``"hist"``/``"histogram"``, ``"box"``/``"boxplot"``,
        sides any of ``"t"``, ``"b"``, ``"l"``, ``"r"``, size a share of the plot (see
        ``ggmarginal``); ``"none"`` draws no margins.
    color : str, optional
        Color of the points and of the margins' outlines.
    fill : str, optional
        Fill of the points (shapes 21 to 25) and of the margins.
    alpha : float, default=0.5
        Transparency of the points and of the margins' fill, in 0..1.
    size : float, default=3.0
        Size of the points.
    shape : int or str, optional
        Shape of the points, 0 to 25.
    line_color : str, default="#FF0000"
        Color of the line (one line per group, in the group color, when ``group`` is
        given and ``line_color`` is not).
    line_size : float, default=0.75
        Width of the line.
    linetype : int or str or list, optional
        Type of the line: a code 0 to 6, a name such as ``"dashed"``, a hex string or
        a dash list.

    Returns
    -------
    PlotSpec
        The plot; add more layers, scales, facets and themes with ``+``.

    Raises
    ------
    SpecError
        If neither ``sample`` nor both ``x`` and ``y`` are given, if both kinds are
        given, or if a ``marginal`` item is not accepted.
    ColumnNotFoundError
        If a named column is not in ``data`` (raised while computing the margins).

    Examples
    --------
    >>> import numpy as np
    >>> from ggrust import *
    >>> from ggrust.bistro.qq import qq_plot
    >>> rng = np.random.default_rng(42)
    >>> data = {"waiting": rng.exponential(10, 150), "group": rng.choice(["a", "b"], 150)}
    >>> plot = qq_plot(data, "waiting", distribution="exp", group="group")
    """
    two_samples = _check_columns(sample, x, y)
    points_mapping: dict[str, Any] = {"sample": sample} if sample is not None else {}
    if two_samples:
        assert x is not None and y is not None
        points_mapping = {"x": x, "y": y}
    line_mapping = dict(points_mapping)
    if group is not None:
        # As lets-plot: a numeric group column still makes groups.
        points_mapping |= {"color": as_discrete(group), "fill": as_discrete(group)}
        line_mapping |= {"color": group} if line_color is None else {"group": group}
    point_options: dict[str, Any] = {
        "color": color,
        "fill": fill,
        "alpha": _ALPHA_DEFAULT if alpha is None else alpha,
        "size": _SIZE_DEFAULT if size is None else size,
        "shape": shape,
        "show_legend": show_legend,
    }
    line_options: dict[str, Any] = {
        "quantiles": None if quantiles is None else list(quantiles),
        "color": _line_color(line_color, group),
        "size": _LINE_SIZE_DEFAULT if line_size is None else line_size,
        "linetype": linetype,
        "show_legend": show_legend,
    }
    if two_samples:
        points = geom_qq2(aes(**points_mapping), **point_options)
        line = geom_qq2_line(aes(**line_mapping), **line_options)
        quantiles_layer = geom_qq2(aes(**points_mapping))
        x_title, y_title = f"{x} quantiles", f"{y} quantiles"
    else:
        dparams_list = None if dparams is None else list(dparams)
        points = geom_qq(
            aes(**points_mapping),
            distribution=distribution,
            dparams=dparams_list,
            **point_options,
        )
        line = geom_qq_line(
            aes(**line_mapping),
            distribution=distribution,
            dparams=dparams_list,
            **line_options,
        )
        quantiles_layer = geom_qq(
            aes(**points_mapping), distribution=distribution, dparams=dparams_list
        )
        x_title = f'"{distribution or _DISTRIBUTION_DEFAULT}" distribution quantiles'
        y_title = f"{sample} quantiles"

    plot = ggplot(data) + points + line
    marginal = marginal or _MARGINAL_DEFAULT
    if marginal != "none":
        items = parse_marginal(marginal, "qq_plot")
        if items:
            margin_data = _margin_data(ggplot(data) + quantiles_layer, group)
            plot += _marginal_layers(items, margin_data, group, color, fill, alpha)
    return plot + theme_light() + xlab(x_title) + ylab(y_title)


def _line_color(line_color: str | None, group: str | None) -> str | None:
    """Return the line's constant color: none when the group colors it."""
    if line_color is not None or group is not None:
        return line_color
    return _LINE_COLOR_DEFAULT


def _check_columns(sample: str | None, x: str | None, y: str | None) -> bool:
    """Check the sample / x, y choice; return True for a two-sample plot."""
    if sample is not None:
        if x is not None or y is not None:
            message = (
                "qq_plot: give either sample= (sample against a distribution) "
                "or x= and y= (two samples), not both"
            )
            raise SpecError(message, path="sample")
        return False
    if x is None or y is None:
        missing = "x" if x is None and y is not None else "y" if y is None and x is not None else ""
        if missing:
            message = f"qq_plot: {missing}= is missing; a two-sample Q-Q plot needs both x= and y="
            raise SpecError(message, path=missing)
        message = (
            "qq_plot: no data column given; pass sample= (sample against a distribution) "
            "or x= and y= (two samples)"
        )
        raise SpecError(message, path="sample")
    return True


def _margin_names(group: str | None) -> tuple[str, str]:
    """Return the margin data's x and y column names, never the group's."""
    x_name, y_name = _MARGIN_X, _MARGIN_Y
    while group in (x_name, y_name):
        x_name, y_name = x_name + "_", y_name + "_"
    return x_name, y_name


def _margin_data(points_plot: PlotSpec, group: str | None) -> pl.DataFrame:
    """Return the computed quantile pairs (and the group column) of the qq layer.

    ``points_plot`` holds one unstyled qq layer whose ``color`` maps the group.
    """
    with warnings.catch_warnings():
        # Removed missing values are reported once, when the plot is drawn.
        warnings.simplefilter("ignore", GgrustWarning)
        computed = points_plot.layer_data(0)
    x_name, y_name = _margin_names(group)
    columns = [pl.col("x").alias(x_name), pl.col("y").alias(y_name)]
    if group is not None:
        columns.append(pl.col("color").alias(group))
    return computed.select(columns)


def _marginal_layers(
    items: list[MarginalItem],
    margin_data: pl.DataFrame,
    group: str | None,
    color: str | None,
    fill: str | None,
    alpha: float | None,
) -> FeatureSpec:
    """Return ``ggmarginal`` layers over the computed quantiles, one per side."""
    x_name, y_name = _margin_names(group)
    mapping: dict[str, Any] = {"x": x_name, "y": y_name}
    if group is not None:
        mapping |= {"color": as_discrete(group), "fill": as_discrete(group)}
    options: dict[str, Any] = {
        "data": margin_data,
        "color": color,
        "fill": fill,
        "alpha": _ALPHA_DEFAULT if alpha is None else alpha,
        # lets-plot lists no margin keys in the legend; ggrust would merge them.
        "show_legend": False,
        "inherit_aes": False,
    }
    result: FeatureSpec = DummySpec()
    for item in items:
        for side in item.sides:
            layer = _marginal_layer(item.geom, mapping, options)
            result += ggmarginal(side, size=item.size, layer=layer)
    return result


def _marginal_layer(
    geom_kind: str, mapping: dict[str, Any], options: dict[str, Any]
) -> LayerSpec | FeatureSpecArray:
    """Return one marginal layer of ``geom_kind``."""
    if geom_kind in ("dens", "density"):
        return geom_area(aes(**mapping), stat="density", position="identity", **options)
    if geom_kind in ("hist", "histogram"):
        return geom_histogram(aes(**mapping), **options)
    return cast("FeatureSpecArray", geom_boxplot(aes(**mapping), **options))
