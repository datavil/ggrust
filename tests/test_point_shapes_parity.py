"""Point shapes, fill and stroke must draw lets-plot's markers.

One point per plot; both SVGs are read into the marker's primitives (circle, rect,
path, line) with their paint. Positions are compared relative to the marker's
bounding box (panels differ by a few px: real fonts), sizes and paint exactly.
"""

import re
import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

FRAME = {"x": [1.0], "y": [1.0]}
PRIMITIVE = re.compile(r"<(circle|rect|path|line)\b([^>]*)/?>")
NUMBER = re.compile(r"-?\d+(?:\.\d+)?(?:e-?\d+)?")


def _attributes(tag: str) -> dict:
    return dict(re.findall(r'([\w-]+)="([^"]*)"', tag))


def _hex(color: str | None) -> str | None:
    if color is None or color == "none":
        return None
    match = re.fullmatch(r"rgb\((\d+),(\d+),(\d+)\)", color.replace(" ", ""))
    if match:
        return "#" + "".join(f"{int(part):02x}" for part in match.groups())
    return color.lower()


def _paint(attributes: dict, inherited: dict) -> tuple:
    merged = {**inherited, **attributes}
    fill, stroke = _hex(merged.get("fill")), _hex(merged.get("stroke"))
    fill_opacity = float(merged.get("fill-opacity", 1))
    stroke_opacity = float(merged.get("stroke-opacity", 1))
    width = float(merged.get("stroke-width", 0))
    if fill_opacity == 0:
        fill = None
    if stroke_opacity == 0 or width == 0:
        stroke, width = None, 0.0
    return (
        fill,
        round(fill_opacity * 255) if fill else None,
        stroke,
        round(stroke_opacity * 255) if stroke else None,
        round(width, 2),
    )


def _points(kind: str, attributes: dict) -> list[tuple[float, float]]:
    number = lambda key: float(attributes[key])  # noqa: E731
    if kind == "circle":
        x, y, r = number("cx"), number("cy"), number("r")
        return [(x - r, y - r), (x + r, y + r)]
    if kind == "rect":
        x, y = number("x"), number("y")
        return [(x, y), (x + number("width"), y + number("height"))]
    if kind == "line":
        return [(number("x1"), number("y1")), (number("x2"), number("y2"))]
    values = [float(value) for value in NUMBER.findall(attributes["d"])]
    points = list(zip(values[::2], values[1::2], strict=True))
    # lets-plot repeats the first vertex ("M a L a ...").
    return [point for index, point in enumerate(points) if index == 0 or point != points[0]]


def markers(svg: str, *, ours: bool) -> list:
    """The marker's primitives: (kind, points relative to the marker box, paint)."""
    if ours:
        body = svg[svg.find('<g class="panel"') :]
        body = body[: body.find('<g class="axis-x"')]
        inherited = _attributes(re.search(r"<g\b[^>]*>", body[1:]).group(0))
    else:
        body = svg[svg.find("clip-path") :]
        inherited = {}
    primitives = []
    for match in PRIMITIVE.finditer(body):
        kind, attributes = match.group(1), _attributes(match.group(0))
        # lets-plot writes markers as self-closing "slim" elements; others are not.
        if not ours and not match.group(0).endswith("/>"):
            continue
        if ours and kind == "rect" and "fill" not in attributes and "fill" not in inherited:
            continue  # clip rectangles
        primitives.append((kind, _points(kind, attributes), _paint(attributes, inherited)))
    xs = [x for _, points, _ in primitives for x, _ in points]
    ys = [y for _, points, _ in primitives for _, y in points]
    left, top = min(xs), min(ys)
    return [
        (kind, [(round(x - left, 2), round(y - top, 2)) for x, y in points], paint)
        for kind, points, paint in primitives
    ]


def assert_same(ours: list, theirs: list) -> None:
    """Same primitives and paint; points within 0.025 px (ggrust writes 2 decimals)."""
    assert [(kind, paint) for kind, _, paint in ours] == [
        (kind, paint) for kind, _, paint in theirs
    ]
    for (_, points, _), (_, expected, _) in zip(ours, theirs, strict=True):
        assert len(points) == len(expected), (points, expected)
        for point, other in zip(points, expected, strict=True):
            assert point == pytest.approx(other, abs=0.025), (points, expected)


def plot(module, **parameters):
    return module.ggplot(FRAME, module.aes("x", "y")) + module.geom_point(**parameters)


def both(**parameters):
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        theirs = markers(plot(lp, **parameters).to_svg(), ours=False)
        ours = markers(plot(gg, **parameters).to_svg(), ours=True)
    return ours, theirs


@pytest.mark.parametrize("shape", range(26))
def test_every_shape_matches_lets_plot(shape):
    assert_same(*both(shape=shape))


@pytest.mark.parametrize(
    "parameters",
    [
        {"shape": 21, "fill": "red", "color": "blue", "stroke": 2, "size": 4},
        {"shape": 21, "fill": "red", "alpha": 0.5},
        {"shape": 22, "size": 10, "stroke": 0},
        {"shape": 1, "stroke": 3, "color": "#00ff00"},
        {"shape": 16, "stroke": 5, "alpha": 0.3},
        {"shape": 8, "size": 6, "stroke": 1},
        {"shape": 24, "fill": "yellow", "stroke": 1.5},
    ],
)
def test_fill_stroke_and_alpha_match_lets_plot(parameters):
    assert_same(*both(**parameters))
