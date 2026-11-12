"""Validation suite: ggrust's stat results and scales against R's ``ggplot_build()``.

Each case is the twin of one in ``tests/fixtures/generate_ggplot2_build_reference.R``
(same name, same data, same layer); the R output is checked in as
``tests/fixtures/ggplot2_build_reference.json``, so this test does not need R.
Regenerate it with ``Rscript tests/fixtures/generate_ggplot2_build_reference.R``.

A case compares the listed layer columns (rows matched by sorting, so group order
does not matter) and the panel's x and y scales: expanded range, breaks, labels and
levels. Discrete positions are compared by category name: ggplot2 counts its sorted
levels from 1, ggrust its first-appearance levels from 0.

Every known difference is listed in ``KNOWN`` with its reason (a ``deviations.md``
row). A case fails when it differs in a part that is not listed, and also when a
listed part now agrees, so the list stays exact.
"""

from __future__ import annotations

import json
import math
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path

import ggrust as gg
import polars as pl
import pytest
from ggrust.data import iris, mpg, penguins

REFERENCE = json.loads(
    (Path(__file__).parent / "fixtures" / "ggplot2_build_reference.json").read_text()
)
PENGUINS = penguins.drop_nulls()
TOLERANCE = 1e-6
"""Relative tolerance of a compared number (to the column's span)."""
DENSITY_COLUMNS = ("density", "scaled", "count")
DENSITY_TOLERANCE = 2e-3
"""R's ``density()`` bins the data onto a 512-point grid and convolves by FFT; ggrust
(as lets-plot) sums the kernels exactly. The two agree to about 1e-3 of the peak."""
VALUES = pl.DataFrame({"item": ["a", "b", "c"], "value": [3.0, -1.5, 7.0]})
TINY = pl.DataFrame({"x": [-0.031, 0.0043, 0.017], "y": [-12500.0, 3.0, 98000.0]})


@dataclass(frozen=True)
class Case:
    """One plot and the parts of its build to compare."""

    plot: gg.PlotSpec
    columns: tuple[str, ...]
    """Layer columns compared row by row (as named by ggplot2)."""
    layer: int = 0
    """The ggrust layer holding the stat output (ggplot2's is always layer 0)."""
    renamed: dict[str, str] = field(default_factory=dict)
    """ggplot2 column name -> ggrust column name, where they differ."""
    outliers: bool = False
    """Compare ggplot2's boxplot ``outliers`` with ggrust's outlier layer (layer 1)."""
    loose: tuple[str, ...] = ()
    """Columns compared at ``DENSITY_TOLERANCE``; every other one at ``TOLERANCE``."""
    drop_empty: bool = False
    """Leave out zero-``count`` rows beyond the first and last non-empty bin on each
    side: ggrust lays bins out over a wider range (lets-plot's 0.7 bin of slack)."""
    repeated_vertex: bool = False
    """Drop a ggrust row equal to the one before it: lets-plot repeats the first vertex
    of each density path."""


