"""Colorbars must match lets-plot: segment colors and lengths, tick positions, labels.

Both SVGs are read into ``[(title, segments, ticks, labels), ...]`` per colorbar, with
segments as ``(fill, height)`` and ticks as y offsets from the bar top (within 0.02 px:
ggrust writes 2 decimals; the left-edge tick of each break). Positions on the page are not
compared (ggrust measures text with the real font).
"""

import re
import warnings

import numpy as np
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402


def _hex(fill: str) -> str:
    rgb = re.match(r"rgb\((\d+),(\d+),(\d+)\)", fill)
    return "#{:02x}{:02x}{:02x}".format(*map(int, rgb.groups())) if rgb else fill.lower()


def _theirs(svg: str) -> list:
    result = []
    for part in svg.split('class="legend-title"')[1:]:
        title = re.search(r"<tspan>([^<]*)</tspan>", part).group(1)
        # Bar segments; a box background has a stroke color, segments do not.
        rects = re.findall(
            r'<rect x="0.0" y="([\d.]+)" height="([\d.]+)" width="[\d.]+" '
            r'stroke-width[^>]*fill="([^"]*)"',
            part,
        )
        if not rects:
            continue
        top = float(rects[0][0])
        segments = [(_hex(fill), round(float(height), 2)) for _, height, fill in rects]
        ticks = [float(y) - top for y in re.findall(r'<line x1="0.0" y1="([\d.]+)"', part)]
        labels = re.findall(r'class="legend-item"[^>]*>\s*<tspan>([^<]*)<', part)
        result.append((title, segments, ticks, labels))
    return result


def _ours(svg: str) -> list:
    result = []
    for part in svg.split('<g class="legend">')[1:]:
        texts = re.findall(r'<text[^>]*font-size="(\d+)"[^>]*>([^<]*)</text>', part)
        title = next((text for size, text in texts if size == "15"), "")
        rects = re.findall(
            r'<rect x="[\d.]+" y="([\d.]+)" width="[\d.]+" height="([\d.]+)" fill="([^"]*)"', part
        )
        if not rects:
            continue
        top = float(rects[0][0])
        segments = [(fill.lower(), round(float(height), 2)) for _, height, fill in rects]
        # Ticks: one path per edge; keep the left-edge ones (x == bar x), in order.
        paths = re.findall(r'<path d="M([\d.]+) ([\d.]+)L', part)
        left = min(float(x) for x, _ in paths) if paths else 0
        ticks = [float(y) - top for x, y in paths if float(x) == left]
        labels = [text for size, text in texts if size == "13"]
        result.append((title, segments, ticks, labels))
    return result


RANDOM = np.random.default_rng(9)
N = 60
FRAME = {
    "x": list(RANDOM.normal(size=N)),
    "y": list(RANDOM.normal(size=N)),
    "v": list(RANDOM.normal(size=N) * 3),
    "e": list(RANDOM.uniform(0, 1500, N)),
    "c": [["a", "b"][index % 2] for index in range(N)],
}


def point(m, **mapping):
    return m.ggplot(FRAME, m.aes("x", "y", **mapping)) + m.geom_point()


CASES = {
    "default": lambda m: point(m, color="v"),
    "gradient": lambda m: point(m, color="v") + m.scale_color_gradient(low="red", high="blue"),
    "gradient2": lambda m: point(m, color="v") + m.scale_color_gradient2(),
    "gradient2_midpoint": lambda m: (
        point(m, color="e")
        + m.scale_color_gradient2(low="blue", mid="grey", high="red", midpoint=500)
    ),
    "gradientn": lambda m: (
        point(m, color="v") + m.scale_color_gradientn(colors=["blue", "white", "red", "black"])
    ),
    "viridis": lambda m: point(m, color="e") + m.scale_color_viridis(),
    "magma_cut_reversed": lambda m: (
        point(m, color="e") + m.scale_color_viridis(option="A", begin=0.2, end=0.9, direction=-1)
    ),
    "turbo_alpha": lambda m: point(m, color="e") + m.scale_color_viridis(option="turbo", alpha=0.5),
    "continuous": lambda m: point(m, color="v") + m.scale_color_continuous(low="white", high="red"),
    "limits_breaks_labels": lambda m: (
        point(m, color="v")
        + m.scale_color_gradient(limits=[-5, 5], breaks=[-5, 0, 5], labels=["low", "zero", "high"])
    ),
    "hue_continuous": lambda m: point(m, color="v") + m.scale_color_hue(),
    "manual_continuous": lambda m: (
        point(m, color="v") + m.scale_color_manual(values=["red", "blue"])
    ),
    "nbin": lambda m: point(m, color="e") + m.guides(color={"name": "colorbar", "nbin": 4}),
    "two_colorbars": lambda m: (
        m.ggplot(FRAME, m.aes("x", "y"))
        + m.geom_point(m.aes(color="v"))
        + m.geom_point(m.aes(color="e"), size=1)
    ),
    "multiline_labels": lambda m: (
        point(m, color="v")
        + m.scale_color_gradient(breaks=[-6, -3, 0, 3], labels=["a\nb", "c\nd", "e\nf", "g\nh"])
    ),
    "scale_guide_options": lambda m: (
        point(m, color="e") + m.scale_color_gradient(guide=m.guide_colorbar(nbin=3, title="E"))
    ),
    "guide_colorbar_function": lambda m: (
        point(m, color="e") + m.guides(color=m.guide_colorbar(barheight=200, nbin=6))
    ),
    "colorbar_and_legend": lambda m: point(m, color="v", size="c"),
}


@pytest.mark.parametrize("name", CASES)
def test_colorbars_match_lets_plot(name):
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        theirs = _theirs(CASES[name](lp).to_svg())
        ours = _ours(CASES[name](gg).to_svg())
    assert [(title, segments, labels) for title, segments, _, labels in ours] == [
        (title, segments, labels) for title, segments, _, labels in theirs
    ]
    for (*_, our_ticks, _), (*_, their_ticks, _) in zip(ours, theirs, strict=True):
        assert our_ticks == pytest.approx(their_ticks, abs=0.02)
