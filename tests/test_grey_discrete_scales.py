"""scale_*_grey and scale_*_discrete against lets-plot: point colors and legend labels."""

import re
import warnings
from collections.abc import Callable
from types import ModuleType

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

DATA = {
    "x": [1, 2, 3, 4, 5, 6],
    "y": [1, 2, 3, 4, 5, 6],
    "v": [3, 1, 2, 3, 10, 7.5],
    "s": ["c", "a", "b", "d", "c", "e"],
}

SCALES: dict[str, Callable[[ModuleType], object]] = {
    "color_discrete": lambda m: m.scale_color_discrete(),
    "color_discrete_reversed": lambda m: m.scale_color_discrete(direction=-1),
    "color_discrete_hue": lambda m: m.scale_color_discrete(scale_mapper_kind="color_hue"),
    "color_grey": lambda m: m.scale_color_grey(),
    "color_grey_ends": lambda m: m.scale_color_grey(start=0.9, end=0.1),
    "fill_grey": lambda m: m.scale_fill_grey(start=0.1, end=0.9),
    "fill_discrete": lambda m: m.scale_fill_discrete(direction=-1),
    "discrete_then_manual": lambda m: (
        m.scale_color_discrete()
        + m.scale_color_manual(values=["red", "blue", "green", "black", "pink", "cyan"])
    ),
}


def _colors(svg: str, attribute: str) -> list[str]:
    return re.findall(rf'<circle[^>]*{attribute}="([^"]+)"', svg)[: len(DATA["x"])]


def _labels(svg: str) -> list[str]:
    return re.findall(r">([\w.]+)</t", svg)


@pytest.mark.parametrize("name", list(SCALES))
@pytest.mark.parametrize("column", ["v", "s"])
def test_matches_lets_plot(name: str, column: str) -> None:
    aesthetic = "fill" if name.startswith("fill") else "color"

    def build(module: ModuleType) -> str:
        mapping = module.aes("x", "y", **{aesthetic: column})
        plot = module.ggplot(DATA, mapping) + module.geom_point(shape=21) + SCALES[name](module)
        return plot.to_svg()

    ours, theirs = build(gg), build(lp)
    assert _colors(ours, aesthetic) == _colors(theirs, aesthetic)
    # Legend labels (the axis titles come first and differ in order).
    assert _labels(ours)[-6:] == _labels(theirs)[-6:]


def test_grey_rescales_old_range_and_rejects_others() -> None:
    with pytest.warns(UserWarning, match="read as 0.5"):
        scale = gg.scale_color_grey(start=50)
    assert scale.as_dict()["start"] == 0.5
    with pytest.raises(ValueError, match="'end' must be a lightness between 0"):
        gg.scale_fill_grey(end=150)


def test_spec_matches_lets_plot() -> None:
    for name, build in SCALES.items():
        if "_then_" not in name:
            assert build(gg).as_dict() == build(lp).as_dict()
