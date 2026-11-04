"""gghighlight: ``gghighlight()`` greys out the rows that do not match a predicate.

Not in lets-plot; ggplot2 needs the gghighlight package (MIT). Adding ``gghighlight()``
with ``+`` rewrites the plot in Python, as the R package does at ``+`` time:

- each targeted layer is replaced, in place, by a grey copy over all its rows (the
  "bleached" layer, without its color and fill mappings);
- a copy holding only the matching rows (the "sieved" layer) is added on top;
- optionally, a ``geom_label_repel`` layer labels the highlighted lines (at their
  rightmost point) or points, and the color and fill legends are hidden.

The grey layer keeps only the mapped columns, so a facet added later finds no
faceting column in it and draws it in every panel (``calculate_per_facet=False``).
Predicates are polars expressions, or SQL strings read with ``pl.sql_expr``. No Rust
code is involved (see ``deviations.md``).
"""

from __future__ import annotations

import copy
import re
import warnings
from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Any, Literal, cast

import polars as pl

from ggrust._data import as_arrow
from ggrust._render import GgrustWarning
from ggrust.errors import ColumnNotFoundError, SpecError, UnsupportedValueError, did_you_mean
from ggrust.plot.core import FeatureSpec, LayerSpec, aes

if TYPE_CHECKING:
    from collections.abc import Mapping, Sequence

    from ggrust._typing import PlotData

__all__ = ["gghighlight"]

LineLabelType = Literal["ggrepel_label", "ggrepel_text"]
"""How highlighted lines are labelled (gghighlight's ``line_label_type``)."""

LINE_LABEL_TYPES: tuple[str, ...] = ("ggrepel_label", "ggrepel_text")

UNHIGHLIGHTED_COLOR = "#BEBEBEB2"
"""gghighlight's default grey: R ``alpha("grey", 0.7)`` (alpha 178/255)."""

MAX_LABELS = 20
"""gghighlight's ``getOption("gghighlight_max_labels", 20)``."""

GROUP_COLUMN = "gghighlight_group"
"""Column holding the series id of the grey layer (the R package's secret ``group``)."""

NO_COLOR_GEOMS = frozenset({"bar", "tile", "raster"})
"""Geoms drawn without an outline by default: their grey copy gets no grey outline."""

FILL_GEOMS = frozenset(
    {
        "bar",
        "histogram",
        "area",
        "ribbon",
        "tile",
        "raster",
        "rect",
        "polygon",
        "boxplot",
        "violin",
        "dotplot",
        "ydotplot",
        "crossbar",
        "label",
        "label_repel",
    }
)
"""Geoms whose ggplot2 default ``fill`` is not NA: their grey copy gets a grey fill."""

COLOR_AESTHETICS = ("color", "fill")
"""Mappings the grey layer drops (gghighlight removes ``colour`` and ``fill``)."""


@dataclass(frozen=True)
class HighlightOptions:
    """The validated arguments of ``gghighlight()``."""

    predicates: tuple[pl.Expr, ...]
    n: int | None
    max_highlight: int
    unhighlighted_params: dict[str, object]
    use_group_by: bool | None
    use_direct_label: bool | None
    label_key: str | None
    label_params: dict[str, object]
    keep_scales: bool
    calculate_per_facet: bool
    line_label_type: LineLabelType


class HighlightSpec(FeatureSpec):
    """The ``gghighlight()`` marker: a plot rewrite applied at ``+`` time."""

    def __init__(self, options: HighlightOptions) -> None:
        super().__init__("gghighlight", name=None)
        self.options = options


@dataclass
class HighlightRewrite:
    """What ``+ gghighlight()`` changes in the plot.

    Attributes
    ----------
    layers : list of FeatureSpec
        The new layer list (grey layers in place, highlighted and label layers after).
    data : PlotData or None
        The new plot data (the highlighted rows of the first layer), or the old one.
    hide_legends : bool
        Whether the color and fill legends are hidden (direct labels were added).
    """

    layers: list[FeatureSpec]
    data: PlotData | None
    hide_legends: bool = False


@dataclass
class _Sieved:
    """One layer after the predicates ran: its full and highlighted rows."""

    layer: LayerSpec
    frame: pl.DataFrame
    highlighted: pl.DataFrame
    mapping: dict[str, str]
    meta: dict[str, Any]
    keys: list[str] = field(default_factory=list)


