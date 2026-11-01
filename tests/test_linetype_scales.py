"""Mapped linetype: default, manual and identity scales against lets-plot (dash arrays)."""

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
    "t": [1, 2, 3] * 4,
    "v": [1, 3, 2, 2, 1, 3, 3, 2, 1, 2, 2, 2],
    "g": list("aaabbbcccddd"),
    "lt": ["dashed"] * 3 + ["dotted"] * 3 + ["solid"] * 3 + ["twodash"] * 3,
    "code": [2, 2, 2, 3, 3, 3, 5, 5, 5, 6, 6, 6],
}

CASES: dict[str, Callable[[ModuleType], object]] = {
    "default": lambda m: m.ggplot(DATA, m.aes("t", "v", linetype="g")) + m.geom_line(size=1),
    "more_levels_than_types": lambda m: (
        m.ggplot(
            {"t": list(range(16)), "v": [1, 2] * 8, "g": [str(i // 2) for i in range(16)]},
            m.aes("t", "v", linetype="g"),
        )
        + m.geom_line()
    ),
    "manual": lambda m: (
        m.ggplot(DATA, m.aes("t", "v", linetype="g"))
        + m.geom_line()
        + m.scale_linetype_manual(values=["dotdash", "longdash", [5, 2], "twodash"])
    ),
    "manual_dict": lambda m: (
        m.ggplot(DATA, m.aes("t", "v", linetype="g"))
        + m.geom_line()
        + m.scale_linetype_manual(values={"a": "dotted", "b": "solid", "c": 5, "d": "dashed"})
    ),
    "identity_names": lambda m: (
        m.ggplot(DATA, m.aes("t", "v", linetype="lt", group="g"))
        + m.geom_line()
        + m.scale_linetype_identity()
    ),
    "identity_codes": lambda m: (
        m.ggplot(DATA, m.aes("t", "v", linetype="code", group="g"))
        + m.geom_path()
        + m.scale_linetype_identity()
    ),
}


def _dashes(svg: str) -> list[list[float]]:
    """Dash arrays of the drawn lines (and legend keys), in drawing order."""
    return [
        [round(float(n), 2) for n in re.split(r"[ ,]+", dash.strip())]
        for dash in re.findall(r'stroke-dasharray="([^"]+)"', svg)
    ]


@pytest.mark.parametrize("name", list(CASES))
def test_matches_lets_plot(name: str) -> None:
    ours = sorted(_dashes(CASES[name](gg).to_svg()))
    theirs = sorted(_dashes(CASES[name](lp).to_svg()))
    assert sorted(map(tuple, ours)) == sorted(map(tuple, theirs))


def test_bad_manual_value_names_the_item() -> None:
    plot = gg.ggplot(DATA, gg.aes("t", "v", linetype="g")) + gg.geom_line()
    plot += gg.scale_linetype_manual(values=["solid", "dashy"])
    with pytest.raises(SpecError, match=r"values\[1\].*invalid line type"):
        plot.to_svg()


def test_spec_matches_lets_plot() -> None:
    for build in CASES.values():
        assert build(gg).as_dict()["scales"] == build(lp).as_dict().get("scales", [])
