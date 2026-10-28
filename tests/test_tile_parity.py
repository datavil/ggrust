"""Tiles and rects must match lets-plot: rectangles, paint, axis labels, titles.

Rectangles are compared as fractions of the panel, in drawing order, legends off
(as in `test_bar_parity.py`).
"""

import re
import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from test_bar_parity import _attributes, _ggrust_bars, _hex, _texts  # noqa: E402

GRID = {
    "x": [0.0, 1, 2, 0, 1, 2],
    "y": [0.0, 0, 0, 1, 1, 1],
    "v": [1.0, 2, 3, 4, 5, 6],
    "w": [0.5, 1, 0.8, 1, 1, 0.3],
    "g": list("ppqqrr"),
}
SPARSE = {"x": [0.0, 2, 6], "y": [1.0, 4, 5], "v": [3.0, 1, 2]}
CELLS = {
    "gene": [gene for gene in ("CD3E", "MS4A1", "LYZ", "NKG7") for _ in range(3)],
    "cell": ["T", "B", "Mono"] * 4,
    "v": [2.1, 0.1, 0.3, 0.0, 1.8, 0.2, 0.1, 0.4, 2.6, 0.9, 0.0, 0.3],
}
CORNERS = {
    "a": [0.0, 2, 1.5],
    "b": [1.0, 3, 2.5],
    "c": [0.0, 1, 2],
    "e": [1.0, 4, 2.5],
    "g": ["p", "q", "p"],
}


def _lets_plot_rects(svg: str) -> list[tuple]:
    """Data-layer rectangles relative to the panel (lets-plot translates to it)."""
    clip = re.search(r'<rect x="[\d.]+" y="0\.0" width="([\d.]+)" height="([\d.]+)">', svg)
    width, height = float(clip.group(1)), float(clip.group(2))
    layer = svg[svg.index('buffered-rendering="static"') : clip.start()]
    rects = []
    for tag in re.findall(r"<rect [^>]*>", layer):
        a = _attributes(tag)
        rects.append(
            (
                float(a["x"]) / width,
                float(a["y"]) / height,
                float(a["width"]) / width,
                float(a["height"]) / height,
                _hex(a["fill"]),
                round(float(a.get("fill-opacity", 1)), 2),
                round(float(a["stroke-width"]), 3),
            )
        )
    return rects


CASES = {
    "tile": lambda m: m.ggplot(GRID, m.aes("x", "y")) + m.geom_tile(),
    "tile_fill": lambda m: m.ggplot(GRID, m.aes("x", "y", fill="v")) + m.geom_tile(),
    "tile_discrete_fill": lambda m: m.ggplot(GRID, m.aes("x", "y", fill="g")) + m.geom_tile(),
    "tile_outline": lambda m: (
        m.ggplot(GRID, m.aes("x", "y")) + m.geom_tile(color="red", size=1, width=0.5)
    ),
    "tile_mapped_width": lambda m: (
        m.ggplot(GRID, m.aes("x", "y")) + m.geom_tile(m.aes(width="w"), height=0.5)
    ),
    "tile_identity_units": lambda m: (
        m.ggplot(GRID, m.aes("x", "y"))
        + m.geom_tile(width=0.5, height=0.25, width_unit="identity", height_unit="identity")
    ),
    "tile_sparse": lambda m: m.ggplot(SPARSE, m.aes("x", "y", fill="v")) + m.geom_tile(),
    "tile_alpha": lambda m: m.ggplot(GRID, m.aes("x", "y")) + m.geom_tile(alpha=0.3),
    "heatmap": lambda m: (
        m.ggplot(CELLS, m.aes("gene", "cell", fill="v")) + m.geom_tile(color="white", size=0.5)
    ),
    "rect": lambda m: (
        m.ggplot(CORNERS) + m.geom_rect(m.aes(xmin="a", xmax="b", ymin="c", ymax="e"))
    ),
    "rect_fill": lambda m: (
        m.ggplot(CORNERS)
        + m.geom_rect(m.aes(xmin="a", xmax="b", ymin="c", ymax="e", fill="g"), alpha=0.5)
    ),
    "rect_constant_corners": lambda m: (
        m.ggplot(GRID, m.aes("x", "y"))
        + m.geom_rect(xmin=0.5, xmax=1.5, ymin=0.2, ymax=0.8, fill="blue")
        + m.geom_point()
    ),
}


def _both(build):
    return [(build(module) + module.theme(legend_position="none")).to_svg() for module in (lp, gg)]


@pytest.mark.parametrize("name", sorted(CASES))
def test_rectangles_match_lets_plot(name):
    expected_svg, actual_svg = _both(CASES[name])
    expected = _lets_plot_rects(expected_svg)
    actual = [(*bar[:6], bar[7]) for bar in _ggrust_bars(actual_svg)]
    assert len(actual) == len(expected), (actual, expected)
    for got, want in zip(actual, expected, strict=True):
        assert got[:4] == pytest.approx(want[:4], abs=1e-3), (got, want)
        assert got[4:] == want[4:], (got, want)
    assert sorted(_texts(actual_svg)) == sorted(_texts(expected_svg))
