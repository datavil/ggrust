"""Tables of group comparisons with p-values (ggpubr's ``compare_means``), tested in Rust.

Not in lets-plot; in R it is ``ggpubr::compare_means()``. ``compare_means()`` tests
the groups of a response column and returns one row per comparison, ready for
``stat_pvalue_manual()``. The tests (``t.test``, ``wilcox.test``, ``anova``,
``kruskal.test``) and ``p.adjust`` give R's p-values; ggpubr is GPL, ggrust
implements the statistics, not its code.
"""

from __future__ import annotations

import math
import sys
from collections.abc import Sequence
from typing import TYPE_CHECKING, Literal, NoReturn, TypeAlias

import polars as pl

from ggrust._data import as_arrow
from ggrust._ggrust import compare_means as _compare_means
from ggrust._ggrust import format_r_numbers
from ggrust.errors import ColumnNotFoundError, SpecError, UnsupportedValueError, did_you_mean

if TYPE_CHECKING:
    from ggrust._typing import PlotData

__all__ = ["compare_means"]

CompareMethod: TypeAlias = Literal["t.test", "wilcox.test", "anova", "kruskal.test"]
"""A test of ``compare_means``."""

PAdjustMethod: TypeAlias = Literal[
    "none", "holm", "hochberg", "hommel", "bonferroni", "BH", "BY", "fdr"
]
"""Multiple-comparison adjustment (R ``p.adjust``)."""

_METHODS: tuple[str, ...] = (
    "t.test",
    "wilcox.test",
    "anova",
    "kruskal.test",
    "t",
    "student",
    "wilcox",
    "wilcoxon",
    "aov",
    "kruskal",
)
_ADJUSTMENTS: tuple[str, ...] = (
    "none",
    "holm",
    "hochberg",
    "hommel",
    "bonferroni",
    "BH",
    "BY",
    "fdr",
)


