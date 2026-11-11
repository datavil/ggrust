"""geom_dotplot and geom_ydotplot: stacked dots of binned values, as lets-plot."""

from __future__ import annotations

import re
from collections import Counter

import pytest
from ggrust import (
    aes,
    facet_wrap,
    geom_dotplot,
    geom_point,
    geom_ydotplot,
    ggplot,
    stat_bindot,
    theme,
)
from ggrust.errors import SpecError

DATA = {"x": [1.0, 1.1, 5.0, 5.2, 5.9, 9.0], "g": list("aabbab")}


def _circles(svg: str) -> list[tuple[float, float]]:
    found = re.findall(r'<circle cx="([-\d.]+)" cy="([-\d.]+)" r=', svg)
    return [(float(x), float(y)) for x, y in found]


def _stack_sizes(svg: str, axis: int = 0) -> list[int]:
    counts = Counter(circle[axis] for circle in _circles(svg))
    return [counts[position] for position in sorted(counts)]


def _category_gap(svg: str) -> float:
    found = re.findall(r'<text[^>]*\bx="([-\d.]+)"[^>]*>(?:<tspan[^>]*>)?[ab]<', svg)
    return abs(float(found[1]) - float(found[0]))


def test_dotdensity_groups_values_within_a_bin_of_the_first() -> None:
    # R: layer_data(geom_dotplot(binwidth = 1)) has counts 2, 3, 1.
    plot = ggplot(DATA, aes("x")) + geom_dotplot(binwidth=1)
    assert _stack_sizes(plot.to_svg()) == [2, 3, 1]


def test_dotdensity_centres_train_the_axis() -> None:
    # R: stack centres 1.05, 5.45, 9. The span is one resolution (3.55) wide.
    plot = ggplot(DATA, aes("x")) + geom_dotplot(binwidth=1)
    assert plot.scale_info("x")[0]["limits"] == pytest.approx((1.05 - 1.775, 9.0 + 1.775))


def test_histodot_uses_fixed_bins() -> None:
    plot = ggplot(DATA, aes("x")) + geom_dotplot(method="histodot", bins=2)
    assert sum(_stack_sizes(plot.to_svg())) == len(DATA["x"])
    assert len(_stack_sizes(plot.to_svg())) == 2


def test_stat_bindot_draws_the_same_dots() -> None:
    by_geom = ggplot(DATA, aes("x")) + geom_dotplot(binwidth=1)
    by_stat = ggplot(DATA, aes("x")) + stat_bindot(binwidth=1)
    assert _circles(by_stat.to_svg()) == _circles(by_geom.to_svg())


@pytest.mark.parametrize(
    ("stackdir", "limits"),
    [
        ("up", (0.0, 1.0)),
        ("down", (-1.0, 0.0)),
        ("center", (-0.5, 0.5)),
        ("centerwhole", (-0.5, 0.5)),
    ],
)
def test_the_stacking_direction_sets_the_unpadded_y_axis(
    stackdir: str, limits: tuple[float, float]
) -> None:
    plot = ggplot(DATA, aes("x")) + geom_dotplot(binwidth=1, stackdir=stackdir)
    assert plot.scale_info("y")[0]["limits"] == pytest.approx(limits)


def test_another_layer_trains_y_instead_of_the_stacking() -> None:
    frame = {**DATA, "y": [10.0, 20.0, 30.0, 40.0, 50.0, 60.0]}
    plot = ggplot(frame, aes("x")) + geom_dotplot(binwidth=1) + geom_point(aes(y="y"))
    assert plot.scale_info("y")[0]["limits"] == pytest.approx((10.0, 60.0))


def test_down_stacks_from_the_top() -> None:
    base = ggplot(DATA, aes("x"))
    up = (base + geom_dotplot(binwidth=1)).to_svg()
    down = (base + geom_dotplot(binwidth=1, stackdir="down")).to_svg()
    assert _stack_sizes(down) == _stack_sizes(up)
    up_heights = [y for _, y in _circles(up)]
    down_heights = [y for _, y in _circles(down)]
    assert up_heights[1] < up_heights[0]
    assert down_heights[1] > down_heights[0]


def test_stackgroups_shares_one_stack() -> None:
    layer = geom_dotplot(method="histodot", binwidth=2, stackgroups=True)
    frame = {"x": [1.0, 1.0, 1.0, 9.0], "g": list("abab")}
    plot = ggplot(frame, aes("x", fill="g")) + layer + theme(legend_position="none")
    assert _stack_sizes(plot.to_svg()) == [3, 1]


def test_ydotplot_stacks_along_x_at_each_category() -> None:
    plot = ggplot(DATA, aes("g", "x")) + geom_ydotplot(binwidth=1)
    svg = plot.to_svg()
    assert len(_circles(svg)) == len(DATA["x"])
    # lets-plot's dry run reads the default binwidth of 1: half a unit per side, so
    # the domain is 2.4 units wide against 1.4 for points.
    points = ggplot(DATA, aes("g", "x")) + geom_point()
    assert _category_gap(svg) / _category_gap(points.to_svg()) == pytest.approx(1.4 / 2.4, abs=0.01)


def test_ydotplot_without_x_centres_on_zero() -> None:
    plot = ggplot(DATA, aes(y="x")) + geom_ydotplot(binwidth=1)
    assert plot.scale_info("x")[0]["limits"] == pytest.approx((-0.5, 0.5))


def test_ydotplot_needs_y() -> None:
    with pytest.raises(SpecError, match="requires aesthetics y"):
        (ggplot(DATA, aes("x")) + geom_ydotplot()).to_svg()


def test_bad_stackdir_lists_the_choices() -> None:
    with pytest.raises(SpecError, match=r"layers\[0\]\.stackdir.*'centerwhole'.*'sideways'"):
        (ggplot(DATA, aes("x")) + geom_dotplot(stackdir="sideways")).to_svg()


def test_ydotplot_stackdir_uses_left_and_right() -> None:
    with pytest.raises(SpecError, match=r"layers\[0\]\.stackdir.*'left'"):
        (ggplot(DATA, aes(y="x")) + geom_ydotplot(stackdir="up")).to_svg()


def test_bad_method_lists_the_choices() -> None:
    with pytest.raises(SpecError, match=r"layers\[0\]\.method.*'histodot'"):
        (ggplot(DATA, aes("x")) + geom_dotplot(method="x")).to_svg()


def test_free_y_facets_keep_the_default_empty_axis() -> None:
    # lets-plot's preferred null domain is for shared axes only.
    plot = ggplot(DATA, aes("x")) + geom_dotplot(binwidth=1) + facet_wrap("g", scales="free_y")
    svg = plot.to_svg()
    assert ">0.4<" in svg
    assert ">0.8<" not in svg


def test_a_negative_stackratio_draws_nothing() -> None:
    plot = ggplot(DATA, aes("x")) + geom_dotplot(binwidth=1, stackratio=-1)
    assert _circles(plot.to_svg()) == []