CASES: dict[str, Case] = {
    "histogram_bins": Case(
        gg.ggplot(mpg, gg.aes("hwy")) + gg.geom_histogram(bins=10),
        ("x", "xmin", "xmax", "count", "density"),
        drop_empty=True,
    ),
    "histogram_binwidth_boundary": Case(
        gg.ggplot(mpg, gg.aes("hwy")) + gg.geom_histogram(binwidth=2, boundary=0),
        ("x", "xmin", "xmax", "count", "density"),
        drop_empty=True,
    ),
    "histogram_binwidth_center": Case(
        gg.ggplot(PENGUINS, gg.aes("body_mass_g")) + gg.geom_histogram(binwidth=250, center=0),
        ("x", "xmin", "xmax", "count", "density"),
        drop_empty=True,
    ),
    "histogram_stacked": Case(
        gg.ggplot(mpg, gg.aes("hwy", fill="drv")) + gg.geom_histogram(binwidth=3, boundary=0),
        ("x", "count", "ymin", "ymax"),
        drop_empty=True,
    ),
    "freqpoly": Case(
        gg.ggplot(mpg, gg.aes("cty")) + gg.geom_freqpoly(binwidth=1),
        ("x", "count"),
    ),
    "bar_count": Case(gg.ggplot(mpg, gg.aes("class")) + gg.geom_bar(), ("x", "count")),
    "bar_stacked": Case(
        gg.ggplot(mpg, gg.aes("class", fill="drv")) + gg.geom_bar(),
        ("x", "count", "ymin", "ymax"),
    ),
    "bar_dodged": Case(
        gg.ggplot(mpg, gg.aes("class", fill="drv")) + gg.geom_bar(position="dodge"),
        ("x", "xmin", "xmax", "count"),
    ),
    "bar_filled": Case(
        gg.ggplot(mpg, gg.aes("class", fill="drv")) + gg.geom_bar(position="fill"),
        ("x", "ymin", "ymax"),
    ),
    "col_values": Case(
        gg.ggplot(VALUES, gg.aes("item", "value")) + gg.geom_col(),
        ("x", "ymin", "ymax"),
    ),
    "density_default": Case(
        gg.ggplot(mpg, gg.aes("hwy")) + gg.geom_density(),
        ("x", "density", "scaled", "count"),
        loose=DENSITY_COLUMNS,
        repeated_vertex=True,
    ),
    "density_grouped": Case(
        gg.ggplot(mpg, gg.aes("hwy", color="drv")) + gg.geom_density(),
        ("x", "density", "scaled", "count"),
        loose=DENSITY_COLUMNS,
        repeated_vertex=True,
    ),
    "density_adjust": Case(
        gg.ggplot(PENGUINS, gg.aes("flipper_length_mm"))
        + gg.geom_density(adjust=0.5, kernel="epanechikov"),
        ("x", "density"),
        loose=DENSITY_COLUMNS,
        repeated_vertex=True,
    ),
    "smooth_lm": Case(
        gg.ggplot(mpg, gg.aes("displ", "hwy")) + gg.geom_smooth(method="lm"),
        ("x", "y", "ymin", "ymax", "se"),
    ),
    "smooth_lm_grouped": Case(
        gg.ggplot(mpg, gg.aes("displ", "hwy", color="drv"))
        + gg.geom_smooth(method="lm", level=0.9),
        ("x", "y", "ymin", "ymax", "se"),
    ),
    "smooth_loess": Case(
        gg.ggplot(mpg, gg.aes("displ", "hwy")) + gg.geom_smooth(method="loess"),
        ("x", "y", "ymin", "ymax", "se"),
    ),
    "smooth_glm_poly": Case(
        gg.ggplot(iris, gg.aes("sepal_length", "petal_length"))
        + gg.geom_smooth(method="lm", deg=2),
        ("x", "y", "ymin", "ymax", "se"),
    ),
    "boxplot": Case(
        gg.ggplot(mpg, gg.aes("class", "hwy")) + gg.geom_boxplot(),
        ("x", "ymin", "lower", "middle", "upper", "ymax"),
        outliers=True,
    ),
    "boxplot_coef": Case(
        gg.ggplot(PENGUINS, gg.aes("species", "body_mass_g")) + gg.geom_boxplot(coef=1),
        ("x", "ymin", "lower", "middle", "upper", "ymax"),
        outliers=True,
    ),
    "violin": Case(
        gg.ggplot(iris, gg.aes("species", "sepal_width")) + gg.geom_violin(),
        ("x", "y", "density", "scaled", "count"),
        loose=DENSITY_COLUMNS,
        repeated_vertex=True,
    ),
    "ecdf": Case(gg.ggplot(mpg, gg.aes("hwy")) + gg.stat_ecdf(), ("x", "y")),
    "summary_mean_se": Case(
        gg.ggplot(mpg, gg.aes("class", "hwy")) + gg.stat_summary(fun_data="mean_se"),
        ("x", "y", "ymin", "ymax"),
    ),
    "summary_median_range": Case(
        gg.ggplot(mpg, gg.aes("class", "hwy"))
        + gg.stat_summary(fun="median", fun_min="min", fun_max="max"),
        ("x", "y", "ymin", "ymax"),
    ),
    "count_points": Case(
        gg.ggplot(mpg, gg.aes("cty", "hwy")) + gg.geom_count(), ("x", "y", "n", "prop")
    ),
    "qq": Case(gg.ggplot(mpg, gg.aes(sample="hwy")) + gg.stat_qq(), ("x", "y")),
    "qq_line": Case(gg.ggplot(mpg, gg.aes(sample="hwy")) + gg.geom_qq_line(), ("x", "y")),
    "quantile": Case(
        gg.ggplot(mpg, gg.aes("displ", "hwy")) + gg.geom_quantile(),
        ("x", "y", "quantile"),
    ),
    "bin2d": Case(
        gg.ggplot(mpg, gg.aes("displ", "hwy")) + gg.geom_bin2d(bins=[8, 8]),
        ("x", "y", "count", "density"),
    ),
    "point_scales": Case(
        gg.ggplot(PENGUINS, gg.aes("bill_length_mm", "body_mass_g")) + gg.geom_point(), ()
    ),
    "point_log10": Case(
        gg.ggplot(PENGUINS, gg.aes("bill_length_mm", "body_mass_g"))
        + gg.geom_point()
        + gg.scale_y_log10(),
        (),
    ),
    "point_sqrt": Case(
        gg.ggplot(mpg, gg.aes("displ", "hwy")) + gg.geom_point() + gg.scale_x_sqrt(), ()
    ),
    "point_reverse": Case(
        gg.ggplot(mpg, gg.aes("displ", "hwy")) + gg.geom_point() + gg.scale_x_reverse(), ()
    ),
    "point_limits": Case(
        gg.ggplot(mpg, gg.aes("displ", "hwy"))
        + gg.geom_point(na_rm=True)
        + gg.scale_x_continuous(limits=[2, 6]),
        (),
    ),
    "point_expand": Case(
        gg.ggplot(mpg, gg.aes("displ", "hwy"))
        + gg.geom_point()
        + gg.scale_y_continuous(expand=[0.1, 1]),
        (),
    ),
    "point_negative": Case(gg.ggplot(TINY, gg.aes("x", "y")) + gg.geom_point(), ()),
}