class _NotAggregateError(Exception):
    """A grouped predicate gave more than one value per group."""


def gghighlight(
    *predicates: pl.Expr | str,
    n: int | None = None,
    max_highlight: int = 5,
    unhighlighted_params: Mapping[str, object] | None = None,
    use_group_by: bool | None = None,
    use_direct_label: bool | None = None,
    label_key: str | None = None,
    label_params: Mapping[str, object] | None = None,
    keep_scales: bool = False,
    calculate_per_facet: bool = False,
    line_label_type: LineLabelType = "ggrepel_label",
) -> FeatureSpec:
    """Highlight the rows or series that match predicates (R package gghighlight).

    Added with ``+``, it redraws the layers before it: every row in grey, then the
    matching rows on top in their own colors. A predicate is evaluated per series
    first (the groups of the discrete mappings or ``group``), so
    ``pl.col("y").max() > 10`` keeps whole lines; when it does not give one value per
    series (``pl.col("y") > 10``) it is evaluated per row instead, with a warning.

    Parameters
    ----------
    *predicates : polars.Expr or str
        Conditions on the layer data: polars expressions, or SQL strings such as
        ``"max(y) > 10"`` (read with ``pl.sql_expr``). Several are combined with AND.
        A boolean result filters; a numeric one keeps the ``max_highlight`` series
        (or rows) with the largest values.
    n : int, optional
        Highlight only the last ``n`` layers; None highlights every layer.
    max_highlight : int, default=5
        How many series (or rows) a numeric predicate keeps.
    unhighlighted_params : dict, optional
        Layer parameters of the grey copies, e.g. ``{"color": "pink", "size": 0.5}``.
        The default is gghighlight's grey ``"#BEBEBEB2"`` for ``color`` and ``fill``.
    use_group_by : bool, optional
        True evaluates per series, False per row; None (default) tries per series and
        falls back to per row.
    use_direct_label : bool, optional
        True labels the highlighted lines or points with ``geom_label_repel`` and
        hides the color and fill legends; False keeps the legends. None (default)
        labels when there are at most 20 labels.
    label_key : str, optional
        Column used as the label text; None uses the first series column.
    label_params : dict, optional
        Parameters of the label layer; default ``{"fill": "white"}`` for labels,
        none for ``line_label_type="ggrepel_text"``.
    keep_scales : bool, default=False
        True keeps the color and fill scales trained on every row (adds a
        ``geom_blank`` layer), so the colors match the unhighlighted plot.
    calculate_per_facet : bool, default=False
        True evaluates the predicates per facet panel (the facet must be added before
        ``gghighlight()``), and each panel greys only its own rows. False greys every
        row in every panel.
    line_label_type : {"ggrepel_label", "ggrepel_text"}, default="ggrepel_label"
        Label lines with ``geom_label_repel`` or ``geom_text_repel``.

    Returns
    -------
    FeatureSpec
        The marker, to add to a plot with ``+``.

    Raises
    ------
    SpecError
        When an argument has the wrong value, or a SQL predicate does not parse.
    UnsupportedValueError
        When a predicate is not a polars expression or a string.

    Examples
    --------
    >>> import polars as pl
    >>> from ggrust import *
    >>> data = {
    ...     "day": [1, 2, 3, 4] * 3,
    ...     "sales": [3, 5, 4, 6, 2, 3, 9, 12, 4, 4, 5, 5],
    ...     "shop": ["north"] * 4 + ["south"] * 4 + ["east"] * 4,
    ... }
    >>> plot = (
    ...     ggplot(data, aes("day", "sales", color="shop"))
    ...     + geom_line(size=1.5)
    ...     + gghighlight(pl.col("sales").max() > 8)
    ... )
    >>> plot = ggplot(data, aes("day", "sales")) + geom_point(size=4)
    >>> plot = plot + gghighlight("sales > 4", label_key="shop")
    """
    options = HighlightOptions(
        predicates=tuple(_predicate(index, value) for index, value in enumerate(predicates)),
        n=_optional_count("n", n),
        max_highlight=cast("int", _optional_count("max_highlight", max_highlight)),
        unhighlighted_params=_params("unhighlighted_params", unhighlighted_params),
        use_group_by=_optional_flag("use_group_by", use_group_by),
        use_direct_label=_optional_flag("use_direct_label", use_direct_label),
        label_key=_label_key(label_key),
        label_params=_label_params(label_params, line_label_type),
        keep_scales=bool(_optional_flag("keep_scales", keep_scales)),
        calculate_per_facet=bool(_optional_flag("calculate_per_facet", calculate_per_facet)),
        line_label_type=_line_label_type(line_label_type),
    )
    return HighlightSpec(options)


