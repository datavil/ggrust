"""facet_grid space/margins, as_table and shrink on both facets, facet_null."""

from __future__ import annotations

import re

import pytest
from ggrust import aes, facet_grid, facet_null, facet_wrap, geom_point, ggplot, stat_summary
from ggrust.errors import SpecError

DATA = {
    "x": [1.0, 2.0, 3.0, 10.0, 11.0, 30.0],
    "y": [2.0, 4.0, 3.0, 5.0, 6.0, 1.0],
    "g": ["a", "a", "b", "b", "c", "c"],
    "h": ["p", "q", "p", "q", "p", "q"],
}


def plot():
    return ggplot(DATA, aes("x", "y")) + geom_point()


def strip_order(svg: str, labels: list[str]) -> list[str]:
    """Labels in the order they appear in the SVG."""
    found = [(svg.index(f">{label}<"), label) for label in labels if f">{label}<" in svg]
    return [label for _, label in sorted(found)]


def test_facet_null_removes_faceting() -> None:
    faceted = plot() + facet_wrap("g")
    assert (faceted + facet_null()).facet is None
    assert (faceted + facet_null()).to_svg() == plot().to_svg()


def test_margins_add_all_panels() -> None:
    svg = (plot() + facet_grid("g", "h", margins=True)).to_svg()
    assert svg.count(">(all)<") == 2
    svg = (plot() + facet_grid("g", "h", margins="h")).to_svg()
    assert svg.count(">(all)<") == 1


def test_as_table_false_puts_the_highest_row_on_top() -> None:
    svg = (plot() + facet_grid(y="h", as_table=False)).to_svg()
    assert strip_order(svg, ["p", "q"]) == ["q", "p"]


def test_space_free_changes_panel_widths() -> None:
    fixed = (plot() + facet_grid("g", scales="free_x")).to_svg()
    free = (plot() + facet_grid("g", scales="free_x", space="free_x")).to_svg()
    widths = sorted(
        {
            float(width)
            for width in re.findall(r'<clipPath[^>]*>\s*<rect[^>]*width="([\d.]+)"', free)
        }
    )
    assert fixed != free
    assert len(widths) == 3


def test_shrink_false_trains_on_the_raw_data() -> None:
    data = {"g": ["a"] * 4 + ["b"] * 4, "y": [0.0, 100.0, 40.0, 60.0] * 2, "h": ["p"] * 8}
    summary = ggplot(data, aes("g", "y")) + stat_summary(fun="mean", geom="point")
    assert ">100<" not in (summary + facet_wrap("h")).to_svg()
    assert ">100<" in (summary + facet_wrap("h", shrink=False)).to_svg()


def test_errors() -> None:
    with pytest.raises(SpecError, match="use one of fixed, free"):
        facet_grid("g", space="fre")  # type: ignore[arg-type]
    with pytest.raises(SpecError, match="not a faceting column"):
        facet_grid("g", "h", margins="z")
    with pytest.raises(TypeError, match="as_table"):
        facet_wrap("g", as_table="no")  # type: ignore[arg-type]