# Why ggrust differs, one constant per deviations.md row ("inherited" rows follow
# lets-plot; "gap" rows are open). Quote the row title so it can be found.
BREAKS = "inherited, 'Default continuous axis breaks': lets-plot's denser steps"
TRANSFORMED_BREAKS = "inherited, 'Breaks on log and sqrt axes': nice transformed values"
LEVEL_ORDER = "inherited, 'Order of string categories': first appearance, not sorted"
DISCRETE_RANGE = "inherited, 'Discrete axis expansion': 0.2 past the geom, not 0.6"
ZERO_BASED = "inherited, 'Zero-based count and density axes': no padding below 0"
HISTOGRAM_BINS = "inherited, 'Histogram bins': lets-plot's layout and [low, high) bins"
FREQPOLY_PAD = "inherited, 'geom_freqpoly end bins': no zero bin at either end"
BIN2D_GRID = "inherited, 'geom_bin2d grid': lets-plot's grid, half a bin wider"
SMOOTH_FIT = "inherited, 'Polynomial and loess smooths': y averaged over tied x, Apache loess"
DENSITY_KERNEL = "inherited, 'Density kernels other than gaussian': unit support, not unit sd"
VIOLIN_QUANTILE_ROWS = "inherited, 'Violin quantile rows': ggplot2 4.0 appends the quartiles"
BAR_STACK_ORDER = "inherited, 'Stack order': first group at the bottom, ggplot2 puts it on top"
BAR_DODGE_SLOTS = "inherited, 'Dodge slots': a slot for every group, even where absent"