def _error(path: str, reason: str) -> SpecError:
    """Return a ``SpecError`` at ``gghighlight(path=...)``."""
    return SpecError(f"gghighlight({path}=...): {reason}", path=path, reason=reason)


def _predicate(index: int, value: object) -> pl.Expr:
    """Return predicate ``index`` as a polars expression."""
    path = f"predicates[{index}]"
    if isinstance(value, pl.Expr):
        return value
    if isinstance(value, str):
        try:
            return pl.sql_expr(value)
        except pl.exceptions.PolarsError as error:
            reason = (
                f"could not read the SQL predicate {value!r} ({error}); write it as SQL "
                "(e.g. 'max(y) > 10') or pass a polars expression (pl.col('y').max() > 10)"
            )
            raise _error(path, reason) from error
    reason = (
        f"expected a polars expression or a SQL string, got {type(value).__name__} "
        f"{value!r}; e.g. pl.col('y').max() > 10 or 'max(y) > 10'"
    )
    raise UnsupportedValueError(f"gghighlight({path}=...): {reason}", path=path, reason=reason)


def _optional_count(name: str, value: object) -> int | None:
    """Return ``value`` checked to be a positive int (or None)."""
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, int) or value < 1:
        raise _error(name, f"expected a positive integer, got {value!r}")
    return value


def _optional_flag(name: str, value: object) -> bool | None:
    """Return ``value`` checked to be a bool (or None)."""
    if value is not None and not isinstance(value, bool):
        raise _error(name, f"expected True, False or None, got {value!r}")
    return value


def _params(name: str, value: Mapping[str, object] | None) -> dict[str, object]:
    """Return layer parameters with string keys; ``colour`` is read as ``color``."""
    if value is None:
        return {}
    if not hasattr(value, "items"):
        raise _error(name, f"expected a dict of layer parameters, got {type(value).__name__}")
    params: dict[str, object] = {}
    for key, item in value.items():
        if not isinstance(key, str):
            raise _error(name, f"parameter names must be strings, got {key!r}")
        params["color" if key in ("colour", "col") else key] = item
    return params


def _label_key(value: object) -> str | None:
    """Return the label column name (or None)."""
    if value is not None and not isinstance(value, str):
        raise _error("label_key", f"expected a column name, got {type(value).__name__}")
    return value


def _label_params(value: Mapping[str, object] | None, line_label_type: str) -> dict[str, object]:
    """Return the label layer parameters, gghighlight's defaults when None."""
    if value is None:
        return {} if line_label_type == "ggrepel_text" else {"fill": "white"}
    return _params("label_params", value)


def _line_label_type(value: object) -> LineLabelType:
    """Return ``value`` checked against the supported line label types."""
    if value not in LINE_LABEL_TYPES:
        hint = did_you_mean(value, LINE_LABEL_TYPES) if isinstance(value, str) else ""
        accepted = ", ".join(repr(name) for name in LINE_LABEL_TYPES)
        reason = (
            f"unsupported line label type {value!r}; accepted: {accepted}{hint} "
            "(gghighlight's 'text_path', 'label_path' and 'sec_axis' are not available)"
        )
        raise _error("line_label_type", reason)
    return cast("LineLabelType", value)


# --- The rewrite -------------------------------------------------------------------


