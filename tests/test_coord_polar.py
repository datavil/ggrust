"""coord_polar: pie, rose and radar charts bent from the flat panel."""

from __future__ import annotations

import math
import re

import ggrust as gg
import pytest
from ggrust import aes, coord_polar, geom_bar, geom_point, geom_raster, ggplot, ggsize
from ggrust.errors import SpecError

PIE = {"part": ["a", "b"], "share": [25, 75], "all": ["", ""]}
LINE = {"x": [0.0, 1.0, 2.0, 3.0, 4.0], "y": [1.0, 2.0, 3.0, 4.0, 5.0]}


def _paths(svg: str) -> list[list[tuple[float, float]]]:
    """Return the vertices of every filled, closed path of `svg`."""
    paths = []
    for data in re.findall(r'<path[^>]* d="(M[^"]*Z)"[^>]*fill="#', svg):
        numbers = [float(value) for value in re.findall(r"-?[0-9.]+", data)]
        paths.append(list(zip(numbers[::2], numbers[1::2], strict=False)))
    return paths


def _pie() -> gg.PlotSpec:
    plot = ggplot(PIE, aes("all", "share", fill="part")) + geom_bar(stat="identity", width=1)
    return plot + coord_polar(theta="y") + ggsize(400, 400)


def test_spec_carries_the_options() -> None:
    spec = coord_polar(theta="y", start=0.5, direction=-1, transform_bkgr=False).as_dict()
    assert spec["name"] == "polar"
    assert spec["theta"] == "y"
    assert spec["start"] == 0.5
    assert spec["direction"] == -1
    assert spec["transform_bkgr"] is False


def test_pie_wedges_share_a_centre_and_sweep_their_share() -> None:
    wedges = [path for path in _paths(_pie().to_svg()) if len(path) > 20]
    assert len(wedges) == 2
    sizes = sorted(len(path) for path in wedges)
    # One vertex per 2 degrees along both arcs: three times as many on the 75 percent wedge.
    assert sizes[1] / sizes[0] == pytest.approx(3.0, rel=0.1)


def test_circle_stays_round_on_a_wide_plot() -> None:
    wedges = [path for path in _paths((_pie() + ggsize(700, 300)).to_svg()) if len(path) > 20]
    xs = [x for path in wedges for x, _ in path]
    ys = [y for path in wedges for _, y in path]
    assert max(xs) - min(xs) == pytest.approx(max(ys) - min(ys), rel=0.02)


def test_direction_mirrors_the_circle() -> None:
    base = ggplot(LINE, aes("x", "y")) + geom_point() + ggsize(400, 400)

    def markers(plot: gg.PlotSpec) -> list[tuple[float, float]]:
        pairs = re.findall(r'<circle[^>]* cx="([0-9.]+)" cy="([0-9.]+)"', plot.to_svg())
        return [(float(x), float(y)) for x, y in pairs]

    clockwise = markers(base + coord_polar())
    counter = markers(base + coord_polar(direction=-1))
    assert len(clockwise) == len(counter) == 5
    centre = (clockwise[0][0] + counter[0][0]) / 2
    for left, right in zip(clockwise, counter, strict=True):
        assert math.isclose(left[0] + right[0], 2 * centre, abs_tol=0.1)
        assert math.isclose(left[1], right[1], abs_tol=0.1)


@pytest.mark.parametrize(
    ("options", "path", "accepted"),
    [
        ({"theta": "z"}, "coord.theta", "'x' or 'y'"),
        ({"direction": 2}, "coord.direction", "1 (clockwise)"),
    ],
)
def test_bad_options_name_the_parameter(options: dict, path: str, accepted: str) -> None:
    plot = ggplot(LINE, aes("x", "y")) + geom_point() + coord_polar(**options)
    with pytest.raises(SpecError) as caught:
        plot.to_svg()
    assert path in str(caught.value)
    assert accepted in str(caught.value)


def test_raster_is_rejected_with_a_way_out() -> None:
    plot = ggplot(LINE, aes("x", "y")) + geom_raster() + coord_polar()
    with pytest.raises(SpecError, match=r"layers\[0\]\.geom.*geom_tile"):
        plot.to_svg()


def test_html_notes_that_polar_is_static() -> None:
    plot = ggplot(LINE, aes("x", "y")) + geom_point() + coord_polar() + gg.ggtb()
    with pytest.warns(gg.GgrustWarning, match="coord_polar.*static"):
        plot.to_html()
