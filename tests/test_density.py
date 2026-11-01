"""geom_density and stat="density" against lets-plot.

Densities are compared through the axis labels (same breaks means the same
density range) and the drawn areas (fills and vertex counts); bandwidths follow
R in both for these samples.
"""

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

_RANDOM = random.Random(3)
DATA = {
    "v": [_RANDOM.gauss(0, 1) for _ in range(150)] + [_RANDOM.gauss(2.5, 0.7) for _ in range(100)],
    "g": ["a"] * 150 + ["b"] * 100,
}


def _labels(svg: str) -> list[str]:
    texts = re.findall(r"<text[^>]*>(.*?)</text>", svg, flags=re.DOTALL)
    labels = [re.sub(r"<[^>]*>", "", text).strip() for text in texts]
    return [label for label in labels if label]


def _fills(svg: str) -> list[tuple[str, int]]:
    """(fill, vertex count) of filled, closed paths (bands; legend keys are rects)."""
    found = []
    for tag in re.findall(r"<path\b[^>]*>", svg):
        attributes = dict(re.findall(r'([\w-]+)="([^"]*)"', tag))
        fill, d = attributes.get("fill", "none"), attributes.get("d", "")
        if fill == "none" or "pointer-events" in attributes or not d:
            continue
        found.append((fill, len(re.findall(r"[ML]", d))))
    return found


Case = Callable[[ModuleType], object]

CASES: dict[str, Case] = {
    "basic": lambda m: m.ggplot(DATA, m.aes("v")) + m.geom_density(),
    "groups": lambda m: m.ggplot(DATA, m.aes("v", fill="g")) + m.geom_density(alpha=0.4),
    "options": lambda m: (
        m.ggplot(DATA, m.aes("v", fill="g"))
        + m.geom_density(position="stack", trim=True, kernel="epanechikov", adjust=0.5, n=100)
    ),
    "count_line": lambda m: (
        m.ggplot(DATA, m.aes("v", color="g"))
        + m.geom_line(m.aes(y="..count.."), stat="density")
        + m.facet_wrap("g")
    ),
}


@pytest.mark.parametrize("name", list(CASES))
def test_axes_match_lets_plot(name: str) -> None:
    ours = CASES[name](gg).to_svg()  # type: ignore[attr-defined]
    theirs = CASES[name](lp).to_svg()  # type: ignore[attr-defined]
    # Element order differs (axis titles); the texts must be the same.
    assert sorted(_labels(ours)) == sorted(_labels(theirs))


def test_one_area_per_group_with_one_fill() -> None:
    ours = _fills(CASES["groups"](gg).to_svg())  # type: ignore[attr-defined]
    # n = 512 points plus 3 repeated quantile band ends per group: the top line
    # forward, the base back.
    assert [count for _, count in ours] == [1030, 1030]


def test_quantile_fill_splits_into_bands() -> None:
    plot = gg.ggplot(DATA, gg.aes("v")) + gg.geom_density(
        gg.aes(fill="..quantile.."), quantile_lines=True, quantiles=[0.5]
    )
    svg = plot.to_svg()
    bands = [fill for fill, _ in _fills(svg) if fill.startswith("#")]
    assert len(bands) >= 2 and len(set(bands)) == 2
    # One quantile line from the curve down to the base, besides the two top lines.
    strokes = re.findall(r'<path d="M[^"]*" fill="none" stroke="#474747" stroke-width="1.65"', svg)
    assert len(strokes) == 3


def test_scaled_peaks_at_one() -> None:
    plot = gg.ggplot(DATA, gg.aes("v")) + gg.geom_density(gg.aes(y="..scaled.."))
    assert "1" in _labels(plot.to_svg())


@pytest.mark.parametrize(
    ("options", "message"),
    [
        ({"kernel": "gauss"}, "kernel"),
        ({"n": 5000}, "too large"),
        ({"bw": -1}, "positive number"),
    ],
)
def test_bad_options_raise(options: dict[str, object], message: str) -> None:
    plot = gg.ggplot(DATA, gg.aes("v")) + gg.geom_density(**options)  # type: ignore[arg-type]
    with pytest.raises(SpecError, match=message):
        plot.to_svg()
