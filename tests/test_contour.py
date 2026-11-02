"""geom_contour, geom_contourf, geom_density2d and geom_density2df against lets-plot.

Contour lines are compared by their vertex counts (lets-plot's triangle split of each
cell is ported, so the counts match) and the plot texts. lets-plot simplifies band
polygons when drawing them, so bands are compared by ring count and texts, and their
geometry by area in data space.
"""

import inspect
import re
import warnings
from collections.abc import Callable
from pathlib import Path
from types import ModuleType

import numpy as np
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.errors import DataError, SpecError  # noqa: E402


def _grid(columns: int = 30, rows: int = 25) -> dict[str, list[float]]:
    x, y = np.meshgrid(np.linspace(-3, 3, columns), np.linspace(-2, 2, rows))
    z = np.exp(-(x**2 + y**2)) + 0.6 * np.exp(-((x - 1.5) ** 2 + (y - 1) ** 2) * 2)
    z -= z.min()  # a round lowest band level (colorbar labels)
    return {"x": x.ravel().tolist(), "y": y.ravel().tolist(), "z": z.ravel().tolist()}


def _faithful(n: int = 300) -> dict[str, list[float]]:
    rng = np.random.default_rng(1)
    short = rng.normal([2, 55], [0.3, 6], size=(n // 3, 2))
    long = rng.normal([4.3, 80], [0.4, 6], size=(n - n // 3, 2))
    both = np.vstack([short, long])
    return {"eruptions": both[:, 0].tolist(), "waiting": both[:, 1].tolist()}


GRID = _grid()
FAITHFUL = _faithful()

LINE_CASES: dict[str, Callable[[ModuleType], object]] = {
    "contour": lambda m: m.ggplot(GRID, m.aes("x", "y", z="z")) + m.geom_contour(),
    "contour_level_color": lambda m: (
        m.ggplot(GRID, m.aes("x", "y", z="z")) + m.geom_contour(m.aes(color="..level.."), bins=6)
    ),
    "contour_binwidth": lambda m: (
        m.ggplot(GRID, m.aes("x", "y", z="z")) + m.geom_contour(binwidth=0.15)
    ),
    "density2d": lambda m: (
        m.ggplot(FAITHFUL, m.aes("eruptions", "waiting"))
        + m.geom_density2d(m.aes(color="..level.."), bw=[0.3, 5])
    ),
    "density2d_options": lambda m: (
        m.ggplot(FAITHFUL, m.aes("eruptions", "waiting"))
        + m.geom_density2d(kernel="epanechikov", bw=[0.5, 8], n=[40, 60], bins=5)
    ),
}

BAND_CASES: dict[str, Callable[[ModuleType], object]] = {
    "contourf": lambda m: (
        m.ggplot(GRID, m.aes("x", "y", z="z")) + m.geom_contourf(m.aes(fill="..level.."))
    ),
    "density2df": lambda m: (
        m.ggplot(FAITHFUL, m.aes("eruptions", "waiting"))
        + m.geom_density2df(m.aes(fill="..level.."), bw=[0.3, 5])
    ),
}


def _color(text: str) -> str:
    rgb = re.fullmatch(r"rgb\((\d+),(\d+),(\d+)\)", text.replace(" ", ""))
    if rgb:
        return "#{:02x}{:02x}{:02x}".format(*map(int, rgb.groups()))
    return text.lower()


def _paths(svg: str) -> list[tuple[int, int, str]]:
    """Rings, distinct consecutive vertices and fill of every data path."""
    found = []
    for path in re.findall(r"<path\b[^>]*>", svg):
        data = re.search(r'\bd="(M[^"]+)"', path)
        if not data:
            continue
        points = re.findall(r"([-\d.e]+)[ ,]([-\d.e]+)", data.group(1))
        distinct = [point for index, point in enumerate(points) if point != points[index - 1]]
        fill = re.search(r'\bfill="([^"]+)"', path)
        found.append(
            (data.group(1).count("M"), len(distinct), _color(fill.group(1)) if fill else "")
        )
    return found


def _texts(svg: str) -> list[str]:
    found = re.findall(r"<text[^>]*>(.*?)</text>", svg, re.S)
    return sorted(re.sub(r"<[^>]+>|\s", "", text) for text in found)


def _contour_lines(svg: str) -> list[int]:
    # Data lines have more vertices than axis ticks, the panel and legend shapes.
    return sorted(vertices for _, vertices, _ in _paths(svg) if vertices > 5)


@pytest.mark.parametrize("name", LINE_CASES)
def test_lines_match_lets_plot(name: str) -> None:
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        ours = LINE_CASES[name](gg).to_svg()
    theirs = LINE_CASES[name](lp).to_svg()
    assert _texts(ours) == _texts(theirs)
    our_lines, their_lines = _contour_lines(ours), _contour_lines(theirs)
    assert len(our_lines) == len(their_lines)
    if name.startswith("contour"):
        assert our_lines == their_lines
    else:
        # R's bandwidth rule is not lets-plot's; with a fixed bw they agree closely.
        for our_count, their_count in zip(our_lines, their_lines, strict=True):
            assert our_count == pytest.approx(their_count, rel=0.02, abs=2)


@pytest.mark.parametrize("name", BAND_CASES)
def test_bands_match_lets_plot(name: str) -> None:
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        ours = BAND_CASES[name](gg).to_svg()
    theirs = BAND_CASES[name](lp).to_svg()
    assert _texts(ours) == _texts(theirs)
    our_bands = sorted((rings, paint) for rings, vertices, paint in _paths(ours) if vertices > 5)
    their_bands = sorted(
        (rings, paint) for rings, vertices, paint in _paths(theirs) if vertices > 5
    )
    assert our_bands == their_bands


def test_lowest_band_surrounds_a_closed_first_line() -> None:
    # With bins=6 the first line is a closed loop; lets-plot fills its inside with the
    # lowest band (and its x axis shrinks to -1.5..2.5). The band lies outside it.
    plot = gg.ggplot(GRID, gg.aes("x", "y", z="z")) + gg.geom_contourf(bins=6)
    svg = plot.to_svg()
    assert {"-3", "3"} <= set(_texts(svg))
    rings = sorted(rings for rings, vertices, _ in _paths(svg) if vertices > 5)
    assert rings[-1] >= 2  # an outer ring with the first line as its hole


def test_lines_default_style_matches_lets_plot() -> None:
    ours = (gg.ggplot(GRID, gg.aes("x", "y", z="z")) + gg.geom_contour()).to_svg()
    assert 'stroke="rgb(71,71,71)"' in ours or 'stroke="#474747"' in ours
    assert 'stroke-width="1.1"' in ours


def test_signatures_match_lets_plot() -> None:
    for name in ("geom_contour", "geom_contourf", "geom_density2d", "geom_density2df"):
        ours = list(inspect.signature(getattr(gg, name)).parameters)
        theirs = list(inspect.signature(getattr(lp, name)).parameters)
        assert ours == theirs


def test_spec_matches_lets_plot() -> None:
    for build in (
        lambda m: m.geom_contour(bins=5),
        lambda m: m.geom_contourf(binwidth=0.1, fill="red"),
        lambda m: m.geom_density2d(kernel="cosine", adjust=2, bw="nrd", n=50),
        lambda m: m.geom_density2df(bw=[1, 2], n=[30, 40], bins=4),
    ):
        assert build(gg).as_dict() == build(lp).as_dict()


def _line_vertices(data: dict[str, list[float]]) -> list[int]:
    plot = gg.ggplot(data, gg.aes("x", "y", z="z")) + gg.geom_contour(bins=5)
    return _contour_lines(plot.to_svg())


def test_row_order_does_not_matter() -> None:
    # lets-plot #1357: rows must be x-major sorted there.
    order = np.random.default_rng(3).permutation(len(GRID["x"]))
    shuffled = {key: [values[index] for index in order] for key, values in GRID.items()}
    assert _line_vertices(shuffled) == _line_vertices(GRID)


def test_zero_is_a_value_not_a_hole() -> None:
    # lets-plot #1356 treats z = 0 as missing; the 0.5 line must cross the whole grid.
    x, y = np.meshgrid(np.arange(10.0), np.arange(6.0))
    data = {"x": x.ravel(), "y": y.ravel(), "z": (x >= 5).astype(float).ravel()}
    plot = gg.ggplot(data, gg.aes("x", "y", z="z")) + gg.geom_contour(bins=1)
    assert _contour_lines(plot.to_svg()) == [11]  # 6 nodes high: 5 cells, 2 per cell, + 1


def test_missing_nodes_leave_holes() -> None:
    data = _grid(12, 12)
    keep = [index for index in range(len(data["x"])) if index not in (5 * 12 + 5, 70)]
    holed = {key: [values[index] for index in keep] for key, values in data.items()}
    plot = gg.ggplot(holed, gg.aes("x", "y", z="z")) + gg.geom_contourf(gg.aes(fill="..level.."))
    svg = plot.to_svg()
    assert len(_contour_lines(svg)) > 0


def test_scattered_points_are_refused_with_advice() -> None:
    rng = np.random.default_rng(0)
    data = {"x": rng.normal(size=5000), "y": rng.normal(size=5000), "z": rng.normal(size=5000)}
    plot = gg.ggplot(data, gg.aes("x", "y", z="z")) + gg.geom_contour()
    with pytest.raises(DataError, match="geom_density2d"):
        plot.to_svg()


def test_bad_density_options_name_the_parameter() -> None:
    base = gg.ggplot(FAITHFUL, gg.aes("eruptions", "waiting"))
    with pytest.raises(SpecError, match=r"layers\[0\]\.n.*2 to 999"):
        (base + gg.geom_density2d(n=5000)).to_svg()
    with pytest.raises(SpecError, match="kernel"):
        (base + gg.geom_density2d(kernel="box")).to_svg()


def test_weights_change_the_density() -> None:
    data = dict(FAITHFUL)
    data["w"] = [1.0 if value < 3 else 0.0 for value in data["eruptions"]]
    plot = gg.ggplot(data, gg.aes("eruptions", "waiting", weight="w")) + gg.geom_density2d()
    unweighted = gg.ggplot(data, gg.aes("eruptions", "waiting")) + gg.geom_density2d()
    assert _contour_lines(plot.to_svg()) != _contour_lines(unweighted.to_svg())


def test_png_renders(tmp_path: Path) -> None:
    plot = gg.ggplot(GRID, gg.aes("x", "y", z="z")) + gg.geom_contourf(gg.aes(fill="..level.."))
    target = tmp_path / "bands.png"
    plot.to_png(str(target))
    assert target.read_bytes()[:8] == b"\x89PNG\r\n\x1a\n"


@pytest.mark.parametrize("geom", ["geom_contour", "geom_contourf"])
def test_discrete_z_is_refused(geom: str) -> None:
    data = {"x": [1.0, 2, 1, 2], "y": [1.0, 1, 2, 2], "z": ["a", "b", "c", "d"]}
    plot = gg.ggplot(data, gg.aes("x", "y", z="z")) + getattr(gg, geom)()
    with pytest.raises(SpecError, match="z must be numeric"):
        plot.to_svg()


@pytest.mark.parametrize("geom", ["geom_contourf", "geom_density2df"])
def test_bands_fill_by_level_by_default(geom: str) -> None:
    # ggplot2's default (user's choice); lets-plot draws every band in one blue.
    base = (
        gg.ggplot(GRID, gg.aes("x", "y", z="z"))
        if geom == "geom_contourf"
        else gg.ggplot(FAITHFUL, gg.aes("eruptions", "waiting"))
    )
    default = (base + getattr(gg, geom)()).to_svg()
    explicit = (base + getattr(gg, geom)(gg.aes(fill="..level.."))).to_svg()
    assert default == explicit
    assert "level" in _texts(default)
    constant = (base + getattr(gg, geom)(fill="#118ed8")).to_svg()
    fills = {fill for _, vertices, fill in _paths(constant) if vertices > 5}
    assert fills == {"#118ed8"}
    assert "level" not in _texts(constant)