def compare_means(
    formula: str,
    data: PlotData,
    *,
    method: CompareMethod = "wilcox.test",
    paired: bool = False,
    group_by: str | Sequence[str] | None = None,
    ref_group: str | None = None,
    p_adjust_method: PAdjustMethod = "holm",
) -> pl.DataFrame:
    """Test the groups of a column against each other and list the p-values.

    As ``compare_means`` in the R package ggpubr. ``formula`` is ``"y ~ group"``:
    the values of ``y`` are split by the levels of ``group`` (their order of first
    appearance, or the categories of a polars Enum or pandas Categorical). Two-group
    tests (``"wilcox.test"``, ``"t.test"``) compare every pair of levels, or each
    level with ``ref_group``; ``"anova"`` and ``"kruskal.test"`` test all levels at
    once. With ``group_by``, the tests run within each of its groups.

    Parameters
    ----------
    formula : str
        ``"response ~ group"``, two column names of ``data``, e.g. ``"len ~ dose"``.
    data : DataFrame or dict
        The data: polars, pandas, an Arrow table or a dict of columns.
    method : {"wilcox.test", "t.test", "anova", "kruskal.test"}, default="wilcox.test"
        Wilcoxon rank sum (signed rank when paired), Welch's t test, one-way ANOVA or
        Kruskal-Wallis. ggpubr's aliases (``"t"``, ``"wilcox"``, ``"aov"``,
        ``"kruskal"``, ...) work too.
    paired : bool, default=False
        Paired two-group tests; the values of two levels pair up in row order.
    group_by : str or list of str, optional
        Columns to split the data by first; the result starts with them.
    ref_group : str, optional
        A level to compare every other level with, or ``".all."`` to compare each
        level with all values. Two-group tests only.
    p_adjust_method : PAdjustMethod, default="holm"
        Adjustment of ``p.adj`` over all rows (R ``p.adjust``): ``"holm"``,
        ``"hochberg"``, ``"hommel"``, ``"bonferroni"``, ``"BH"`` (or ``"fdr"``),
        ``"BY"`` or ``"none"``.

    Returns
    -------
    polars.DataFrame
        One row per comparison: the ``group_by`` columns, ``.y.`` (the response
        name), ``group1`` and ``group2`` (level names, two-group tests only), ``p``,
        ``p.adj`` (2 significant digits, as ggpubr), ``p.format`` (R's
        ``format.pval(p, digits = 2)``), ``p.signif`` (``****`` <= 1e-4, ``***``
        <= 0.001, ``**`` <= 0.01, ``*`` <= 0.05, else ``ns``, from the unadjusted p)
        and ``method``.

    Raises
    ------
    SpecError
        If the formula is malformed, an option is unknown, ``ref_group`` is not a
        level, or a test cannot run (constant values for a t test, paired levels of
        different sizes).
    UnsupportedValueError
        If an option has the wrong type.
    ColumnNotFoundError
        If a column is not in ``data``.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"diet": ["veg"] * 6 + ["meat"] * 6 + ["fish"] * 6,
    ...         "weight": [61, 58, 66, 70, 63, 59, 72, 80, 77, 69, 85, 74,
    ...                    65, 68, 71, 64, 70, 66]}
    >>> table = compare_means("weight ~ diet", data)
    >>> table.select("group1", "group2", "p.format").rows()
    [('veg', 'meat', '0.0043'), ('veg', 'fish', '0.092'), ('meat', 'fish', '0.0087')]
    >>> plot = (ggplot(data, aes("diet", "weight")) + geom_boxplot()
    ...         + stat_pvalue_manual(table, label="p.signif", y_position=88, step_increase=0.1))
    """
    function = "compare_means"
    response, group = _parse_formula(formula)
    _check_choice(function, "method", method, _METHODS)
    _check_choice(function, "p_adjust_method", p_adjust_method, _ADJUSTMENTS)
    if not isinstance(paired, bool):
        _fail(function, "paired", paired, "expected True or False", UnsupportedValueError)
    by = _group_by(function, group_by)
    if ref_group is not None and not isinstance(ref_group, str):
        _fail(function, "ref_group", ref_group, "expected a level name", UnsupportedValueError)
    frame = _frame(data)
    for column in (response, group, *by):
        if column not in frame.columns:
            message = (
                f"{function}(): column '{column}' not found in data; available columns: "
                f"{', '.join(frame.columns)}{did_you_mean(column, frame.columns)}"
            )
            raise ColumnNotFoundError(message, column=column, available=list(frame.columns))
    if not frame.schema[response].is_numeric():
        _fail(
            function,
            "formula",
            formula,
            f"the response '{response}' must be a numeric column, got {frame.schema[response]}",
            SpecError,
        )
    levels = _levels(data, frame, group)
    reference: int | None = None
    if ref_group is not None and ref_group != ".all.":
        if ref_group not in levels:
            _fail(
                function,
                "ref_group",
                ref_group,
                f"'{ref_group}' is not a level of '{group}'{did_you_mean(ref_group, levels)}; "
                f"the levels are: {', '.join(levels)}",
                SpecError,
            )
        reference = levels.index(ref_group)
    parts = frame.partition_by(by, maintain_order=True) if by else [frame]
    partitions: list[list[list[float]]] = []
    for part in parts:
        values = part[response].cast(pl.Float64).fill_null(math.nan).to_list()
        part_names = _level_names(part[group])
        groups: list[list[float]] = [[] for _ in levels]
        for value, name in zip(values, part_names, strict=True):
            if name is not None:
                groups[levels.index(name)].append(value)
        partitions.append(groups)
    rows = _compare_means(
        partitions,
        levels,
        method,
        paired,
        reference,
        ref_group == ".all.",
        p_adjust_method,
    )
    partition, group1, group2, p, p_adjusted, p_format, p_signif, method_name = rows
    columns: dict[str, pl.Series] = {}
    for column in by:
        keys = [parts[index][column][0] for index in partition]
        columns[column] = pl.Series(column, keys, dtype=frame.schema[column])
    columns[".y."] = pl.Series(".y.", [response] * len(p), dtype=pl.String)
    if group1 or group2:
        columns["group1"] = pl.Series(
            "group1", [".all." if g is None else levels[g] for g in group1], dtype=pl.String
        )
        columns["group2"] = pl.Series("group2", [levels[g] for g in group2], dtype=pl.String)
    columns["p"] = pl.Series("p", p, dtype=pl.Float64)
    columns["p.adj"] = pl.Series("p.adj", p_adjusted, dtype=pl.Float64)
    columns["p.format"] = pl.Series("p.format", p_format, dtype=pl.String)
    columns["p.signif"] = pl.Series("p.signif", p_signif, dtype=pl.String)
    columns["method"] = pl.Series("method", [method_name] * len(p), dtype=pl.String)
    return pl.DataFrame(list(columns.values()))


