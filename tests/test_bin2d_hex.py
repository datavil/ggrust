"""geom_bin2d and geom_hex against lets-plot.

Each bin's fill (its count through the color scale) and the plot's texts (axis labels,
legend titles and breaks) are compared. Pixel positions differ by a few px (real
fonts move the panel), so tile and hexagon sizes are compared relative to each other.
"""

import inspect
import re
import warnings
from collections import Counter
from collections.abc import Callable
from types import ModuleType

import numpy as np
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.errors import SpecError  # noqa: E402

RANDOM = np.random.default_rng(7)
NORMAL = {
    "x": RANDOM.normal(size=400).tolist(),
    "y": (RANDOM.normal(size=400) * 3 + 10).tolist(),
    "g": RANDOM.choice(["a", "b"], size=400).tolist(),
    "w": RANDOM.uniform(0, 2, size=400).tolist(),
}
# Integer data puts many points on bin and hexagon borders.
INTEGERS = {"x": [x % 7 for x in range(60)], "y": [(x * 3) % 5 for x in range(60)]}


def _color(text: str) -> str:
    rgb = re.fullmatch(r"rgb\((\d+),(\d+),(\d+)\)", text.replace(" ", ""))
    if rgb:
        return "#{:02x}{:02x}{:02x}".format(*map(int, rgb.groups()))
    return text.lower()


def _shapes(svg: str) -> list[tuple[str, float, float]]:
    """Fill, width and height of every data tile or hexagon.

    Tiles are the rectangles of the most common size (one size per layer), which
    leaves out the background and the color bar.
    """
    rects = []
    for rect in re.findall(r"<rect\b[^>]*>", svg):
        fill = re.search(r'fill="([^"]+)"', rect)
        width = re.search(r'\swidth="([-\d.e]+)"', rect)
        height = re.search(r'height="([-\d.e]+)"', rect)
        # Color bar slices are a few px thin; one fill can sit on a parent group.
        if width and height and min(float(width.group(1)), float(height.group(1))) > 4:
            color = _color(fill.group(1)) if fill else ""
            rects.append((color, float(width.group(1)), float(height.group(1))))
    sizes = Counter((round(width, 1), round(height, 1)) for _, width, height in rects)
    shapes = []
    if sizes:
        size, count = sizes.most_common(1)[0]
        if count > 1:
            shapes = [rect for rect in rects if (round(rect[1], 1), round(rect[2], 1)) == size]
    for path in re.findall(r"<path\b[^>]*>", svg):
        data = re.search(r'\bd="(M[^"]+)"', path)
        fill = re.search(r'fill="([^"]+)"', path)
        if not data or not fill:
            continue
        points = {
            (float(x), float(y)) for x, y in re.findall(r"([-\d.]+)[ ,]([-\d.]+)", data.group(1))
        }
        if len(points) == 6:
            xs = [x for x, _ in points]
            ys = [y for _, y in points]
            shapes.append((_color(fill.group(1)), max(xs) - min(xs), max(ys) - min(ys)))
    return shapes


def _texts(svg: str) -> list[str]:
    found = re.findall(r"<text[^>]*>(.*?)</text>", svg, re.S)
    return sorted(re.sub(r"<[^>]+>|\s", "", text) for text in found)


CASES: dict[str, Callable[[ModuleType], object]] = {
    "bin2d_default": lambda m: m.ggplot(NORMAL, m.aes("x", "y")) + m.geom_bin2d(),
    "bin2d_bins": lambda m: m.ggplot(NORMAL, m.aes("x", "y")) + m.geom_bin2d(bins=[7, 4]),
    "bin2d_binwidth": lambda m: (
        m.ggplot(NORMAL, m.aes("x", "y")) + m.geom_bin2d(binwidth=[0.5, None], bins=[3, 6])
    ),
    "bin2d_keep_empty": lambda m: (
        m.ggplot(NORMAL, m.aes("x", "y")) + m.geom_bin2d(bins=[6, 5], drop=False)
    ),
    "bin2d_density": lambda m: (
        m.ggplot(NORMAL, m.aes("x", "y")) + m.geom_bin2d(m.aes(fill="..density.."), bins=[5, 5])
    ),
    "bin2d_weight": lambda m: (
        m.ggplot(NORMAL, m.aes("x", "y", weight="w")) + m.geom_bin2d(bins=[5, 5])
    ),
    "bin2d_integers": lambda m: m.ggplot(INTEGERS, m.aes("x", "y")) + m.geom_bin2d(bins=[4, 3]),
    "hex_default": lambda m: m.ggplot(NORMAL, m.aes("x", "y")) + m.geom_hex(),
    "hex_bins": lambda m: m.ggplot(NORMAL, m.aes("x", "y")) + m.geom_hex(bins=[6, 5]),
    "hex_binwidth": lambda m: m.ggplot(NORMAL, m.aes("x", "y")) + m.geom_hex(binwidth=[0.8, 2.5]),
    "hex_keep_empty": lambda m: (
        m.ggplot(NORMAL, m.aes("x", "y")) + m.geom_hex(bins=[4, 4], drop=False)
    ),
    "hex_integers": lambda m: m.ggplot(INTEGERS, m.aes("x", "y")) + m.geom_hex(bins=[5, 4]),
    "hex_vertex_points": lambda m: (
        m.ggplot({"x": [0, 5, 0, 5], "y": [0, 3, 2, 2]}, m.aes("x", "y"))
        + m.geom_text(m.aes(label="..count.."), stat="binhex", bins=[2, 2], drop=False)
    ),
    "hex_one_column": lambda m: m.ggplot(NORMAL, m.aes("x", "y")) + m.geom_hex(bins=[1, 4]),
    "hex_identity_units": lambda m: (
        m.ggplot(NORMAL, m.aes("x", "y"))
        + m.geom_hex(bins=[5, 5], width=0.4, height=1.0, width_unit="identity")
    ),
}


