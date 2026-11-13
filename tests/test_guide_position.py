"""Per-guide ``position`` (ggplot2 3.5): guides on several sides of one plot."""

from __future__ import annotations

import re

import pytest
from ggrust import (
    aes,
    geom_point,
    ggplot,
    guide_bins,
    guide_colorbar,
    guide_legend,
    guides,
    scale_size_binned,
    theme,
)
from ggrust.errors import SpecError

DATA = {
    "x": [1, 2, 3, 4, 5, 6],
    "y": [2, 4, 3, 5, 6, 1],
    "pet": ["cat", "cat", "dog", "dog", "fox", "fox"],
    "size": ["s", "m", "l", "s", "m", "l"],
}


def svg(*specs: object) -> str:
    plot = ggplot(DATA, aes("x", "y", color="pet", shape="size")) + geom_point()
    for spec in specs:
        plot = plot + spec
    return plot.to_svg()


def anchor(out: str, label: str) -> tuple[float, float]:
    match = re.search(rf'<text x="([0-9.]+)" y="([0-9.]+)"[^>]*>{label}<', out)
    assert match, f"no text {label!r}"
    return float(match[1]), float(match[2])


@pytest.mark.parametrize("function", [guide_legend, guide_colorbar, guide_bins])
def test_position_is_in_the_spec(function) -> None:
    assert function(position="left").as_dict()["position"] == "left"


@pytest.mark.parametrize("function", [guide_legend, guide_colorbar, guide_bins])
def test_unknown_position_is_rejected(function) -> None:
    with pytest.raises(SpecError, match=r"position='botom'.*did you mean 'bottom'"):
        function(position="botom")


def test_guides_go_to_different_sides() -> None:
    default = svg()
    out = svg(guides(color=guide_legend(position="left"), shape=guide_legend(position="bottom")))
    width = float(re.search(r'<svg[^>]* width="([0-9.]+)"', out)[1])  # type: ignore[index]
    pet_x, _ = anchor(out, "pet")
    size_x, size_y = anchor(out, "size")
    assert pet_x < width / 4
    assert size_y > anchor(default, "size")[1]
    assert size_x > pet_x


def test_bottom_guide_runs_in_a_row() -> None:
    out = svg(guides(color=guide_legend(position="bottom")))
    assert anchor(out, "cat")[1] == anchor(out, "fox")[1]
    # The other guide stays right, in a column.
    assert anchor(out, "s")[1] != anchor(out, "l")[1]


def test_inside_guide_uses_legend_position_inside() -> None:
    lower = svg(
        guides(color=guide_legend(position="inside")), theme(legend_position_inside=[0.3, 0.2])
    )
    upper = svg(
        guides(color=guide_legend(position="inside")), theme(legend_position_inside=[0.3, 0.8])
    )
    assert anchor(lower, "pet")[1] > anchor(upper, "pet")[1]


def test_legend_position_none_hides_positioned_guides() -> None:
    out = svg(guides(color=guide_legend(position="top")), theme(legend_position="none"))
    assert ">pet<" not in out


def test_binned_guide_takes_a_position() -> None:
    data = {"x": [1, 2, 3, 4, 5], "y": [2, 4, 3, 5, 1], "score": [3, 18, 41, 66, 97]}
    plot = (
        ggplot(data, aes("x", "y", size="score"))
        + geom_point()
        + scale_size_binned(guide=guide_bins(position="top"))
    )
    out = plot.to_svg()
    _, title_y = anchor(out, "score")
    assert title_y < 100
