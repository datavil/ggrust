"""Identity scales and float shape codes must draw lets-plot's points.

Compares the drawn markers (fill, radius, opacity) of both SVGs, sorted, since
panel positions differ by a few px.
"""

import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from test_point_shapes_parity import assert_same, markers  # noqa: E402

FRAME = {
    "x": [1.0, 2.0, 3.0, 4.0],
    "y": [1.0, 2.0, 3.0, 4.0],
    "col": ["red", "#00ff00", "blue", "gray"],
    "size": [1.0, 2.0, 4.0, 8.0],
    "alpha": [0.2, 0.4, 0.6, 1.0],
    "shape": [1.0, 2.0, 21.0, 15.0],
}


def plot(m, mapping, scale):
    return m.ggplot(FRAME, m.aes("x", "y", **mapping)) + m.geom_point() + scale(m)


CASES = {
    "color": ({"color": "col"}, lambda m: m.scale_color_identity()),
    "size": ({"size": "size"}, lambda m: m.scale_size_identity()),
    "alpha": ({"alpha": "alpha"}, lambda m: m.scale_alpha_identity()),
    "shape": ({"shape": "shape"}, lambda m: m.scale_shape_identity()),
}


@pytest.mark.parametrize("name", CASES)
def test_identity_scales_match_lets_plot(name):
    mapping, scale = CASES[name]
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        theirs = markers(plot(lp, mapping, scale).to_svg(), ours=False)
        ours = markers(plot(gg, mapping, scale).to_svg(), ours=True)
    key = lambda primitive: (primitive[0], primitive[1][0], str(primitive[2]))  # noqa: E731
    assert len(ours) == len(theirs)
    assert sorted(str((kind, paint)) for kind, _, paint in ours) == sorted(
        str((kind, paint)) for kind, _, paint in theirs
    ), (sorted(ours, key=key), sorted(theirs, key=key))


@pytest.mark.parametrize("shape", [21.0, 2.7, 15.2])
def test_float_shape_codes_truncate_like_lets_plot(shape):
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        theirs = markers(
            (
                lp.ggplot({"x": [1.0], "y": [1.0]}, lp.aes("x", "y")) + lp.geom_point(shape=shape)
            ).to_svg(),
            ours=False,
        )
        ours = markers(
            (
                gg.ggplot({"x": [1.0], "y": [1.0]}, gg.aes("x", "y")) + gg.geom_point(shape=shape)
            ).to_svg(),
            ours=True,
        )
    assert_same(ours, theirs)


def test_unconvertible_identity_values_fail_like_lets_plot():
    plot = (
        gg.ggplot({"x": [1.0], "y": [1.0], "c": ["nope"]}, gg.aes("x", "y", color="c"))
        + gg.geom_point()
        + gg.scale_color_identity()
    )
    with pytest.raises(ValueError, match="can't use 'nope' as a color"):
        plot.to_svg()
