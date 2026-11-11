"""geom_spoke: segments of a given length and angle, as lets-plot."""

from __future__ import annotations

import math

import pytest
from ggrust import aes, geom_spoke, ggplot
from ggrust.errors import SpecError

DATA = {"x": [0.0, 1.0], "y": [0.0, 0.0], "a": [0.0, math.pi / 2], "r": [3.0, 2.0]}


def _domain(axis: str, **options: object) -> tuple[float, float]:
    layer = geom_spoke(aes(angle="a", radius="r"), **options)
    return (ggplot(DATA, aes("x", "y")) + layer).scale_info(axis)[0]["limits"]


def test_the_tail_trains_the_axes_to_the_far_end() -> None:
    assert _domain("x") == pytest.approx((0.0, 3.0))
    assert _domain("y") == pytest.approx((0.0, 2.0))


def test_the_tip_moves_the_segment_back() -> None:
    assert _domain("x", pivot="tip") == pytest.approx((-3.0, 1.0))
    assert _domain("y", pivot="tip") == pytest.approx((-2.0, 0.0))


def test_the_middle_centres_the_segment() -> None:
    assert _domain("x", pivot="mid") == pytest.approx((-1.5, 1.5))


def test_constants_are_accepted() -> None:
    plot = ggplot(DATA, aes("x", "y")) + geom_spoke(angle=0.0, radius=2.0)
    assert plot.scale_info("x")[0]["limits"] == pytest.approx((0.0, 3.0))
    assert "<path" in plot.to_svg() or "<line" in plot.to_svg()


def test_bad_pivot_names_the_parameter() -> None:
    plot = ggplot(DATA, aes("x", "y")) + geom_spoke(pivot="end", angle=0.0, radius=1.0)
    with pytest.raises(SpecError, match=r"pivot.*'tail'.*'end'"):
        plot.to_svg()