@pytest.mark.parametrize("name", CASES)
def test_matches_lets_plot(name: str) -> None:
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        ours = CASES[name](gg).to_svg()
    theirs = CASES[name](lp).to_svg()
    our_shapes = _shapes(ours)
    their_shapes = _shapes(theirs)
    assert sorted(fill for fill, _, _ in our_shapes) == sorted(fill for fill, _, _ in their_shapes)
    assert _texts(ours) == _texts(theirs)
    if not their_shapes:  # counts drawn as text only
        return
    # Shape proportions: all bins of one layer share one size, panels differ by a few px.
    our_ratio = our_shapes[0][2] / our_shapes[0][1]
    their_ratio = their_shapes[0][2] / their_shapes[0][1]
    assert our_ratio == pytest.approx(their_ratio, rel=0.03)


def test_signatures_match_lets_plot() -> None:
    for name in ("geom_bin2d", "geom_hex"):
        ours = list(inspect.signature(getattr(gg, name)).parameters)
        theirs = list(inspect.signature(getattr(lp, name)).parameters)
        assert ours == theirs


def test_spec_matches_lets_plot() -> None:
    for build in (
        lambda m: m.geom_bin2d(),
        lambda m: m.geom_bin2d(bins=[10, 5], binwidth=[1, None], drop=False, fill="red"),
        lambda m: m.geom_hex(),
        lambda m: m.geom_hex(bins=[10, 5], width_unit="identity", height_unit="res"),
    ):
        assert build(gg).as_dict() == build(lp).as_dict()


def test_one_bin_along_y_spans_the_bin() -> None:
    # lets-plot #1290 draws a 1-unit-tall tile (data resolution); the bin is taller.
    data = {"x": [0.0, 1.0, 2.0, 3.0], "y": [0.0, 10.0, 20.0, 30.0]}
    svg = (gg.ggplot(data, gg.aes("x", "y")) + gg.geom_bin2d(bins=[4, 1])).to_svg()
    shapes = _shapes(svg)
    assert len(shapes) == 4
    # The y labels 0 and 30 lie inside every tile.
    label_y = {
        text: float(y)
        for y, text in re.findall(r'<text[^>]*\by="([-\d.]+)"[^>]*>(0|30)</text>', svg)
    }
    assert shapes[0][2] > abs(label_y["0"] - label_y["30"])


@pytest.mark.parametrize("stat", ["bin2d", "binhex"])
def test_counts_sum_to_the_row_count(stat: str) -> None:
    base = gg.ggplot(NORMAL, gg.aes("x", "y")) + gg.geom_hex(bins=[6, 5])
    labelled = base + gg.geom_text(gg.aes(label="..count.."), stat=stat, bins=[6, 5])
    counts = _texts(labelled.to_svg())
    for text in _texts(base.to_svg()):
        counts.remove(text)
    assert sum(float(count) for count in counts) == 400


def test_bad_bins_are_reported() -> None:
    plot = gg.ggplot(NORMAL, gg.aes("x", "y")) + gg.geom_bin2d(bins=[0, 3])
    with pytest.raises(SpecError, match="bins must be at least 1"):
        plot.to_svg()
    plot = gg.ggplot(NORMAL, gg.aes("x", "y")) + gg.geom_hex(bins=[1, 2, 3])
    with pytest.raises(SpecError, match=r"list of two numbers \[x, y\]"):
        plot.to_svg()


def test_identity_hexagons_are_twice_the_x_resolution_wide() -> None:
    data = {"x": [0, 1, 2, 0.5, 1.5], "y": [0, 0, 0, 1, 1], "v": [1, 2, 3, 4, 5]}
    widths = []
    for m in (gg, lp):
        plot = m.ggplot(data, m.aes("x", "y", fill="v")) + m.geom_hex(stat="identity")
        shapes = _shapes(plot.to_svg())
        widths.append(shapes[0][1] / shapes[0][2])
    assert widths[0] == pytest.approx(widths[1], rel=0.03)
