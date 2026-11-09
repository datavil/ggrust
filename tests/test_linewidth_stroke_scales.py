"""`scale_linewidth*` and `scale_stroke*` against lets-plot 4.11 (`to_svg()`).

Widths are compared as the set of SVG stroke widths (lets-plot's background rect
adds a 0 width, dropped here). Two deliberate deviations are pinned instead:
shape 19 grows with `stroke` (ggplot2; lets-plot#1158) and limits that exclude the
data draw `na_value` (lets-plot#769 fails the plot).
"""

from __future__ import annotations

import re
import warnings
from collections.abc import Callable

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

FRAME = {"x": [1, 2, 3], "y": [1, 2, 3], "v": [0, None, 10]}


def widths(svg: str) -> list[float]:
    """Distinct non-zero stroke widths, rounded to 0.001 px."""
    found = {round(float(width), 3) for width in re.findall(r'stroke-width="([\d.]+)"', svg)}
    return sorted(found - {0.0})


def radii(svg: str) -> list[float]:
    return sorted({round(float(radius), 3) for radius in re.findall(r' r="([\d.]+)"', svg)})


def pointrange(m: object) -> object:
    return m.ggplot(FRAME, m.aes("x", "y")) + m.geom_pointrange(
        m.aes(ymin="x", ymax="y", linewidth="v")
    )


def outlined_points(m: object) -> object:
    return m.ggplot(FRAME, m.aes("x", "y")) + m.geom_point(m.aes(stroke="v"), shape=21)


DATA_CASES: dict[str, Callable[[object], object]] = {
    "linewidth_default": pointrange,
    "linewidth_range": lambda m: pointrange(m) + m.scale_linewidth(range=[0.5, 3]),
    "linewidth_identity": lambda m: pointrange(m) + m.scale_linewidth_identity(),
    "linewidth_discrete": lambda m: (
        m.ggplot({"x": [1, 2, 3], "y": [1, 2, 3], "v": ["a", "b", "c"]}, m.aes("x", "y"))
        + m.geom_pointrange(m.aes(ymin="x", ymax="y", linewidth="v"))
    ),
    "stroke_default": outlined_points,
    "stroke_range": lambda m: outlined_points(m) + m.scale_stroke(range=[0.5, 3]),
    "stroke_identity": lambda m: outlined_points(m) + m.scale_stroke_identity(),
}


def render(m: object, build: Callable[[object], object], legend: bool) -> str:
    plot = build(m) + m.theme_void()
    if not legend:
        plot += m.theme(legend_position="none")
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        return plot.to_svg()


@pytest.mark.parametrize("case", DATA_CASES)
def test_mapped_widths_match_lets_plot(case: str) -> None:
    ours = render(gg, DATA_CASES[case], legend=False)
    theirs = render(lp, DATA_CASES[case], legend=False)
    assert widths(ours) == widths(theirs)
    assert radii(ours) == radii(theirs)


def test_stroke_legend_matches_lets_plot() -> None:
    # Keys for 0, 2.5, ..., 10 grow with their outline, as in lets-plot.
    ours = render(gg, outlined_points, legend=True)
    theirs = render(lp, outlined_points, legend=True)
    assert widths(ours) == widths(theirs)
    assert radii(ours) == radii(theirs)


def test_default_ranges_are_one_to_seven() -> None:
    # 0 -> 1 unit (2.2 px), 10 -> 7 units (15.4 px); missing -> 1 unit.
    assert widths(render(gg, pointrange, legend=False)) == [2.2, 15.4]


def test_shape_19_grows_with_stroke_like_shape_21() -> None:
    # lets-plot#1158 ignores stroke on shape 19; ggplot2 draws 19 as 21 filled in
    # its color.
    def outer_radius(shape: int) -> float:
        plot = gg.ggplot({"x": [1], "y": [1]}, gg.aes("x", "y"))
        plot += gg.geom_point(shape=shape, size=5, stroke=2) + gg.theme_void()
        svg = plot.to_svg()
        radius = float(re.search(r' r="([\d.]+)"', svg).group(1))
        width = re.search(r'stroke-width="([\d.]+)"', svg)
        return radius + (float(width.group(1)) / 2 if width else 0.0)

    assert outer_radius(19) == pytest.approx(outer_radius(21))
    assert outer_radius(19) > outer_radius(16)


def test_limits_outside_the_data_draw_na_value() -> None:
    # lets-plot#769: limits that exclude every value fail the plot. ggrust draws
    # the values outside the limits with na_value (1 unit by default).
    svg = render(gg, lambda m: outlined_points(m) + m.scale_stroke(limits=[20, 30]), False)
    assert widths(svg) == [2.2]
    svg = render(
        gg,
        lambda m: pointrange(m) + m.scale_linewidth(range=[1, 7], limits=[20, 30], na_value=3),
        False,
    )
    assert widths(svg) == [6.6]
