"""Stat layers order their groups by level, as lets-plot does with ``factor_levels``.

lets-plot 4.11 dodges and stacks the groups of a polars Enum, an ordered pandas
categorical or ``as_discrete(order=1)`` in level order (first level on the left, at
the bottom of a stack), and the groups of a plain string column in first appearance.
Identity layers (points, jitterdodge) always did; stat layers (boxplot, violin,
count, summary) used first appearance up to 0.9.0, so jittered points landed on
the wrong box.
"""

import pandas as pd
import polars as pl
import pytest
from ggrust import (
    aes,
    as_discrete,
    facet_wrap,
    geom_bar,
    geom_boxplot,
    geom_point,
    geom_violin,
    ggplot,
    position_dodge,
    position_jitterdodge,
    stat_summary,
)
from ggrust.plot.core import PlotSpec

LEVELS = pl.Enum(["a", "b"])
# "b" rows come first, so first appearance and level order disagree.
FRAME = pl.DataFrame(
    {
        "x": ["u"] * 4 + ["v"] * 4,
        "y": [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
        "g": pl.Series(["b", "b", "a", "a"] * 2, dtype=LEVELS),
    }
)
STRINGS = FRAME.with_columns(pl.col("g").cast(pl.String))


def left_to_right(plot: PlotSpec, column: str = "xmin") -> list[str]:
    """Fill (or color) levels of the first x slot's rows, from left to right."""
    data = plot.layer_data(0).filter(pl.col("x").is_between(-0.5, 0.5)).sort(column)
    paint = "fill" if "fill" in data.columns else "color"
    return list(dict.fromkeys(data[paint].to_list()))


@pytest.mark.parametrize(
    "plot",
    [
        ggplot(FRAME, aes("x", "y", fill="g")) + geom_boxplot(),
        ggplot(FRAME, aes("x", "y", fill="g")) + geom_violin(),
        ggplot(FRAME, aes("x", fill="g")) + geom_bar(position="dodge"),
        ggplot(FRAME, aes("x", "y", color="g")) + stat_summary(position=position_dodge(0.8)),
        ggplot(FRAME.with_columns(f=pl.lit("F")), aes("x", "y", fill="g"))
        + geom_boxplot()
        + facet_wrap("f"),
    ],
    ids=["boxplot", "violin", "bar", "summary", "facet"],
)
def test_stat_layers_dodge_in_level_order(plot: PlotSpec) -> None:
    assert left_to_right(plot, "x") == ["a", "b"]


def test_stack_puts_the_first_level_at_the_bottom() -> None:
    data = (ggplot(FRAME, aes("x", fill="g")) + geom_bar()).layer_data(0)
    bottom_up = data.filter(pl.col("x").round(0) == 0).sort("ymin")["fill"].to_list()
    assert bottom_up == ["a", "b"]


def test_jitterdodge_points_land_on_their_box() -> None:
    plot = (
        ggplot(FRAME, aes("x", "y", fill="g"))
        + geom_boxplot()
        + geom_point(position=position_jitterdodge(jitter_width=0, seed=1))
    )
    first_slot = pl.col("x") < 0.5
    boxes = plot.layer_data(0).filter(first_slot).select("fill", "x")
    # Layer 1 is the boxplot's outlier layer.
    points = plot.layer_data(2).filter(first_slot).group_by("fill").agg(pl.col("x").mean())
    joined = boxes.join(points, on="fill", suffix="_point")
    assert joined.height == 2
    # Same side of the slot (jitterdodge's default dodge width differs slightly).
    assert (joined["x"] - joined["x_point"]).abs().max() < 0.05


def test_ordered_pandas_and_as_discrete_follow_their_order() -> None:
    frame = FRAME.to_pandas()
    frame["g"] = pd.Categorical(frame["g"].astype(str), categories=["a", "b"], ordered=True)
    assert left_to_right(ggplot(frame, aes("x", "y", fill="g")) + geom_boxplot()) == ["a", "b"]
    by_order = aes("x", "y", fill=as_discrete("g", order=1))
    assert left_to_right(ggplot(STRINGS, by_order) + geom_boxplot()) == ["a", "b"]


def test_strings_keep_first_appearance() -> None:
    assert left_to_right(ggplot(STRINGS, aes("x", "y", fill="g")) + geom_boxplot()) == ["b", "a"]
