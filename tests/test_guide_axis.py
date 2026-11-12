"""guide_axis(cap=): the axis line cut back to the outermost breaks (ggplot2)."""

from __future__ import annotations

import re
import warnings

import ggrust as gg
import pytest
from ggrust.errors import SpecError, UnsupportedValueError

DATA = {"x": [1.3, 2.0, 3.0, 4.6], "y": [11.0, 15.0, 22.0, 38.0], "g": ["a", "b", "c", "d"]}


def _axis_paths(svg: str, axis: str) -> list[tuple[float, float, float, float]]:
    """Every two-point path of an axis group: (x0, y0, x1, y1)."""
    start = svg.index(f'"{axis}"')
    end = svg.index("</g>", start)
    pattern = r'd="M([\d.]+) ([\d.]+)L([\d.]+) ([\d.]+)"'
    return [
        (float(a), float(b), float(c), float(d))
        for a, b, c, d in re.findall(pattern, svg[start:end])
    ]


def _line_and_ticks(svg: str, axis: str) -> tuple[tuple[float, ...], list[tuple[float, ...]]]:
    paths = _axis_paths(svg, axis)
    return paths[0], paths[1:]


def _render(**guides: gg.FeatureSpec) -> str:
    plot = (
        gg.ggplot(DATA, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.theme_classic()
        + gg.guides(**guides)
    )
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        return plot.to_svg()


def test_no_cap_spans_the_panel() -> None:
    line, ticks = _line_and_ticks(_render(), "axis-x")
    assert line[0] < min(tick[0] for tick in ticks)
    assert line[2] > max(tick[0] for tick in ticks)


@pytest.mark.parametrize(
    ("cap", "low_capped", "high_capped"),
    [("both", True, True), (True, True, True), ("upper", False, True), ("lower", True, False)],
)
def test_x_cap_ends(cap: str | bool, low_capped: bool, high_capped: bool) -> None:
    full, _ = _line_and_ticks(_render(), "axis-x")
    line, ticks = _line_and_ticks(_render(x=gg.guide_axis(cap=cap)), "axis-x")
    first, last = min(tick[0] for tick in ticks), max(tick[0] for tick in ticks)
    assert line[0] == pytest.approx(first if low_capped else full[0])
    assert line[2] == pytest.approx(last if high_capped else full[2])


def test_y_upper_is_the_top_end() -> None:
    full, _ = _line_and_ticks(_render(), "axis-y")
    line, ticks = _line_and_ticks(_render(y=gg.guide_axis(cap="upper")), "axis-y")
    top = min(tick[1] for tick in ticks)
    assert min(line[1], line[3]) == pytest.approx(top)
    assert max(line[1], line[3]) == pytest.approx(max(full[1], full[3]))


def test_cap_false_and_none_draw_the_whole_line() -> None:
    full = _render()
    assert _render(x=gg.guide_axis(cap=False)) == full
    assert _render(x=gg.guide_axis(cap="none")) == full


def test_discrete_axis_caps_at_the_outer_categories() -> None:
    plot = (
        gg.ggplot(DATA, gg.aes("g", "y"))
        + gg.geom_col()
        + gg.theme_classic()
        + gg.scale_x_discrete(guide=gg.guide_axis(cap="both"))
    )
    line, ticks = _line_and_ticks(plot.to_svg(), "axis-x")
    assert line[0] == pytest.approx(min(tick[0] for tick in ticks))
    assert line[2] == pytest.approx(max(tick[0] for tick in ticks))


def test_guide_axis_title_names_the_axis() -> None:
    svg = _render(x=gg.guide_axis("Engine size", cap="both"))
    assert "Engine size" in svg


def test_spec() -> None:
    assert gg.guide_axis(cap="both").as_dict() == {"name": "axis", "cap": "both"}


def test_errors() -> None:
    with pytest.raises(SpecError, match="did you mean 'upper'"):
        gg.guide_axis(cap="uper")  # ty: ignore[invalid-argument-type]
    with pytest.raises(UnsupportedValueError, match="cap"):
        gg.guide_axis(cap=1)  # ty: ignore[invalid-argument-type]


def test_scale_guide_title_wins_over_the_scale_name() -> None:
    plot = (
        gg.ggplot(DATA, gg.aes("g", "y"))
        + gg.geom_point()
        + gg.scale_x_discrete("Scale", guide=gg.guide_axis("Guide"))
    )
    svg = plot.to_svg()
    assert ">Guide<" in svg and ">Scale<" not in svg


def test_other_ggplot2_options_warn() -> None:
    plot = (
        gg.ggplot(DATA, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.guides(x={"name": "axis", "angle": 45, "cap": "both"})
    )
    with pytest.warns(gg.GgrustWarning, match="guides.x.angle"):
        plot.to_svg()


def _flipped_lines(plot: gg.PlotSpec) -> tuple[tuple[float, ...], tuple[float, ...]]:
    svg = (plot + gg.theme_classic() + gg.coord_flip()).to_svg()
    return _axis_paths(svg, "axis-x")[0], _axis_paths(svg, "axis-y")[0]


def test_guides_follow_the_screen_axis_under_flip() -> None:
    # R (ggplot2 4.0.1) with coord_flip(): guides(x=) caps the bottom axis,
    # guides(y=) the left one, a scale's own guide= its scale's (left) axis.
    base = gg.ggplot(DATA, gg.aes("g", "y")) + gg.geom_point()
    full_bottom, full_left = _flipped_lines(base)
    bottom, left = _flipped_lines(base + gg.guides(x=gg.guide_axis(cap="both")))
    assert bottom != full_bottom and left == full_left
    bottom, left = _flipped_lines(base + gg.guides(y=gg.guide_axis(cap="upper")))
    assert bottom == full_bottom and left != full_left
    assert min(left[1], left[3]) > min(full_left[1], full_left[3])
    assert max(left[1], left[3]) == pytest.approx(max(full_left[1], full_left[3]))
    plot = base + gg.scale_x_discrete(guide=gg.guide_axis(cap="both"))
    bottom, left = _flipped_lines(plot)
    assert bottom == full_bottom and left != full_left