def highlight_layers(
    plot_props: dict[str, Any], layers: Sequence[FeatureSpec], spec: HighlightSpec
) -> HighlightRewrite:
    """Return the plot's layers and data after ``+ gghighlight()``.

    Parameters
    ----------
    plot_props : dict
        The plot's properties (``data``, ``mapping``, ``data_meta``, ``facet``).
    layers : list of FeatureSpec
        The plot's layers in ``+`` order.
    spec : HighlightSpec
        The added ``gghighlight()``.

    Returns
    -------
    HighlightRewrite
        The new layers and plot data.

    Raises
    ------
    SpecError
        When ``n`` exceeds the number of layers, when no layer could be highlighted,
        or when ``use_direct_label=True`` finds nothing to label.
    """
    options = spec.options
    plot_data = plot_props.get("data")
    plot_mapping = _mapping_of(plot_props.get("mapping"), "the plot")
    plot_meta = _data_meta(plot_props)
    facet_columns = _facet_columns(plot_props.get("facet")) if options.calculate_per_facet else []
    if options.calculate_per_facet and not facet_columns:
        message = (
            "gghighlight(calculate_per_facet=True): the plot has no facet yet, so the "
            "predicates run on all rows; add facet_wrap() / facet_grid() before gghighlight()"
        )
        warnings.warn(message, GgrustWarning, stacklevel=4)
    if not layers:
        return _highlight_plot_data(plot_data, plot_mapping, plot_meta, facet_columns, options)
    if options.n is not None and options.n > len(layers):
        reason = f"n={options.n} is larger than the number of layers ({len(layers)})"
        raise _error("n", reason)
    first_target = len(layers) - options.n if options.n is not None else 0

    new_layers: list[FeatureSpec] = list(layers)
    sieved_layers: list[_Sieved] = []
    failures: list[str] = []
    for index in range(first_target, len(layers)):
        layer = layers[index]
        if not isinstance(layer, LayerSpec):
            continue
        try:
            sieved = _sieve_layer(layer, plot_data, plot_mapping, plot_meta, facet_columns, options)
        except _LayerFailedError as error:
            failures.append(f"layers[{index}] ({layer.geom}): {error}")
            continue
        new_layers[index] = _bleach_layer(sieved, options)
        sieved_layers.append(sieved)

    if not sieved_layers:
        reason = "no layer could be highlighted: " + "; ".join(failures)
        raise _error("predicates", reason)
    if failures:
        message = "gghighlight(): some layers were left unhighlighted: " + "; ".join(failures)
        warnings.warn(message, GgrustWarning, stacklevel=4)

    new_data = plot_data
    first = sieved_layers[0]
    if plot_data is not None and first.layer.props().get("data") is None:
        # As gghighlight: the plot data becomes the first layer's highlighted rows, so
        # layers added later (``+ geom_text()``) see only them. Inheriting layers left
        # out by ``n`` keep the full data.
        new_data = first.highlighted
        for index in range(first_target):
            layer = new_layers[index]
            if isinstance(layer, LayerSpec) and layer.props().get("data") is None:
                pinned = LayerSpec.duplicate(layer)
                pinned.props()["data"] = plot_data
                new_layers[index] = pinned

    new_layers.extend(_sieved_layer(sieved) for sieved in sieved_layers)
    if options.keep_scales:
        new_layers.extend(_blank_layer(sieved) for sieved in sieved_layers)
    labels, hide_legends = _label_layers(sieved_layers, options)
    new_layers.extend(labels)
    return HighlightRewrite(layers=new_layers, data=new_data, hide_legends=hide_legends)


class _LayerFailedError(Exception):
    """A layer could not be highlighted (the message says why)."""


def _highlight_plot_data(
    plot_data: PlotData | None,
    plot_mapping: dict[str, str],
    plot_meta: dict[str, Any],
    facet_columns: list[str],
    options: HighlightOptions,
) -> HighlightRewrite:
    """With no layers yet, filter the plot data (as gghighlight)."""
    if plot_data is None:
        raise _error("predicates", "the plot has no layers and no data to highlight")
    frame = _as_polars(plot_data)
    keys = _group_keys(frame, plot_mapping, plot_meta)
    try:
        mask = _evaluate(frame, [*keys, *facet_columns], options)
    except _LayerFailedError as error:
        raise _error("predicates", f"could not evaluate on the plot data: {error}") from None
    return HighlightRewrite(layers=[], data=frame.filter(mask))


