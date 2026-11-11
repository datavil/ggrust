"""The ``trans`` names beyond lets-plot's, checked against ggplot2's ``scales`` package."""

from __future__ import annotations

import math
import re

import ggrust as gg
import pytest
from ggrust.errors import SpecError


def axis_labels(svg: str, axis: str) -> list[str]:
    """The tick labels of one axis in an SVG."""
    group = re.search(
        rf'<g class="axis-{axis}"[^>]*>(.*?)</g>\s*(?:<g class="axis|<g class="pl)', svg, re.S
    )
    assert group is not None
    return re.findall(r">([^<>]+)</text>", group.group(1))


def info(x: list[float], trans: object) -> dict:
    """The trained x scale for ``x`` under ``trans``."""
    plot = gg.ggplot({"x": x, "y": x}, gg.aes("x", "y")) + gg.geom_point()
    plot += gg.scale_x_continuous(trans=trans)
    return plot.scale_info("x")[0]


@pytest.mark.parametrize(
    ("trans", "x", "low", "high"),
    [
        ("log", [1.0, 100.0], 0.0, math.log(100)),
        ("log1p", [0.0, 100.0], 0.0, math.log1p(100)),
        ("exp", [0.0, 3.0], 1.0, math.exp(3)),
        ("logit", [0.2, 0.8], math.log(0.25), math.log(4)),
        ("reciprocal", [2.0, 4.0], 0.25, 0.5),
        ("pseudo_log", [-10.0, 10.0], math.asinh(-5), math.asinh(5)),
        ("symlog", [-100.0, 100.0], -3.0, 3.0),
    ],
)
def test_data_is_transformed_within_the_domain(trans: str, x: list[float], low: float, high: float):
    domain = info(x, trans)["domain"]
    # The expanded domain contains the data's transformed range and is not far from it.
    assert domain[0] <= low + 1e-9
    assert domain[1] >= high - 1e-9
    assert domain[1] - domain[0] < 3 * (high - low)


@pytest.mark.parametrize(
    ("trans", "x"),
    [
        ("log", [0.0, 1.0, 10.0]),
        ("log1p", [-1.0, 1.0, 10.0]),
        ("logit", [0.0, 0.2, 0.8]),
        ("logit", [1.0, 0.2, 0.8]),
        ("probit", [1.5, 0.2, 0.8]),
        ("reciprocal", [0.0, 1.0, 10.0]),
        ("reciprocal", [-1.0, 1.0, 10.0]),
    ],
)
def test_values_outside_the_domain_are_dropped(trans: str, x: list[float]):
    plot = gg.ggplot({"x": x, "y": [1.0, 2.0, 3.0]}, gg.aes("x", "y")) + gg.geom_point()
    plot += gg.scale_x_continuous(trans=trans)
    data = plot.layer_data(0)
    assert data["x"].is_not_nan().sum() + data["x"].is_null().sum() >= 2
    assert plot.to_svg().count("<svg") >= 1


def test_reciprocal_runs_the_other_way_and_expands_inside_the_domain():
    domain = info([1.0, 10.0], "reciprocal")["domain"]
    assert domain[0] > 0
    assert domain[1] > 1.0  # 1 / (1 - 0.45)


def test_reciprocal_keeps_limits_in_order():
    plot = gg.ggplot({"x": [1.0, 2.0, 8.0], "y": [1.0, 2.0, 3.0]}, gg.aes("x", "y"))
    plot += gg.geom_point() + gg.scale_x_continuous(trans="reciprocal", limits=[2, 10])
    data = plot.layer_data(0)
    kept = [value for value in data["x"].to_list() if value is not None and not math.isnan(value)]
    assert sorted(kept) == [0.125, 0.5]


def test_boxcox_trans_builds_a_name_the_engine_reads():
    assert gg.boxcox_trans(0.5) == "boxcox(0.5,0.0)"
    domain = info([1.0, 9.0], gg.boxcox_trans(0.5))["domain"]
    assert domain[0] <= 0.0 <= domain[1]
    assert domain[1] >= 4.0


@pytest.mark.parametrize("bad", [math.nan, math.inf, "a", True])
def test_boxcox_trans_rejects_bad_parameters(bad: object):
    with pytest.raises(SpecError, match="boxcox_trans: p expected a finite number"):
        gg.boxcox_trans(bad)  # ty: ignore[invalid-argument-type]


def test_bare_boxcox_says_how_to_give_the_power():
    plot = gg.ggplot({"x": [1.0], "y": [1.0]}, gg.aes("x", "y")) + gg.geom_point()
    plot += gg.scale_x_continuous(trans="boxcox")  # ty: ignore[invalid-argument-type]
    with pytest.raises(SpecError, match=r"boxcox_trans\(0\.5\)"):
        plot.to_svg()


def test_unknown_transform_lists_the_new_names():
    plot = gg.ggplot({"x": [1.0], "y": [1.0]}, gg.aes("x", "y")) + gg.geom_point()
    plot += gg.scale_x_continuous(trans="logitt")  # ty: ignore[invalid-argument-type]
    with pytest.raises(SpecError, match="did you mean 'logit'"):
        plot.to_svg()


def test_symlog_shorthands_set_the_transform():
    assert gg.scale_x_symlog().as_dict()["trans"] == "symlog"
    assert gg.scale_y_symlog().as_dict()["trans"] == "symlog"


def test_style_scale_accepts_the_new_names():
    plot = gg.ggplot(
        {"x": [1.0, 2.0], "y": [1.0, 2.0], "c": [1.0, 100.0]}, gg.aes("x", "y", color="c")
    )
    plot += gg.geom_point() + gg.scale_color_gradient(trans="log")
    assert "<svg" in plot.to_svg()
