"""Seeded jitters must put every point where lets-plot does (Kotlin's Random).

Point centres are compared as fractions of the panel, in drawing order. Without
a seed lets-plot reseeds every render; ggrust uses a fixed default seed so a
plot renders the same every time (`test_default_seed_is_reproducible`).
"""

import re
import warnings

import pytest
from test_bar_parity import _attributes

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

DATA = {
    "g": list("aaaabbbbbccc") * 2,
    "h": list("uv") * 12,
    "y": [1.0, 1.5, 2.0, 2.0, 3.0, 3.5, 3.0, 4.0, 5.0, 2.0, 2.5, 6.0] * 2,
    "x": [1.0, 2.0, 2.0, 3.0] * 6,
}


def _lets_plot_points(svg):
    clip = re.search(r'<rect x="[\d.]+" y="0\.0" width="([\d.]+)" height="([\d.]+)">', svg)
    width, height = float(clip.group(1)), float(clip.group(2))
    return [
        (float(a["cx"]) / width, float(a["cy"]) / height)
        for a in map(_attributes, re.findall(r"<circle\b([^>]*)>", svg))
    ]


def _ggrust_points(svg):
    clip = _attributes(re.search(r"<clipPath[^>]*>\s*<rect([^>]*)/>", svg).group(1))
    left, top = float(clip["x"]), float(clip["y"])
    width, height = float(clip["width"]), float(clip["height"])
    panel = svg[svg.index('<g class="panel"') :]
    panel = panel[: panel.index('<g class="axis-x"')]
    return [
        ((float(a["cx"]) - left) / width, (float(a["cy"]) - top) / height)
        for a in map(_attributes, re.findall(r"<circle\b([^>]*)>", panel))
    ]


CASES = {
    "geom_jitter_named_position": lambda m: (
        m.ggplot(DATA, m.aes("g", "y")) + m.geom_jitter(seed=3, position="jitter")
    ),
    "geom_jitter_merged_position": lambda m: (
        m.ggplot(DATA, m.aes("g", "y"))
        + m.geom_jitter(seed=3, position=m.position_jitter(width=0.2))
    ),
    "geom_jitter_composed": lambda m: (
        m.ggplot(DATA, m.aes("g", "y"))
        + m.geom_jitter(width=0.1, seed=3, position=m.position_jitter(width=0.3, seed=9))
    ),
    "geom_jitter_then_dodge": lambda m: (
        m.ggplot(DATA, m.aes("g", "y", color="h"))
        + m.geom_jitter(width=0.1, height=0, seed=4, position=m.position_dodge(width=0.7))
    ),
    "jitter_default_size": lambda m: (
        m.ggplot(DATA, m.aes("g", "y")) + m.geom_point(position=m.position_jitter(seed=42))
    ),
    "jitter_width_height": lambda m: (
        m.ggplot(DATA, m.aes("x", "y"))
        + m.geom_point(position=m.position_jitter(width=0.2, height=0.3, seed=7))
    ),
    "geom_jitter": lambda m: (
        m.ggplot(DATA, m.aes("g", "y")) + m.geom_jitter(width=0.25, height=0, seed=123)
    ),
    "jitterdodge": lambda m: (
        m.ggplot(DATA, m.aes("g", "y", color="h"))
        + m.geom_point(position=m.position_jitterdodge(seed=5))
    ),
    "jitterdodge_options": lambda m: (
        m.ggplot(DATA, m.aes("g", "y", color="h"))
        + m.geom_point(
            position=m.position_jitterdodge(
                dodge_width=0.6, jitter_width=0.1, jitter_height=0.2, seed=99
            )
        )
    ),
    "large_seed": lambda m: (
        m.ggplot(DATA, m.aes("g", "y"))
        + m.geom_point(position=m.position_jitter(seed=2**40 + 12345))
    ),
}


@pytest.mark.parametrize("name", sorted(CASES))
def test_jitter_matches_lets_plot(name):
    expected, actual = (
        points((CASES[name](m) + m.theme(legend_position="none")).to_svg())
        for m, points in ((lp, _lets_plot_points), (gg, _ggrust_points))
    )
    assert len(actual) == len(expected) == len(DATA["y"])
    assert [value for point in actual for value in point] == pytest.approx(
        [value for point in expected for value in point], abs=1e-3
    )


def test_default_seed_is_reproducible():
    """Deliberate difference: lets-plot jitters differently on every render."""
    plot = gg.ggplot(DATA, gg.aes("g", "y")) + gg.geom_jitter()
    first = _ggrust_points(plot.to_svg())
    assert first == _ggrust_points(plot.to_svg())
    assert first != _ggrust_points((gg.ggplot(DATA, gg.aes("g", "y")) + gg.geom_point()).to_svg())