def _sieve_layer(
    layer: LayerSpec,
    plot_data: PlotData | None,
    plot_mapping: dict[str, str],
    plot_meta: dict[str, Any],
    facet_columns: list[str],
    options: HighlightOptions,
) -> _Sieved:
    """Evaluate the predicates on one layer's rows."""
    data = layer.props().get("data")
    if data is None:
        data = plot_data
    if data is None:
        raise _LayerFailedError("the layer has no data")
    frame = _as_polars(data)
    layer_mapping = _mapping_of(layer.props().get("mapping"), f"layer '{layer.geom}'")
    inherited = {} if layer.props().get("inherit_aes") is False else plot_mapping
    mapping = {**inherited, **layer_mapping}
    meta = _merged_meta(plot_meta, _data_meta(layer.props()), inherited, layer_mapping)
    keys = _group_keys(frame, mapping, meta)
    missing = [column for column in facet_columns if column not in frame.columns]
    if missing:
        raise _LayerFailedError(f"faceting column(s) {missing} are not in the layer data")
    mask = _evaluate(frame, [*keys, *facet_columns], options)
    return _Sieved(
        layer=layer,
        frame=frame,
        highlighted=frame.filter(mask),
        mapping=mapping,
        meta=meta,
        keys=keys,
    )


def _evaluate(frame: pl.DataFrame, keys: list[str], options: HighlightOptions) -> pl.Series:
    """Return the row mask of the highlighted rows (gghighlight's ``sieve_layer``)."""
    if not options.predicates:
        return pl.Series([True] * frame.height, dtype=pl.Boolean)
    if options.use_group_by is not False and keys:
        try:
            return _evaluate_grouped(frame, keys, options)
        except (_NotAggregateError, pl.exceptions.PolarsError) as error:
            reason = (
                "each series must give one value per predicate"
                if isinstance(error, _NotAggregateError)
                else _polars_reason(error, frame)
            )
            message = (
                f"gghighlight(): could not evaluate the predicates per series ({reason}); "
                "evaluating them per row instead (pass use_group_by=False to silence this)"
            )
            warnings.warn(message, GgrustWarning, stacklevel=6)
    elif options.use_group_by is True:
        message = (
            "gghighlight(use_group_by=True): the layer has no series (no discrete mapping "
            "and no group); evaluating the predicates per row"
        )
        warnings.warn(message, GgrustWarning, stacklevel=6)
    try:
        return _evaluate_ungrouped(frame, options)
    except pl.exceptions.PolarsError as error:
        raise _LayerFailedError(_polars_reason(error, frame)) from None


def _predicate_columns(options: HighlightOptions) -> list[pl.Expr]:
    """Return the predicates under private names (``__predicate_0``, ...)."""
    return [
        predicate.alias(f"__predicate_{index}")
        for index, predicate in enumerate(options.predicates)
    ]


def _evaluate_grouped(frame: pl.DataFrame, keys: list[str], options: HighlightOptions) -> pl.Series:
    """Evaluate the predicates once per series, then select whole series."""
    summary = frame.group_by(keys, maintain_order=True).agg(_predicate_columns(options))
    names = [f"__predicate_{index}" for index in range(len(options.predicates))]
    lists = [name for name in names if isinstance(summary.schema[name], pl.List)]
    if lists:
        # A per-row expression gives a list per series; one value each still counts as
        # one value per series (as R's summarise()).
        single = summary.select(pl.all_horizontal(pl.col(lists).list.len() == 1).all()).item()
        if not single:
            raise _NotAggregateError
        summary = summary.with_columns(pl.col(lists).list.first())
    selected = _select(summary, names, options.max_highlight)
    rows = frame.select(keys).with_row_index("__row")
    matched = rows.join(selected.select(keys), on=keys, how="semi", nulls_equal=True)
    mask = pl.Series([False] * frame.height, dtype=pl.Boolean)
    if matched.height:
        mask = mask.scatter(matched["__row"], True)
    return mask


def _evaluate_ungrouped(frame: pl.DataFrame, options: HighlightOptions) -> pl.Series:
    """Evaluate the predicates per row, then select rows."""
    names = [f"__predicate_{index}" for index in range(len(options.predicates))]
    # The row index is computed after the predicates, so polars errors list only the
    # user's columns.
    row = pl.int_range(pl.len(), dtype=pl.UInt32).alias("__row")
    results = frame.select(*_predicate_columns(options), row)
    for name in names:
        if isinstance(results.schema[name], pl.List):
            raise _LayerFailedError(f"predicate {name[12:]} gave a list per row")
    selected = _select(results, names, options.max_highlight)
    mask = pl.Series([False] * frame.height, dtype=pl.Boolean)
    if selected.height:
        mask = mask.scatter(selected["__row"], True)
    return mask


