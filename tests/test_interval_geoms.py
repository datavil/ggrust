"""geom_errorbar, geom_linerange, geom_pointrange and geom_crossbar against lets-plot.

Both SVGs are reduced to the layer's marks. Positions differ by a few px (real
fonts move the panel), so the tests compare stroke widths, point radii and mark
sizes as shares of the panel width.
"""

import re
import warnings
from collections.abc import Callable
from types import ModuleType

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust._render import GgrustWarning  # noqa: E402
from ggrust.errors import SpecError  # noqa: E402

DATA = {
    "x": ["a", "b", "c"],
    "y": [2.0, 3.0, 1.5],
    "low": [1.0, 2.5, 0.5],
    "high": [3.0, 4.0, 2.0],
    "g": ["p", "q", "p"],
}
RANGE = {"x": "x", "ymin": "low", "ymax": "high"}
FULL = {**RANGE, "y": "y"}


def _numbers(text: str) -> list[float]:
    return [float(value) for value in re.findall(r"-?\d+(?:\.\d+)?", text)]


def _attributes(tag: str) -> dict[str, str]:
    return dict(re.findall(r'([\w-]+)="([^"]*)"', tag))


def _panel_width(svg: str) -> float:
    """Width of the panel clip rectangle (a rect with geometry and no paint)."""
    for tag in re.findall(r"<rect\b[^>]*>", svg):
        attributes = _attributes(tag)
        unpainted = not {"fill", "stroke"} & attributes.keys()
        if "x" in attributes and unpainted and float(attributes["width"]) < 600:
            return float(attributes["width"])
    raise AssertionError("no panel clip rectangle")


def _marks(svg: str) -> dict[str, list[float]]:
    """Layer marks: line widths and lengths (horizontal, vertical) and radii."""
    found: dict[str, list[float]] = {"stroke": [], "horizontal": [], "vertical": [], "radius": []}
    for tag in re.findall(r"<(?:line|path)\b[^>]*>", svg):
        attributes = _attributes(tag)
        width = float(attributes.get("stroke-width", 0))
        if width in (0.0, 1.0) or "class" in attributes:
            continue
        if "d" in attributes:
            values = _numbers(attributes["d"])
            if len(values) != 4:
                continue
            x1, y1, x2, y2 = values
        else:
            x1, y1, x2, y2 = (float(attributes[key]) for key in ("x1", "y1", "x2", "y2"))
        found["stroke"].append(round(width, 2))
        if y1 == y2:
            found["horizontal"].append(abs(x2 - x1))
        else:
            found["vertical"].append(abs(y2 - y1))
    for tag in re.findall(r"<circle\b[^>]*>", svg):
        found["radius"].append(round(float(_attributes(tag)["r"]), 2))
    return found


def _svg(module: ModuleType, layer: Callable[[ModuleType], object]) -> str:
    plot = module.ggplot(DATA) + layer(module) + module.theme(legend_position="none")
    return plot.to_svg()


CASES: dict[str, Callable[[ModuleType], object]] = {
    "errorbar": lambda m: m.geom_errorbar(m.aes(**RANGE)),
    "errorbar width": lambda m: m.geom_errorbar(m.aes(**RANGE), width=0.2, size=1.5),
    "linerange": lambda m: m.geom_linerange(m.aes(**RANGE)),
    "pointrange": lambda m: m.geom_pointrange(m.aes(**FULL)),
    "pointrange styled": lambda m: m.geom_pointrange(m.aes(**FULL), size=2, linewidth=2, fatten=3),
    "pointrange hollow": lambda m: m.geom_pointrange(m.aes(**FULL), shape=21, fill="red", stroke=1),
    "crossbar": lambda m: m.geom_crossbar(m.aes(**FULL)),
    "crossbar fatten": lambda m: m.geom_crossbar(m.aes(**FULL), fatten=1, width=0.5),
    "crossbar dodged": lambda m: m.geom_crossbar(m.aes(**FULL, fill="g")),
}


