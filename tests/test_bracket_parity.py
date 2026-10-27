"""Brackets must match lets-plot: bar and tip points, line paint, label text and place.

Bracket paths are compared as fractions of the panel in drawing order (extractors
from the violin parity test). Labels are compared by text, font size, colour and
position (panel fractions, loosely: fonts differ by a fraction of a pixel).
"""

import math
import re
import warnings

import pytest
from test_bar_parity import _attributes, _texts
from test_violin_parity import _ggrust_shapes, _lets_plot_shapes

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

BOXES = {"x": ["a"] * 3 + ["b"] * 3 + ["c"] * 3, "y": [0.0, 1, 2, 1, 2, 3, 2, 3, 4]}
BRACKETS = {
    "xmin": ["a", "a", "b"],
    "xmax": ["b", "c", "c"],
    "y": [4.5, 5.2, 5.9],
    "label": ["*", "**", "ns"],
}
NUMERIC = {"lo": [1.0, 2.5], "hi": [2.0, 4.0], "at": [3.0, 4.5], "text": ["p = 0.01", "x"]}
MAPPING = {"xmin": "xmin", "xmax": "xmax", "y": "y", "label": "label"}


def _boxes(m, *layers, flip=False):
    plot = m.ggplot(BOXES, m.aes("x", "y", color="x")) + m.geom_point() + m.ggsize(400, 300)
    for layer in layers:
        plot += layer
    return plot + m.coord_flip() if flip else plot


CASES = {
    "discrete": lambda m: _boxes(m, m.geom_bracket(m.aes(**MAPPING), data=BRACKETS)),
    "styled": lambda m: _boxes(
        m,
        m.geom_bracket(
            m.aes(**MAPPING),
            data=BRACKETS,
            color="#1f1f1f",
            segment_size=0.5,
            size=9,
            linetype="dashed",
            alpha=0.6,
        ),
    ),
    "segment_paint": lambda m: _boxes(
        m,
        m.geom_bracket(
            m.aes(**MAPPING),
            data=BRACKETS,
            color="red",
            segment_color="blue",
            segment_alpha=0.4,
            segment_size=1.5,
        ),
    ),
    # lets-plot's "unset" segment colour is opaque white: white falls back to `color`.
    "segment_white": lambda m: _boxes(
        m, m.geom_bracket(m.aes(**MAPPING), data=BRACKETS, color="red", segment_color="white")
    ),
    "tips_identity": lambda m: _boxes(
        m,
        m.geom_bracket(
            m.aes(**MAPPING),
            data=BRACKETS,
            lenstart=0.4,
            lenend=-0.2,
            tiplength_unit="identity",
        ),
    ),
    "tips_px_shortened": lambda m: _boxes(
        m,
        m.geom_bracket(
            m.aes(**MAPPING), data=BRACKETS, lenstart=20, tiplength_unit="px", bracket_shorten=0.3
        ),
    ),
    "tips_res": lambda m: _boxes(
        m,
        m.geom_bracket(m.aes(**MAPPING), data=BRACKETS, lenend=0.5, tiplength_unit="res"),
    ),
    "numeric": lambda m: (
        m.ggplot(NUMERIC)
        + m.geom_bracket(m.aes(xmin="lo", xmax="hi", y="at", label="text"), nudge_y=0.2)
        + m.ggsize(400, 300)
    ),
    "numeric_on_discrete": lambda m: _boxes(
        m,
        m.geom_bracket(
            m.aes(xmin="lo", xmax="hi", y="at", label="text"),
            data={"lo": [-0.3], "hi": [1.3], "at": [4.6], "text": ["group"]},
        ),
    ),
    "multiline_angle": lambda m: _boxes(
        m,
        m.geom_bracket(
            m.aes(**MAPPING),
            data={**BRACKETS, "label": ["*\np=0.04", "**", "ns"]},
            angle=20,
        ),
    ),
    "flipped": lambda m: _boxes(m, m.geom_bracket(m.aes(**MAPPING), data=BRACKETS), flip=True),
}


def _rotated(x, y, angle, centre_x, centre_y):
    """SVG `rotate(angle centre)` applied to a point (degrees, clockwise on screen)."""
    cos, sin = math.cos(math.radians(angle)), math.sin(math.radians(angle))
    dx, dy = x - centre_x, y - centre_y
    return centre_x + dx * cos - dy * sin, centre_y + dx * sin + dy * cos


