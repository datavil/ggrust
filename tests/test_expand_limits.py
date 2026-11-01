"""expand_limits against lets-plot: the spec and the axis/legend labels it produces."""

import re
import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

DATA = {"x": [-3, 0, 1], "y": [2, 3, -1], "g": ["a", "b", "a"], "v": [1.0, 2.0, 3.0]}

ARGUMENTS: list[dict[str, object]] = [
    {"x": -10},
    {"y": 0},
    {"x": -10, "y": [-5, 5]},
    {"y": range(-10, 10)},
    {"color": "q"},
    {"size": 10},
    {"x": (0, 20), "y": None},
]


@pytest.mark.parametrize("arguments", ARGUMENTS)
def test_spec_matches_lets_plot(arguments: dict[str, object]) -> None:
    assert gg.expand_limits(**arguments).as_dict() == lp.expand_limits(**arguments).as_dict()


@pytest.mark.parametrize("arguments", ARGUMENTS)
def test_labels_match_lets_plot(arguments: dict[str, object]) -> None:
    def labels(module: object) -> list[str]:
        plot = (
            module.ggplot(DATA, module.aes("x", "y", color="g", size="v"))
            + module.geom_point()
            + module.expand_limits(**arguments)
        )
        return sorted(re.findall(r">(-?[\w.]+)</t", plot.to_svg()))

    assert labels(gg) == labels(lp)


def test_array_likes_become_lists() -> None:
    import numpy as np
    import polars as pl

    expected = gg.expand_limits(x=[-10, 10]).as_dict()
    for values in (np.array([-10, 10]), pl.Series([-10, 10]), (v for v in (-10, 10))):
        assert gg.expand_limits(x=values).as_dict() == expected
    plot = gg.ggplot(DATA, gg.aes("x", "y")) + gg.geom_point()
    svg = (plot + gg.expand_limits(x=np.array([-10, 10]))).to_svg()
    assert ">-10<" in svg and ">10<" in svg