KNOWN: dict[str, dict[str, str]] = {
    "histogram_bins": {
        "layer.count": HISTOGRAM_BINS,
        "layer.density": HISTOGRAM_BINS,
        "layer.x": HISTOGRAM_BINS,
        "layer.xmax": HISTOGRAM_BINS,
        "layer.xmin": HISTOGRAM_BINS,
        "x.breaks": BREAKS,
        "x.range": HISTOGRAM_BINS,
        "y.breaks": BREAKS,
        "y.range": ZERO_BASED,
    },
    "histogram_binwidth_boundary": {
        "layer.rows": HISTOGRAM_BINS,
        "x.breaks": BREAKS,
        "x.range": HISTOGRAM_BINS,
        "y.breaks": BREAKS,
        "y.range": ZERO_BASED,
    },
    "histogram_binwidth_center": {
        "layer.rows": HISTOGRAM_BINS,
        "x.breaks": BREAKS,
        "x.range": HISTOGRAM_BINS,
        "y.breaks": BREAKS,
        "y.range": ZERO_BASED,
    },
    "histogram_stacked": {
        "layer.count": HISTOGRAM_BINS,
        "layer.ymax": HISTOGRAM_BINS,
        "layer.ymin": HISTOGRAM_BINS,
        "x.breaks": BREAKS,
        "x.range": HISTOGRAM_BINS,
        "y.breaks": BREAKS,
        "y.range": ZERO_BASED,
    },
    "freqpoly": {
        "layer.rows": FREQPOLY_PAD,
        "x.breaks": BREAKS,
        "x.range": FREQPOLY_PAD,
        "y.breaks": BREAKS,
    },
    "bar_count": {
        "x.levels": LEVEL_ORDER,
        "x.range": DISCRETE_RANGE,
        "y.breaks": BREAKS,
        "y.range": ZERO_BASED,
    },
    "bar_stacked": {
        "layer.ymax": BAR_STACK_ORDER,
        "layer.ymin": BAR_STACK_ORDER,
        "x.levels": LEVEL_ORDER,
        "x.range": DISCRETE_RANGE,
        "y.breaks": BREAKS,
        "y.range": ZERO_BASED,
    },
    "bar_dodged": {
        "layer.count": BAR_DODGE_SLOTS,
        "layer.x": BAR_DODGE_SLOTS,
        "layer.xmax": BAR_DODGE_SLOTS,
        "layer.xmin": BAR_DODGE_SLOTS,
        "x.levels": LEVEL_ORDER,
        "x.range": DISCRETE_RANGE,
        "y.range": ZERO_BASED,
    },
    "bar_filled": {
        "layer.ymax": BAR_STACK_ORDER,
        "layer.ymin": BAR_STACK_ORDER,
        "x.levels": LEVEL_ORDER,
        "x.range": DISCRETE_RANGE,
        "y.breaks": BREAKS,
        "y.range": ZERO_BASED,
    },
    "col_values": {
        "x.range": DISCRETE_RANGE,
        "y.breaks": BREAKS,
    },
    "density_default": {
        "x.breaks": BREAKS,
        "y.breaks": BREAKS,
        "y.range": ZERO_BASED,
    },
    "density_grouped": {
        "x.breaks": BREAKS,
        "y.breaks": BREAKS,
        "y.range": ZERO_BASED,
    },
    "density_adjust": {
        "layer.density": DENSITY_KERNEL,
        "y.breaks": BREAKS,
        "y.range": ZERO_BASED,
    },
    "smooth_loess": {
        "layer.se": SMOOTH_FIT,
        "layer.y": SMOOTH_FIT,
        "layer.ymax": SMOOTH_FIT,
        "layer.ymin": SMOOTH_FIT,
        "y.breaks": BREAKS,
        "y.range": SMOOTH_FIT,
    },
    "smooth_glm_poly": {
        "layer.se": SMOOTH_FIT,
        "layer.y": SMOOTH_FIT,
        "layer.ymax": SMOOTH_FIT,
        "layer.ymin": SMOOTH_FIT,
        "x.breaks": BREAKS,
        "y.breaks": BREAKS,
        "y.range": SMOOTH_FIT,
    },
    "boxplot": {
        "x.levels": LEVEL_ORDER,
        "x.range": DISCRETE_RANGE,
        "y.breaks": BREAKS,
    },
    "boxplot_coef": {
        "x.range": DISCRETE_RANGE,
        "y.breaks": BREAKS,
    },
    "violin": {
        "layer.rows": VIOLIN_QUANTILE_ROWS,
        "x.range": DISCRETE_RANGE,
    },
    "ecdf": {
        "x.breaks": BREAKS,
        "y.breaks": BREAKS,
    },
    "summary_mean_se": {
        "x.levels": LEVEL_ORDER,
        "x.range": DISCRETE_RANGE,
        "y.breaks": BREAKS,
    },
    "summary_median_range": {
        "x.levels": LEVEL_ORDER,
        "x.range": DISCRETE_RANGE,
        "y.breaks": BREAKS,
    },
    "count_points": {
        "y.breaks": BREAKS,
    },
    "qq": {
        "y.breaks": BREAKS,
    },
    "qq_line": {
        "y.breaks": BREAKS,
    },
    "bin2d": {
        "layer.rows": BIN2D_GRID,
        "x.breaks": BREAKS,
        "x.range": BIN2D_GRID,
        "y.breaks": BREAKS,
        "y.range": BIN2D_GRID,
    },
    "point_scales": {
        "x.breaks": BREAKS,
        "y.breaks": BREAKS,
    },
    "point_log10": {
        "x.breaks": BREAKS,
        "y.breaks": TRANSFORMED_BREAKS,
    },
    "point_sqrt": {
        "x.breaks": TRANSFORMED_BREAKS,
        "y.breaks": BREAKS,
    },
    "point_reverse": {
        "y.breaks": BREAKS,
    },
    "point_limits": {
        "x.breaks": BREAKS,
        "y.breaks": BREAKS,
    },
    "point_expand": {
        "y.breaks": BREAKS,
    },
    "point_negative": {
        "x.breaks": BREAKS,
        "y.breaks": BREAKS,
    },
}
"""Case -> {differing part: reason}. Parts are ``layer.<column>``, ``layer.rows``,
``layer.outliers`` and ``<x|y>.<range|breaks|labels|levels>``."""


