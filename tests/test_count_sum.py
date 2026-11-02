"""geom_count and stat_sum against lets-plot.

Circle radii and the plot's texts (axis labels, legend titles and keys, counts drawn
with ``geom="text"``) are compared; pixel positions differ by a few px (real fonts
move the panel).
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
    "x": [0, 0, 0, 1, 1, 2, 0, 1, -1, 2, 2, 0, 1, -1, 0],
    "y": [0, 0, 1, 1, 1, 2, 0, 1, 0, 2, 0, 0, 1, 0, 2],
    "g": list("aaabbbbaabababa"),
}


def _radii(svg: str) -> list[float]:
    return sorted(round(float(r), 1) for r in re.findall(r'<circle[^>]*\br="([0-9.]+)"', svg))


def _texts(svg: str) -> list[str]:
    found = re.findall(r"<tspan>([^<]*)</tspan>", svg) + re.findall(
        r"<text[^>]*>([^<]+)</text>", svg
    )
    return sorted(text.strip() for text in found if text.strip())


def _discrete(m: ModuleType) -> object:
    return m.aes(x=m.as_discrete("x", order=1), y=m.as_discrete("y", order=1))


CASES: dict[str, Callable[[ModuleType], object]] = {
    "default": lambda m: m.ggplot(DATA, m.aes("x", "y")) + m.geom_count(),
    "discrete": lambda m: m.ggplot(DATA, _discrete(m)) + m.geom_count(),
    "prop_by_x": lambda m: (
        m.ggplot(DATA, _discrete(m)) + m.geom_count(m.aes(size="..prop..", group="x"))
    ),
    "proppct_color": lambda m: (
        m.ggplot(DATA, m.aes("x", "y")) + m.geom_count(m.aes(color="..proppct.."))
    ),
    "groups": lambda m: m.ggplot(DATA, m.aes("x", "y", color="g")) + m.geom_count(alpha=0.7),
    "stat_text": lambda m: (
        m.ggplot(DATA, m.aes("x", "y")) + m.stat_sum(m.aes(label="..n.."), geom="text", size=6)
    ),
    "stat_text_prop": lambda m: (
        m.ggplot(DATA, m.aes("x", "y", group="g"))
        + m.stat_sum(m.aes(label="..prop.."), geom="text", size=6)
    ),
}


@pytest.mark.parametrize("name", CASES)
def test_matches_lets_plot(name: str) -> None:
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        ours = CASES[name](gg).to_svg()
    theirs = CASES[name](lp).to_svg()
    assert _radii(ours) == _radii(theirs)
    assert _texts(ours) == _texts(theirs)


def test_spec_matches_lets_plot() -> None:
    for build in (
        lambda m: m.geom_count(),
        lambda m: m.geom_count(stat="identity", color="red"),
        lambda m: m.stat_sum(),
        lambda m: m.stat_sum(geom="text", fill_by="color"),
    ):
        assert build(gg).as_dict() == build(lp).as_dict()


def test_constant_size_replaces_the_default_count_mapping() -> None:
    svg = (gg.ggplot(DATA, gg.aes("x", "y")) + gg.geom_count(size=4)).to_svg()
    assert len(set(_radii(svg))) == 1
    assert ">n<" not in svg and "<tspan>n</tspan>" not in svg


def test_weight_is_summed_as_ggplot2() -> None:
    data = {"x": [0, 0, 1], "y": [0, 0, 1], "w": [2, 3, 4]}
    svg = (
        gg.ggplot(data, gg.aes("x", "y"))
        + gg.stat_sum(gg.aes(label="..n..", weight="w"), geom="text", size=6)
    ).to_svg()
    labels = re.findall(r"<text[^>]*>([^<]+)</text>", svg) + re.findall(
        r"<tspan>([^<]*)</tspan>", svg
    )
    assert "5" in labels and "4" in labels


@pytest.mark.parametrize(
    "build",
    [
        lambda m: (
            m.ggplot({"x": [1, 1, 2, 3], "y": [1, 1, 1, 2], "w": [1, 2, 3, 4]}, m.aes("x", "y"))
            + m.geom_count(size=5, color="red")
            + m.geom_point(m.aes(size="w"))
        ),
        lambda m: m.ggplot({"x": ["a", "b"]}, m.aes("x")) + m.geom_bar(y=3),
        lambda m: (
            m.ggplot({"x": ["a", "b"], "w": [1, 2]}, m.aes("x"))
            + m.geom_bar(y=3)
            + m.geom_point(m.aes(y="w"))
        ),
    ],
    ids=["legend_title", "axis_title", "axis_title_two_layers"],
)
def test_constant_leaves_the_default_variable_out_of_titles(
    build: Callable[[ModuleType], object],
) -> None:
    assert _texts(build(gg).to_svg()) == _texts(build(lp).to_svg())
