"""gghalves and ggdist layers: shapes land where R's packages put them.

The reference numbers come from ggdist 3.3.3 and gghalves 0.1.4 (``ggplot_build``),
written by ``tests/fixtures/generate_ggdist_reference.R``. The density and interval
numbers themselves are checked in Rust (``stat/ggdist_density.rs``,
``stat/pointinterval.rs``); here the drawn geometry is compared, in data units
recovered from the SVG pixels by a linear fit.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import TYPE_CHECKING

import ggrust as gg
import numpy as np
import polars as pl
import pytest
from ggrust.errors import SpecError, UnsupportedValueError

if TYPE_CHECKING:
    from collections.abc import Callable

REFERENCE = json.loads((Path(__file__).parent / "fixtures" / "ggdist_reference.json").read_text())
SAMPLES = REFERENCE["samples"]
FRAME = pl.DataFrame(
    {
        "g": ["A"] * 60 + ["B"] * 60 + ["C"] * 60,
        "k": ["u", "v"] * 90,
        "v": SAMPLES["normal"]["values"]
        + SAMPLES["bimodal"]["values"]
        + SAMPLES["skewed"]["values"],
    }
)
LAYERS = REFERENCE["layers"]
SLAB_FILL = "#a6a6a6"


def paths(svg: str, fill: str | None = None) -> list[np.ndarray]:
    """Vertices of every ``<path>`` (optionally only those with this fill)."""
    found = []
    for tag in re.findall(r"<path[^>]*>", svg):
        if fill is not None and f'fill="{fill}"' not in tag:
            continue
        data = re.search(r' d="([^"]+)"', tag)
        if data is None:
            continue
        numbers = [float(value) for value in re.findall(r"-?\d+(?:\.\d+)?", data.group(1))]
        found.append(np.array(numbers).reshape(-1, 2))
    return found


def fit_error(pixels: np.ndarray, data: np.ndarray) -> float:
    """Largest distance, in data units, from the best linear pixel mapping."""
    design = np.vstack([data, np.ones(len(data))]).T
    coefficients, *_ = np.linalg.lstsq(design, pixels, rcond=None)
    return float(np.abs(design @ coefficients - pixels).max() / abs(coefficients[0]))


def reference(name: str) -> dict[str, np.ndarray]:
    """A layer's R columns as float arrays (R's ``NA`` as NaN)."""
    return {
        key: np.array([np.nan if value == "NA" else value for value in values], dtype=float)
        for key, values in LAYERS[name].items()
    }


def test_slab_outline_matches_ggdist() -> None:
    expected = reference("halfeye_slab")
    plot = gg.ggplot(FRAME, gg.aes("g", "v")) + gg.stat_slab()
    slabs = paths(plot.to_svg(), SLAB_FILL)
    assert len(slabs) == 3
    largest = expected["thickness"].max()
    pixels, data = [], []
    for index, slab in enumerate(slabs):
        rows = expected["group"] == index + 1
        # The outer edge: the first 501 vertices, x + 0.9 x thickness / largest.
        outer = slab[:501]
        pixels.append(outer)
        data.append(
            np.column_stack(
                [
                    expected["x"][rows] + 0.9 * expected["thickness"][rows] / largest,
                    expected["y"][rows],
                ]
            )
        )
    pixels_all, data_all = np.vstack(pixels), np.vstack(data)
    assert fit_error(pixels_all[:, 0], data_all[:, 0]) < 2e-3
    assert fit_error(pixels_all[:, 1], data_all[:, 1]) < 2e-3


def test_slab_orientation_y_is_the_flipped_slab() -> None:
    vertical = paths((gg.ggplot(FRAME, gg.aes("g", "v")) + gg.stat_slab()).to_svg(), SLAB_FILL)
    horizontal = paths((gg.ggplot(FRAME, gg.aes("v", "g")) + gg.stat_slab()).to_svg(), SLAB_FILL)
    explicit = gg.ggplot(FRAME, gg.aes("v", "g")) + gg.stat_slab(orientation="y")
    np.testing.assert_allclose(horizontal[0], paths(explicit.to_svg(), SLAB_FILL)[0])
    # Same shape: the vertical outline's x offsets are the horizontal one's y offsets.
    up = horizontal[0][0, 1] - horizontal[0][:501, 1]
    right = vertical[0][:501, 0] - vertical[0][0, 0]
    assert fit_error(up, right) < 1e-2 * right.max()


@pytest.mark.parametrize("name", ["pointinterval", "interval"])
def test_intervals_match_ggdist(name: str) -> None:
    expected = reference(name)
    layer = gg.stat_pointinterval() if name == "pointinterval" else gg.stat_interval()
    svg = (gg.ggplot(FRAME, gg.aes("g", "v")) + layer + gg.theme(legend_position="none")).to_svg()
    # Interval lines: two-point vertical paths with a stroke wider than the grid.
    lines = [
        path
        for path, tag in zip(paths(svg), re.findall(r"<path[^>]*>", svg), strict=True)
        if len(path) == 2 and path[0, 0] == path[1, 0] and 'stroke-width="1"' not in tag
    ]
    assert len(lines) == len(expected["ymin"])
    # Both sides list the intervals by x, then by width (R narrowest first).
    drawn = sorted(lines, key=lambda line: (line[0, 0], -abs(line[1, 1] - line[0, 1])))
    order = np.lexsort((-expected[".width"], expected["x"]))
    tops = np.array([min(line[:, 1]) for line in drawn])
    bottoms = np.array([max(line[:, 1]) for line in drawn])
    pixels = np.concatenate([bottoms, tops])
    data = np.concatenate([expected["ymin"][order], expected["ymax"][order]])
    assert fit_error(pixels, data) < 1e-3


def test_interval_widths_and_levels() -> None:
    svg = (gg.ggplot(FRAME, gg.aes("g", "v")) + gg.stat_interval()).to_svg()
    widths = set(re.findall(r'stroke-width="([\d.]+)"', svg))
    assert "11.44" in widths  # size 5.2 x 2.2 px: ggdist's linewidth 4 in ggplot2
    assert ">0.95<" in svg and ">0.5<" in svg  # the level legend, widest first
    assert svg.index(">0.95<") < svg.index(">0.5<")


def test_pointinterval_sizes_follow_ggdist() -> None:
    svg = (gg.ggplot(FRAME, gg.aes("g", "v")) + gg.stat_pointinterval()).to_svg()
    widths = sorted({float(value) for value in re.findall(r'stroke-width="([\d.]+)"', svg)})
    assert pytest.approx(widths[1], abs=0.01) == 0.78 * 2.2
    assert pytest.approx(widths[-1], abs=0.01) == 1.81 * 2.2
    radii = {float(value) for value in re.findall(r'<circle[^>]*r="([\d.]+)"', svg)}
    assert max(radii) == pytest.approx(1.81 * 2.5 * 2.2 / 2, abs=0.01)


def test_halfeye_is_slab_plus_pointinterval() -> None:
    features = gg.stat_halfeye(fill="red", color="blue").as_dict()["feature-list"]
    slab, interval = (feature["layer"] for feature in features)
    assert (slab["geom"], slab["stat"], slab["fill"]) == ("slab", "slab", "red")
    assert (interval["geom"], interval["stat"], interval["color"]) == (
        "pointinterval",
        "pointinterval",
        "blue",
    )
    assert "fill" not in interval


def half_violin_outline(svg: str, fill: str) -> np.ndarray:
    """The filled outline of the first half violin with this fill."""
    return paths(svg, fill)[0]


def test_half_violin_matches_gghalves() -> None:
    expected = reference("half_violin")
    rows = expected["group"] == 1
    plot = gg.ggplot(FRAME, gg.aes("g", "v")) + gg.geom_half_violin(fill="red")
    outline = paths(plot.to_svg(), "#ff0000")
    assert len(outline) == 3
    # Left side: x + violinwidth x (xmin - x), then the straight side at x.
    x, xmin, width = expected["x"][rows], expected["xmin"][rows], expected["violinwidth"][rows]
    outer = outline[0][: len(outline[0]) // 2]
    # Quantile band edges repeat a vertex (lets-plot's ydensity); drop them.
    outer = outer[np.concatenate([[True], np.diff(outer[:, 1]) != 0])]
    assert len(outer) == len(x)
    # lets-plot's ydensity grid matches R's on these data (512 points, trimmed).
    assert fit_error(outer[:, 1], expected["y"][rows]) < 1e-3
    assert fit_error(outer[:, 0], x + width * (xmin - x)) < 1e-3


def test_half_violin_dodge_spans_match_gghalves() -> None:
    expected = reference("half_violin_dodge")
    plot = gg.ggplot(FRAME, gg.aes("g", "v", fill="k")) + gg.geom_half_violin()
    outlines = paths(plot.to_svg(), "#e41a1c") + paths(plot.to_svg(), "#377eb8")
    centers = sorted(outline[-1, 0] for outline in outlines)  # the straight side
    r_centers = sorted(set(expected["x"]))
    assert fit_error(np.array(centers), np.array(r_centers)) < 1e-3


def test_half_boxplot_matches_gghalves() -> None:
    expected = reference("half_boxplot")
    for side, low, high in [("l", "xmin", "x"), ("r", "x", "xmax")]:
        plot = gg.ggplot(FRAME, gg.aes("g", "v")) + gg.geom_half_boxplot(side=side)
        boxes = re.findall(
            r'<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="([\d.]+)"', plot.to_svg()
        )[1:]  # after the panel background
        boxes = np.array(boxes, dtype=float)
        assert len(boxes) == 3
        lefts, rights = boxes[:, 0], boxes[:, 0] + boxes[:, 2]
        edges = np.concatenate([lefts, rights])
        assert fit_error(edges, np.concatenate([expected[low], expected[high]])) < 1e-3
        tops, bottoms = boxes[:, 1], boxes[:, 1] + boxes[:, 3]
        assert (
            fit_error(
                np.concatenate([tops, bottoms]),
                np.concatenate([expected["upper"], expected["lower"]]),
            )
            < 1e-3
        )


def test_half_boxplot_nudges_outliers_with_the_whiskers() -> None:
    data = {"g": ["a"] * 8, "v": [1, 2, 2.5, 3, 3.2, 3.4, 4, 20]}
    layers = gg.geom_half_boxplot(side="r", nudge=0.1)
    outlier_layer = layers.as_dict()["feature-list"][1]["layer"]
    assert outlier_layer["outlier_nudge"] == 0.1
    svg = (gg.ggplot(data, gg.aes("g", "v")) + layers).to_svg()
    circle = re.search(r'<circle[^>]*cx="([\d.]+)"', svg)
    assert circle is not None
    whisker = [path for path in paths(svg) if len(path) == 2 and path[0, 0] == path[1, 0]]
    assert any(abs(path[0, 0] - float(circle.group(1))) < 0.01 for path in whisker)


def test_half_points_stay_in_their_half() -> None:
    def centers(transformation: str | None) -> np.ndarray:
        layer = gg.geom_half_point(transformation=transformation)
        svg = (gg.ggplot(FRAME, gg.aes("g", "v")) + layer).to_svg()
        found = re.findall(r'<circle[^>]*cx="([\d.]+)"[^>]*cy="([\d.]+)"', svg)
        return np.array(found, dtype=float)

    jittered, still = centers(None), centers("identity")
    assert len(jittered) == 180
    # Unjittered points sit a quarter of the 0.75 slot right of x (A = 1, B = 2).
    unit = still[60, 0] - still[0, 0]
    offsets = (jittered[:, 0] - still[:, 0]) / unit
    reach = 0.75 * 0.75 / 4  # range_scale x a quarter of the slot
    assert offsets.min() >= -reach - 1e-3
    assert offsets.max() <= reach + 1e-3
    assert offsets.max() > 0.8 * reach  # the jitter does use the room
    plot = gg.ggplot(FRAME, gg.aes("g", "v")) + gg.geom_half_point()
    assert plot.to_svg() == plot.to_svg()


def test_half_point_transformations() -> None:
    still = gg.ggplot(FRAME, gg.aes("g", "v")) + gg.geom_half_point(transformation="identity")
    x = {
        round(float(value), 2) for value in re.findall(r'<circle[^>]*cx="([\d.]+)"', still.to_svg())
    }
    assert len(x) == 3
    seeded = gg.geom_half_point(transformation=gg.position_jitter(width=0.05, seed=3)).as_dict()
    assert (seeded["jitter_width"], seeded["seed"]) == (0.05, 3)


def test_raincloud_renders(tmp_path: Path) -> None:
    plot = (
        gg.ggplot(FRAME, gg.aes("g", "v", fill="g"))
        + gg.geom_half_violin(side="r", nudge=0.05)
        + gg.geom_half_boxplot(side="r", width=0.15, outlier_alpha=0)
        + gg.geom_half_point(side="l", alpha=0.6)
    )
    plot.to_png(tmp_path / "raincloud.png")
    assert (tmp_path / "raincloud.png").read_bytes().startswith(b"\x89PNG")
    flipped = (
        gg.ggplot(FRAME, gg.aes("v", "g", fill="g"))
        + gg.geom_half_violin(side="r")
        + gg.geom_half_boxplot(side="r")
        + gg.geom_half_point(side="l")
    )
    assert "<circle" in flipped.to_svg()


@pytest.mark.parametrize(
    ("build", "error", "fragment"),
    [
        (lambda: gg.geom_half_violin(side="left-ish"), SpecError, "side"),
        (lambda: gg.geom_half_violin(nudge="far"), UnsupportedValueError, "nudge"),
        (lambda: gg.geom_half_violin(draw_quantiles=[0.5, 2]), SpecError, "draw_quantiles"),
        (lambda: gg.geom_half_boxplot(errorbar_length=1.5), SpecError, "errorbar_length"),
        (lambda: gg.geom_half_point(transformation="dodge"), SpecError, "transformation"),
        (lambda: gg.stat_slab(density="bounde"), SpecError, "did you mean 'bounded'"),
        (lambda: gg.stat_slab(bandwidth="sj"), SpecError, "bandwidth"),
        (lambda: gg.stat_slab(bandwidth=-1), SpecError, "positive"),
        (lambda: gg.stat_slab(n=1), SpecError, "n must be"),
        (lambda: gg.stat_slab(side="up"), SpecError, "side"),
        (lambda: gg.stat_halfeye(point_interval="median_hdi"), SpecError, "median_qi"),
        (lambda: gg.stat_interval(widths=[0.5, 1.2]), SpecError, "(0, 1]"),
        (lambda: gg.stat_interval(widths="wide"), UnsupportedValueError, "widths"),
        (lambda: gg.stat_pointinterval(interval_size_range=(1,)), SpecError, "interval_size_range"),
    ],
)
def test_python_errors(build: Callable[[], object], error: type[Exception], fragment: str) -> None:
    with pytest.raises(error, match=re.escape(fragment)):
        build()


def test_engine_errors_name_the_option() -> None:
    plot = gg.ggplot(FRAME, gg.aes("g", "v")) + gg.layer(geom="slab", stat="slab", normalize="max")
    with pytest.raises(SpecError, match=r"layers\[0\]\.normalize"):
        plot.to_svg()
    plot = gg.ggplot(FRAME, gg.aes("g", "v")) + gg.layer(geom="half_violin", side="up")
    with pytest.raises(SpecError, match=r"layers\[0\]\.side"):
        plot.to_svg()


def test_halfeye_maps_slab_variables_to_the_slab_only() -> None:
    plot = gg.ggplot(FRAME, gg.aes("g", "v")) + gg.stat_halfeye(gg.aes(fill="..cdf.."))
    assert "<path" in plot.to_svg()
    features = gg.stat_halfeye(gg.aes(fill="..cdf..", color="..level..")).as_dict()["feature-list"]
    slab, interval = (feature["layer"]["mapping"] for feature in features)
    assert slab == {"fill": "..cdf.."}
    assert interval == {"color": "..level.."}


def test_x_only_mapping_is_horizontal() -> None:
    only_x = gg.ggplot(FRAME, gg.aes(x="v")) + gg.stat_halfeye()
    explicit = gg.ggplot(FRAME, gg.aes(x="v")) + gg.stat_halfeye(orientation="y")
    assert only_x.to_svg() == explicit.to_svg()


def test_widths_accept_arrays() -> None:
    layer = gg.stat_pointinterval(widths=np.array([0.5, 0.9])).as_dict()
    assert layer["widths"] == [0.5, 0.9]


def test_stat_is_rejected_with_a_spec_error() -> None:
    with pytest.raises(SpecError, match="takes no 'stat'"):
        gg.stat_pointinterval(stat="identity")


@pytest.mark.parametrize("geom", ["half_violin", "half_boxplot", "half_point"])
def test_half_geoms_reject_non_positive_widths(geom: str) -> None:
    plot = gg.ggplot(FRAME, gg.aes("g", "v")) + gg.layer(geom=geom, width=0)
    with pytest.raises(SpecError, match=r"layers\[0\]\.width"):
        plot.to_svg()
