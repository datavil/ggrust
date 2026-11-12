"""``rescaler=`` on continuous color scales (ggplot2 ``function(x, to, from)``)."""

from __future__ import annotations

import re
from collections.abc import Callable

import ggrust as gg
import polars as pl
import pytest
from ggrust.errors import SpecError, UnsupportedValueError

DATA = {"x": [1, 2, 3, 4, 5], "v": [1.0, 2.0, 3.0, 4.0, 10.0]}


def point_colors(scale: gg.FeatureSpec, data: dict[str, list[float]] = DATA) -> list[str]:
    plot = gg.ggplot(data, gg.aes("x", "x", color="v")) + gg.geom_point(shape=16) + scale
    return re.findall(r'<circle[^>]*?fill="(#[0-9a-fA-F]+)"', plot.to_svg())[: len(data["x"])]


def sqrt_rescaler(x: pl.Series, to: tuple[float, float], from_: tuple[float, float]) -> pl.Series:
    low, high = from_[0] ** 0.5, from_[1] ** 0.5
    return (x.sqrt() - low) / (high - low) * (to[1] - to[0]) + to[0]


def test_sqrt_rescaler_matches_ggplot2():
    """R: scale_colour_gradient(rescaler = <sqrt rescale>) on v = 1, 2, 3, 4, 10."""
    colors = point_colors(gg.scale_color_gradient(rescaler=sqrt_rescaler))
    assert colors == ["#132b43", "#1f4262", "#28557b", "#306591", "#56b1f7"]


def test_results_outside_zero_one_get_the_na_value():
    """R gives grey50 for every value the rescaler puts outside 0..1."""

    def wide(x: pl.Series, to: tuple[float, float], from_: tuple[float, float]) -> pl.Series:
        return (x - from_[0]) / (from_[1] - from_[0]) * 2 - 0.5

    colors = point_colors(gg.scale_color_gradient(rescaler=wide, na_value="#808080"))
    assert colors == ["#808080", "#808080", "#808080", "#1d3f5e", "#808080"]


def test_the_rescaler_gets_finite_values_once_with_to_and_from():
    calls: list[tuple[list[float], tuple[float, float], tuple[float, float]]] = []

    def record(x: pl.Series, to: tuple[float, float], from_: tuple[float, float]) -> list[float]:
        calls.append((x.to_list(), tuple(to), tuple(from_)))
        return [(value - from_[0]) / (from_[1] - from_[0]) for value in x]

    data = {"x": [1, 2, 3], "v": [2.0, None, 6.0]}
    point_colors(gg.scale_fill_gradient(rescaler=record), data)  # fill unused: no call
    assert calls == []
    point_colors(gg.scale_color_gradient(rescaler=record), data)
    # The layer first, then the guide (colorbar bins, breaks): one call each.
    assert calls[0] == ([2.0, 6.0], (0.0, 1.0), (2.0, 6.0))
    assert all(call[1:] == ((0.0, 1.0), (2.0, 6.0)) for call in calls)


def test_rescaler_works_in_transformed_units():
    calls: list[tuple[float, float]] = []

    def record(x: pl.Series, to: tuple[float, float], from_: tuple[float, float]) -> pl.Series:
        calls.append(tuple(from_))
        return (x - from_[0]) / (from_[1] - from_[0])

    data = {"x": [1, 2], "v": [10.0, 1000.0]}
    point_colors(gg.scale_color_gradient(rescaler=record, trans="log10"), data)
    assert calls[0] == pytest.approx((1.0, 3.0))


@pytest.mark.parametrize(
    "make",
    [
        lambda **options: gg.scale_color_gradientn(colors=["red", "blue"], **options),
        gg.scale_color_viridis,
        gg.scale_color_continuous,
        gg.scale_color_distiller,
        gg.scale_fill_gradient,
    ],
)
def test_other_continuous_color_scales_take_a_rescaler(make: Callable[..., gg.FeatureSpec]):
    aesthetic = make().as_dict()["aesthetic"]

    def colors(scale: gg.FeatureSpec) -> list[str]:
        plot = gg.ggplot(DATA, gg.aes("x", "x", **{aesthetic: "v"})) + gg.geom_point(shape=21)
        svg = (plot + scale).to_svg()
        return re.findall(
            rf'<circle[^>]*?{aesthetic if aesthetic == "fill" else "stroke"}="(#[0-9a-fA-F]+)"', svg
        )[:5]

    plain = colors(make())
    rescaled = colors(make(rescaler=sqrt_rescaler))
    assert plain[0] == rescaled[0] and plain[-1] == rescaled[-1]
    assert plain[1:4] != rescaled[1:4]


def test_linear_rescaler_changes_nothing():
    def linear(x: pl.Series, to: tuple[float, float], from_: tuple[float, float]) -> pl.Series:
        return (x - from_[0]) / (from_[1] - from_[0])

    assert point_colors(gg.scale_color_gradient(rescaler=linear)) == point_colors(
        gg.scale_color_gradient()
    )


def test_numpy_and_list_results_are_accepted():
    np = pytest.importorskip("numpy")
    expected = point_colors(gg.scale_color_gradient(rescaler=sqrt_rescaler))
    as_numpy = point_colors(
        gg.scale_color_gradient(
            rescaler=lambda x, to, from_: np.asarray(sqrt_rescaler(x, to, from_))
        )
    )
    as_list = point_colors(
        gg.scale_color_gradient(rescaler=lambda x, to, from_: sqrt_rescaler(x, to, from_).to_list())
    )
    assert as_numpy == as_list == expected


def test_a_rescaler_that_is_not_callable_fails_early():
    with pytest.raises(UnsupportedValueError, match="expected a function rescaler"):
        gg.scale_color_gradient(rescaler=0.5)  # ty: ignore[invalid-argument-type]


def test_a_failing_rescaler_names_the_exception():
    scale = gg.scale_color_gradient(rescaler=lambda x, to, from_: 1 / 0)
    with pytest.raises(SpecError, match="rescaler failed: ZeroDivisionError"):
        point_colors(scale)


def test_a_rescaler_returning_text_is_an_error():
    scale = gg.scale_color_gradient(rescaler=lambda x, to, from_: "a")
    with pytest.raises(SpecError, match="return a Series, array or list"):
        point_colors(scale)


def test_a_rescaler_returning_the_wrong_length_is_an_error():
    scale = gg.scale_color_gradient(rescaler=lambda x, to, from_: [0.5])
    with pytest.raises(SpecError, match="returned 1 values for 5 inputs"):
        point_colors(scale)
