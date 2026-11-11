"""position_dodge2 and position_dodge(preserve=)."""

from __future__ import annotations

import re

import pytest
from ggrust import aes, geom_bar, geom_boxplot, ggplot, position_dodge, position_dodge2
from ggrust.errors import SpecError

BOXES = {
    "day": ["Mon"] * 8 + ["Tue"] * 4,
    "shift": ["early", "late"] * 4 + ["early"] * 4,
    "minutes": [31, 40, 28, 45, 35, 38, 90, 5, 22, 25, 30, 70],
}
BARS = {"m": ["a", "a", "b"], "g": ["x", "y", "x"], "v": [3, 4, 5]}


def rect_widths(svg: str) -> list[float]:
    """Widths of the data rectangles: inside the panel, not the panel or legend keys."""
    rects = [
        (float(x), float(width))
        for x, width in re.findall(r'<rect x="([0-9.]+)" y="[0-9.]+" width="([0-9.]+)"', svg)
    ]
    panel_x, panel_width = rects[0]
    return [width for x, width in rects[1:] if x + width <= panel_x + panel_width]


def test_spec() -> None:
    assert position_dodge2().as_dict() == {
        "name": "dodge2",
        "preserve": "total",
        "padding": 0.1,
        "reverse": False,
    }
    assert position_dodge(preserve="single").as_dict() == {"name": "dodge", "preserve": "single"}
    assert position_dodge().as_dict() == {"name": "dodge"}


def test_dodge2_lone_box_takes_the_full_width_unless_single() -> None:
    plot = ggplot(BOXES, aes("day", "minutes", fill="shift"))
    total = (plot + geom_boxplot(position="dodge2")).to_svg()
    single = (plot + geom_boxplot(position=position_dodge2(preserve="single"))).to_svg()
    assert total != single
    assert max(rect_widths(total)) > max(rect_widths(single)) * 1.5


def test_dodge2_outliers_follow_their_box() -> None:
    plot = ggplot(BOXES, aes("day", "minutes", fill="shift"))
    dodged = (plot + geom_boxplot(position="dodge2")).to_svg()
    plain = (plot + geom_boxplot()).to_svg()
    circles = re.findall(r'<circle cx="([0-9.]+)"', dodged)
    assert len(circles) == 3 and circles != re.findall(r'<circle cx="([0-9.]+)"', plain)


def test_dodge_single_keeps_one_slot_width() -> None:
    plot = ggplot(BARS, aes("m", "v", fill="g"))
    widths = rect_widths(
        (plot + geom_bar(stat="identity", position=position_dodge(preserve="single"))).to_svg()
    )
    assert len(widths) == 3 and max(widths) - min(widths) < 0.01


def test_errors() -> None:
    with pytest.raises(ValueError, match="preserve"):
        position_dodge2(preserve="singel")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="padding"):
        position_dodge2(padding=2)
    with pytest.raises(TypeError, match="reverse"):
        position_dodge2(reverse="yes")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="preserve"):
        position_dodge(preserve="all")  # type: ignore[arg-type]
    with pytest.raises(SpecError, match="did you mean 'single'"):
        ggplot(BARS, aes("m", "v")).__add__(
            geom_bar(stat="identity", position={"name": "dodge", "preserve": "singel"})  # type: ignore[arg-type]
        ).to_svg()


def test_dodge2_varwidth_outliers_sit_on_their_box() -> None:
    data = {
        "x": ["A"] * 12 + ["C"] * 8,
        "g": ["a"] * 8 + ["c"] * 4 + ["a"] * 4 + ["c"] * 4,
        "y": [10, 11, 12, 13, 14, 12, 11, 13, 10, 12, 14, 60, 10, 12, 14, 12, 10, 12, 14, -40],
    }
    svg = (
        ggplot(data, aes("x", "y", fill="g")) + geom_boxplot(varwidth=True, position="dodge2")
    ).to_svg()
    boxes = [
        (float(x), float(width))
        for x, width in re.findall(r'<rect x="([0-9.]+)" y="[0-9.]+" width="([0-9.]+)"', svg)
    ][1:5]
    centres = sorted(x + width / 2 for x, width in boxes)
    circles = sorted(float(cx) for cx in re.findall(r'<circle cx="([0-9.]+)"', svg))
    assert len(circles) == 2
    for circle in circles:
        assert min(abs(circle - centre) for centre in centres) < 0.05
