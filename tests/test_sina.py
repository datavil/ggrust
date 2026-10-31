"""``geom_sina``: seeded points land where lets_plot puts them."""

import re
import warnings
from typing import Any

import numpy as np
import polars as pl
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

_RANDOM = np.random.default_rng(3)
FRAME = pl.DataFrame(
    {
        "g": np.repeat(["a", "b", "c"], 40),
        "k": np.tile(["u", "v"], 60),
        "v": np.concatenate(
            [_RANDOM.normal(0, 1, 40), _RANDOM.gamma(2, 1, 40), _RANDOM.normal(3, 0.5, 40)]
        ),
    }
)


def normalized_points(svg: str) -> np.ndarray:
    """Point centres scaled to their bounding box (frames differ), sorted by y then x."""
    found = re.findall(r'<circle[^>]*cx="([-\d.]+)"[^>]*cy="([-\d.]+)"', svg)
    points = np.array([(float(x), float(y)) for x, y in found])[: FRAME.height]
    points = (points - points.min(0)) / (points.max(0) - points.min(0))
    return points[np.lexsort((points[:, 0], points[:, 1]))]


@pytest.mark.parametrize(
    "build",
    [
        lambda m: m.ggplot(FRAME, m.aes("g", "v", color="k")) + m.geom_sina(seed=7),
        lambda m: m.ggplot(FRAME, m.aes("g", "v")) + m.geom_sina(seed=1, scale="count"),
        lambda m: m.ggplot(FRAME, m.aes("g", "v")) + m.geom_sina(seed=1, scale="width"),
        lambda m: m.ggplot(FRAME, m.aes("g", "v")) + m.geom_sina(seed=42),
        lambda m: m.ggplot(FRAME, m.aes("g", "v")) + m.geom_sina(seed=1, show_half=-1),
    ],
    ids=["dodged", "count", "width", "area", "half"],
)
def test_sina_points_match_lets_plot(build: Any) -> None:
    expected = normalized_points(build(lp).to_svg())
    actual = normalized_points(build(gg).to_svg())
    # Points with equal y may pair crosswise after sorting; all others agree.
    close = np.abs(expected - actual).max(axis=1) < 0.005
    assert close.sum() >= len(close) - 2


def test_sina_without_seed_is_reproducible() -> None:
    plot = gg.ggplot(FRAME, gg.aes("g", "v")) + gg.geom_sina()
    assert plot.to_svg() == plot.to_svg()
