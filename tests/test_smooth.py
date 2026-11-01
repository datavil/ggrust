"""geom_smooth against lets-plot.

The curve and band paths are compared as fractions of the curve's own extent
(panel sizes differ with real fonts), and the axis labels as sorted texts.
"""

import math
import random
import re
import warnings
from collections.abc import Callable
from types import ModuleType

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.errors import SpecError  # noqa: E402

_RANDOM = random.Random(5)
_X = [_RANDOM.uniform(0, 10) for _ in range(120)]
DATA = {
    "x": _X,
    "y": [math.sin(x) + x * x / 30 + _RANDOM.gauss(0, 0.4) for x in _X],
    "g": ["a" if index % 2 else "b" for index in range(120)],
}
_BIG_X = [_RANDOM.uniform(0, 10) for _ in range(3000)]
BIG = {"x": _BIG_X, "y": [math.cos(x) + _RANDOM.gauss(0, 0.5) for x in _BIG_X]}

Points = list[tuple[float, float]]


def _labels(svg: str) -> list[str]:
    texts = re.findall(r"<text[^>]*>(.*?)</text>", svg, flags=re.DOTALL)
    return sorted(label for text in texts if (label := re.sub(r"<[^>]*>", "", text).strip()))


def _curves(svg: str, stroke: str) -> list[Points]:
    """Vertices of the long unfilled paths with this stroke, in drawing order."""
    found = []
    for tag in re.findall(r"<path\b[^>]*>", svg):
        attributes = dict(re.findall(r'([\w-]+)="([^"]*)"', tag))
        if (
            attributes.get("stroke", "").replace(" ", "") != stroke
            or "pointer-events" in attributes
        ):
            continue
        numbers = [float(n) for n in re.findall(r"-?\d+(?:\.\d+)?(?:e-?\d+)?", attributes["d"])]
        points = list(zip(numbers[0::2], numbers[1::2], strict=True))
        # lets-plot repeats a path's first vertex.
        points = [
            point for index, point in enumerate(points) if index == 0 or point != points[index - 1]
        ]
        if len(points) > 10:
            found.append(points)
    return found


def _relative(points: Points) -> Points:
    xs, ys = [x for x, _ in points], [y for _, y in points]
    width, height = max(xs) - min(xs), (max(ys) - min(ys)) or 1.0
    return [((x - min(xs)) / width, (y - min(ys)) / height) for x, y in points]


Case = Callable[[ModuleType], object]

CASES: dict[str, tuple[Case, str, str]] = {
    "lm": (
        lambda m: m.ggplot(DATA, m.aes("x", "y")) + m.geom_smooth(),
        "#ff00ff",
        "rgb(255,0,255)",
    ),
    "polynomial": (
        lambda m: m.ggplot(DATA, m.aes("x", "y")) + m.geom_smooth(deg=3, level=0.99, color="red"),
        "#ff0000",
        "rgb(255,0,0)",
    ),
    "loess": (
        lambda m: (
            m.ggplot(DATA, m.aes("x", "y")) + m.geom_smooth(method="loess", span=0.3, color="red")
        ),
        "#ff0000",
        "rgb(255,0,0)",
    ),
    # 3000 rows: loess fits lets-plot's seeded sample of 1000.
    "loess_sampled": (
        lambda m: (
            m.ggplot(BIG, m.aes("x", "y")) + m.geom_smooth(method="loess", se=False, color="red")
        ),
        "#ff0000",
        "rgb(255,0,0)",
    ),
}


@pytest.mark.parametrize("name", list(CASES))
def test_curve_matches_lets_plot(name: str) -> None:
    build, ours_stroke, their_stroke = CASES[name]
    ours_svg = build(gg).to_svg()  # type: ignore[attr-defined]
    theirs_svg = build(lp).to_svg()  # type: ignore[attr-defined]
    assert _labels(ours_svg) == _labels(theirs_svg)
    (ours,) = _curves(ours_svg, ours_stroke)
    (theirs,) = _curves(theirs_svg, their_stroke)
    assert len(ours) == len(theirs) == 80
    for (x, y), (their_x, their_y) in zip(_relative(ours), _relative(theirs), strict=True):
        assert x == pytest.approx(their_x, abs=0.005)
        assert y == pytest.approx(their_y, abs=0.005)


def test_band_and_legend_key_per_group() -> None:
    plot = gg.ggplot(DATA, gg.aes("x", "y", color="g")) + gg.geom_smooth()
    svg = plot.to_svg()
    bands = re.findall(r'<path d="M[^"]*" fill="#474747" fill-opacity="0.149"', svg)
    assert len(bands) == 2
    # Keys are lines stroked without the band's alpha.
    keys = re.findall(r'<path d="M[^"]*" fill="none" stroke="#e41a1c" stroke-width="1.65"/>', svg)
    assert len(keys) == 2  # curve and key


def test_se_false_draws_no_band() -> None:
    svg = (gg.ggplot(DATA, gg.aes("x", "y")) + gg.geom_smooth(se=False)).to_svg()
    assert "fill-opacity" not in svg


@pytest.mark.parametrize(
    ("options", "message"),
    [
        ({"method": "gam"}, "not supported"),
        ({"method": "loes"}, "did you mean 'loess'"),
        ({"level": 1.5}, "confidence level"),
        ({"span": 0}, "span"),
        ({"deg": 0}, "at least 1"),
    ],
)
def test_bad_options_raise(options: dict[str, object], message: str) -> None:
    plot = gg.ggplot(DATA, gg.aes("x", "y")) + gg.geom_smooth(**options)  # type: ignore[arg-type]
    with pytest.raises(SpecError, match=message):
        plot.to_svg()
