"""Computed statistics as labels (R packages ggpubr and ggpmisc).

Not in lets-plot; ggplot2 needs ggpubr (``stat_compare_means``, ``stat_cor``) or
ggpmisc (``stat_poly_eq``). Every test statistic and p-value is computed in the Rust
engine and matches R's ``t.test``, ``wilcox.test``, ``anova``, ``kruskal.test``,
``cor.test``, ``p.adjust`` and ``lm``. ggpubr and ggpmisc are GPL; ggrust implements
the statistics, not their code.

R's dotted parameter names are snake_case here: ``ref.group`` is ``ref_group``,
``hide.ns`` is ``hide_ns``, ``label.x.npc`` is ``label_x_npc``. Labels are plain
text (``"R = -0.87, p = 1.3e-10"``, ``"R² = 0.75"``), not plotmath.
"""

from __future__ import annotations

from collections.abc import Sequence
from numbers import Integral, Real
from typing import TYPE_CHECKING, Any, Literal, NoReturn, TypeAlias

from ggrust.errors import SpecError, UnsupportedValueError
from ggrust.plot.geom import _geom
from ggrust.plot.geom_beeswarm import _check_choice

if TYPE_CHECKING:
    from ggrust._typing import ManualKey, Number, PlotData, Position
    from ggrust.plot.core import FeatureSpec, LayerSpec

__all__ = ["stat_compare_means", "stat_cor", "stat_poly_eq"]

CompareMethod: TypeAlias = Literal["t.test", "wilcox.test", "anova", "kruskal.test"]
"""Test of ``stat_compare_means``."""

CorrelationMethod: TypeAlias = Literal["pearson", "spearman", "kendall"]
"""Correlation of ``stat_cor``."""

PAdjustMethod: TypeAlias = Literal[
    "none", "holm", "hochberg", "hommel", "bonferroni", "BH", "BY", "fdr"
]
"""Multiple-comparison adjustment (R ``p.adjust``)."""

Level: TypeAlias = str | int
"""An x category, by name or 1-based position."""

PolyLabel: TypeAlias = Literal["eq", "R2", "adj.R2", "P", "n", "R", "slope"]
"""A part of a ``stat_poly_eq`` label (ggpmisc ``use_label`` names, plus ``slope``)."""