def _select(results: pl.DataFrame, names: list[str], max_highlight: int) -> pl.DataFrame:
    """Filter on the boolean predicates, keep the top ``max_highlight`` by the others."""
    logical = [name for name in names if results.schema[name] == pl.Boolean]
    numeric = [name for name in names if name not in logical]
    if logical:
        results = results.filter(pl.all_horizontal(pl.col(logical).fill_null(value=False)))
    if numeric:
        # Missing values (null, NaN) never rank first; polars sorts NaN as the largest.
        floats = [name for name in numeric if results.schema[name].is_float()]
        results = results.drop_nulls(numeric)
        if floats:
            results = results.filter(~pl.any_horizontal(pl.col(floats).is_nan()))
        # gghighlight: arrange() by the values, then tail(max_highlight).
        results = results.sort(numeric, maintain_order=True).tail(max_highlight)
    return results


def _bleach_layer(sieved: _Sieved, options: HighlightOptions) -> LayerSpec:
    """Return the grey copy of a layer over all its rows."""
    layer = LayerSpec.duplicate(sieved.layer)
    props = layer.props()
    mapping = {
        aesthetic: column
        for aesthetic, column in sieved.mapping.items()
        if aesthetic not in COLOR_AESTHETICS
    }
    frame = sieved.frame
    group_keys = [*sieved.keys]
    if options.calculate_per_facet:
        bleached = frame
    else:
        # Only the mapped columns: a facet added later finds no faceting column here,
        # so the grey rows are drawn in every panel.
        columns = list(dict.fromkeys(c for c in mapping.values() if c in frame.columns))
        bleached = frame.select(columns)
    if group_keys:
        ids = frame.select(pl.struct(group_keys).rank("dense").alias(GROUP_COLUMN))
        bleached = bleached.with_columns(ids[GROUP_COLUMN])
        mapping["group"] = GROUP_COLUMN
    props["data"] = bleached
    props["mapping"] = aes(**mapping)
    props["inherit_aes"] = False
    props["merge_plot_data"] = False
    props["data_meta"] = _without_aesthetics(sieved.meta, COLOR_AESTHETICS)
    props["tooltips"] = None
    geom = sieved.layer.geom or ""
    if geom not in NO_COLOR_GEOMS:
        props["color"] = UNHIGHLIGHTED_COLOR
    if geom in FILL_GEOMS:
        props["fill"] = UNHIGHLIGHTED_COLOR
    props.update(copy.deepcopy(options.unhighlighted_params))
    return layer


def _sieved_layer(sieved: _Sieved) -> LayerSpec:
    """Return the highlighted copy of a layer over the matching rows."""
    layer = LayerSpec.duplicate(sieved.layer)
    layer.props()["data"] = sieved.highlighted
    return layer


def _blank_layer(sieved: _Sieved) -> LayerSpec:
    """Return a ``geom_blank`` over all rows, so style scales train on every row."""
    return LayerSpec(
        geom="blank",
        data=sieved.frame,
        mapping=aes(**sieved.mapping),
        inherit_aes=False,
        data_meta=copy.deepcopy(sieved.meta),
        **_scale_group(sieved.layer),
    )


