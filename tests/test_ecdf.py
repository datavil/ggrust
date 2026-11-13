"""stat_ecdf against lets-plot.

Layer paths are compared as vertex coordinates relative to their first vertex,
with ``theme_void`` and fixed y limits so both libraries lay out the same panel.
No x limits: lets-plot drops the -inf / +inf padding rows when x has limits (ggrust
draws them at the panel edges, as ggplot2; see deviations.md).
"""

import re
import warnings
from collections.abc import Callable
from types import ModuleType

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

DATA = {
    "x": [0.3, -1.2, 0.8, 0.8, 2.1, -0.4, 1.5, 0.0, 1.1, -0.7],
    "g": ["a", "b"] * 5,
}


def _vertices(svg: str) -> list[list[tuple[float, float]]]:
    """Every layer line or point as coordinates relative to the first one drawn."""
    shapes: list[list[tuple[float, float]]] = []
    for tag in re.findall(r"<(?:path|circle)\b[^>]*>", svg):
        attributes = dict(re.findall(r'([\w-]+)="([^"]*)"', tag))
        if "pointer-events" in attributes or "fill-rule" in attributes:
            continue
        if tag.startswith("<circle"):
            shapes.append([(float(attributes["cx"]), float(attributes["cy"]))])
            continue
        numbers = [float(n) for n in re.findall(r"-?\d+(?:\.\d+)?", attributes.get("d", ""))]
        points = list(zip(numbers[::2], numbers[1::2], strict=True))
        deduped = points[:1]
        for point in points[1:]:
            if point != deduped[-1]:
                deduped.append(point)
        shapes.append(deduped)
    origin_x, origin_y = shapes[0][0]
    return sorted([(x - origin_x, y - origin_y) for x, y in shape] for shape in shapes)


def _svg(module: ModuleType, layer: Callable[[ModuleType], object]) -> str:
    plot = (
        module.ggplot(DATA, module.aes("x"))
        + layer(module)
        + module.theme_void()
        + module.theme(legend_position="none")
        + module.scale_y_continuous(limits=[0, 1])
        + module.ggsize(400, 300)
    )
    return plot.to_svg()


CASES: dict[str, Callable[[ModuleType], object]] = {
    "padded": lambda m: m.stat_ecdf(),
    "unpadded": lambda m: m.stat_ecdf(pad=False),
    "groups": lambda m: m.stat_ecdf(m.aes(color="g")),
    # lets-plot drops points at -inf / +inf (ggrust draws them at the edges).
    "points_n": lambda m: m.stat_ecdf(geom="point", n=7, pad=False),
    "line": lambda m: m.stat_ecdf(geom="line", pad=False),
}


@pytest.mark.parametrize("name", list(CASES))
def test_matches_lets_plot(name: str) -> None:
    ours = _vertices(_svg(gg, CASES[name]))
    theirs = _vertices(_svg(lp, CASES[name]))
    assert [len(shape) for shape in ours] == [len(shape) for shape in theirs]
    for mine, other in zip(ours, theirs, strict=True):
        for point, expected in zip(mine, other, strict=True):
            assert point == pytest.approx(expected, abs=0.15)


def test_padding_reaches_the_panel_edges_under_x_limits() -> None:
    def svg(layer: Callable[[ModuleType], object]) -> str:
        plot = (
            gg.ggplot(DATA, gg.aes("x"))
            + layer(gg)
            + gg.theme_void()
            + gg.scale_x_continuous(limits=[-2, 3])
            + gg.ggsize(400, 300)
        )
        return plot.to_svg()

    # ggplot2: the padded line runs from the left to the right panel edge, up to 1.
    line = _vertices(svg(lambda m: m.stat_ecdf()))[0]
    assert len(line) == 20
    # The panel is 388 px wide (400 minus the 6 px plot margins).
    assert max(x for x, _ in line) - min(x for x, _ in line) == pytest.approx(388, abs=0.5)
    # Padded points: the two padding rows are drawn on the edges, not dropped.
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        points = _vertices(svg(lambda m: m.stat_ecdf(geom="point", n=7)))
    assert len(points) == 9


def test_fractional_and_negative_n_as_lets_plot() -> None:
    for n in (2.5, -2):

        def circles(module: ModuleType, n: float = n) -> int:
            svg = _svg(module, lambda m: m.stat_ecdf(n=n, pad=False, geom="point"))
            return len(re.findall(r"<circle\b", svg))

        assert circles(gg) == circles(lp)


def test_spec_matches_lets_plot() -> None:
    for build in CASES.values():
        ours = build(gg).as_dict()
        theirs = build(lp).as_dict()
        assert ours == {key: theirs[key] for key in ours}
