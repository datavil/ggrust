"""Significance brackets for group comparisons (R package ggpubr).

Not in lets-plot; ggplot2 needs ggpubr. ``geom_pwc`` tests every pair of groups
(rstatix's ``wilcox_test`` / ``t_test``) and draws a bracket per comparison;
``stat_pvalue_manual`` draws brackets from a table of comparisons, such as the one
``compare_means()`` returns. Tests and placement run in the Rust engine; both draw
through ``geom_bracket``. ggpubr is GPL; ggrust implements the behavior, not its code.

R's dotted parameter names are snake_case here: ``ref.group`` is ``ref_group``,
``p.adjust.method`` is ``p_adjust_method``, ``y.position`` is ``y_position``.
"""

from __future__ import annotations

import math
from collections.abc import Sequence
from numbers import Integral, Real
from typing import TYPE_CHECKING, Any, Literal, NoReturn, TypeAlias

import polars as pl

from ggrust._ggrust import format_r_numbers
from ggrust.compare_means import _frame
from ggrust.errors import ColumnNotFoundError, SpecError, UnsupportedValueError, did_you_mean
from ggrust.plot.core import aes
from ggrust.plot.geom import _geom
from ggrust.plot.geom_beeswarm import _check_choice

if TYPE_CHECKING:
    from ggrust._typing import ManualKey, Number, PlotData, Position
    from ggrust.plot.core import FeatureSpec, LayerSpec

__all__ = ["geom_pwc", "stat_pvalue_manual"]

PwcMethod: TypeAlias = Literal["wilcox_test", "t_test"]
"""A test of ``geom_pwc`` (rstatix names)."""

PAdjustMethod: TypeAlias = Literal[
    "none", "holm", "hochberg", "hommel", "bonferroni", "BH", "BY", "fdr"
]
"""Multiple-comparison adjustment (R ``p.adjust``)."""