def _lets_plot_labels(svg):
    """(line, font size, rotation, x, y) per label line; anchors as panel fractions."""
    clip = re.search(r'<rect x="[\d.]+" y="0\.0" width="([\d.]+)" height="([\d.]+)">', svg)
    width, height = float(clip.group(1)), float(clip.group(2))
    body = svg[svg.index('clip-path="url(') :]
    body = body[: body.index("<rect")]
    lines = []
    for angle, centre_x, centre_y, x, y, texts in re.findall(
        r'<g transform="rotate\(([-\d.]+) ([-\d.]+) ([-\d.]+) \) ">\s*'
        r'<g transform="translate\(([-\d.]+) ([-\d.]+) \) ">(.*?)</g>',
        body,
        re.S,
    ):
        # One <text> per line, `y` relative to the label's translate.
        for attributes, content in re.findall(r"<text([^>]*)>(.*?)</text>", texts, re.S):
            a = _attributes(attributes)
            size = float(re.search(r"font-size:([\d.]+)px", a["style"]).group(1))
            text = "".join(re.findall(r"<tspan[^>]*>([^<]*)</tspan>", content))
            anchor = _rotated(
                float(x),
                float(y) + float(a.get("y", 0)),
                float(angle),
                float(centre_x),
                float(centre_y),
            )
            lines.append(
                (text, size, round(float(angle) % 360, 1), anchor[0] / width, anchor[1] / height)
            )
    return lines


def _ggrust_labels(svg):
    """(line, font size, rotation, x, y) per label line; anchors as panel fractions."""
    clip = _attributes(re.search(r"<clipPath[^>]*>\s*<rect([^>]*)/>", svg).group(1))
    left, top = float(clip["x"]), float(clip["y"])
    width, height = float(clip["width"]), float(clip["height"])
    panel = svg[svg.index('<g class="panel"') :]
    panel = panel[: panel.index('<g class="axis-x"')]
    lines = []
    for attributes, content in re.findall(r"<text\b([^>]*)>([^<]*)</text>", panel):
        a = _attributes(attributes)
        x, y = float(a["x"]), float(a["y"])
        rotation = re.search(r"rotate\(([-\d.]+) ([-\d.]+) ([-\d.]+)\)", a.get("transform", ""))
        angle = 0.0
        if rotation:
            angle, centre_x, centre_y = map(float, rotation.groups())
            x, y = _rotated(x, y, angle, centre_x, centre_y)
        lines.append(
            (
                content,
                float(a["font-size"]),
                round(angle % 360, 1),
                (x - left) / width,
                (y - top) / height,
            )
        )
    return lines


def _render(build):
    return [(build(m) + m.theme(legend_position="none")).to_svg() for m in (lp, gg)]


@pytest.mark.parametrize("name", sorted(CASES))
def test_brackets_match_lets_plot(name):
    expected_svg, actual_svg = _render(CASES[name])
    expected = [shape for shape in _lets_plot_shapes(expected_svg) if len(shape[1]) == 4]
    actual = [shape for shape in _ggrust_shapes(actual_svg) if len(shape[1]) == 4]
    assert actual, "ggrust drew no bracket"
    assert len(actual) == len(expected)
    for got, want in zip(actual, expected, strict=True):
        assert got[2] == want[2], (got[2], want[2])
        assert [value for point in got[1] for value in point] == pytest.approx(
            [value for point in want[1] for value in point], abs=2e-3
        )
    assert sorted(_texts(actual_svg)) == sorted(_texts(expected_svg))


@pytest.mark.parametrize("name", sorted(CASES))
def test_bracket_labels_match_lets_plot(name):
    expected_svg, actual_svg = _render(CASES[name])
    expected, actual = _lets_plot_labels(expected_svg), _ggrust_labels(actual_svg)
    assert [label[:3] for label in actual] == [label[:3] for label in expected]
    for got, want in zip(actual, expected, strict=True):
        assert got[3:] == pytest.approx(want[3:], abs=0.012), got[0]


def test_bracket_does_not_inherit_the_plot_mapping():
    """lets-plot's bracket defaults: `inherit_aes=False`, no legend."""
    svg = _boxes(gg, gg.geom_bracket(gg.aes(**MAPPING), data=BRACKETS)).to_svg()
    strokes = {
        _attributes(path).get("stroke")
        for path in re.findall(r"<path\b([^>]*)>", svg)
        if len(re.findall(r"[ML]", _attributes(path).get("d", ""))) == 4
    }
    assert strokes == {"#474747"}
    # One legend (the points' colour), with no key for the brackets.
    assert svg.count('class="legend"') <= 1


def test_vertical_brackets_are_reported():
    plot = gg.ggplot(BRACKETS) + gg.geom_bracket(gg.aes(**MAPPING), data=BRACKETS, orientation="y")
    with pytest.raises(Exception, match="orientation 'y' is not supported by ggrust yet"):
        plot.to_svg()