def close(
    left: float | None, right: float | None, span: float, tolerance: float = TOLERANCE
) -> bool:
    """Whether two numbers agree within ``tolerance`` of ``span`` (NaN equals None)."""
    left_missing = left is None or (isinstance(left, float) and math.isnan(left))
    right_missing = right is None or (isinstance(right, float) and math.isnan(right))
    if left_missing or right_missing:
        return left_missing and right_missing
    assert left is not None and right is not None
    return abs(left - right) <= tolerance * max(span, abs(left), abs(right), 1e-12)


def span(values: list[float | None]) -> float:
    finite = [value for value in values if value is not None and math.isfinite(value)]
    return max(finite) - min(finite) if finite else 0.0


def without_repeats(columns: dict[str, list[float | None]]) -> dict[str, list[float | None]]:
    """Drop a row equal to the one before it: lets-plot repeats a path's first vertex."""
    rows = list(zip(*columns.values(), strict=True))
    kept = [row for index, row in enumerate(rows) if index == 0 or row != rows[index - 1]]
    return {name: [row[at] for row in kept] for at, name in enumerate(columns)}


def without_empty_edges(
    rows: list[tuple[float, ...]], columns: list[str]
) -> list[tuple[float, ...]]:
    """Drop zero-count rows whose x lies outside the non-empty bins' x range."""
    x, count = columns.index("x"), columns.index("count")
    filled = [row[x] for row in rows if row[count] != 0 and row[x] is not None]
    low, high = min(filled), max(filled)
    return [row for row in rows if row[count] != 0 or low <= row[x] <= high]


