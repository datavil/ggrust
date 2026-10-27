"""Boxplots must match lets-plot: boxes, midlines, whiskers, outliers, axis text.

Shapes are compared as fractions of the panel, in drawing order. Groups have an
odd number of values so the quartiles fall on order statistics or halfway
between them, where lets-plot's quantile and ggplot2's (type 7, used by ggrust)
agree; `test_quartiles_follow_ggplot2` pins the deliberate difference.
"""

import re
import warnings

import pytest
from test_bar_parity import _attributes, _hex, _texts

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

DATA = {
    "g": ["a"] * 7 + ["b"] * 5 + ["c"] * 9,
    "h": list("uvuvuvu") + list("uuvvv") + list("uvuvuvuvu"),
    "y": [1.0, 2, 3, 4, 5, 6, 20, 2.0, 3, 3, 4, 5, -4.0, 1, 2, 2, 3, 3, 4, 9, 10],
}
# Every (g, h) group has an odd size too, for the dodged cases.
DODGE = {
    "g": ["a"] * 8 + ["b"] * 8,
    "h": list("uuuvvvvv") + list("uuuuuvvv"),
    "y": [1.0, 4, 6, 2, 3, 3, 5, 12, 0.0, 2, 3, 3, 8, 4, 5, 7],
}
ELEMENT = re.compile(r"<(rect|line|path|circle)\b([^>]*)>")


def _shape(kind: str, a: dict, left: float, top: float, width: float, height: float):
    """(kind, coordinates as panel fractions, paint) of one element."""

    def fx(value):
        return (float(value) - left) / width

    def fy(value):
        return (float(value) - top) / height

    if kind == "rect":
        coordinates = (
            fx(a["x"]),
            fy(a["y"]),
            float(a["width"]) / width,
            float(a["height"]) / height,
        )
        paint = (
            _hex(a.get("fill")),
            _hex(a.get("stroke")),
            round(float(a.get("stroke-width", 0)), 3),
        )
    elif kind == "circle":
        coordinates = (fx(a["cx"]), fy(a["cy"]), float(a["r"]))
        paint = (_hex(a.get("fill")),)
    else:  # a two-point line: <line> in lets-plot, <path d="M.. L.."> in ggrust
        if kind == "line":
            points = [a["x1"], a["y1"], a["x2"], a["y2"]]
        else:
            points = re.findall(r"-?\d+(?:\.\d+)?", a["d"])
        coordinates = (fx(points[0]), fy(points[1]), fx(points[2]), fy(points[3]))
        paint = (_hex(a.get("stroke")), round(float(a.get("stroke-width", 0)), 3))
        kind = "line"
    return kind, coordinates, paint


def _lets_plot_shapes(svg: str) -> list[tuple]:
    clip = re.search(r'<rect x="[\d.]+" y="0\.0" width="([\d.]+)" height="([\d.]+)">', svg)
    width, height = float(clip.group(1)), float(clip.group(2))
    body = svg[svg.index('clip-path="url(') :]
    body = body[: body.index("clip-bounds-jfx", 30)] if body.count("clip-bounds-jfx") > 1 else body
    shapes = []
    for kind, attributes in ELEMENT.findall(body):
        a = _attributes(attributes)
        if kind == "rect" and "stroke-width" not in a:
            break  # the axes start
        if kind == "path":
            continue
        shapes.append(_shape(kind, a, 0.0, 0.0, width, height))
    return shapes


def _ggrust_shapes(svg: str) -> list[tuple]:
    clip = _attributes(re.search(r"<clipPath[^>]*>\s*<rect([^>]*)/>", svg).group(1))
    left, top = float(clip["x"]), float(clip["y"])
    width, height = float(clip["width"]), float(clip["height"])
    panel = svg[svg.index('<g class="panel"') :]
    panel = panel[: panel.index('<g class="axis-x"')]
    shapes, groups = [], []
    for token in re.finditer(r"<(/?)(g|rect|path|circle)\b([^>]*?)/?>", panel):
        closing, kind, attributes = token.groups()
        if kind == "g":
            if closing:
                groups.pop()
            else:
                groups.append(_attributes(attributes))
            continue
        a = {}
        for group in groups:
            a.update(group)
        a.update(_attributes(attributes))
        shapes.append(_shape(kind, a, left, top, width, height))
    return shapes