def _label_layers(
    sieved_layers: list[_Sieved], options: HighlightOptions
) -> tuple[list[LayerSpec], bool]:
    """Return the direct-label layers and whether the color legends are hidden."""
    if options.use_direct_label is False:
        return [], False
    target = _label_target(sieved_layers)
    if target is None:
        if options.use_direct_label:
            raise _error("use_direct_label", "no line, point or bar layer can be labelled")
        return [], False
    sieved, kind = target
    if kind == "bar":
        # gghighlight labels no bars but still hides the legends.
        return [], True
    key = options.label_key
    if key is None:
        key = sieved.keys[0] if sieved.keys else None
    if key is None:
        if options.use_direct_label:
            reason = "no column to label with; pass label_key= (e.g. label_key='name')"
            raise _error("use_direct_label", reason)
        return [], False
    if key not in sieved.frame.columns:
        raise ColumnNotFoundError(
            f"gghighlight(label_key=...): column {key!r} is not in the data of the "
            f"{kind} layer; columns: {sieved.frame.columns}"
            f"{did_you_mean(key, sieved.frame.columns)}",
            column=key,
            available=list(sieved.frame.columns),
            path="label_key",
        )
    x_column = sieved.mapping.get("x")
    y_column = sieved.mapping.get("y")
    if not (isinstance(x_column, str) and isinstance(y_column, str)):
        if options.use_direct_label:
            raise _error("use_direct_label", f"the {kind} layer maps no x and y columns")
        return [], False
    rows = sieved.highlighted
    if kind == "line":
        rows = _rightmost_rows(rows, sieved.keys, x_column)
    if options.use_direct_label is None and rows.height > MAX_LABELS:
        return [], False
    if rows.height == 0:
        return [], True
    return [_label_layer(sieved, rows, key, kind, options)], True


def _label_target(sieved_layers: list[_Sieved]) -> tuple[_Sieved, str] | None:
    """Return the layer to label: the first line, else point, else bar (gghighlight)."""
    for kind, geoms in (("line", ("line",)), ("point", ("point",)), ("bar", ("bar", "histogram"))):
        for sieved in sieved_layers:
            stat = sieved.layer.stat
            identity = stat in (None, "identity") or kind == "bar"
            if sieved.layer.geom in geoms and identity:
                return sieved, kind
    return None


def _rightmost_rows(frame: pl.DataFrame, keys: list[str], x_column: str) -> pl.DataFrame:
    """Return the first row with the largest x of each series."""
    if x_column not in frame.columns or frame.height == 0:
        return frame.head(0)
    if not keys:
        return frame.filter(pl.col(x_column) == pl.col(x_column).max()).head(1)
    rightmost = frame.filter(pl.col(x_column) == pl.col(x_column).max().over(keys))
    return rightmost.group_by(keys, maintain_order=True).first().select(frame.columns)


def _label_layer(
    sieved: _Sieved, rows: pl.DataFrame, key: str, kind: str, options: HighlightOptions
) -> LayerSpec:
    """Return the ``geom_label_repel`` (or ``geom_text_repel``) layer."""
    geom = "text_repel" if kind == "line" and options.line_label_type == "ggrepel_text" else None
    aesthetics = ("x", "y", "color")
    mapping = {name: sieved.mapping[name] for name in aesthetics if name in sieved.mapping}
    mapping["label"] = key
    meta = _only_aesthetics(sieved.meta, aesthetics)
    return LayerSpec(
        geom=geom or "label_repel",
        data=rows,
        mapping=aes(**mapping),
        inherit_aes=False,
        show_legend=False,
        data_meta=meta,
        **_scale_group(sieved.layer),
        **copy.deepcopy(options.label_params),
    )


# --- Helpers -------------------------------------------------------------------------


def _polars_reason(error: Exception, frame: pl.DataFrame) -> str:
    """Return a polars error as one line; a missing column gets a close match."""
    reason = str(error).splitlines()[0]
    if isinstance(error, pl.exceptions.ColumnNotFoundError):
        match = re.search(r'"([^"]+)"', reason)
        if match is not None:
            column = match.group(1)
            hint = did_you_mean(column, frame.columns)
            return f"column {column!r} is not in the data; columns: {frame.columns}{hint}"
    return reason


def _as_polars(data: PlotData) -> pl.DataFrame:
    """Return layer data as a polars DataFrame (columns unchanged)."""
    arrow = as_arrow(data)
    if isinstance(arrow, pl.DataFrame):
        return arrow
    return pl.DataFrame(arrow)


def _mapping_of(mapping: object, owner: str) -> dict[str, str]:
    """Return an ``aes()`` as ``{aesthetic: column}``; data vectors are rejected."""
    if not isinstance(mapping, FeatureSpec):
        return {}
    result: dict[str, str] = {}
    for aesthetic, column in mapping.as_dict().items():
        if column is None:
            continue
        if not isinstance(column, str):
            reason = (
                f"{owner} maps '{aesthetic}' to {type(column).__name__} values; "
                "gghighlight() needs column names in aes() (put the values in the data)"
            )
            raise _error("predicates", reason)
        result[aesthetic] = column
    return result


