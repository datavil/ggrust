"""geom_abline against lets-plot.

Both SVGs are reduced to the abline strokes (width 1.65 px, size 0.75). Their ends
are compared as fractions of the panel, which the first line spans corner to
corner, so different panel sizes (real fonts) do not matter.
"""

import re
import warnings
from types import ModuleType

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

Line = tuple[str, tuple[float, float, float, float]]


def _rgb(color: str) -> str:
    """Normalise `#rrggbb` and `rgb(r,g,b)` to `r,g,b`."""
    if color.startswith("#"):
        return ",".join(str(int(color[i : i + 2], 16)) for i in (1, 3, 5))
    return color.removeprefix("rgb(").removesuffix(")").replace(" ", "")


def _lines(svg: str, width: str) -> list[Line]:
    """(stroke, (x1, y1, x2, y2)) of the long strokes of this width, ends sorted by x."""
    found = []
    for tag in re.findall(r"<(?:path|line)\b[^>]*>", svg):
        attributes = dict(re.findall(r'([\w-]+)="([^"]*)"', tag))
        if not attributes.get("stroke-width", "").startswith(width):
            continue
        if "d" in attributes:
            numbers = [float(n) for n in re.findall(r"-?[\d.]+", attributes["d"])]
        else:
            numbers = [float(attributes[k]) for k in ("x1", "y1", "x2", "y2")]
        x1, y1, x2, y2 = numbers
        if abs(x2 - x1) + abs(y2 - y1) < 30:  # legend keys
            continue
        if x2 < x1:
            x1, y1, x2, y2 = x2, y2, x1, y1
        found.append((_rgb(attributes["stroke"]), (x1, y1, x2, y2)))
    return found


def _relative(lines: list[Line]) -> list[Line]:
    """Ends as fractions of the first line's box (the panel)."""
    x1, y1, x2, y2 = lines[0][1]
    left, right, top, bottom = min(x1, x2), max(x1, x2), min(y1, y2), max(y1, y2)

    def fraction(point: tuple[float, float, float, float]) -> tuple[float, float, float, float]:
        a, b, c, d = point
        width, height = right - left, bottom - top
        return ((a - left) / width, (b - top) / height, (c - left) / width, (d - top) / height)

    return [(color, fraction(point)) for color, point in lines]


def _mapped(module: ModuleType) -> object:
    frame = {"x": [0, 4], "y": [0, 4], "a": [0, 1], "b": [1, -1], "g": ["p", "q"]}
    return (
        module.ggplot(frame, module.aes("x", "y"))
        + module.geom_point()
        + module.geom_abline(module.aes(intercept="a", slope="b", color="g"))
    )


def test_mapped_lines_match_lets_plot() -> None:
    ours = _relative(_lines(_mapped(gg).to_svg(), "1.65"))  # type: ignore[attr-defined]
    theirs = _relative(_lines(_mapped(lp).to_svg(), "1.65"))  # type: ignore[attr-defined]
    assert len(ours) == len(theirs) == 2
    for (color, ends), (their_color, their_ends) in zip(ours, theirs, strict=True):
        assert color == their_color
        assert ends == pytest.approx(their_ends, abs=0.01)


def test_constants_draw_one_line_over_plot_data() -> None:
    frame = {"x": [0, 1, 2, 3, 4], "y": [1, 3, 2, 5, 4]}
    plot = (
        gg.ggplot(frame, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.geom_abline(intercept=1, slope=0.5, color="red", size=2, linetype="dashed")
    )
    lines = _lines(plot.to_svg(), "4.4")
    assert [color for color, _ in lines] == ["255,0,0"]


def test_default_is_identity_line_and_axes_ignore_it() -> None:
    frame = {"x": [0, 1], "y": [0, 1]}
    plain = gg.ggplot(frame, gg.aes("x", "y")) + gg.geom_point()
    svg = (plain + gg.geom_abline()).to_svg()
    ((_, (x1, y1, x2, y2)),) = _lines(svg, "1.65")
    # y = x on a square data range runs corner to corner.
    assert x2 > x1 and y2 < y1
    # Far-away lines do not widen the axes and are not drawn.
    far = (plain + gg.geom_abline(intercept=100, slope=0)).to_svg()
    assert _lines(far, "1.65") == []
    assert far.count("<text") == plain.to_svg().count("<text")