@pytest.mark.parametrize("name", CASES)
def test_marks_match_lets_plot(name: str) -> None:
    ours, theirs = _svg(gg, CASES[name]), _svg(lp, CASES[name])
    ours_marks, their_marks = _marks(ours), _marks(theirs)
    assert sorted(ours_marks["stroke"]) == sorted(their_marks["stroke"])
    assert sorted(ours_marks["radius"]) == sorted(their_marks["radius"])
    ours_width, their_width = _panel_width(ours), _panel_width(theirs)
    ours_share = sorted(length / ours_width for length in ours_marks["horizontal"])
    their_share = sorted(length / their_width for length in their_marks["horizontal"])
    assert ours_share == pytest.approx(their_share, abs=0.004)
    ours_boxes = sorted(width / ours_width for width in _box_widths(ours))
    their_boxes = sorted(width / their_width for width in _box_widths(theirs))
    assert ours_boxes == pytest.approx(their_boxes, abs=0.004)


def _box_widths(svg: str) -> list[float]:
    """Widths of the layer's rectangles (not the background or the panel)."""
    widths = []
    for tag in re.findall(r"<rect\b[^>]*>", svg):
        attributes = _attributes(tag)
        if "x" in attributes and float(attributes.get("height", 0)) < 300:
            widths.append(float(attributes["width"]))
    return widths


def test_pointrange_without_y_draws_only_lines() -> None:
    marks = _marks(_svg(gg, lambda m: m.geom_pointrange(m.aes(**RANGE))))
    assert marks["radius"] == []
    assert len(marks["vertical"]) == 3


def test_errorbar_x_axis_spans_the_fences() -> None:
    """A lone numeric error bar keeps its fences inside the panel (width span)."""
    data = {"x": [1.0], "low": [0.0], "high": [1.0]}
    svg = (gg.ggplot(data) + gg.geom_errorbar(gg.aes(**RANGE), width=0.8)).to_svg()
    assert _marks(svg)["horizontal"][0] < _panel_width(svg)


def test_unsupported_width_unit_is_an_error() -> None:
    plot = gg.ggplot(DATA) + gg.geom_errorbar(gg.aes(**RANGE), width_unit="px")
    with pytest.raises(SpecError, match="width_unit 'px'"):
        plot.to_svg()


def test_negative_fatten_is_an_error() -> None:
    plot = gg.ggplot(DATA) + gg.geom_crossbar(gg.aes(**FULL), fatten=-1)
    with pytest.raises(SpecError, match="fatten"):
        plot.to_svg()


@pytest.mark.parametrize("name", ["errorbar", "linerange", "pointrange", "crossbar"])
def test_as_dict_matches_lets_plot(name: str) -> None:
    def build(module: ModuleType) -> dict:
        return (module.ggplot(DATA) + getattr(module, f"geom_{name}")(module.aes(**FULL))).as_dict()

    assert build(gg) == build(lp)


def test_mapped_width_sets_each_box() -> None:
    data = {**DATA, "w": [0.2, 0.5, 0.9]}
    for module in (gg, lp):
        mapping = module.aes(**FULL, width="w")
        svg = (module.ggplot(data) + module.geom_crossbar(mapping)).to_svg()
        panel = _panel_width(svg)
        shares = sorted(width / panel for width in _box_widths(svg))
        if module is gg:
            ours = shares
    assert ours == pytest.approx(shares, abs=0.004)


def test_errorbar_dodges_by_its_width() -> None:
    data = {
        "x": [1.0, 1.0, 2.0, 2.0],
        "low": [1.0, 2, 1, 2],
        "high": [3.0, 4, 3, 4],
        "g": list("abab"),
    }

    def centers(module: ModuleType) -> list[float]:
        mapping = module.aes(**RANGE, color="g", group="g")
        svg = (module.ggplot(data) + module.geom_errorbar(mapping, position="dodge")).to_svg()
        panel = _panel_width(svg)
        return sorted(length / panel for length in _marks(svg)["horizontal"])

    assert centers(gg) == pytest.approx(centers(lp), abs=0.004)


HORIZONTAL = {"y": "x", "xmin": "low", "xmax": "high"}
HORIZONTAL_FULL = {**HORIZONTAL, "x": "y"}

