"""geom_step and geom_freqpoly against lets-plot.

Each SVG is reduced to the layer's line paths as a sequence of moves: ``h`` along x,
``v`` along y, ``d`` diagonal. Pixel positions differ by a few px (real fonts move
the panel), so the shape is compared, plus the axis labels.
"""

import itertools
import re
import warnings
from collections.abc import Callable
from types import ModuleType

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.errors import SpecError  # noqa: E402

DATA = {
    "x": [1, 2, 3, 4, 5, 1, 2, 3, 4, 5],
    "y": [1.0, 3.0, 2.0, 4.0, 3.0, 2.0, 1.0, 2.0, 0.5, 1.0],
    "g": ["a"] * 5 + ["b"] * 5,
}
HEIGHTS = {
    "v": [158.0, 161, 163, 166, 167, 169, 170, 171, 172, 172.5, 174, 176, 179, 183, 190],
    "s": ["m", "f", "m"] * 5,
}


def _points(d: str) -> list[tuple[float, float]]:
    numbers = [float(n) for n in re.findall(r"-?\d+(?:\.\d+)?(?:e-?\d+)?", d)]
    points = list(zip(numbers[::2], numbers[1::2], strict=True))
    # lets-plot repeats each piece's first vertex.
    deduped = [points[0]] if points else []
    for point in points[1:]:
        if point != deduped[-1]:
            deduped.append(point)
    return deduped


def _moves(points: list[tuple[float, float]]) -> str:
    moves = []
    for (x0, y0), (x1, y1) in itertools.pairwise(points):
        if abs(y1 - y0) < 0.01:
            moves.append("h")
        elif abs(x1 - x0) < 0.01:
            moves.append("v")
        else:
            moves.append("d")
    return "".join(moves)


def _line_shapes(svg: str) -> list[str]:
    """Moves of every stroked, unfilled path wider than the 1 px axis lines."""
    shapes = []
    for tag in re.findall(r"<path\b[^>]*>", svg):
        attributes = dict(re.findall(r'([\w-]+)="([^"]*)"', tag))
        if attributes.get("fill", "none") != "none" or "pointer-events" in attributes:
            continue
        if float(attributes.get("stroke-width", 1)) <= 1.0 or not attributes.get("d"):
            continue
        shapes.append(_moves(_points(attributes["d"])))
    return sorted(shapes)


def _labels(svg: str) -> list[str]:
    return sorted(label.strip() for label in re.findall(r">([^<>]+)</t", svg) if label.strip())


def _svg(module: ModuleType, build: Callable[[ModuleType], object]) -> str:
    plot = build(module) + module.theme(legend_position="none")
    return plot.to_svg()


CASES: dict[str, Callable[[ModuleType], object]] = {
    "step_hv": lambda m: m.ggplot(DATA, m.aes("x", "y", color="g")) + m.geom_step(),
    "step_vh": lambda m: m.ggplot(DATA, m.aes("x", "y", color="g")) + m.geom_step(direction="vh"),
    "step_unsorted": lambda m: (
        m.ggplot({"x": [3, 1, 2], "y": [1, 2, 3]}, m.aes("x", "y")) + m.geom_step()
    ),
    "freqpoly": lambda m: m.ggplot(HEIGHTS, m.aes("v", color="s")) + m.geom_freqpoly(bins=6),
    "freqpoly_binwidth": lambda m: m.ggplot(HEIGHTS, m.aes("v")) + m.geom_freqpoly(binwidth=5),
}


@pytest.mark.parametrize("name", list(CASES))
def test_matches_lets_plot(name: str) -> None:
    ours = _svg(gg, CASES[name])
    theirs = _svg(lp, CASES[name])
    assert _line_shapes(ours) == _line_shapes(theirs)
    assert _labels(ours) == _labels(theirs)


def test_step_mid_as_ggplot2() -> None:
    plot = gg.ggplot({"x": [0, 2, 3], "y": [1, 3, 0]}, gg.aes("x", "y")) + gg.geom_step(
        direction="mid"
    )
    shapes = _line_shapes(plot.to_svg())
    # (0,1) (1,1) (1,3) (2.5,3) (2.5,0) (3,0): ggplot2 stairstep(direction = "mid").
    assert shapes == ["hvhvh"]


def test_step_padded_reaches_panel_edges() -> None:
    data = {"x": [float("-inf"), 1, 2, float("inf")], "y": [0, 1, 2, 3]}
    plot = gg.ggplot(data, gg.aes("x", "y")) + gg.geom_step(padded=True)
    shapes = _line_shapes(plot.to_svg())
    assert shapes == ["hvhvhv"]
    unpadded = gg.ggplot(data, gg.aes("x", "y")) + gg.geom_step()
    assert _line_shapes(unpadded.to_svg()) == ["hv"]


def test_unknown_direction_suggests() -> None:
    plot = gg.ggplot(DATA, gg.aes("x", "y")) + gg.geom_step(direction="vhh")
    with pytest.raises(SpecError, match=r"layers\[0\]\.direction.*'hv', 'vh' or 'mid'.*'vh'"):
        plot.to_svg()


def test_spec_matches_lets_plot() -> None:
    for build in CASES.values():
        ours = build(gg).as_dict()
        theirs = build(lp).as_dict()
        assert ours["layers"][0]["geom"] == theirs["layers"][0]["geom"]


def test_padded_infinite_rows_dropped_by_limits_are_reported() -> None:
    data = {"x": [float("-inf"), 1, 2, float("inf")], "y": [0, 1, 2, 3]}
    plot = gg.ggplot(data, gg.aes("x", "y")) + gg.geom_step(padded=True) + gg.xlim(0, 10)
    with pytest.warns(UserWarning, match="removed 2 rows"):
        plot.to_svg()


def test_step_arrow_is_supported() -> None:
    plot = gg.ggplot(DATA, gg.aes("x", "y")) + gg.geom_step(arrow=gg.arrow())
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        plot.to_svg()