def sorted_rows(columns: dict[str, list[float | None]]) -> list[tuple[float, ...]]:
    def key(value: float | None) -> float:
        return math.inf if value is None or math.isnan(value) else round(value, 9)

    rows = zip(*columns.values(), strict=True)
    return sorted(rows, key=lambda row: tuple(key(value) for value in row))


POSITIONS = ("x", "xmin", "xmax")
"""Layer columns holding x positions."""


def discrete_positions(
    expected: dict, actual: dict
) -> Callable[[float | None], float | None] | None:
    """For a discrete x: ggrust position -> the same category's position in ggplot2,
    0-based. Categories may be in another order (``KNOWN`` lists ``x.levels``); a
    dodged offset inside the category is kept. None for a continuous x."""
    if not expected["discrete"]:
        return None
    reference = {level: index for index, level in enumerate(expected["levels"])}
    ours = [reference.get(level, math.nan) for level in actual["levels"]]

    def convert(value: float | None) -> float | None:
        if value is None or math.isnan(value):
            return value
        slot = min(max(round(value), 0), len(ours) - 1)
        return ours[slot] + (value - slot)

    return convert


def layer_differences(name: str, case: Case, built: dict) -> dict[str, str]:
    reference = REFERENCE["cases"][name]
    expected_layer = reference["layers"][0]
    frame: pl.DataFrame = built["layers"][case.layer]
    to_reference = discrete_positions(reference["x"], built["panels"][0]["x"])
    differences: dict[str, str] = {}
    expected: dict[str, list[float | None]] = {}
    actual: dict[str, list[float | None]] = {}
    for column in case.columns:
        ours = case.renamed.get(column, column)
        if column not in expected_layer:
            differences[f"layer.{column}"] = "not in ggplot2's layer data"
            continue
        if ours not in frame.columns:
            differences[f"layer.{column}"] = f"ggrust has no column {ours!r}"
            continue
        expected[column] = [
            None
            if value is None
            else value - 1.0
            if column in POSITIONS and to_reference
            else value
            for value in expected_layer[column]
        ]
        # JSON has no infinity: ggplot2's -Inf / Inf (ecdf padding) arrive as null.
        values = [
            None if value is not None and math.isinf(value) else value
            for value in frame[ours].cast(pl.Float64).to_list()
        ]
        if column in POSITIONS and to_reference:
            values = [to_reference(value) for value in values]
        actual[column] = values
    if expected:
        expected_rows = sorted_rows(expected)
        actual_rows = sorted_rows(without_repeats(actual) if case.repeated_vertex else actual)
        if case.drop_empty:
            expected_rows = without_empty_edges(expected_rows, list(expected))
            actual_rows = without_empty_edges(actual_rows, list(expected))
        if len(expected_rows) != len(actual_rows):
            differences["layer.rows"] = (
                f"ggplot2 has {len(expected_rows)} rows, ggrust {len(actual_rows)}"
            )
        else:
            for index, column in enumerate(expected):
                width = span(expected[column])
                bad = [
                    (left[index], right[index])
                    for left, right in zip(expected_rows, actual_rows, strict=True)
                    if not close(
                        left[index],
                        right[index],
                        width,
                        DENSITY_TOLERANCE if column in case.loose else TOLERANCE,
                    )
                ]
                if bad:
                    differences[f"layer.{column}"] = (
                        f"{len(bad)} of {len(expected_rows)} rows differ, first "
                        f"(ggplot2, ggrust) = {bad[0]}"
                    )
    if case.outliers:
        # Per box: x position (ggplot2's 0-based level index) -> sorted outliers.
        expected_outliers: dict[float, list[float]] = {}
        for x, values in zip(expected_layer["x"], expected_layer["outliers"], strict=True):
            values = values if isinstance(values, list) else [values]
            expected_outliers[round(x - 1.0, 6)] = sorted(values)
        actual_outliers: dict[float, list[float]] = {}
        outlier_frame: pl.DataFrame = built["layers"][1]
        for x, y in outlier_frame.select("x", "y").rows():
            if y is None or math.isnan(y):
                continue
            box = to_reference(x) if to_reference else x
            assert box is not None
            actual_outliers.setdefault(round(box, 6), []).append(y)
        actual_outliers = {box: sorted(values) for box, values in actual_outliers.items()}
        expected_outliers = {box: values for box, values in expected_outliers.items() if values}
        if expected_outliers != actual_outliers:
            differences["layer.outliers"] = f"ggplot2 {expected_outliers}, ggrust {actual_outliers}"
    return differences