HORIZONTAL_CASES: dict[str, Callable[[ModuleType], object]] = {
    "errorbar": lambda m: m.geom_errorbar(m.aes(**HORIZONTAL)),
    "errorbar orientation": lambda m: m.geom_errorbar(m.aes(**HORIZONTAL), orientation="y"),
    "errorbar width": lambda m: m.geom_errorbar(m.aes(**HORIZONTAL), width=0.2, size=1.5),
    "linerange": lambda m: m.geom_linerange(m.aes(**HORIZONTAL)),
    "pointrange": lambda m: m.geom_pointrange(m.aes(**HORIZONTAL_FULL)),
    "crossbar": lambda m: m.geom_crossbar(m.aes(**HORIZONTAL_FULL)),
    "crossbar dodged": lambda m: m.geom_crossbar(m.aes(**HORIZONTAL_FULL, fill="g")),
}


def _panel_height(svg: str) -> float:
    """Height of the panel clip rectangle (a rect with geometry and no paint)."""
    for tag in re.findall(r"<rect\b[^>]*>", svg):
        attributes = _attributes(tag)
        unpainted = not {"fill", "stroke"} & attributes.keys()
        if "x" in attributes and unpainted and float(attributes["width"]) < 600:
            return float(attributes["height"])
    raise AssertionError("no panel clip rectangle")


def _box_heights(svg: str) -> list[float]:
    """Heights of the layer's rectangles (not the background or the panel)."""
    heights = []
    for tag in re.findall(r"<rect\b[^>]*>", svg):
        attributes = _attributes(tag)
        if "x" in attributes and float(attributes.get("height", 0)) < 300:
            heights.append(float(attributes["height"]))
    return heights


@pytest.mark.parametrize("name", HORIZONTAL_CASES)
def test_horizontal_marks_match_lets_plot(name: str) -> None:
    ours, theirs = _svg(gg, HORIZONTAL_CASES[name]), _svg(lp, HORIZONTAL_CASES[name])
    ours_marks, their_marks = _marks(ours), _marks(theirs)
    assert sorted(ours_marks["stroke"]) == sorted(their_marks["stroke"])
    assert sorted(ours_marks["radius"]) == sorted(their_marks["radius"])
    ours_height, their_height = _panel_height(ours), _panel_height(theirs)
    ours_share = sorted(length / ours_height for length in ours_marks["vertical"])
    their_share = sorted(length / their_height for length in their_marks["vertical"])
    assert ours_share == pytest.approx(their_share, abs=0.004)
    ours_boxes = sorted(height / ours_height for height in _box_heights(ours))
    their_boxes = sorted(height / their_height for height in _box_heights(theirs))
    assert ours_boxes == pytest.approx(their_boxes, abs=0.004)


def test_horizontal_errorbar_draws_ranges_along_x() -> None:
    marks = _marks(_svg(gg, HORIZONTAL_CASES["errorbar"]))
    assert (len(marks["horizontal"]), len(marks["vertical"])) == (3, 6)


def test_orientation_y_with_vertical_mapping_is_an_error() -> None:
    plot = gg.ggplot(DATA) + gg.geom_errorbar(gg.aes(**RANGE), orientation="y")
    with pytest.raises(SpecError, match=r"with orientation='y'.*missing: y, xmin, xmax"):
        plot.to_svg()


def test_missing_range_error_offers_the_horizontal_mapping() -> None:
    plot = gg.ggplot(DATA) + gg.geom_errorbar(gg.aes(x="x"))
    with pytest.raises(SpecError, match=r"missing: ymin, ymax\. Map y, xmin and xmax"):
        plot.to_svg()


def test_errorbar_height_is_a_deprecated_alias_of_width() -> None:
    with pytest.warns(GgrustWarning, match="'height' parameter is deprecated"):
        old = gg.geom_errorbar(gg.aes(**HORIZONTAL), height=0.2)
    assert old.as_dict()["width"] == 0.2
    assert "height" not in old.as_dict()


def test_errorbar_height_aesthetic_is_a_deprecated_alias_of_width() -> None:
    with pytest.warns(GgrustWarning, match="'height' aesthetic is deprecated"):
        old = gg.geom_errorbar(gg.aes(**HORIZONTAL, height="y"))
    assert old.as_dict()["mapping"]["width"] == "y"


def test_errorbar_height_unit_is_dropped_with_a_warning() -> None:
    with pytest.warns(GgrustWarning, match="'height_unit' parameter is deprecated"):
        old = gg.geom_errorbar(gg.aes(**HORIZONTAL), height_unit="res")
    assert "height_unit" not in old.as_dict()
