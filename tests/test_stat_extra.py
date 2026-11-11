"""``stat_unique``, ``stat_hull`` and ``stat_summary_2d``."""

from __future__ import annotations

import ggrust as gg
import pytest
from ggrust.errors import SpecError


def test_stat_unique_drops_duplicates():
    data = {"city": ["Oslo", "Oslo", "Rome", "Rome", "Rome"], "temp": [4, 4, 18, 18, 20]}
    plot = gg.ggplot(data, gg.aes("city", "temp")) + gg.stat_unique()
    assert plot.layer_data(0).height == 3


def test_stat_hull_closes_each_group_and_reports_the_area():
    data = {
        "x": [0.0, 2.0, 2.0, 0.0, 1.0, 10.0, 12.0, 11.0],
        "y": [0.0, 0.0, 2.0, 2.0, 1.0, 0.0, 0.0, 3.0],
        "g": ["a"] * 5 + ["b"] * 3,
    }
    plot = gg.ggplot(data, gg.aes("x", "y", group="g")) + gg.stat_hull(geom="polygon")
    frame = plot.layer_data(0)
    assert frame.height == 5 + 4
    first = frame.head(5)
    assert first["x"][0] == first["x"][4]
    assert first["area"][0] == pytest.approx(4.0)
    assert frame["area"][5] == pytest.approx(3.0)
    assert "<svg" in plot.to_svg()


def test_stat_summary_2d_matches_ggplot2_means():
    data = {"x": [0.0, 0.1, 1.0, 1.0], "y": [0.0, 0.1, 1.0, 1.0], "z": [1.0, 3.0, 10.0, 20.0]}
    plot = gg.ggplot(data, gg.aes("x", "y", z="z")) + gg.stat_summary_2d(bins=2)
    frame = plot.layer_data(0)
    assert frame["value"].to_list() == [2.0, 15.0]
    assert "<svg" in plot.to_svg()


def test_stat_summary_2d_unknown_fun():
    data = {"x": [0.0], "y": [0.0], "z": [1.0]}
    plot = gg.ggplot(data, gg.aes("x", "y", z="z")) + gg.stat_summary_2d(fun="mena")  # ty: ignore[invalid-argument-type]
    with pytest.raises(SpecError, match="did you mean 'mean'"):
        plot.to_svg()


def test_stat_summary_2d_rejects_a_discrete_z() -> None:
    data = {"x": [1.0, 2, 3, 4], "y": [1.0, 3, 2, 5], "s": list("abab")}
    plot = gg.ggplot(data, gg.aes("x", "y", z="s")) + gg.stat_summary_2d(bins=2)
    with pytest.raises(SpecError, match="numeric z column"):
        plot.to_svg()


def test_stat_unique_treats_negative_zero_as_zero() -> None:
    data = {"x": [0.0, -0.0, 1.0], "y": [1.0, 1.0, 1.0]}
    built = (gg.ggplot(data, gg.aes("x", "y")) + gg.stat_unique()).to_svg()
    assert built.count("<circle") == 2
