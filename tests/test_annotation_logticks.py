"""``annotation_logticks()``: log tick marks on the panel edges (ggplot2)."""

from __future__ import annotations

import re

import ggrust as gg
import pytest
from ggrust.errors import SpecError

DATA = {"x": [1.0, 10.0, 100.0], "y": [1.0, 10.0, 100.0]}


def tick_count(plot: gg.PlotSpec, color: str = "#b22222") -> int:
    """Two-point paths stroked in ``color`` (the ticks)."""
    svg = plot.to_svg()
    return len(re.findall(rf'<(?:path|line|polyline)[^>]*stroke="{color}"', svg, re.IGNORECASE))


def base() -> gg.PlotSpec:
    return gg.ggplot(DATA, gg.aes("x", "y")) + gg.geom_point()


def test_ticks_per_decade_on_log_scales():
    plot = base() + gg.scale_x_log10() + gg.scale_y_log10()
    one_side = tick_count(plot + gg.annotation_logticks(sides="b", color="firebrick"))
    both = tick_count(plot + gg.annotation_logticks(color="firebrick"))
    # 1..100 padded: decades 1-10 and 10-100 give 19 ticks inside, and the padding a few more.
    assert one_side >= 19
    assert both == 2 * one_side


def test_unscaled_ticks_follow_coord_trans():
    plot = base() + gg.coord_trans(x="log10", y="log10")
    scaled = plot + gg.annotation_logticks(sides="b", scaled=False, color="firebrick")
    assert tick_count(scaled) >= 19


def test_tick_lengths_and_outside():
    plot = base() + gg.scale_x_log10()
    svg = (plot + gg.annotation_logticks(sides="b", long=20, color="firebrick")).to_svg()
    assert "firebrick" in svg.lower() or "#b22222" in svg.lower()
    outside = plot + gg.annotation_logticks(sides="b", outside=True, color="firebrick")
    assert tick_count(outside) == tick_count(
        plot + gg.annotation_logticks(sides="b", color="firebrick")
    )


def test_bad_sides_are_rejected_early():
    with pytest.raises(SpecError, match=r"sides='bx'.*'t', 'r', 'b', 'l'"):
        gg.annotation_logticks(sides="bx")


def test_bad_base_names_the_parameter():
    plot = base() + gg.scale_x_log10() + gg.annotation_logticks(base=1)
    with pytest.raises(SpecError, match=r"base.*above 1"):
        plot.to_svg()


def test_discrete_axis_gets_no_ticks():
    plot = gg.ggplot({"f": ["a", "b"], "y": [1.0, 100.0]}, gg.aes("f", "y")) + gg.geom_point()
    plot += gg.scale_y_log10() + gg.annotation_logticks(color="firebrick")
    left_only = tick_count(plot)
    assert left_only > 0
    assert left_only == tick_count(
        gg.ggplot({"f": ["a", "b"], "y": [1.0, 100.0]}, gg.aes("f", "y"))
        + gg.geom_point()
        + gg.scale_y_log10()
        + gg.annotation_logticks(sides="l", color="firebrick")
    )
