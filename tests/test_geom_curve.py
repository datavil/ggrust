"""geom_curve: same curves as lets-plot (control points, B-spline, arrows, spacer).

Both SVGs use ``theme_void``; the panels have the same size but lets-plot's sits
6 px further up and left, so lets-plot's curve path (``M L C... L``) is moved onto
ggrust's start point, sampled, and every sample must lie on ggrust's flattened polyline.
"""

import itertools
import math
import re
import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.errors import SpecError  # noqa: E402

Point = tuple[float, float]
PATH = re.compile(r'<path\b[^>]*\bd="([^"]*)"[^>]*>')


def _stroked_paths(svg: str) -> list[str]:
    """The `d` of every path drawn with the curve's 1.65 px (size 0.75) stroke."""
    return [
        match.group(1)
        for match in PATH.finditer(svg)
        if re.search(r'stroke-width="1\.65', match.group(0))
    ]


def _letsplot_samples(d: str) -> list[Point]:
    """Points along a lets-plot path made of M, L and C commands."""
    tokens = re.findall(r"[MLC]|-?\d+(?:\.\d+)?(?:[eE]-?\d+)?", d)
    samples: list[Point] = []
    current: Point = (0.0, 0.0)
    index = 0
    while index < len(tokens):
        command = tokens[index]
        index += 1
        if command in ("M", "L"):
            current = (float(tokens[index]), float(tokens[index + 1]))
            index += 2
            samples.append(current)
        elif command == "C":
            p1, p2, p3 = (
                (float(tokens[index + 2 * k]), float(tokens[index + 2 * k + 1])) for k in range(3)
            )
            index += 6
            for step in range(1, 21):
                t = step / 20
                u = 1 - t
                samples.append(
                    (
                        u**3 * current[0]
                        + 3 * u * u * t * p1[0]
                        + 3 * u * t * t * p2[0]
                        + t**3 * p3[0],
                        u**3 * current[1]
                        + 3 * u * u * t * p1[1]
                        + 3 * u * t * t * p2[1]
                        + t**3 * p3[1],
                    )
                )
            current = p3
    return samples


def _polyline(d: str) -> list[Point]:
    numbers = [float(value) for value in re.findall(r"-?\d+(?:\.\d+)?(?:[eE]-?\d+)?", d)]
    return list(zip(numbers[0::2], numbers[1::2], strict=True))


def _distance_to_polyline(point: Point, polyline: list[Point]) -> float:
    best = math.inf
    for (ax, ay), (bx, by) in itertools.pairwise(polyline):
        dx, dy = bx - ax, by - ay
        length = dx * dx + dy * dy
        t = (
            0.0
            if length == 0
            else max(0.0, min(1.0, ((point[0] - ax) * dx + (point[1] - ay) * dy) / length))
        )
        best = min(best, math.hypot(point[0] - ax - t * dx, point[1] - ay - t * dy))
    return best


def _barb_angles(head: list[Point]) -> list[float]:
    """Directions of both barbs from the tip, in degrees, smallest first."""
    tip = head[1]
    return sorted(
        math.degrees(math.atan2(barb[1] - tip[1], barb[0] - tip[0])) % 360
        for barb in (head[0], head[2])
    )


def _plot(module, **options):
    return (
        module.ggplot()
        + module.geom_curve(x=0, y=0, xend=4, yend=1, **options)
        + module.geom_point(x=-1, y=-1, alpha=0)
        + module.geom_point(x=5, y=2, alpha=0)
        + module.theme_void()
    )


@pytest.mark.parametrize(
    "options",
    [
        {},
        {"curvature": -0.4, "ncp": 1},
        {"curvature": 1.0, "angle": 30, "ncp": 10},
        {"curvature": 0.7, "angle": 150},
        {"curvature": 0.5, "angle": -60},
        {"curvature": 0.3, "spacer": 20},
    ],
)
def test_curve_matches_lets_plot(options: dict) -> None:
    theirs = _stroked_paths(_plot(lp, **options).to_svg())
    ours = _stroked_paths(_plot(gg, **options).to_svg())
    assert len(theirs) == len(ours) == 1
    polyline = _polyline(ours[0])
    samples = _letsplot_samples(theirs[0])
    shift = (polyline[0][0] - samples[0][0], polyline[0][1] - samples[0][1])
    assert math.hypot(*shift) < 9, "panels moved apart"
    moved = [(x + shift[0], y + shift[1]) for x, y in samples]
    assert math.dist(moved[-1], polyline[-1]) < 0.05
    worst = max(_distance_to_polyline(point, polyline) for point in moved)
    assert worst < 0.5, f"{options}: {worst:.3f} px off"


def test_zero_curvature_is_straight() -> None:
    (d,) = _stroked_paths(_plot(gg, curvature=0).to_svg())
    assert len(_polyline(d)) == 2


@pytest.mark.parametrize("ncp", [1, 60])
def test_arrow_heads_aim_like_lets_plot(ncp: int) -> None:
    """Heads aim at the control vertex one head length back (with 60 control points,
    vertices are closer than that), so the barbs point where lets-plot's do."""
    for ends in ("last", "both"):
        theirs_svg = _plot(lp, curvature=0.8, ncp=ncp, arrow=lp.arrow(ends=ends)).to_svg()
        ours_svg = _plot(gg, curvature=0.8, ncp=ncp, arrow=gg.arrow(ends=ends)).to_svg()
        theirs = [_polyline(d) for d in _stroked_paths(theirs_svg) if len(_polyline(d)) == 3]
        ours = [_polyline(d) for d in _stroked_paths(ours_svg) if len(_polyline(d)) == 3]
        assert len(theirs) == len(ours) == (1 if ends == "last" else 2)
        # Barb directions (unit vectors from the tip) agree within a degree.
        for their_head in theirs:
            our_head = min(ours, key=lambda head: math.dist(head[1], their_head[1]))
            assert _barb_angles(our_head) == pytest.approx(_barb_angles(their_head), abs=1.0)


@pytest.mark.parametrize(
    ("options", "message"),
    [
        ({"ncp": -1}, r"ncp.*whole number of control points"),
        ({"ncp": 2.5}, r"ncp.*whole number of control points"),
        ({"spacer": -3}, r"spacer.*non-negative"),
        ({"curvature": "big"}, r"curvature"),
    ],
)
def test_bad_options_name_the_parameter(options: dict, message: str) -> None:
    with pytest.raises(SpecError, match=message):
        _plot(gg, **options).to_svg()
