"""image_matrix (bistro) against lets-plot 4.11.

lets_plot places the cells with ``ggbunch(regions=...)``; ggrust places the same
regions as ``inset_element(align_to="full")`` figures on an empty plot of the same
``ggsize``. The regions must be equal, and the rendered images must sit inside them.
"""

import re
import warnings

import numpy as np
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

from ggrust.bistro import image_matrix  # noqa: E402
from ggrust.errors import SpecError  # noqa: E402
from lets_plot.bistro.im import image_matrix as lp_image_matrix  # noqa: E402

IMAGE = re.compile(r'<image [^>]*x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="([\d.]+)"')


def _matrix(rows: int = 2, columns: int = 3) -> np.ndarray:
    matrix = np.empty((rows, columns), dtype=object)
    for row in range(rows):
        for column in range(columns):
            width, height = 32 + 16 * column, 32 + 16 * row
            matrix[row, column] = 256 * np.linspace(
                np.linspace(0, 0.5, width), np.linspace(0.5, 0.5, width), height
            )
    return matrix


def _regions(spec: dict) -> list[tuple[float, float, float, float]]:
    """(left, top-down y, width, height) per inset, as lets_plot's ggbunch regions."""
    return [
        (
            inset["left"],
            1 - inset["top"],
            inset["right"] - inset["left"],
            inset["top"] - inset["bottom"],
        )
        for inset in spec["insets"]
    ]


@pytest.mark.parametrize(
    ("matrix", "options"),
    [
        (_matrix(), {}),
        (_matrix(), {"scale": 1.5, "spacer": 10, "norm": False}),
        (_matrix(3, 1), {"scale": 0.5}),
        (np.array([[np.zeros((10, 20)), np.zeros((60, 30, 3), dtype=np.uint8)]], dtype=object), {}),
    ],
    ids=["default", "scaled_spaced", "column_small", "mixed"],
)
def test_regions_and_size_match_lets_plot(matrix: np.ndarray, options: dict) -> None:
    ours = image_matrix(matrix, **options).as_dict()
    theirs = lp_image_matrix(matrix, **options).as_dict()
    assert ours["ggsize"] == pytest.approx(theirs["ggsize"])
    flat = [value for region in _regions(ours) for value in region]
    assert flat == pytest.approx(
        [value for region in theirs["layout"]["regions"] for value in region]
    )


def test_images_render_in_their_cells() -> None:
    svg = image_matrix(_matrix(), scale=2, spacer=4).to_svg()
    boxes = [tuple(float(value) for value in found) for found in IMAGE.findall(svg)]
    # Cells are 128 x 96 (widest image 64 px x 2, tallest 48 px x 2) and 4 px apart;
    # each image fills the top left of its cell at twice its size.
    assert boxes == [
        (column * 132.0, row * 100.0, (32 + 16 * column) * 2.0, (32 + 16 * row) * 2.0)
        for row in range(2)
        for column in range(3)
    ]
    assert 'width="392" height="196"' in svg


@pytest.mark.parametrize(
    ("rows", "side", "spacer"), [(4, 64, 1), (2, 28, 4), (3, 28, 3), (3, 64, 5)]
)
def test_edges_stay_inside_the_figure(rows: int, side: int, spacer: int) -> None:
    matrix = np.empty((rows, 1), dtype=object)
    for row in range(rows):
        matrix[row, 0] = np.zeros((side, side))
    spec = image_matrix(matrix, spacer=spacer).as_dict()
    edges = [inset[key] for inset in spec["insets"] for key in ("left", "bottom", "right", "top")]
    assert min(edges) == 0 and max(edges) == 1


def test_empty_cells_are_skipped() -> None:
    matrix = _matrix()
    matrix[1, 2] = None
    spec = image_matrix(matrix).as_dict()
    assert len(spec["insets"]) == 5
    # lets_plot fails on a None cell (zero-size region); ggrust leaves the cell blank.
    assert len(IMAGE.findall(image_matrix(matrix).to_svg())) == 5


def test_renders_without_warnings() -> None:
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        image_matrix(_matrix()).to_svg()


def test_color_map_and_no_legend() -> None:
    svg = image_matrix(_matrix(1, 1), cmap=["#ff0000", "#0000ff"]).to_svg()
    assert len(IMAGE.findall(svg)) == 1 and "legend" not in svg


@pytest.mark.parametrize(
    ("arguments", "match"),
    [
        ({"image_data_array": [[np.zeros((2, 2))]]}, r"expected a 2D numpy array.*got list"),
        ({"image_data_array": np.zeros((2, 2, 2))}, r"got a 3D array"),
        (
            {"image_data_array": np.array([[np.zeros(3), None]], dtype=object)},
            r"\[0, 0\].*got a 1D",
        ),
        ({"image_data_array": np.array([["a", None]], dtype=object)}, r"\[0, 0\].*got str"),
        ({"image_data_array": _matrix(), "scale": 0}, r"scale=0.*> 0"),
        ({"image_data_array": _matrix(), "spacer": -1}, r"spacer=-1.*>= 0"),
        ({"image_data_array": np.empty((0, 3), dtype=object)}, r"grid is empty"),
        ({"image_data_array": _matrix(), "scale": 1000}, r"over the 10000 x 10000 px.*scale=1000"),
    ],
)
def test_bad_input(arguments: dict, match: str) -> None:
    with pytest.raises(SpecError, match=match):
        image_matrix(**arguments)