_PWC_METHODS: tuple[str, ...] = ("wilcox_test", "t_test", "wilcox.test", "t.test", "wilcoxon")
_OTHER_METHODS: frozenset[str] = frozenset(
    {
        "sign_test",
        "dunn_test",
        "emmeans_test",
        "emmeans",
        "tukey_hsd",
        "games_howell_test",
        "games_howell",
    }
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
_HIDE_NS: tuple[str, ...] = ("p", "p.adj")
# ggpubr's guess_signif_label_column order.
_LABEL_GUESSES: tuple[str, ...] = (
    "label",
    "labels",
    "p.adj.signif",
    "p.adj",
    "padj",
    "p.signif",
    "p.value",
    "pval",
    "p.val",
    "p",
)
_STARS: frozenset[str] = frozenset({"****", "***", "**", "*"})


def _fail(function: str, name: str, value: object, reason: str, kind: type[SpecError]) -> NoReturn:
    """Raise ``kind`` for ``function(name=value)``."""
    raise kind(f"{function}({name}={value!r}): {reason}", path=name, reason=reason)


def _check_number(function: str, name: str, value: object, *, non_negative: bool = False) -> None:
    """Raise unless ``value`` is None or a number (>= 0 with ``non_negative``)."""
    if value is None:
        return
    if isinstance(value, bool) or not isinstance(value, Real):
        _fail(function, name, value, "expected a number", UnsupportedValueError)
    if non_negative and float(value) < 0:
        _fail(function, name, value, "expected a number >= 0", SpecError)


def _numbers(function: str, name: str, value: object) -> list[float] | float | None:
    """``value`` as a number or a non-empty list of numbers."""
    if value is None:
        return None
    if isinstance(value, Sequence) and not isinstance(value, str):
        if not value or any(isinstance(v, bool) or not isinstance(v, Real) for v in value):
            _fail(
                function,
                name,
                value,
                "expected a number or a non-empty list of numbers",
                UnsupportedValueError,
            )
        return [float(v) for v in value]
    _check_number(function, name, value)
    return float(value)  # ty: ignore[invalid-argument-type]


def geom_pwc(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    method: PwcMethod | None = None,
    ref_group: str | int | None = None,
    label: str | None = None,
    y_position: Number | Sequence[Number] | None = None,
    group_by: Literal["x.var", "legend.var"] | None = None,
    dodge: Number | None = None,
    step_increase: Number | None = None,
    tip_length: Number | None = None,
    bracket_nudge_y: Number | None = None,
    bracket_shorten: Number | None = None,
    p_adjust_method: PAdjustMethod | None = None,
    p_adjust_by: Literal["group", "panel"] | None = None,
    hide_ns: bool | Literal["p", "p.adj"] | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    manual_key: ManualKey | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Test every pair of groups and draw a labelled bracket per comparison.

    As ``geom_pwc`` in the R package ggpubr (tests from rstatix). Along a discrete
    x, the x categories are compared pairwise and the brackets stacked above the
    data. When other aesthetics (``color``, ``fill``) split each x into groups, those
    groups are compared within each x (``group_by="x.var"``, brackets between the
    dodged groups), or the x categories within each group (``"legend.var"``,
    brackets in the group's color). p-values are adjusted for multiple comparisons
    (``"holm"``) within each set of comparisons.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``: ``x`` (categories) and ``y``; inherits the
        plot's.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    method : {"wilcox_test", "t_test"}, default="wilcox_test"
        Wilcoxon rank sum test or Welch's t test. ``"wilcox.test"``, ``"t.test"``
        and ``"wilcoxon"`` work too.
    ref_group : str or int, optional
        Compare every other level with this one only: an x category (name or 1-based
        position), or with ``group_by="x.var"`` the 1-based rank of a group (1 for
        the first). ``"all"`` compares each level with all values (labels only).
    label : str, default="p.format"
        What the brackets show: ``"p"``, ``"p.adj"``, ``"p.format"``,
        ``"p.adj.format"``, ``"p.signif"``, ``"p.adj.signif"``, ``"method"``,
        ``"n1"``, ``"n2"``, or a template of them in braces, e.g.
        ``"p = {p.adj.format}"``. ``p.format`` reads ``"0.0012"`` or
        ``"<0.0001"``; ``p.signif`` reads ``****`` (<= 1e-4) to ``ns`` (> 0.05).
    y_position : float or list of float, optional
        Height of the first bracket (later ones step up from it), or of every
        bracket. Default: the top of the data.
    group_by : {"x.var", "legend.var"}, default="x.var"
        With groups within x: compare the groups at each x, or the x categories of
        each group.
    dodge : float, default=0.95
        Dodge width of the groups within x: 0.95 puts the bracket ends on the boxes
        of ``geom_boxplot`` and ``geom_violin`` (ggpubr's 0.8 fits its ``ggboxplot``).
    step_increase : float, default=0.12
        Gap between stacked brackets, as a share of the y range.
    tip_length : float, default=0.03
        Bracket tip length, as a share of the y range.
    bracket_nudge_y : float, default=0.05
        Lift of the first bracket, as a share of the y range.
    bracket_shorten : float, optional
        Pull both bracket ends toward the middle: 0 keeps the full width.
    p_adjust_method : PAdjustMethod, default="holm"
        R ``p.adjust`` method: ``"holm"``, ``"hochberg"``, ``"hommel"``,
        ``"bonferroni"``, ``"BH"`` (or ``"fdr"``), ``"BY"`` or ``"none"``.
    p_adjust_by : {"group", "panel"}, default="group"
        Adjust within each set of comparisons, or over all of the panel's.
    hide_ns : bool or {"p", "p.adj"}, default=False
        Drop comparisons that are not significant (> 0.05): by ``p.adj`` (True or
        ``"p.adj"``) or by ``p``.
    position : str or FeatureSpec, optional
        Position adjustment.
    show_legend : bool, optional
        True shows this layer in the legend.
    inherit_aes : bool, default=True
        False ignores the plot's default mapping.
    manual_key : str or FeatureSpec, optional
        This layer's key in a manual legend.
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics of the brackets and labels, e.g. ``size``, ``color``.

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If a name is not accepted.
    UnsupportedValueError
        If an option has the wrong type.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"diet": ["veg"] * 6 + ["meat"] * 6 + ["fish"] * 6,
    ...         "weight": [61, 58, 66, 70, 63, 59, 72, 80, 77, 69, 85, 74,
    ...                    65, 68, 71, 64, 70, 66]}
    >>> plot = (ggplot(data, aes("diet", "weight")) + geom_boxplot()
    ...         + geom_pwc(label="p.adj.signif"))
    """
    function = "geom_pwc"
    if isinstance(method, str) and method.replace(".", "_") in _OTHER_METHODS:
        _fail(
            function,
            "method",
            method,
            "this rstatix test is not available in ggrust yet; use 'wilcox_test' or 't_test'",
            SpecError,
        )
    _check_choice(function, "method", method, _PWC_METHODS)
    if ref_group is not None and not (
        isinstance(ref_group, str)
        or (isinstance(ref_group, Integral) and not isinstance(ref_group, bool))
    ):
        _fail(
            function,
            "ref_group",
            ref_group,
            "expected a category name or a 1-based position",
            UnsupportedValueError,
        )
    if isinstance(ref_group, Integral) and int(ref_group) < 1:
        _fail(function, "ref_group", ref_group, "positions start at 1", SpecError)
    if label is not None and not isinstance(label, str):
        _fail(function, "label", label, "expected a string", UnsupportedValueError)
    _check_choice(function, "group_by", group_by, ("x.var", "legend.var"))
    _check_choice(function, "p_adjust_method", p_adjust_method, _ADJUSTMENTS)
    _check_choice(function, "p_adjust_by", p_adjust_by, ("group", "panel"))
    if hide_ns is not None and not isinstance(hide_ns, bool):
        _check_choice(function, "hide_ns", hide_ns, _HIDE_NS)
    positions = _numbers(function, "y_position", y_position)
    for name, value in (
        ("dodge", dodge),
        ("step_increase", step_increase),
        ("tip_length", tip_length),
        ("bracket_shorten", bracket_shorten),
    ):
        _check_number(function, name, value, non_negative=True)
    _check_number(function, "bracket_nudge_y", bracket_nudge_y)
    other_args.setdefault("tiplength_unit", "identity")
    return _geom(
        "bracket",
        mapping=mapping,
        data=data,
        stat="pwc",
        position=position,
        show_legend=show_legend,
        inherit_aes=True if inherit_aes is None else inherit_aes,
        manual_key=manual_key,
        sampling=None,
        method=method,
        ref_group=ref_group,
        label=label,
        y_position=positions,
        group_by=group_by,
        dodge=dodge,
        step_increase=step_increase,
        tip_length=tip_length,
        bracket_nudge_y=bracket_nudge_y,
        bracket_shorten=bracket_shorten,
        p_adjust_method=p_adjust_method,
        p_adjust_by=p_adjust_by,
        hide_ns=hide_ns,
        na_rm=na_rm,
        **other_args,
    )


def stat_pvalue_manual(
    data: PlotData,
    label: str | None = None,
    y_position: str | Number | Sequence[Number] = "y.position",
    xmin: str = "group1",
    xmax: str | None = "group2",
    x: str | None = None,
    *,
    tip_length: Number = 0.03,
    step_increase: Number = 0,
    step_group_by: str | None = None,
    hide_ns: bool | Literal["p", "p.adj"] = False,
    bracket_nudge_y: Number = 0,
    bracket_shorten: Number | None = None,
    remove_bracket: bool = False,
    vjust: Number | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    manual_key: ManualKey | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Draw brackets with p-value labels from a table of comparisons.

    As ``stat_pvalue_manual`` in the R package ggpubr. Each row of ``data`` is a
    bracket from category ``xmin`` to ``xmax`` (by default the ``group1`` and
    ``group2`` columns, as ``compare_means()`` returns them) at ``y_position``,
    labelled with ``label``. With ``x`` (or a table of comparisons against
    ``".all."``), the labels are drawn at ``x`` without brackets.

    Parameters
    ----------
    data : DataFrame or dict
        The comparisons, one per row. Columns not used here are kept, so a
        facet variable in the table places each bracket in its panel.
    label : str, optional
        A column to show, or a template of columns in braces, e.g.
        ``"p = {p.adj}"``. Default: the first of ``label``, ``labels``,
        ``p.adj.signif``, ``p.adj``, ``padj``, ``p.signif``, ``p.value``, ``pval``,
        ``p.val``, ``p``. Numbers print as R prints them.
    y_position : str, float or list of float, default="y.position"
        A column of bracket heights, one height, or one per row (after
        ``hide_ns``), in data units.
    xmin : str, default="group1"
        Column of the left ends (x category names). As in ggpubr, the x axis
        must be discrete: for a numeric x, map ``as_discrete("dose")``.
    xmax : str or None, default="group2"
        Column of the right ends; None draws labels only, at ``xmin``.
    x : str, optional
        Column of label positions: labels only, no brackets.
    tip_length : float, default=0.03
        Bracket tip length, as a share of the y range.
    step_increase : float, default=0
        Lift each bracket by this share of the y range over the one before.
    step_group_by : str, optional
        Column whose groups each restart the steps.
    hide_ns : bool or {"p", "p.adj"}, default=False
        Drop rows that are not significant: True uses the first of ``p.adj.signif``,
        ``p.adj``, ``p.signif`` and ``p`` (``"ns"``, or above 0.05).
    bracket_nudge_y : float, default=0
        Lift every bracket, in data units.
    bracket_shorten : float, optional
        Pull both bracket ends toward the middle: 0 keeps the full width.
    remove_bracket : bool, default=False
        With one ``group1`` level (comparisons against a reference), labels only,
        at ``xmax``.
    vjust : float, optional
        Vertical justification of labels drawn without brackets. Default 0.5 for
        star labels, else 0, as ggpubr.
    position : str or FeatureSpec, optional
        Position adjustment.
    show_legend : bool, optional
        True shows this layer in the legend.
    manual_key : str or FeatureSpec, optional
        This layer's key in a manual legend.
    **other_args
        Constant aesthetics of the brackets and labels, e.g. ``size``, ``color``.

    Returns
    -------
    LayerSpec
        The layer (it does not inherit the plot's mapping). As for any layer that
        maps ``y`` to another column, the y axis title becomes ``"y"``; set it with
        ``labs(y=...)``.

    Raises
    ------
    ColumnNotFoundError
        If a named column is not in ``data``.
    SpecError
        If an option is not accepted or ``y_position`` does not fit the rows.
    UnsupportedValueError
        If an option has the wrong type.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"diet": ["veg"] * 6 + ["meat"] * 6 + ["fish"] * 6,
    ...         "weight": [61, 58, 66, 70, 63, 59, 72, 80, 77, 69, 85, 74,
    ...                    65, 68, 71, 64, 70, 66]}
    >>> table = compare_means("weight ~ diet", data)
    >>> plot = (ggplot(data, aes("diet", "weight")) + geom_boxplot()
    ...         + stat_pvalue_manual(table, label="p = {p.adj}", y_position=88,
    ...                              step_increase=0.1))
    """
    function = "stat_pvalue_manual"
    frame = _frame(data)
    for name, value in (("xmin", xmin), ("xmax", xmax), ("x", x), ("step_group_by", step_group_by)):
        if value is not None and not isinstance(value, str):
            _fail(function, name, value, "expected a column name", UnsupportedValueError)
    if label is not None and not isinstance(label, str):
        _fail(
            function, "label", label, "expected a column name or a template", UnsupportedValueError
        )
    for name, value in (
        ("tip_length", tip_length),
        ("step_increase", step_increase),
        ("bracket_shorten", bracket_shorten),
    ):
        _check_number(function, name, value, non_negative=True)
    _check_number(function, "bracket_nudge_y", bracket_nudge_y)
    _check_number(function, "vjust", vjust)
    for name, value in (("remove_bracket", remove_bracket),):
        if not isinstance(value, bool):
            _fail(function, name, value, "expected True or False", UnsupportedValueError)
    if label is None:
        label = next((name for name in _LABEL_GUESSES if name in frame.columns), None)
        if label is None:
            _fail(
                function,
                "label",
                None,
                f"no label column found; name one with label=, e.g. one of: "
                f"{', '.join(frame.columns)}",
                SpecError,
            )
    frame = _hide_ns(function, frame, hide_ns)
    if x is None and xmin == "group1" and xmax == "group2" and "group1" in frame.columns:
        group1 = frame["group1"].cast(pl.String).to_list()
        group2 = frame["group2"].to_list() if "group2" in frame.columns else []
        if (
            group1
            and all(g in ("all", ".all.") for g in group1)
            and len(set(group2)) == len(group2)
        ):
            x = "group2"
    if remove_bracket and xmin in frame.columns and frame[xmin].n_unique() == 1:
        x = xmax
    texts = _label_texts(function, frame, label)
    heights = _heights(function, frame, y_position)
    columns: dict[str, pl.Series] = {
        "label": pl.Series("label", texts, dtype=pl.String),
        "y.position": pl.Series("y.position", heights, dtype=pl.Float64),
    }
    if x is None and xmax is None:
        # ggpubr: without xmax, labels only, at xmin.
        x = xmin
    ends: dict[str, str] = {"x": x} if x is not None else {"xmin": xmin, "xmax": str(xmax)}
    for name, column in ends.items():
        _require(function, frame, column, name)
        columns[name] = frame[column]
    if step_group_by is not None:
        _require(function, frame, step_group_by, "step_group_by")
        columns["step.group"] = frame[step_group_by].cast(pl.String)
    # The other columns ride along, so facets (and later layers) can match them.
    extra = [frame[name] for name in frame.columns if name not in columns]
    table = pl.DataFrame([series.alias(name) for name, series in columns.items()] + extra)
    if x is not None:
        if vjust is None:
            vjust = 0.5 if texts and all(text in _STARS for text in texts) else 0
        return _geom(
            "text",
            mapping=aes(x="x", y="y.position", label="label"),
            data=table,
            position=position,
            show_legend=show_legend,
            inherit_aes=False,
            manual_key=manual_key,
            vjust=vjust,
            **other_args,
        )
    mapping = aes(xmin="xmin", xmax="xmax", y="y.position", label="label")
    if step_group_by is not None:
        mapping = aes(xmin="xmin", xmax="xmax", y="y.position", label="label", group="step.group")
    other_args.setdefault("tiplength_unit", "identity")
    return _geom(
        "bracket",
        mapping=mapping,
        data=table,
        stat="pvalue_manual",
        position=position,
        show_legend=show_legend,
        inherit_aes=False,
        manual_key=manual_key,
        sampling=None,
        tip_length=tip_length,
        step_increase=step_increase,
        step_group_by=True if step_group_by is not None else None,
        bracket_nudge_y=bracket_nudge_y,
        bracket_shorten=bracket_shorten,
        **other_args,
    )


def _require(function: str, frame: pl.DataFrame, column: str, option: str) -> None:
    """Raise unless ``column`` (named by ``option``) is in ``frame``."""
    if column not in frame.columns:
        message = (
            f"{function}({option}={column!r}): column '{column}' not found in the table; "
            f"available columns: {', '.join(frame.columns)}{did_you_mean(column, frame.columns)}"
        )
        raise ColumnNotFoundError(
            message, column=column, available=list(frame.columns), path=option
        )


def _hide_ns(function: str, frame: pl.DataFrame, hide_ns: object) -> pl.DataFrame:
    """Return the rows of ``frame`` kept by ``hide_ns`` (ggpubr ``remove_ns``)."""
    if hide_ns is False or hide_ns is None:
        return frame
    if hide_ns is True:
        candidates = ("p.adj.signif", "p.adj", "p.signif", "p")
        column = next((name for name in candidates if name in frame.columns), None)
        if column is None:
            _fail(
                function,
                "hide_ns",
                hide_ns,
                f"needs one of the columns {', '.join(candidates)}",
                SpecError,
            )
    elif isinstance(hide_ns, str):
        _check_choice(function, "hide_ns", hide_ns, _HIDE_NS)
        column = hide_ns
        _require(function, frame, column, "hide_ns")
    else:
        _fail(
            function,
            "hide_ns",
            hide_ns,
            "expected True, False, 'p' or 'p.adj'",
            UnsupportedValueError,
        )
    values = frame[column]
    if values.dtype.is_numeric():
        return frame.filter(values.cast(pl.Float64) <= 0.05)
    return frame.filter(~values.cast(pl.String).is_in(["ns", "NS"]))


def _format_column(column: pl.Series) -> list[str]:
    """Each value as R's ``as.character`` prints it (``"NA"`` when missing)."""
    if column.dtype.is_float():
        values = column.to_list()
        present = [v for v in values if v is not None and not math.isnan(v)]
        printed = iter(format_r_numbers(present, 15))
        return ["NA" if v is None or math.isnan(v) else next(printed) for v in values]
    return ["NA" if v is None else str(v) for v in column.to_list()]


def _label_texts(function: str, frame: pl.DataFrame, label: str) -> list[str]:
    """Return the label of every row: a column, or a template of ``{column}`` parts."""
    if "{" not in label:
        _require(function, frame, label, "label")
        return _format_column(frame[label])
    texts = [""] * frame.height
    rest = label
    while rest:
        start = rest.find("{")
        if start < 0:
            texts = [text + rest for text in texts]
            break
        end = rest.find("}", start)
        if end < 0:
            _fail(function, "label", label, "unclosed '{' in the label template", SpecError)
        literal, name = rest[:start], rest[start + 1 : end].strip()
        _require(function, frame, name, "label")
        values = _format_column(frame[name])
        texts = [text + literal + value for text, value in zip(texts, values, strict=True)]
        rest = rest[end + 1 :]
    return texts


def _heights(function: str, frame: pl.DataFrame, y_position: object) -> list[float]:
    """Bracket heights from a column name, one number or one per row."""
    if isinstance(y_position, str):
        _require(function, frame, y_position, "y_position")
        if not frame.schema[y_position].is_numeric():
            _fail(function, "y_position", y_position, "the column must hold numbers", SpecError)
        return frame[y_position].cast(pl.Float64).fill_null(math.nan).to_list()
    numbers = _numbers(function, "y_position", y_position)
    if isinstance(numbers, list):
        if len(numbers) == 1:
            return numbers * frame.height
        if len(numbers) != frame.height:
            _fail(
                function,
                "y_position",
                y_position,
                f"expected one height or one per row ({frame.height} rows after hide_ns), "
                f"got {len(numbers)}",
                SpecError,
            )
        return numbers
    if numbers is None:
        _fail(function, "y_position", y_position, "expected a column name or numbers", SpecError)
    return [numbers] * frame.height
