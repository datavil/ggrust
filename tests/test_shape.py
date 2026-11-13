"""``geom_shape`` (ggforce): polygons with rounded corners, grown by ``expand``.

Checked on the drawn SVG: radii are screen millimetres (round under any aspect
ratio), a large radius turns a rectangle into a stadium, ``expand`` grows the shape,
zero options draw exactly ``geom_polygon``, PNG and HTML render, and bad options
raise typed errors. The corner geometry is unit-tested in Rust (``geom/shape.rs``).
"""

from __future__ import annotations

import io
import math
import re

import ggrust as gg
import numpy as np
import pytest
from ggrust.errors import SpecError, UnsupportedValueError
from PIL import Image

PIXELS_PER_MM = 96 / 25.4
Point = tuple[float, float]
FILL = (0x33, 0x66, 0xCC)
CELL = {"x": [0.0, 10.0, 10.0, 0.0], "y": [0.0, 0.0, 2.0, 2.0]}


def _shape_paths(svg: str) -> list[list[Point]]:
    """Vertices of the closed, filled paths drawn with the test's fill color."""
    paths = []
    for match in re.finditer(r'<path d="([^"]+Z)"[^>]*fill="#3366cc"', svg):
        numbers = [float(value) for value in re.findall(r"-?\d+(?:\.\d+)?", match.group(1))]
        paths.append(list(zip(numbers[::2], numbers[1::2], strict=True)))
    return paths


def _bounds(points: list[Point]) -> tuple[float, float, float, float]:
    xs, ys = [x for x, _ in points], [y for _, y in points]
    return min(xs), min(ys), max(xs), max(ys)


def _plot(layer: gg.LayerSpec, *extra: gg.FeatureSpec) -> gg.PlotSpec:
    plot = gg.ggplot(CELL, gg.aes("x", "y")) + layer + gg.ggsize(500, 300)
    for feature in extra:
        plot = plot + feature
    return plot


def _single_shape(plot: gg.PlotSpec) -> list[Point]:
    paths = _shape_paths(plot.to_svg())
    assert len(paths) == 1
    return paths[0]


@pytest.mark.parametrize("fixed", [True, False])
def test_large_radius_draws_a_stadium(fixed: bool) -> None:
    extra = [gg.coord_fixed()] if fixed else []
    points = _single_shape(_plot(gg.geom_shape(radius=1000, fill="#3366cc"), *extra))
    low_x, low_y, high_x, high_y = _bounds(points)
    half = (high_y - low_y) / 2
    centre_y = (low_y + high_y) / 2
    assert len(points) > 20
    for x, y in points:
        along = min(max(x, low_x + half), high_x - half)
        assert math.hypot(x - along, y - centre_y) == pytest.approx(half, abs=0.02)


def test_radius_is_screen_millimetres_under_any_aspect() -> None:
    radius = 4 * PIXELS_PER_MM
    points = _single_shape(_plot(gg.geom_shape(radius=4, fill="#3366cc")))
    low_x, low_y, high_x, _ = _bounds(points)
    corner = [(x, y) for x, y in points if x < low_x + radius and y < low_y + radius]
    assert len(corner) > 3
    for x, y in corner:
        assert math.hypot(x - low_x - radius, y - low_y - radius) == pytest.approx(radius, abs=0.02)
    # The straight edges keep their middle part.
    assert any(abs(y - low_y) < 0.01 and low_x + radius < x < high_x - radius for x, y in points)


def test_zero_options_draw_the_polygon() -> None:
    plain = _plot(gg.geom_polygon(fill="#3366cc")).to_svg()
    shape = _plot(gg.geom_shape(fill="#3366cc")).to_svg()
    assert _shape_paths(shape) == _shape_paths(plain)


def test_expand_grows_the_shape() -> None:
    plain = _bounds(_single_shape(_plot(gg.geom_shape(radius=1000, fill="#3366cc"))))
    grown = _bounds(_single_shape(_plot(gg.geom_shape(radius=1000, expand=2, fill="#3366cc"))))
    expand = 2 * PIXELS_PER_MM
    for plain_side, grown_side, sign in zip(plain, grown, (-1, -1, 1, 1), strict=True):
        assert grown_side - plain_side == pytest.approx(sign * expand, abs=0.6)


def test_groups_get_their_own_shapes() -> None:
    data = {
        "x": [0, 4, 2, 5, 9, 9, 7, 7, 5],
        "y": [0, 0, 3, 0, 0, 4, 4, 2, 2],
        "g": ["triangle"] * 3 + ["L"] * 6,
    }
    plot = gg.ggplot(data, gg.aes("x", "y", group="g")) + gg.geom_shape(
        radius=3, fill="#3366cc", color="black"
    )
    assert len(_shape_paths(plot.to_svg())) == 2


def test_png_and_html_render() -> None:
    generator = np.random.default_rng(0)
    spots = {"x": generator.uniform(1, 9, 30), "y": generator.uniform(0.4, 1.6, 30)}
    plot = _plot(
        gg.geom_shape(radius=1000, fill="#3366cc", color="black"),
        gg.geom_point(gg.aes("x", "y"), data=spots, color="#d62728"),
        gg.coord_fixed(),
    )

    def fill_pixels(plot: gg.PlotSpec) -> int:
        buffer = io.BytesIO()
        plot.to_png(buffer)
        image = Image.open(io.BytesIO(buffer.getvalue())).convert("RGB")
        return sum(count for count, color in image.getcolors(1 << 24) or [] if color == FILL)

    rounded = fill_pixels(
        _plot(gg.geom_shape(radius=1000, fill="#3366cc", color="black"), gg.coord_fixed())
    )
    square = fill_pixels(_plot(gg.geom_polygon(fill="#3366cc", color="black"), gg.coord_fixed()))
    # The rounded ends leave the rectangle's corners empty: (1 - pi / 4) of a square
    # of the height's side at each end.
    assert rounded / square == pytest.approx(1 - (4 - math.pi) / 4 * 2 / 10, abs=0.01)
    html = plot._repr_html_()
    assert len(_shape_paths(html)) == 1


@pytest.mark.parametrize(
    ("layer", "error", "message"),
    [
        (lambda: gg.geom_shape(radius=-1), SpecError, "non-negative length in millimetres"),
        (lambda: gg.geom_shape(expand=-2), SpecError, "non-negative length in millimetres"),
        (lambda: gg.geom_shape(radius="2mm"), UnsupportedValueError, "length in millimetres"),
        (lambda: gg.geom_shape(expand=math.inf), SpecError, "finite"),
    ],
)
def test_bad_options_raise(layer: object, error: type[Exception], message: str) -> None:
    with pytest.raises(error, match=re.escape(message)):
        layer()  # type: ignore[operator]


def test_engine_rejects_negative_lengths() -> None:
    plot = _plot(gg.layer("shape", "identity", radius=-1))
    with pytest.raises(SpecError, match=r"layers\[0\]\.radius': radius must be a non-negative"):
        plot.to_svg()
