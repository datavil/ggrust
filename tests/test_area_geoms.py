"""geom_area and geom_ribbon against lets-plot.

Both SVGs are reduced to the layer's paths, in drawing order: fill, stroke,
widths and vertex counts. Positions differ by a few px (real fonts move the
panel), so coordinates are compared through the y axis labels instead.
"""

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
    "x": [1, 2, 3, 4, 1, 2, 3, 4],
    "y": [1.0, 3.0, 2.0, 2.5, 2.0, 1.0, 2.0, 0.5],
    "low": [0.0, 1.0, 1.0, 0.5, 1.0, 0.0, 0.0, -0.5],
    "high": [2.0, 4.0, 3.0, 3.5, 3.0, 2.0, 3.0, 1.5],
    "g": ["a", "a", "a", "a", "b", "b", "b", "b"],
}


def _attributes(tag: str) -> dict[str, str]:
    return dict(re.findall(r'([\w-]+)="([^"]*)"', tag))


def _rgb(color: str) -> str:
    """Normalise `#rrggbb` and `rgb(r,g,b)` to `r,g,b`."""
    if color.startswith("#"):
        return ",".join(str(int(color[i : i + 2], 16)) for i in (1, 3, 5))
    return color.removeprefix("rgb(").removesuffix(")").replace(" ", "")


def _paths(svg: str) -> list[tuple[str, float, str, float, int]]:
    """(fill, fill opacity, stroke, stroke width, vertices) of the layer paths."""
    found = []
    for tag in re.findall(r"<path\b[^>]*>", svg):
        attributes = _attributes(tag)
        d = attributes.get("d", "")
        if "pointer-events" in attributes or "fill-rule" in attributes or not d:
            continue
        vertices = len(re.findall(r"[ML]", d))
        # lets-plot repeats each piece's first vertex (M p L p ...).
        numbers = re.findall(r"-?\d+(?:\.\d+)?", d)
        if len(numbers) >= 4 and numbers[:2] == numbers[2:4]:
            vertices -= 1
        fill = attributes.get("fill", "none")
        stroke = attributes.get("stroke", "none")
        width = float(attributes.get("stroke-width", 0)) if stroke != "none" else 0.0
        if fill == "none" and width == 1.0:
            continue  # axis lines
        found.append(
            (
                "none" if fill == "none" else _rgb(fill),
                round(float(attributes.get("fill-opacity", 1)), 2),
                "none" if stroke == "none" else _rgb(stroke),
                round(width, 2),
                vertices,
            )
        )
    return found


def _labels(svg: str) -> list[str]:
    """Axis labels and titles, sorted (the two libraries order titles differently)."""
    return sorted(label.strip() for label in re.findall(r">([^<>]+)</t", svg) if label.strip())


def _svg(module: ModuleType, layer: Callable[[ModuleType], object]) -> str:
    plot = module.ggplot(DATA) + layer(module) + module.theme(legend_position="none")
    return plot.to_svg()


CASES: dict[str, Callable[[ModuleType], object]] = {
    "area": lambda m: m.geom_area(m.aes("x", "y")),
    "area stacked": lambda m: m.geom_area(m.aes("x", "y", fill="g")),
    "area identity": lambda m: m.geom_area(m.aes("x", "y", group="g"), position="identity"),
    "area styled": lambda m: m.geom_area(m.aes("x", "y"), alpha=0.5, color="red", size=2),
    "area fill position": lambda m: m.geom_area(m.aes("x", "y", fill="g"), position="fill"),
    "ribbon": lambda m: m.geom_ribbon(m.aes("x", ymin="low", ymax="high")),
    "ribbon groups": lambda m: m.geom_ribbon(m.aes("x", ymin="low", ymax="high", fill="g")),
}


@pytest.mark.parametrize("name", CASES)
def test_paths_match_lets_plot(name: str) -> None:
    ours, theirs = (_svg(module, CASES[name]) for module in (gg, lp))
    assert _paths(ours) == _paths(theirs)
    assert _labels(ours) == _labels(theirs)


@pytest.mark.filterwarnings("ignore:.*removed 1 rows")
def test_area_splits_at_missing_values() -> None:
    data = {"x": [1, 2, 3, 4, 5], "y": [1.0, 2.0, None, 2.0, 1.0]}
    for module in (gg, lp):
        svg = (module.ggplot(data, module.aes("x", "y")) + module.geom_area()).to_svg()
        assert [path[4] for path in _paths(svg)] == [4, 4, 2, 2], module.__name__


def test_ribbon_needs_its_bounds() -> None:
    with pytest.raises(SpecError, match="ymax"):
        (gg.ggplot(DATA) + gg.geom_ribbon(gg.aes("x", ymin="low"))).to_svg()


def test_edges_split_where_the_color_changes_as_lines() -> None:
    """Area edges follow ggrust's geom_line styling (lets-plot splits both at midpoints)."""
    data = {"x": [1, 2, 3], "y": [1.0, 2.0, 3.0], "c": [1.0, 2.0, 3.0]}
    strokes = []
    for geom in (gg.geom_area, gg.geom_line):
        plot = gg.ggplot(data) + geom(gg.aes("x", "y", color="c"))
        strokes.append([path[2] for path in _paths(plot.to_svg()) if path[2] != "none"])
    assert strokes[0] == strokes[1]
    assert len(set(strokes[0])) == 2