def same_label(theirs: str, mine: str) -> bool:
    """Labels name the same number; ggrust writes ``3,000`` and ``2`` where ggplot2
    writes ``3000`` and ``2.0`` (deviations.md 'Axis label decimals')."""
    try:
        return float(theirs.replace(",", "")) == float(mine.replace(",", ""))
    except ValueError:
        return theirs == mine


def scale_differences(name: str, built: dict) -> dict[str, str]:
    reference = REFERENCE["cases"][name]
    differences: dict[str, str] = {}
    for axis in ["x", "y"]:
        expected = reference[axis]
        actual = built["panels"][0][axis]
        if expected["discrete"] != actual["discrete"]:
            differences[f"{axis}.discrete"] = f"{expected['discrete']} vs {actual['discrete']}"
            continue
        offset = 1.0 if expected["discrete"] else 0.0
        low, high = (value - offset for value in expected["range"])
        width = high - low
        domain = actual["domain"]
        if not (close(low, domain[0], width) and close(high, domain[1], width)):
            differences[f"{axis}.range"] = f"ggplot2 {[low, high]}, ggrust {list(domain)}"
        if expected["discrete"]:
            if list(expected["levels"]) != list(actual["levels"]):
                differences[f"{axis}.levels"] = (
                    f"ggplot2 {expected['levels']}, ggrust {actual['levels']}"
                )
            continue
        breaks = actual["breaks"]
        if len(expected["breaks"]) != len(breaks) or not all(
            close(left, right, width)
            for left, right in zip(expected["breaks"], breaks, strict=False)
        ):
            differences[f"{axis}.breaks"] = f"ggplot2 {expected['breaks']}, ggrust {breaks}"
        # Labels of the breaks both sides draw (even when the break lists differ).
        ours = dict(zip(breaks, actual["labels"], strict=True))
        shared = [
            (label, ours[mine])
            for value, label in zip(expected["breaks"], expected["labels"], strict=True)
            for mine in ours
            if close(value, mine, width)
        ]
        if any(not same_label(theirs, mine) for theirs, mine in shared):
            differences[f"{axis}.labels"] = f"(ggplot2, ggrust) on shared breaks: {shared}"
    return differences


def differences(name: str) -> dict[str, str]:
    case = CASES[name]
    built = case.plot.build()
    return {**layer_differences(name, case, built), **scale_differences(name, built)}


def test_reference_was_made_by_ggplot2_4() -> None:
    assert REFERENCE["ggplot2"].startswith("4.")
    assert set(REFERENCE["cases"]) == set(CASES)


@pytest.mark.parametrize("name", list(CASES))
def test_matches_ggplot2(name: str) -> None:
    found = differences(name)
    known = KNOWN.get(name, {})
    unexpected = {part: detail for part, detail in found.items() if part not in known}
    fixed = sorted(set(known) - set(found))
    assert not unexpected, f"{name}: new differences from ggplot2: {unexpected}"
    assert not fixed, f"{name}: now agrees with ggplot2 on {fixed}; remove them from KNOWN"
