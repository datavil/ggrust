"""ggplot2's ``geom_col``: ``geom_bar(stat="identity")`` under its ggplot2 name.

Reference values are ggplot2's ``layer_data()`` for the same specs (ggplot2 3.5.2;
the bar setup is unchanged in 4.x). Stacking order follows ggrust's lets-plot
``position_stack`` (first group at the bottom), see deviations.md.
"""

from __future__ import annotations

import ggrust as gg
import polars as pl
import pytest
from ggrust.errors import SpecError

FRUIT = pl.DataFrame(
    {
        "fruit": ["apple", "apple", "banana", "banana", "cherry", "cherry"],
        "sold": [3.0, 2.0, -1.0, 4.0, 5.0, -2.0],
        "shop": ["north", "south", "north", "south", "north", "south"],
    }
)
CORNERS = ["xmin", "xmax", "ymin", "ymax"]


def corners(plot: gg.PlotSpec) -> list[list[float]]:
    frame = plot.layer_data(0)
    return [[round(value, 12) for value in row] for row in frame.select(CORNERS).rows()]


def test_col_equals_bar_identity() -> None:
    col = gg.ggplot(FRUIT, gg.aes("fruit", "sold", fill="shop")) + gg.geom_col()
    bar = gg.ggplot(FRUIT, gg.aes("fruit", "sold", fill="shop")) + gg.geom_bar(stat="identity")
    assert corners(col) == corners(bar)
    assert col.to_svg() == bar.to_svg()
    assert col.as_dict()["layers"][0]["geom"] == "col"


def test_col_stacks_positive_and_negative_apart() -> None:
    plot = gg.ggplot(FRUIT, gg.aes("fruit", "sold", fill="shop")) + gg.geom_col()
    spans = corners(plot)
    # Same x spans as ggplot2 (width 0.9 of resolution 1).
    assert [row[:2] for row in spans] == [[-0.45, 0.45]] * 2 + [[0.55, 1.45]] * 2 + [
        [1.55, 2.45]
    ] * 2
    # Negative bars hang from 0, positives stack: apple 0..3 then 3..5.
    heights = sorted((min(row[2:]), max(row[2:])) for row in spans)
    assert heights == [(-2.0, 0.0), (-1.0, 0.0), (0.0, 3.0), (0.0, 4.0), (0.0, 5.0), (3.0, 5.0)]


def test_col_just_and_width_match_ggplot2() -> None:
    data = {"x": [1.0, 2.0, 4.0], "y": [1.0, 2.0, 3.0]}
    plot = gg.ggplot(data, gg.aes("x", "y")) + gg.geom_col(just=1, width=0.5)
    # ggplot2: xmin 0.5 1.5 3.5, xmax 1 2 4 (resolution 1 from the x gaps).
    assert corners(plot) == [[0.5, 1.0, 0.0, 1.0], [1.5, 2.0, 0.0, 2.0], [3.5, 4.0, 0.0, 3.0]]
    left = gg.ggplot(data, gg.aes("x", "y")) + gg.geom_col(just=0)
    assert [row[:2] for row in corners(left)] == [[1.0, 1.9], [2.0, 2.9], [4.0, 4.9]]


def test_col_orientation_from_discrete_y() -> None:
    plot = gg.ggplot(FRUIT, gg.aes("sold", "fruit", fill="shop")) + gg.geom_col()
    frame = plot.layer_data(0)
    # Horizontal bars: y spans 0.9 around each fruit, x from 0 to sold.
    assert frame["ymin"].round(12).to_list()[:2] == [-0.45, -0.45]
    assert frame["xmax"].to_list()[0] == 3.0
    explicit = gg.ggplot(FRUIT, gg.aes("sold", "fruit", fill="shop")) + gg.geom_col(orientation="y")
    assert explicit.layer_data(0).equals(frame)


def test_col_dodge() -> None:
    plot = gg.ggplot(FRUIT, gg.aes("fruit", "sold", fill="shop")) + gg.geom_col(position="dodge")
    # ggplot2: 0.55-1.00 and 1.00-1.45 around x = 1 (here x = 0).
    assert [row[:2] for row in corners(plot)][:2] == [[-0.45, 0.0], [0.0, 0.45]]


def test_col_needs_y() -> None:
    plot = gg.ggplot(FRUIT, gg.aes("fruit")) + gg.geom_col()
    with pytest.raises(SpecError, match="y"):
        plot.to_svg()


def test_col_rejects_bad_just() -> None:
    plot = gg.ggplot(FRUIT, gg.aes("fruit", "sold")) + gg.geom_col(just="left")
    with pytest.raises(SpecError, match="just"):
        plot.to_svg()


def test_gghighlight_greys_col_like_bar() -> None:
    data = {"k": ["a", "b", "c", "d"], "v": [1.0, 5.0, 3.0, 7.0]}
    col = gg.ggplot(data, gg.aes("k", "v", fill="k")) + gg.geom_col()
    bar = gg.ggplot(data, gg.aes("k", "v", fill="k")) + gg.geom_bar(stat="identity")
    highlight = gg.gghighlight(pl.col("v") > 4)
    assert (col + highlight).to_svg() == (bar + highlight).to_svg()
