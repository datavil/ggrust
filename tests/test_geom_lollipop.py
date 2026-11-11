"""geom_lollipop: sticks from a baseline, as lets-plot."""

from __future__ import annotations

import re

import pytest
from ggrust import aes, geom_lollipop, ggplot, ggsize
from ggrust.errors import SpecError

DATA = {"x": [1.0, 2.0, 3.0], "y": [2.0, 5.0, 1.0]}


def _plot(**options: object) -> str:
    plot = ggplot(DATA, aes("x", "y")) + geom_lollipop(**options) + ggsize(400, 300)
    return plot.to_svg()


def test_each_row_draws_a_marker_and_a_stick() -> None:
    svg = _plot()
    assert svg.count("<circle") >= 3


def test_baseline_at_zero_pins_the_axis() -> None:
    plot = ggplot(DATA, aes("x", "y")) + geom_lollipop()
    limits = plot.scale_info("y")[0]["domain"]
    assert limits[0] == 0


def test_baseline_trains_the_axis() -> None:
    plot = ggplot(DATA, aes("x", "y")) + geom_lollipop(intercept=-4)
    assert plot.scale_info("y")[0]["domain"][0] < -4


def test_horizontal_sticks_do_not_pin_y_to_zero() -> None:
    plot = ggplot(DATA, aes("x", "y")) + geom_lollipop(dir="h", intercept=2)
    assert plot.scale_info("y")[0]["domain"][0] > 0


def test_discrete_y_runs_horizontally() -> None:
    frame = {"k": ["a", "b"], "v": [3.0, 5.0]}
    plot = ggplot(frame, aes("v", "k")) + geom_lollipop()
    assert plot.scale_info("x")[0]["domain"][0] == 0


def test_a_short_stick_is_hidden_by_its_marker() -> None:
    frame = {"x": [1.0, 2.0], "y": [0.001, 5.0]}
    svg = (ggplot(frame, aes("x", "y")) + geom_lollipop(linetype="dashed")).to_svg()
    assert len(re.findall(r"stroke-dasharray", svg)) == 1


def test_bad_dir_names_the_parameter() -> None:
    with pytest.raises(SpecError, match=r"dir.*'v', 'h', 's'.*'z'"):
        _plot(dir="z")