_COMPARE_METHODS: tuple[str, ...] = (
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
_CORRELATIONS: tuple[str, ...] = ("pearson", "spearman", "kendall")
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
_P_LABELS: tuple[str, ...] = ("p.format", "p.signif")
_POLY_LABELS: tuple[str, ...] = ("eq", "R2", "adj.R2", "P", "n", "R", "slope")
_X_NPC: tuple[str, ...] = ("left", "center", "centre", "right")
_Y_NPC: tuple[str, ...] = ("bottom", "center", "centre", "top")


def _fail(function: str, name: str, value: object, reason: str, kind: type[SpecError]) -> NoReturn:
    """Raise ``kind`` for ``function(name=value)``."""
    raise kind(f"{function}({name}={value!r}): {reason}", path=name, reason=reason)


def _check_level(function: str, name: str, value: object) -> None:
    """Raise unless ``value`` is an x category name or a 1-based position."""
    if isinstance(value, str) or (isinstance(value, Integral) and not isinstance(value, bool)):
        if isinstance(value, Integral) and int(value) < 1:
            _fail(function, name, value, "category positions start at 1", SpecError)
        return
    _fail(
        function,
        name,
        value,
        f"expected an x category name or 1-based position, got {type(value).__name__}",
        UnsupportedValueError,
    )


def _comparison_pairs(
    function: str, comparisons: Sequence[Sequence[Level]] | None
) -> list[list[Level]] | None:
    """``comparisons`` as a list of 2-item lists, validated."""
    if comparisons is None:
        return None
    if isinstance(comparisons, str) or not isinstance(comparisons, Sequence) or not comparisons:
        _fail(
            function,
            "comparisons",
            comparisons,
            "expected a non-empty list of pairs of x categories, e.g. [('a', 'b')]",
            UnsupportedValueError,
        )
    pairs: list[list[Level]] = []
    for pair in comparisons:
        if isinstance(pair, str) or not isinstance(pair, Sequence) or len(pair) != 2:
            _fail(
                function,
                "comparisons",
                comparisons,
                f"every comparison is a pair of x categories, got {pair!r}",
                SpecError,
            )
        for level in pair:
            _check_level(function, "comparisons", level)
        pairs.append(list(pair))
    return pairs


def _check_numbers(function: str, name: str, value: object) -> None:
    """Raise unless ``value`` is None, a number or a non-empty list of numbers."""
    if value is None:
        return
    items = value if isinstance(value, Sequence) and not isinstance(value, str) else [value]
    if not items or any(isinstance(item, bool) or not isinstance(item, Real) for item in items):
        _fail(
            function,
            name,
            value,
            "expected a number or a non-empty list of numbers",
            UnsupportedValueError,
        )


def _check_npc(function: str, name: str, value: object, names: tuple[str, ...]) -> None:
    """Raise unless ``value`` is None, one of ``names`` or a number in [0, 1]."""
    if value is None or isinstance(value, str):
        _check_choice(function, name, value, names)
        return
    if isinstance(value, bool) or not isinstance(value, Real):
        _fail(function, name, value, "expected a name or a number in [0, 1]", UnsupportedValueError)
    if not 0 <= float(value) <= 1:
        _fail(function, name, value, "a share of the range is in [0, 1]", SpecError)


def _check_digits(function: str, name: str, value: object) -> None:
    """Raise unless ``value`` is None or a whole number from 1 to 15."""
    if value is None:
        return
    if isinstance(value, bool) or not isinstance(value, Integral):
        _fail(function, name, value, "expected a whole number of digits", UnsupportedValueError)
    if not 1 <= int(value) <= 15:
        _fail(function, name, value, "digits are from 1 to 15", SpecError)


def _check_decimals(function: str, name: str, value: object) -> None:
    """Raise unless ``value`` is None or a whole number from 0 to 15."""
    if value is None:
        return
    if isinstance(value, bool) or not isinstance(value, Integral):
        _fail(function, name, value, "expected a whole number of decimals", UnsupportedValueError)
    if not 0 <= int(value) <= 15:
        _fail(function, name, value, "decimals are from 0 to 15", SpecError)


def stat_compare_means(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    method: CompareMethod | None = None,
    paired: bool | None = None,
    comparisons: Sequence[Sequence[Level]] | None = None,
    ref_group: Level | None = None,
    hide_ns: bool | None = None,
    label: Literal["p.format", "p.signif"] | None = None,
    label_sep: str | None = None,
    label_x: Number | None = None,
    label_y: Number | Sequence[Number] | None = None,
    step_increase: Number | None = None,
    tip_length: Number | None = None,
    p_adjust_method: PAdjustMethod | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    manual_key: ManualKey | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Test the groups of y along a discrete x and label the p-values.

    As ``stat_compare_means`` in the R package ggpubr. Without ``comparisons`` or
    ``ref_group``, one global test across the x categories is labelled at the top
    left (``"Kruskal-Wallis, p = 1.5e-09"``); when other aesthetics (``color``)
    split each x into groups, those groups are tested per x. With ``ref_group``,
    every other category is tested against the reference and labelled above it.
    With ``comparisons``, each pair gets a bracket (``geom_bracket``) stacked above
    the data, labelled with its p-value.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``: ``x`` (categories) and ``y``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    method : {"t.test", "wilcox.test", "anova", "kruskal.test"}, optional
        The test: Welch's t test, Wilcoxon rank sum (signed rank when paired), one-way
        ANOVA or Kruskal-Wallis. Default ``"wilcox.test"`` for two groups (and for
        ``comparisons`` and ``ref_group``), ``"kruskal.test"`` for more.
    paired : bool, default=False
        Paired t or Wilcoxon tests; values pair up in row order.
    comparisons : list of pairs, optional
        Pairs of x categories to compare, by name or 1-based position, e.g.
        ``[("0.5", "1"), ("1", "2")]``.
    ref_group : str or int, optional
        Reference category, or ``".all."`` to compare each category with all values.
    hide_ns : bool, default=False
        With ``label="p.signif"``, hide ``ns`` (not significant) marks.
    label : {"p.format", "p.signif"}, optional
        ``"p.format"``: the p-value; ``"p.signif"``: stars (``****`` <= 1e-4,
        ``***`` <= 0.001, ``**`` <= 0.01, ``*`` <= 0.05, else ``ns``). Default: the
        method and p-value (``"Wilcoxon, p = 0.064"``); for brackets, the p-value.
    label_sep : str, default=", "
        Separator between the method and the p-value.
    label_x : float, optional
        x of the global label, in data units.
    label_y : float or list of float, optional
        y of the labels, or the base of the brackets (one per comparison).
    step_increase : float, optional
        Gap between stacked brackets, as a share of the y range: 0.12, or 0 when
        ``label_y`` is given.
    tip_length : float, default=0.03
        Bracket tip length, as a share of the y range.
    p_adjust_method : PAdjustMethod, optional
        Adjust the layer's p-values for multiple comparisons (R ``p.adjust``):
        ``"holm"``, ``"hochberg"``, ``"hommel"``, ``"bonferroni"``, ``"BH"`` (or
        ``"fdr"``), ``"BY"`` or ``"none"``. The
        default ``"none"`` shows unadjusted p-values, as ggpubr.
    geom : str, optional
        ``"text"`` for labels, ``"bracket"`` for ``comparisons`` (the defaults).
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
        Constant aesthetics of the labels and brackets, e.g. ``size``, ``color``.

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If a name is not accepted or a comparison is not a pair.
    UnsupportedValueError
        If an option has the wrong type.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"diet": ["veg"] * 6 + ["meat"] * 6 + ["fish"] * 6,
    ...         "weight": [61, 58, 66, 70, 63, 59, 72, 80, 77, 69, 85, 74,
    ...                    65, 68, 71, 64, 70, 66]}
    >>> plot = (ggplot(data, aes("diet", "weight")) + geom_boxplot()
    ...         + stat_compare_means(comparisons=[("veg", "meat"), ("veg", "fish")]))
    """
    function = "stat_compare_means"
    _check_choice(function, "method", method, _COMPARE_METHODS)
    _check_choice(function, "label", label, _P_LABELS)
    _check_choice(function, "p_adjust_method", p_adjust_method, _ADJUSTMENTS)
    pairs = _comparison_pairs(function, comparisons)
    if ref_group is not None:
        _check_level(function, "ref_group", ref_group)
    if pairs is not None and ref_group is not None:
        _fail(
            function,
            "ref_group",
            ref_group,
            "use either comparisons= or ref_group=, not both",
            SpecError,
        )
    _check_numbers(function, "label_x", label_x)
    _check_numbers(function, "label_y", label_y)
    _check_numbers(function, "step_increase", step_increase)
    _check_numbers(function, "tip_length", tip_length)
    if pairs is not None:
        other_args.setdefault("tiplength_unit", "identity")
    return _geom(
        geom if geom is not None else ("bracket" if pairs is not None else "text"),
        mapping=mapping,
        data=data,
        stat="compare_means",
        position=position,
        show_legend=show_legend,
        inherit_aes=True if inherit_aes is None else inherit_aes,
        manual_key=manual_key,
        sampling=None,
        method=method,
        paired=paired,
        comparisons=pairs,
        ref_group=ref_group,
        hide_ns=hide_ns,
        label=label,
        label_sep=label_sep,
        label_x=label_x,
        label_y=list(label_y) if isinstance(label_y, Sequence) else label_y,
        step_increase=step_increase,
        tip_length=tip_length,
        p_adjust_method=p_adjust_method,
        na_rm=na_rm,
        **other_args,
    )


def stat_cor(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    method: CorrelationMethod | None = None,
    cor_coef_name: str | None = None,
    label_sep: str | None = None,
    label_x_npc: Literal["left", "center", "right"] | Number | None = None,
    label_y_npc: Literal["bottom", "center", "top"] | Number | None = None,
    label_x: Number | Sequence[Number] | None = None,
    label_y: Number | Sequence[Number] | None = None,
    r_digits: int | None = None,
    p_digits: int | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    manual_key: ManualKey | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Label the correlation of x and y and its p-value, per group.

    As ``stat_cor`` in the R package ggpubr: ``"R = -0.87, p = 1.3e-10"`` at the top
    left of the data, one label per group (colored by group), stacked downward.

    Computed variables, for ``aes(label=after_stat(...))``: ``"r.label"``
    (``"R = -0.87"``, the label without its p-value), ``"rr.label"``
    (``"R² = 0.75"``) and ``"p.label"`` (``"p = 1.3e-10"``), as in ggpubr.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``: ``x``, ``y`` and grouping aesthetics.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    method : {"pearson", "spearman", "kendall"}, default="pearson"
        The correlation and its test (R ``cor.test``).
    cor_coef_name : str, default="R"
        Name of the coefficient in the label, e.g. ``"rho"`` or ``"tau"``.
    label_sep : str, default=", "
        Separator between the coefficient and the p-value.
    label_x_npc : {"left", "center", "right"} or float, default="left"
        Horizontal place in the x range (or a share of it).
    label_y_npc : {"bottom", "center", "top"} or float, default="top"
        Vertical place in the y range (or a share of it).
    label_x, label_y : float or list of float, optional
        Place in data units (one value, or one per group), over the ``_npc`` places.
    r_digits, p_digits : int, default=2
        Significant digits of the coefficient and the p-value.
    geom : str, default="text"
        Geom that draws the labels.
    position : str or FeatureSpec, optional
        Position adjustment.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping.
    manual_key : str or FeatureSpec, optional
        This layer's key in a manual legend.
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics of the labels, e.g. ``size``, ``color``.

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
    >>> data = {"height": [150, 160, 165, 170, 175, 182, 190],
    ...         "weight": [52, 58, 63, 66, 74, 80, 88]}
    >>> plot = ggplot(data, aes("height", "weight")) + geom_point() + stat_cor()

    Only the coefficient, without the p-value:

    >>> plot = (ggplot(data, aes("height", "weight")) + geom_point()
    ...         + stat_cor(aes(label=after_stat("r.label"))))
    """
    function = "stat_cor"
    _check_choice(function, "method", method, _CORRELATIONS)
    _check_npc(function, "label_x_npc", label_x_npc, _X_NPC)
    _check_npc(function, "label_y_npc", label_y_npc, _Y_NPC)
    _check_numbers(function, "label_x", label_x)
    _check_numbers(function, "label_y", label_y)
    _check_digits(function, "r_digits", r_digits)
    _check_digits(function, "p_digits", p_digits)
    return _geom(
        geom if geom is not None else "text",
        mapping=mapping,
        data=data,
        stat="cor",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=None,
        method=method,
        cor_coef_name=cor_coef_name,
        label_sep=label_sep,
        label_x_npc=label_x_npc,
        label_y_npc=label_y_npc,
        label_x=list(label_x) if isinstance(label_x, Sequence) else label_x,
        label_y=list(label_y) if isinstance(label_y, Sequence) else label_y,
        r_digits=r_digits,
        p_digits=p_digits,
        na_rm=na_rm,
        **other_args,
    )


def stat_poly_eq(
    mapping: FeatureSpec | None = None,
    *,
    data: PlotData | None = None,
    formula: str | None = None,
    label: PolyLabel | Sequence[PolyLabel] | None = None,
    label_sep: str | None = None,
    label_x_npc: Literal["left", "center", "right"] | Number | None = None,
    label_y_npc: Literal["bottom", "center", "top"] | Number | None = None,
    label_x: Number | Sequence[Number] | None = None,
    label_y: Number | Sequence[Number] | None = None,
    coef_digits: int | None = None,
    rr_digits: int | None = None,
    r_digits: int | None = None,
    slope_digits: int | None = None,
    slope_name: str | None = None,
    small_r: bool | None = None,
    geom: str | None = None,
    position: Position | None = None,
    show_legend: bool | None = None,
    inherit_aes: bool | None = None,
    manual_key: ManualKey | None = None,
    na_rm: bool | None = None,
    **other_args: Any,
) -> LayerSpec:
    """Label a least squares polynomial fit of y on x (equation, R²), per group.

    As ``stat_poly_eq`` in the R package ggpmisc. The default label is ``"R² = 0.75"``;
    ``label=["eq", "R2"]`` (ggpmisc ``use_label("eq", "R2")``) also shows the equation
    ``"y = 37.3 - 5.34x"``. Labels sit at the top left of the data, stacked per group,
    as ``stat_cor``. For a line, ``label=["R", "slope"]`` shows the signed Pearson r
    and the slope: ``"R = 0.58, slope = 0.53"`` (``small_r=True`` and
    ``slope_name="m"`` give ``"r = 0.58, m = 0.53"``).

    Computed variables, for ``aes(label=after_stat(...))``: ``"eq.label"``,
    ``"rr.label"``, ``"adj.rr.label"``, ``"p.value.label"`` and ``"n.label"``, as
    in ggpmisc.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``: ``x``, ``y`` and grouping aesthetics.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    formula : str, default="y ~ x"
        The model: ``"y ~ x"``, ``"y ~ poly(x, 2, raw = TRUE)"`` or
        ``"y ~ x + I(x^2)"``. Pair it with ``geom_smooth(method="lm", deg=2)``.
    label : str or list of str, default="R2"
        Parts of the label: ``"eq"`` (equation), ``"R2"``, ``"adj.R2"``, ``"P"`` (the
        fit's F test) and ``"n"``; for a linear ``formula`` also ``"R"`` (Pearson's
        r, with the sign of the slope) and ``"slope"``.
    label_sep : str, default=", "
        Separator between the parts.
    label_x_npc : {"left", "center", "right"} or float, default="left"
        Horizontal place in the x range (or a share of it).
    label_y_npc : {"bottom", "center", "top"} or float, default="top"
        Vertical place in the y range (or a share of it).
    label_x, label_y : float or list of float, optional
        Place in data units (one value, or one per group).
    coef_digits : int, default=3
        Significant digits of the coefficients.
    rr_digits : int, default=2
        Decimals of R².
    r_digits : int, default=2
        Decimals of r (label part ``"R"``), from 0.
    slope_digits : int, default=2
        Decimals of the slope (label part ``"slope"``), from 0.
    slope_name : str, default="slope"
        Name of the slope in the label, e.g. ``"m"`` or ``"b"``.
    small_r : bool, default=False
        Write ``r`` and ``r²`` instead of ``R`` and ``R²`` (ggpmisc ``small.r``).
    geom : str, default="text"
        Geom that draws the labels.
    position : str or FeatureSpec, optional
        Position adjustment.
    show_legend : bool, optional
        False hides this layer from the legend.
    inherit_aes : bool, optional
        False ignores the plot's default mapping.
    manual_key : str or FeatureSpec, optional
        This layer's key in a manual legend.
    na_rm : bool, optional
        True removes rows with missing values silently.
    **other_args
        Constant aesthetics of the labels, e.g. ``size``, ``color``.

    Returns
    -------
    LayerSpec
        The layer.

    Raises
    ------
    SpecError
        If a label part or place name is not accepted. At render time, if
        ``"R"`` or ``"slope"`` is asked of a non-linear ``formula``.
    UnsupportedValueError
        If an option has the wrong type.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"hours": [1, 2, 3, 4, 5, 6, 7, 8],
    ...         "score": [52, 55, 61, 64, 70, 71, 78, 83]}
    >>> plot = (ggplot(data, aes("hours", "score")) + geom_point()
    ...         + stat_poly_eq(label=["eq", "R2"]))

    Pearson's r and the slope, per group, as ``"r = 0.99, m = 4.4"``:

    >>> plot = (ggplot(data, aes("hours", "score")) + geom_point()
    ...         + stat_poly_eq(label=["R", "slope"], small_r=True,
    ...                        slope_name="m", slope_digits=1))
    """
    function = "stat_poly_eq"
    if formula is not None and not isinstance(formula, str):
        _fail(
            function,
            "formula",
            formula,
            "expected a formula string, e.g. 'y ~ x'",
            UnsupportedValueError,
        )
    parts: list[str] | None = None
    if label is not None:
        parts = [label] if isinstance(label, str) else list(label)
        if not parts:
            _fail(function, "label", label, "expected at least one label part", SpecError)
        for part in parts:
            _check_choice(function, "label", part, _POLY_LABELS)
    _check_npc(function, "label_x_npc", label_x_npc, _X_NPC)
    _check_npc(function, "label_y_npc", label_y_npc, _Y_NPC)
    _check_numbers(function, "label_x", label_x)
    _check_numbers(function, "label_y", label_y)
    _check_digits(function, "coef_digits", coef_digits)
    _check_digits(function, "rr_digits", rr_digits)
    _check_decimals(function, "r_digits", r_digits)
    _check_decimals(function, "slope_digits", slope_digits)
    if slope_name is not None and not isinstance(slope_name, str):
        _fail(
            function, "slope_name", slope_name, "expected a string, e.g. 'm'", UnsupportedValueError
        )
    if small_r is not None and not isinstance(small_r, bool):
        _fail(function, "small_r", small_r, "expected True or False", UnsupportedValueError)
    return _geom(
        geom if geom is not None else "text",
        mapping=mapping,
        data=data,
        stat="poly_eq",
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=None,
        formula=formula,
        label=parts,
        label_sep=label_sep,
        label_x_npc=label_x_npc,
        label_y_npc=label_y_npc,
        label_x=list(label_x) if isinstance(label_x, Sequence) else label_x,
        label_y=list(label_y) if isinstance(label_y, Sequence) else label_y,
        coef_digits=coef_digits,
        rr_digits=rr_digits,
        r_digits=r_digits,
        slope_digits=slope_digits,
        slope_name=slope_name,
        small_r=small_r,
        na_rm=na_rm,
        **other_args,
    )
