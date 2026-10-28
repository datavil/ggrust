"""Rasters must match lets-plot: image placement (fraction of the panel), pixels,
axis labels and titles. Legends are off. Under coord_flip ggrust transposes the
bitmap (cells stay at their data positions); lets-plot does not.
"""

import base64
import io
import re
import warnings

import numpy as np
import pytest
from PIL import Image

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from test_bar_parity import _attributes, _texts  # noqa: E402

SMALL = {
    "x": [0.0, 1, 2, 0, 1, 2, 0],
    "y": [0.0, 0, 0, 1, 1, 1, 3],
    "v": [1.0, 2, 3, 4, 5, 6, 2],
    "g": list("pqrpqrp"),
}
GRID_X, GRID_Y = np.meshgrid(np.arange(-3, 3.01, 0.5), np.arange(-2, 2.01, 0.5))
WAVE = {
    "x": GRID_X.ravel(),
    "y": GRID_Y.ravel(),
    "z": np.sin(GRID_X.ravel()) * np.cos(GRID_Y.ravel()),
}


def _image(svg: str) -> tuple[dict, np.ndarray]:
    tag = re.search(r"<image [^>]*>", svg).group(0)
    attributes = _attributes(tag)
    href = re.search(r'href="data:image/png;base64,([^"]+)"', tag).group(1)
    pixels = np.asarray(Image.open(io.BytesIO(base64.b64decode(href))).convert("RGBA"))
    return attributes, pixels.astype(int)


def _lets_plot_image(svg: str) -> tuple[tuple, np.ndarray]:
    clip = re.search(r'<rect x="[\d.]+" y="0\.0" width="([\d.]+)" height="([\d.]+)">', svg)
    width, height = float(clip.group(1)), float(clip.group(2))
    a, pixels = _image(svg)
    box = (
        float(a["x"]) / width,
        float(a["y"]) / height,
        float(a["width"]) / width,
        float(a["height"]) / height,
    )
    return box, pixels


def _ggrust_image(svg: str) -> tuple[tuple, np.ndarray]:
    clip = _attributes(re.search(r"<clipPath[^>]*>\s*<rect([^>]*)/>", svg).group(1))
    left, top = float(clip["x"]), float(clip["y"])
    width, height = float(clip["width"]), float(clip["height"])
    a, pixels = _image(svg)
    box = (
        (float(a["x"]) - left) / width,
        (float(a["y"]) - top) / height,
        float(a["width"]) / width,
        float(a["height"]) / height,
    )
    return box, pixels


CASES = {
    "fill": lambda m: m.ggplot(SMALL, m.aes("x", "y", fill="v")) + m.geom_raster(),
    "default_fill": lambda m: m.ggplot(SMALL, m.aes("x", "y")) + m.geom_raster(),
    "discrete_alpha": lambda m: (
        m.ggplot(SMALL, m.aes("x", "y", fill="g")) + m.geom_raster(alpha=0.6)
    ),
    "wave": lambda m: m.ggplot(WAVE, m.aes("x", "y", fill="z")) + m.geom_raster(),
    "reverse_y": lambda m: (
        m.ggplot(SMALL, m.aes("x", "y", fill="v")) + m.geom_raster() + m.scale_y_reverse()
    ),
    "reverse_x": lambda m: (
        m.ggplot(SMALL, m.aes("x", "y", fill="v")) + m.geom_raster() + m.scale_x_reverse()
    ),
    "cartesian": lambda m: (
        m.ggplot(SMALL, m.aes("x", "y", fill="v")) + m.geom_raster() + m.coord_cartesian()
    ),
}


def _both(build):
    return [(build(module) + module.theme(legend_position="none")).to_svg() for module in (lp, gg)]


@pytest.mark.parametrize("name", sorted(CASES))
def test_rasters_match_lets_plot(name):
    expected_svg, actual_svg = _both(CASES[name])
    expected_box, expected = _lets_plot_image(expected_svg)
    actual_box, actual = _ggrust_image(actual_svg)
    assert actual_box == pytest.approx(expected_box, abs=1e-3)
    assert actual.shape == expected.shape
    # Opaque pixels equal; alpha within PNG premultiplication rounding.
    assert np.abs(actual - expected).max() <= 2, (actual, expected)
    assert sorted(_texts(actual_svg)) == sorted(_texts(expected_svg))


def test_flipped_raster_keeps_cells_at_their_data_positions():
    plot = gg.ggplot(SMALL, gg.aes("x", "y", fill="g")) + gg.geom_raster() + gg.coord_flip()
    _, pixels = _image(plot.to_svg())
    # 4 y steps across (columns), 3 x steps down (rows, x = 2 on top).
    assert pixels.shape[:2] == (3, 4)
    painted = pixels[..., 3] > 0
    # Only (x=0, y=3) is filled in column 3: bottom row (x = 0).
    assert painted[:, 3].tolist() == [False, False, True]
    assert painted[:, 2].tolist() == [False, False, False]
    assert painted[:, :2].all()