def _fail(function: str, name: str, value: object, reason: str, kind: type[SpecError]) -> NoReturn:
    """Raise ``kind`` for ``function(name=value)``."""
    raise kind(f"{function}({name}={value!r}): {reason}", path=name, reason=reason)


def _check_choice(function: str, name: str, value: object, accepted: tuple[str, ...]) -> None:
    """Raise unless ``value`` is one of ``accepted``."""
    if isinstance(value, str) and value in accepted:
        return
    listing = ", ".join(map(repr, accepted))
    if isinstance(value, str):
        _fail(
            function,
            name,
            value,
            f"unknown {name} {value!r}; use one of {listing}{did_you_mean(value, accepted)}",
            SpecError,
        )
    _fail(
        function,
        name,
        value,
        f"expected one of {listing}, got {type(value).__name__}",
        UnsupportedValueError,
    )


def _parse_formula(formula: object) -> tuple[str, str]:
    """``"y ~ group"`` as ``(y, group)``."""
    if not isinstance(formula, str):
        _fail(
            "compare_means",
            "formula",
            formula,
            "expected a string such as 'len ~ dose'",
            UnsupportedValueError,
        )
    sides = [side.strip() for side in str(formula).split("~")]
    if len(sides) != 2 or not all(sides):
        _fail(
            "compare_means",
            "formula",
            formula,
            "expected 'response ~ group', two column names, e.g. 'len ~ dose'",
            SpecError,
        )
    response, group = sides
    if group == "1":
        _fail(
            "compare_means",
            "formula",
            formula,
            "one-sample tests ('y ~ 1') are not available; name a grouping column",
            SpecError,
        )
    return response, group


def _group_by(function: str, group_by: object) -> list[str]:
    """``group_by`` as a list of column names."""
    if group_by is None:
        return []
    if isinstance(group_by, str):
        return [group_by]
    if (
        isinstance(group_by, Sequence)
        and group_by
        and all(isinstance(column, str) for column in group_by)
    ):
        return list(group_by)
    _fail(
        function,
        "group_by",
        group_by,
        "expected a column name or a non-empty list of column names",
        UnsupportedValueError,
    )


def _frame(data: object) -> pl.DataFrame:
    """Return ``data`` as a polars DataFrame."""
    table = as_arrow(data)
    if isinstance(table, pl.DataFrame):
        return table
    return pl.DataFrame(table)


def _level_names(column: pl.Series) -> list[str | None]:
    """Each value as its level name: floats as R prints them, others as text."""
    if column.dtype.is_float():
        values = column.to_list()
        present = [value for value in values if value is not None and not math.isnan(value)]
        printed = iter(format_r_numbers(present, 15))
        return [None if value is None or math.isnan(value) else next(printed) for value in values]
    return [None if value is None else str(value) for value in column.to_list()]


def _levels(data: object, frame: pl.DataFrame, group: str) -> list[str]:
    """Return the level names of ``group``: Enum or pandas categories, else first appearance."""
    dtype = frame.schema[group]
    if isinstance(dtype, pl.Enum):
        return [str(level) for level in dtype.categories.to_list()]
    pandas = sys.modules.get("pandas")
    if pandas is not None and isinstance(data, pandas.DataFrame):
        column = data[group]
        if isinstance(column.dtype, pandas.CategoricalDtype):
            return [str(level) for level in column.cat.categories]
    seen: dict[str, None] = {}
    for name in _level_names(frame[group]):
        if name is not None:
            seen.setdefault(name, None)
    return list(seen)
