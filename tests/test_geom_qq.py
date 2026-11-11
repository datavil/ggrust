"""Quantile-quantile layers against lets_plot (``geom_qq``, ``geom_qq_line``, ...)."""

from __future__ import annotations

import re

import numpy as np
import pytest
from ggrust import (
    aes,
    geom_qq,
    geom_qq2,
    geom_qq2_line,
    geom_qq_line,
    ggplot,
    scale_y_log10,
    stat_qq,
    stat_qq_line,
)
from ggrust.errors import SpecError

SAMPLE = [4.0, 1.0, 3.0, 2.0, 9.0, 7.0, 5.0, 8.0, 6.0, 12.0]
# R: qnorm((1:10 - 0.5) / 10)
NORMAL_QUANTILES = [
    -1.6448536269514729,
    -1.0364333894937898,
    -0.6744897501960817,
    -0.38532046640756773,
    -0.12566134685507402,
    0.12566134685507402,
    0.38532046640756773,
    0.6744897501960817,
    1.0364333894937898,
    1.6448536269514729,
]


def test_geom_qq_pairs_sorted_values_with_theoretical_quantiles() -> None:
    frame = (ggplot({"v": SAMPLE}, aes(sample="v")) + geom_qq()).layer_data(0)
    assert frame["y"].to_list() == sorted(SAMPLE)
    assert frame["x"].to_list() == pytest.approx(NORMAL_QUANTILES, abs=1e-12)
    assert frame["theoretical"].to_list() == frame["x"].to_list()


@pytest.mark.parametrize(
    ("distribution", "dparams", "expected_first"),
    [
        ("uniform", [2, 4], 2.1),
        ("exp", [2], 0.0256466471938),
        ("t", [5], -2.0150483733330233),
        ("chi2", [3], 0.3518463177492714),
    ],
)
def test_geom_qq_distributions(
    distribution: str, dparams: list[float], expected_first: float
) -> None:
    layer = geom_qq(distribution=distribution, dparams=dparams)  # ty: ignore[invalid-argument-type]
    frame = (ggplot({"v": SAMPLE}, aes(sample="v")) + layer).layer_data(0)
    assert frame["x"][0] == pytest.approx(expected_first, abs=1e-3)


def test_geom_qq_keeps_row_colors_with_their_values() -> None:
    plot = ggplot({"v": SAMPLE}, aes(sample="v", color="v")) + geom_qq()
    frame = plot.layer_data(0)
    assert frame["color"].to_list() == sorted(SAMPLE)


def test_geom_qq_groups_each_on_its_own() -> None:
    data = {"v": SAMPLE, "g": ["a", "b"] * 5}
    frame = (ggplot(data, aes(sample="v", color="g")) + geom_qq()).layer_data(0)
    assert frame["group"].n_unique() == 2
    assert frame.filter(frame["group"] == 0)["x"].to_list() == pytest.approx(
        frame.filter(frame["group"] == 1)["x"].to_list()
    )


def test_geom_qq_drops_missing_values() -> None:
    frame = (ggplot({"v": [1.0, None, 3.0]}, aes(sample="v")) + geom_qq()).layer_data(0)
    assert frame["y"].to_list() == [1.0, 3.0]


def test_geom_qq_line_passes_through_quartiles() -> None:
    plot = ggplot({"v": SAMPLE}, aes(sample="v")) + geom_qq_line()
    frame = plot.layer_data(0)
    assert frame.height == 2
    assert frame["x"].to_list() == pytest.approx(
        [NORMAL_QUANTILES[0], NORMAL_QUANTILES[-1]], abs=1e-12
    )
    # R: quantile(sample, c(.25, .75), type = 8) = 3, 8.25 ... line through them.
    sorted_sample = sorted(SAMPLE)
    low = np.quantile(sorted_sample, 0.25, method="median_unbiased")
    high = np.quantile(sorted_sample, 0.75, method="median_unbiased")
    z = 0.6744897501960817
    slope = (high - low) / (2 * z)
    assert frame["y"][1] == pytest.approx(low + slope * (NORMAL_QUANTILES[-1] + z))


def test_geom_qq2_compares_quantiles_of_two_samples() -> None:
    x = [1.0, 2.0, 3.0, 4.0]
    frame = (
        ggplot({"x": x, "y": [10.0, 40.0, 20.0, 30.0]}, aes("x", "y")) + geom_qq2()
    ).layer_data(0)
    assert frame["y"].to_list() == pytest.approx([value * 10 for value in frame["x"].to_list()])


def test_geom_qq2_line_spans_the_x_sample() -> None:
    data = {"x": [1.0, 2.0, 3.0, 4.0], "y": [10.0, 40.0, 20.0, 30.0]}
    frame = (ggplot(data, aes("x", "y")) + geom_qq2_line()).layer_data(0)
    assert frame["x"].to_list() == [1.0, 4.0]
    assert frame["y"].to_list() == pytest.approx([10.0, 40.0])


def test_stat_functions_match_geoms() -> None:
    base = ggplot({"v": SAMPLE}, aes(sample="v"))
    assert (base + stat_qq()).layer_data(0).equals((base + geom_qq()).layer_data(0))
    assert (base + stat_qq_line()).layer_data(0).equals((base + geom_qq_line()).layer_data(0))


def test_axis_titles_name_the_computed_variables() -> None:
    svg = (ggplot({"v": SAMPLE}, aes(sample="v")) + geom_qq()).to_svg()
    texts = re.findall(r">([a-z]+)<", svg)
    assert "theoretical" in texts
    assert "sample" in texts


def test_sample_follows_the_y_transform() -> None:
    plot = ggplot({"v": [1.0, 10.0, 100.0]}, aes(sample="v")) + geom_qq() + scale_y_log10()
    assert plot.layer_data(0)["y"].to_list() == pytest.approx([0.0, 1.0, 2.0])


@pytest.mark.parametrize(
    ("options", "path", "fragment"),
    [
        ({"distribution": "normal"}, "distribution", "did you mean 'norm'"),
        ({"dparams": [0, -1]}, "dparams", "positive standard deviation"),
        ({"distribution": "uniform", "dparams": [2, 1]}, "dparams", "a < b"),
        ({"dparams": 3}, "dparams", "list of numbers"),
    ],
)
def test_bad_distribution_options_say_what_is_accepted(
    options: dict[str, object], path: str, fragment: str
) -> None:
    plot = ggplot({"v": SAMPLE}, aes(sample="v")) + geom_qq(**options)  # ty: ignore[invalid-argument-type]
    with pytest.raises(SpecError, match=fragment) as caught:
        plot.to_svg()
    assert path in caught.value.path


@pytest.mark.parametrize("quantiles", [[0.5], [0.5, 0.5], [-0.1, 0.5]])
def test_bad_quantiles_are_rejected(quantiles: list[float]) -> None:
    plot = ggplot({"v": SAMPLE}, aes(sample="v")) + geom_qq_line(quantiles=quantiles)
    with pytest.raises(SpecError, match="two different probabilities"):
        plot.to_svg()