def _both(build):
    return [(build(m) + m.theme(legend_position="none")).to_svg() for m in (lp, gg)]


CASES = {
    "basic": lambda m: m.ggplot(DATA, m.aes("g", "y")) + m.geom_boxplot(),
    "dodged": lambda m: m.ggplot(DODGE, m.aes("g", "y", fill="h")) + m.geom_boxplot(),
    "styled": lambda m: (
        m.ggplot(DATA, m.aes("g", "y", color="g"))
        + m.geom_boxplot(alpha=0.4, fill="#ffcc00", size=1.2, width=0.5, fatten=3, whisker_width=1)
    ),
    "outlier_style": lambda m: (
        m.ggplot(DATA, m.aes("g", "y"))
        + m.geom_boxplot(outlier_color="red", outlier_size=1.5, outlier_shape=21)
    ),
    "varwidth": lambda m: m.ggplot(DATA, m.aes("g", "y")) + m.geom_boxplot(varwidth=True),
    "numeric_x": lambda m: (
        m.ggplot({"x": [1.0] * 5 + [3.0] * 5, "y": [1.0, 2, 3, 4, 9] * 2}, m.aes("x", "y"))
        + m.geom_boxplot()
    ),
    "no_x": lambda m: m.ggplot(DATA, m.aes(y="y")) + m.geom_boxplot(),
    "log_y": lambda m: (
        m.ggplot({"g": ["a"] * 5, "y": [1.0, 10, 100, 1000, 1e5]}, m.aes("g", "y"))
        + m.geom_boxplot()
        + m.scale_y_log10()
    ),
    "dodge_position": lambda m: (
        m.ggplot(DODGE, m.aes("g", "y", color="h"))
        + m.geom_boxplot(position=m.position_dodge(width=0.6), width=0.5)
    ),
}


@pytest.mark.parametrize("name", sorted(CASES))
def test_boxplots_match_lets_plot(name):
    expected_svg, actual_svg = _both(CASES[name])
    expected, actual = _lets_plot_shapes(expected_svg), _ggrust_shapes(actual_svg)
    assert [shape[0] for shape in actual] == [shape[0] for shape in expected], (actual, expected)
    for got, want in zip(actual, expected, strict=True):
        assert got[1] == pytest.approx(want[1], abs=1e-3), (got, want)
        assert got[2] == want[2], (got, want)
    assert sorted(_texts(actual_svg)) == sorted(_texts(expected_svg))


def test_quartiles_follow_ggplot2():
    """Deliberate difference: lets-plot puts Q1 of 1..6 at 2.5, ggplot2 (and ggrust) at 2.25."""
    svg = (gg.ggplot({"y": [1.0, 2, 3, 4, 5, 6]}, gg.aes(y="y")) + gg.geom_boxplot()).to_svg()
    box = _ggrust_shapes(svg)[0]
    # The panel spans 1..6 padded by 5%: lower hinge 2.25, upper 4.75.
    low, high = 1 - 0.25, 6 + 0.25
    bottom = 1 - (2.25 - low) / (high - low)
    assert box[1][1] + box[1][3] == pytest.approx(bottom, abs=1e-3)


def test_coef_keeps_the_outliers():
    """Deliberate difference: lets_plot loses points beyond whiskers shortened by `coef`."""
    data = {"g": ["a"] * 7, "y": [1.0, 2, 3, 4, 5, 6, 9]}
    svg = (gg.ggplot(data, gg.aes("g", "y")) + gg.geom_boxplot(coef=0.5)).to_svg()
    assert [shape[0] for shape in _ggrust_shapes(svg)].count("circle") == 1
