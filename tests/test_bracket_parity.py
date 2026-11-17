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
DODGED = {
    "x": ["a", "b", "c", "c"],
    "y": [4.5, 5.0, 5.5, 6.2],
    "start": [0, 0, 1, 0],
    "end": [1, 2, 2, 2],
    "label": ["*", "**", "ns", "***"],
}
DODGE_MAPPING = {"x": "x", "y": "y", "istart": "start", "iend": "end", "label": "label"}


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
    # geom_bracket_dodge: slots inferred (3 groups), or set with ngroup / dodge_width.
    "dodge": lambda m: _boxes(m, m.geom_bracket_dodge(m.aes(**DODGE_MAPPING), data=DODGED)),
    "dodge_ngroup_width": lambda m: _boxes(
        m,
        m.geom_bracket_dodge(
            m.aes(x="x", y="y", label="label"),
            data=DODGED,
            istart=1,
            iend=3,
            ngroup=4,
            dodge_width=0.6,
            bracket_shorten=0.2,
        ),
    ),
    "dodge_numeric": lambda m: (
        m.ggplot({"x": [1.0, 3.0], "y": [0.0, 1.0]}, m.aes("x", "y"))
        + m.geom_point()
        + m.geom_bracket_dodge(
            m.aes(x="at", y="y", istart="s", iend="e", label="t"),
            data={"at": [1.0, 3.0], "y": [1.2, 1.4], "s": [0, 1], "e": [1, 2], "t": ["a", "b"]},
        )
        + m.ggsize(400, 300)
    ),
    "dodge_flipped": lambda m: _boxes(
        m, m.geom_bracket_dodge(m.aes(**DODGE_MAPPING), data=DODGED), flip=True
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


def test_bracket_dodge_errors():
    plot = _boxes(gg, gg.geom_bracket_dodge(gg.aes(**DODGE_MAPPING), data=DODGED, ngroup=0))
    with pytest.raises(Exception, match=r"ngroup=0 is not a group count.*whole number >= 1"):
        plot.to_svg()


def test_bracket_dodge_without_slots_draws_nothing():
    svg = _boxes(gg, gg.geom_bracket_dodge(gg.aes(x="x", y="y"), data=DODGED)).to_svg()
    assert not [shape for shape in _ggrust_shapes(svg) if len(shape[1]) == 4]


def _label_texts(svg: str, axis_labels: set[str]) -> list[str]:
    """Bracket label texts: the visible texts minus the axis labels and titles."""
    return [text for text in _texts(svg) if text not in axis_labels]


@pytest.mark.parametrize(
    ("label", "options"),
    [
        ("p", {"label_format": "p = {.3f}", "na_text": "none"}),
        ("p", {"label_format": ".1%"}),
        ("s", {"label_format": "[{}]", "na_text": "-"}),
        (None, {"label": 0.5, "label_format": "{.2f} kg"}),
    ],
)
def test_bracket_label_format_and_na_text_match_lets_plot(
    label: str | None, options: dict[str, object]
) -> None:
    data = {
        "lo": [0.0, 1.0, 2.0],
        "hi": [1.0, 2.0, 3.0],
        "at": [1.0, 2.0, 3.0],
        "p": [0.012345, None, 0.5],
        "s": ["a", None, "b"],
    }
    mapping = {"xmin": "lo", "xmax": "hi", "y": "at"} | ({"label": label} if label else {})
    rendered = []
    for m in (lp, gg):
        plot = m.ggplot() + m.geom_bracket(m.aes(**mapping), data=data, **options)
        plain = m.ggplot() + m.geom_bracket(m.aes(xmin="lo", xmax="hi", y="at"), data=data)
        rendered.append(_label_texts(plot.to_svg(), set(_texts(plain.to_svg()))))
    lets_plot_labels, ggrust_labels = rendered
    assert len(ggrust_labels) == 3
    assert ggrust_labels == lets_plot_labels


def test_bracket_dodge_label_format_and_na_text_match_lets_plot() -> None:
    data = DODGED | {"p": [0.5, None, 0.25, 0.125]}
    mapping = DODGE_MAPPING | {"label": "p"}
    rendered = []
    for m in (lp, gg):
        layer = m.geom_bracket_dodge(m.aes(**mapping), data=data, label_format=".0%", na_text="-")
        rendered.append(_texts(_boxes(m, layer).to_svg()))
    lets_plot_texts, ggrust_texts = rendered
    for text in ("50%", "-", "25%", "13%"):
        assert text in ggrust_texts
        assert ggrust_texts.count(text) == lets_plot_texts.count(text)


def test_bracket_label_format_is_not_reported_as_ignored() -> None:
    plot = gg.ggplot() + gg.geom_bracket(
        gg.aes(xmin="lo", xmax="hi", y="at", label="at"),
        data=NUMERIC,
        label_format="{.2f} u",
        na_text="-",
    )
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        svg = plot.to_svg()
    assert "3.00 u" in _texts(svg)
    assert "4.50 u" in _texts(svg)
