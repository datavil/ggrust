"""``geom_imshow`` must match lets-plot: the same spec (``as_dict``, PNG href included),
the same image box (fraction of the panel), the same pixels, axis labels and colorbar
labels. Legends are off for the box comparison (font widths move the panel).
"""

import warnings

import numpy as np
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from test_bar_parity import _texts  # noqa: E402
from test_raster_parity import _ggrust_image, _image, _lets_plot_image  # noqa: E402

RANDOM = np.random.default_rng(7)
RGB_FLOAT = RANDOM.random((5, 6, 3))
RGBA_BYTES = RANDOM.integers(0, 256, (4, 7, 4), dtype=np.uint8)
GREY = np.arange(12, dtype=float).reshape(3, 4)
GREY_NAN = GREY.copy()
GREY_NAN[1, 1] = np.nan
GREY_INT = RANDOM.integers(0, 1000, (6, 5))

CASES = {
    "rgb_float": lambda m: m.ggplot() + m.geom_imshow(RGB_FLOAT),
    "rgba_bytes_alpha": lambda m: m.ggplot() + m.geom_imshow(RGBA_BYTES, alpha=0.5),
    "rgb_alpha": lambda m: m.ggplot() + m.geom_imshow(RGB_FLOAT, alpha=0.3),
    "grey": lambda m: m.ggplot() + m.geom_imshow(GREY),
    "grey_nan": lambda m: m.ggplot() + m.geom_imshow(GREY_NAN),
    "grey_int": lambda m: m.ggplot() + m.geom_imshow(GREY_INT),
    "grey_vmin_vmax": lambda m: m.ggplot() + m.geom_imshow(GREY, vmin=2, vmax=8),
    "grey_no_norm": lambda m: m.ggplot() + m.geom_imshow(GREY_INT, norm=False),
    "cmap_list_nan": lambda m: (
        m.ggplot() + m.geom_imshow(GREY_NAN, cmap=["#ff0000", "rgb(0, 0, 255)"])
    ),
    "cmap_viridis": lambda m: m.ggplot() + m.geom_imshow(GREY_NAN, cmap="viridis"),
    "cmap_magma_no_norm": lambda m: m.ggplot() + m.geom_imshow(GREY_INT, cmap="magma", norm=False),
    "extent": lambda m: m.ggplot() + m.geom_imshow(RGB_FLOAT, extent=[0, 30, 10, 20]),
    "extent_flipped": lambda m: m.ggplot() + m.geom_imshow(RGB_FLOAT, extent=[6, 0, 5, 0]),
    "with_points": lambda m: m.ggplot() + m.geom_imshow(RGB_FLOAT) + m.geom_point(x=8.0, y=1.0),
    "cartesian": lambda m: m.ggplot() + m.geom_imshow(GREY) + m.coord_cartesian(),
}


@pytest.mark.parametrize("name", sorted(CASES))
def test_imshow_spec_matches_lets_plot(name):
    assert CASES[name](gg).as_dict() == CASES[name](lp).as_dict()


@pytest.mark.parametrize("name", sorted(CASES))
def test_imshow_matches_lets_plot(name):
    expected_svg, actual_svg = (
        (CASES[name](module) + module.theme(legend_position="none")).to_svg() for module in (lp, gg)
    )
    expected_box, expected = _lets_plot_image(expected_svg)
    actual_box, actual = _ggrust_image(actual_svg)
    assert actual_box == pytest.approx(expected_box, abs=1e-3)
    assert actual.shape == expected.shape
    assert (actual == expected).all()
    assert sorted(_texts(actual_svg)) == sorted(_texts(expected_svg))


@pytest.mark.parametrize("name", ["grey", "grey_vmin_vmax", "cmap_viridis", "grey_no_norm"])
def test_imshow_colorbar_labels_match_lets_plot(name):
    expected_svg, actual_svg = (CASES[name](module).to_svg() for module in (lp, gg))
    assert sorted(_texts(actual_svg)) == sorted(_texts(expected_svg))


def test_imshow_without_legend_has_no_scale():
    spec = (gg.ggplot() + gg.geom_imshow(GREY, show_legend=False)).as_dict()
    assert spec.get("scales", []) == []
    assert "mapping" not in spec["layers"][0] or not spec["layers"][0]["mapping"]


def test_flipped_coord_transposes_the_image():
    image = np.zeros((2, 3, 3))
    image[0, 2] = [1, 0, 0]  # top right: x = 2, y = 1
    _, pixels = _image((gg.ggplot() + gg.geom_imshow(image) + gg.coord_flip()).to_svg())
    # x runs up, y runs right: (x = 2, y = 1) is the top right pixel of a 3 x 2 bitmap.
    assert pixels.shape[:2] == (3, 2)
    assert pixels[0, 1, :3].tolist() == [255, 0, 0]
    assert pixels[..., 0].sum() == 255


@pytest.mark.parametrize(
    ("image_data", "kwargs", "match"),
    [
        ([[1, 2]], {}, "ndarray is expected"),
        (np.zeros(3), {}, "2d or 3d array"),
        (GREY, {"alpha": 2}, "Invalid alpha"),
        (GREY, {"compression": 11}, "Invalid compression"),
        (GREY, {"vmin": 5, "vmax": 1}, "vmin value must be less than vmax"),
        (GREY, {"cmap": 3}, "cmap must be a string"),
        (GREY, {"cmap": ["red"]}, "Unsupported color format"),
        (GREY, {"extent": [0, 1, "a", 2]}, "Invalid `extent`"),
        (np.zeros((2, 2, 2)), {}, "3 \\(RGB\\) or 4 \\(RGBA\\) channels"),
    ],
)
def test_imshow_errors(image_data, kwargs, match):
    with pytest.raises(ValueError, match=match):
        gg.geom_imshow(image_data, **kwargs)


def test_image_layer_needs_a_png_href():
    layer = gg.geom_imshow(RGB_FLOAT)
    layer.props()["href"] = "https://example.org/image.png"
    with pytest.raises(Exception, match="data URL"):
        (gg.ggplot() + layer).to_svg()