def _data_meta(props: dict[str, Any]) -> dict[str, Any]:
    """Return a copy of the ``data_meta`` of a plot or layer (``{}`` when unset)."""
    meta = props.get("data_meta")
    return copy.deepcopy(meta) if isinstance(meta, dict) else {}


def _merged_meta(
    plot_meta: dict[str, Any],
    layer_meta: dict[str, Any],
    inherited: dict[str, str],
    layer_mapping: dict[str, str],
) -> dict[str, Any]:
    """Return the layer's ``data_meta`` with the plot's for the inherited aesthetics.

    The rewritten layers no longer inherit the plot mapping, so the plot's
    ``as_discrete`` annotations of inherited aesthetics move into the layer, and the
    plot's column annotations (types, factor levels) are kept for its columns.
    """
    meta = copy.deepcopy(layer_meta)
    plot_annotations = [
        annotation
        for annotation in plot_meta.get("mapping_annotations", [])
        if annotation.get("aes") in inherited and annotation.get("aes") not in layer_mapping
    ]
    annotations = [*plot_annotations, *meta.get("mapping_annotations", [])]
    if annotations:
        meta["mapping_annotations"] = copy.deepcopy(annotations)
    own_columns = {item.get("column") for item in meta.get("series_annotations", [])}
    series = [
        *meta.get("series_annotations", []),
        *(
            copy.deepcopy(item)
            for item in plot_meta.get("series_annotations", [])
            if item.get("column") not in own_columns
        ),
    ]
    if series:
        meta["series_annotations"] = series
    return meta


def _without_aesthetics(meta: dict[str, Any], aesthetics: Sequence[str]) -> dict[str, Any]:
    """Return ``meta`` without the ``as_discrete`` annotations of ``aesthetics``."""
    result = copy.deepcopy(meta)
    if "mapping_annotations" in result:
        result["mapping_annotations"] = [
            item for item in result["mapping_annotations"] if item.get("aes") not in aesthetics
        ]
    return result


def _only_aesthetics(meta: dict[str, Any], aesthetics: Sequence[str]) -> dict[str, Any]:
    """Return ``meta`` with only the ``as_discrete`` annotations of ``aesthetics``."""
    result = copy.deepcopy(meta)
    if "mapping_annotations" in result:
        result["mapping_annotations"] = [
            item for item in result["mapping_annotations"] if item.get("aes") in aesthetics
        ]
    return result


def _group_keys(frame: pl.DataFrame, mapping: dict[str, str], meta: dict[str, Any]) -> list[str]:
    """Return the series columns: ``group``, else every discrete mapped column."""
    group = mapping.get("group")
    if group is not None:
        return [group] if group in frame.columns else []
    discrete_aesthetics = {
        item.get("aes")
        for item in meta.get("mapping_annotations", [])
        if item.get("annotation") == "as_discrete"
    }
    keys: list[str] = []
    for aesthetic, column in mapping.items():
        # Position aesthetics never form series (lets-plot's default grouping, which
        # ggrust follows; ggplot2 and R gghighlight also group by a discrete x).
        if aesthetic.startswith(("x", "y")) or column not in frame.columns or column in keys:
            continue
        dtype = frame.schema[column]
        discrete = dtype in (pl.String, pl.Boolean) or isinstance(dtype, pl.Categorical | pl.Enum)
        if discrete or aesthetic in discrete_aesthetics:
            keys.append(column)
    return keys


def _facet_columns(facet: object) -> list[str]:
    """Return the faceting columns of a ``facet_wrap()`` / ``facet_grid()`` spec."""
    if not isinstance(facet, FeatureSpec):
        return []
    props = facet.props()
    values: list[object] = []
    if props.get("name") == "wrap":
        facets = props.get("facets")
        values = [facets] if isinstance(facets, str) else list(facets or [])
    else:
        values = [props.get("x"), props.get("y")]
    return [value for value in values if isinstance(value, str)]


def _scale_group(layer: LayerSpec) -> dict[str, object]:
    """Return the layer's ggnewscale ``scale_group`` as keyword arguments."""
    group = layer.props().get("scale_group")
    return {"scale_group": copy.deepcopy(group)} if group else {}
