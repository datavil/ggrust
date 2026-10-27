"""Violins must match lets-plot: bands, sides, quantile lines, axis text.

Every path point is compared as a fraction of the panel, in drawing order. Cases
give `bw` as a number: the default bandwidth follows R's `bw.nrd0` in ggrust and
differs from lets-plot's on purpose (`test_default_bandwidth_follows_r`).
"""

import re
import shutil
import subprocess
import warnings

import pytest
from test_bar_parity import _attributes, _hex, _texts

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

DATA = {
    "g": ["a"] * 7 + ["b"] * 9,
    "h": list("uvuvuvu") + list("uuvvuvuvv"),
    "y": [1.0, 2, 3, 4, 5, 6, 9, -1, 2, 2.5, 3, 3, 4, 6, 7, 8],
    "w": [1.0, 2, 1, 1, 3, 1, 1, 1, 1, 2, 1, 1, 1, 1, 0.5, 1],
}
NUMBER = re.compile(r"-?\d+(?:\.\d+)?(?:e-?\d+)?")


def _points(d: str) -> list[tuple[float, float]]:
    numbers = [float(value) for value in NUMBER.findall(d)]
    return list(zip(numbers[::2], numbers[1::2], strict=True))


def _shape(kind, a, left, top, width, height):
    if kind == "line":
        points = [(float(a["x1"]), float(a["y1"])), (float(a["x2"]), float(a["y2"]))]
    else:
        points = _points(a["d"])
    closed = kind == "path" and "Z" in a.get("d", "")
    relative = [((x - left) / width, (y - top) / height) for x, y in points]
    fill = _hex(a.get("fill")) if closed else None
    stroke = None if closed else (_hex(a.get("stroke")), round(float(a.get("stroke-width", 0)), 3))
    opacity = round(float(a.get("fill-opacity", 1)), 2) if closed else None
    return closed, relative, (fill, opacity, stroke)


def _lets_plot_shapes(svg):
    clip = re.search(r'<rect x="[\d.]+" y="0\.0" width="([\d.]+)" height="([\d.]+)">', svg)
    width, height = float(clip.group(1)), float(clip.group(2))
    body = svg[svg.index('clip-path="url(') :]
    shapes = []
    for kind, attributes in re.findall(r"<(path|line|rect)\b([^>]*)>", body):
        if kind == "rect":
            break  # the axes start
        closed, points, paint = _shape(kind, _attributes(attributes), 0.0, 0.0, width, height)
        # lets-plot writes paths as `M p L p ...`: the first point twice.
        if kind == "path" and len(points) > 1 and points[0] == points[1]:
            points = points[1:]
        shapes.append((closed, points, paint))
    return shapes


def _ggrust_shapes(svg):
    clip = _attributes(re.search(r"<clipPath[^>]*>\s*<rect([^>]*)/>", svg).group(1))
    left, top = float(clip["x"]), float(clip["y"])
    width, height = float(clip["width"]), float(clip["height"])
    panel = svg[svg.index('<g class="panel"') :]
    panel = panel[: panel.index('<g class="axis-x"')]
    return [
        _shape("path", _attributes(attributes), left, top, width, height)
        for attributes in re.findall(r"<path\b([^>]*)>", panel)
    ]


def _both(build):
    return [(build(m) + m.theme(legend_position="none")).to_svg() for m in (lp, gg)]


CASES = {
    "basic": lambda m: m.ggplot(DATA, m.aes("g", "y")) + m.geom_violin(bw=0.8),
    "quantile_lines": lambda m: (
        m.ggplot(DATA, m.aes("g", "y"))
        + m.geom_violin(bw=0.8, quantile_lines=True, quantiles=[0.1, 0.5, 0.9])
    ),
    "untrimmed": lambda m: (
        m.ggplot(DATA, m.aes("g", "y")) + m.geom_violin(bw=0.6, trim=False, n=64)
    ),
    "overall_range": lambda m: (
        m.ggplot(DATA, m.aes("g", "y")) + m.geom_violin(bw=0.6, trim=False, tails_cutoff=None, n=64)
    ),
    "scale_count": lambda m: m.ggplot(DATA, m.aes("g", "y")) + m.geom_violin(bw=0.8, scale="count"),
    "scale_width": lambda m: m.ggplot(DATA, m.aes("g", "y")) + m.geom_violin(bw=0.8, scale="width"),
    "styled": lambda m: (
        m.ggplot(DATA, m.aes("g", "y", color="g"))
        + m.geom_violin(bw=0.8, fill="#ffcc00", alpha=0.5, size=1.2, width=0.6, show_half=-1)
    ),
    "kernel": lambda m: (
        m.ggplot(DATA, m.aes("g", "y"))
        + m.geom_violin(bw=1.2, kernel="epanechikov", adjust=0.7, n=40)
    ),
    "weighted": lambda m: m.ggplot(DATA, m.aes("g", "y", weight="w")) + m.geom_violin(bw=0.8),
    "quantile_fill": lambda m: (
        m.ggplot(DATA, m.aes("g", "y", fill="..quantile..")) + m.geom_violin(bw=0.8, n=64)
    ),
    "quantile_tie": lambda m: (
        m.ggplot({"y": [2.0, 4, 6]}, m.aes(y="y"))
        + m.geom_violin(bw=0.8, n=30, quantiles=[0.5], quantile_lines=True)
    ),
    "dodged": lambda m: m.ggplot(DATA, m.aes("g", "y", fill="h")) + m.geom_violin(bw=0.8, n=32),
}


@pytest.mark.parametrize("name", sorted(CASES))
def test_violins_match_lets_plot(name):
    expected_svg, actual_svg = _both(CASES[name])
    expected, actual = _lets_plot_shapes(expected_svg), _ggrust_shapes(actual_svg)
    if name == "dodged":
        # lets-plot draws by x, then group; ggrust by group, then x: same shapes.
        expected, actual = sorted(expected, key=repr), sorted(actual, key=repr)
    assert len(actual) == len(expected), (len(actual), len(expected))
    for got, want in zip(actual, expected, strict=True):
        assert got[0] == want[0] and got[2] == want[2], (got[2], want[2])
        assert len(got[1]) == len(want[1])
        assert [value for point in got[1] for value in point] == pytest.approx(
            [value for point in want[1] for value in point], abs=1e-3
        )
    assert sorted(_texts(actual_svg)) == sorted(_texts(expected_svg))


@pytest.mark.skipif(shutil.which("Rscript") is None, reason="needs R")
def test_default_bandwidth_follows_r():
    """Deliberate difference: R's bw.nrd0 (sample sd, type 7 IQR); lets-plot uses the
    population sd. A trimmed violin's width at its centre pins the bandwidth."""
    values = [1.0, 2, 3, 4, 5, 6, 9]
    script = "x <- c({}); cat(bw.nrd0(x))".format(", ".join(map(str, values)))
    bandwidth = float(
        subprocess.run(["Rscript", "-e", script], capture_output=True, text=True, check=True).stdout
    )
    explicit, default = (
        _ggrust_shapes(
            (
                gg.ggplot({"y": values}, gg.aes(y="y"))
                + gg.geom_violin(**options)
                + gg.theme(legend_position="none")
            ).to_svg()
        )[0][1]
        for options in ({"bw": bandwidth}, {})
    )
    assert default == pytest.approx(explicit, abs=1e-6)
