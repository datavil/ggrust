"""Ridgelines must match lets-plot: bands, top lines, quantile lines, axis text.

Path points are compared as panel fractions in drawing order (extractors from the
violin parity test); cases give `bw` as a number (the default follows R).
"""

import warnings

import pytest
from test_bar_parity import _texts
from test_violin_parity import _ggrust_shapes, _lets_plot_shapes

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

DATA = {
    "g": ["a"] * 6 + ["b"] * 7 + ["c"] * 5,
    "h": list("uvuvuv") + list("uvuvuvu") + list("uvuvu"),
    "v": [1.0, 2, 3, 4, 6, 6.5, 2, 3, 3, 5, 7, 8, 8.5, 0, 1, 1.5, 2, 9],
}

CASES = {
    "basic": lambda m: m.ggplot(DATA, m.aes("v", "g")) + m.geom_area_ridges(bw=0.8),
    "quantile_lines": lambda m: (
        m.ggplot(DATA, m.aes("v", "g"))
        + m.geom_area_ridges(bw=0.8, quantile_lines=True, quantiles=[0.2, 0.5, 0.8])
    ),
    "scale_min_height": lambda m: (
        m.ggplot(DATA, m.aes("v", "g"))
        + m.geom_area_ridges(bw=0.6, scale=1.6, min_height=0.05, n=64)
    ),
    "trimmed": lambda m: (
        m.ggplot(DATA, m.aes("v", "g")) + m.geom_area_ridges(bw=0.8, trim=True, n=64)
    ),
    "tails": lambda m: (
        m.ggplot(DATA, m.aes("v", "g")) + m.geom_area_ridges(bw=0.8, tails_cutoff=1.5, n=64)
    ),
    "styled": lambda m: (
        m.ggplot(DATA, m.aes("v", "g", fill="g"))
        + m.geom_area_ridges(bw=0.8, alpha=0.5, color="black", size=1.2, n=64)
    ),
    "quantile_fill": lambda m: (
        m.ggplot(DATA, m.aes("v", "g", fill="..quantile..")) + m.geom_area_ridges(bw=0.8, n=64)
    ),
    "groups_at_one_level": lambda m: (
        m.ggplot(DATA, m.aes("v", "g", fill="h")) + m.geom_area_ridges(bw=0.8, n=48, alpha=0.5)
    ),
    "quantile_lines_per_run": lambda m: (
        m.ggplot(DATA, m.aes("v", "g"))
        + m.geom_area_ridges(bw=0.8, n=48, min_height=0.3, quantile_lines=True)
    ),
    "numeric_y": lambda m: (
        m.ggplot({"v": DATA["v"], "y": [1.0] * 6 + [3.0] * 7 + [4.0] * 5}, m.aes("v", "y"))
        + m.geom_area_ridges(bw=0.8, n=48)
    ),
}


def _assert_same_band(got, want):
    """Bands are simplified in pixels (Douglas-Peucker, 0.25 px); the panels differ by
    a pixel (fonts), so a point near the threshold may stay on one side only. Both
    are subsets of one outline: the shorter must lie on the longer."""
    shorter, longer = sorted((got, want), key=len)
    assert len(longer) - len(shorter) <= 2, (len(got), len(want))
    for x, y in shorter:
        assert min(abs(x - u) + abs(y - v) for u, v in longer) < 2e-3, (x, y)


@pytest.mark.parametrize("name", sorted(CASES))
def test_ridges_match_lets_plot(name):
    expected_svg, actual_svg = (
        (CASES[name](m) + m.theme(legend_position="none")).to_svg() for m in (lp, gg)
    )
    expected, actual = _lets_plot_shapes(expected_svg), _ggrust_shapes(actual_svg)
    assert len(actual) == len(expected), (len(actual), len(expected))
    for got, want in zip(actual, expected, strict=True):
        assert got[0] == want[0] and got[2] == want[2], (got[2], want[2])
        if got[0]:
            _assert_same_band(got[1], want[1])
            continue
        assert len(got[1]) == len(want[1]), (len(got[1]), len(want[1]))
        assert [value for point in got[1] for value in point] == pytest.approx(
            [value for point in want[1] for value in point], abs=1e-3
        )
    assert sorted(_texts(actual_svg)) == sorted(_texts(expected_svg))
