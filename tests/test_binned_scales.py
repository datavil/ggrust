"""Binned color scales: ``scale_*_steps``, ``steps2``, ``stepsn``, ``binned`` (ggplot2)."""

from __future__ import annotations

import re

import ggrust as gg
import pytest
from ggrust.errors import SpecError

DATA = {"x": list(range(1, 8)), "v": [3.0, 17.0, 22.0, 48.0, 51.0, 77.0, 99.0]}


def point_colors(scale: gg.FeatureSpec) -> list[str]:
    plot = gg.ggplot(DATA, gg.aes("x", "x", color="v")) + gg.geom_point() + scale
    return re.findall(r'<circle[^>]*?fill="(#[0-9a-fA-F]+)"', plot.to_svg())[:7]


def guide_labels(scale: gg.FeatureSpec) -> list[str]:
    plot = gg.ggplot(DATA, gg.aes("x", "x", color="v")) + gg.geom_point() + scale
    return plot.guide_data()["label"].to_list()


def test_values_in_one_bin_share_a_color():
    colors = point_colors(gg.scale_color_steps())
    # Bins 0-20-40-60-80-100: 3 and 17 together, 48 and 51 together.
    assert colors[0] == colors[1] != colors[2]
    assert colors[3] == colors[4]
    assert len(set(colors)) == 5


def test_steps_colors_are_the_gradient_at_bin_midpoints():
    """ggplot2 colors bin 0-20 like a gradient over 0..100 at 10."""
    steps = point_colors(gg.scale_color_steps(low="black", high="white"))
    frame = {"x": [1, 2], "v": [10.0, 0.0]}
    gradient = gg.ggplot(frame, gg.aes("x", "x", color="v")) + gg.geom_point()
    gradient += gg.scale_color_gradient(low="black", high="white", limits=[0, 100])
    expected = re.findall(r'<circle[^>]*?fill="(#[0-9a-fA-F]+)"', gradient.to_svg())[0]
    assert steps[0] == expected


def test_binned_is_steps_and_colour_spellings_alias():
    assert point_colors(gg.scale_color_binned()) == point_colors(gg.scale_color_steps())
    assert gg.scale_colour_steps2 is gg.scale_color_steps2
    assert gg.scale_colour_stepsn is gg.scale_color_stepsn


def test_right_false_moves_a_value_on_a_break_up():
    frame = {"x": [1, 2, 3], "v": [0.0, 20.0, 100.0]}
    plot = gg.ggplot(frame, gg.aes("x", "x", color="v")) + gg.geom_point()
    breaks = gg.scale_color_steps(breaks=[20, 40, 60, 80], limits=[0, 100])
    right = re.findall(r'<circle[^>]*?fill="(#[0-9a-fA-F]+)"', (plot + breaks).to_svg())
    left_scale = gg.scale_color_steps(breaks=[20, 40, 60, 80], limits=[0, 100], right=False)
    left = re.findall(r'<circle[^>]*?fill="(#[0-9a-fA-F]+)"', (plot + left_scale).to_svg())
    assert right[0] == right[1]
    assert left[0] != left[1]


def colorbar_texts(scale: gg.FeatureSpec) -> list[str]:
    """Text of the plot (axes 1..7 and the colorbar labels)."""
    plot = gg.ggplot(DATA, gg.aes("x", "x", color="v")) + gg.geom_point() + scale
    return [text for text in re.findall(r">([^<>]+)</text>", plot.to_svg()) if text.strip()]


def test_colorbar_labels_inner_breaks_and_show_limits():
    texts = colorbar_texts(gg.scale_color_steps())
    assert {"20", "40", "60", "80"} <= set(texts) and "100" not in texts
    assert {"0", "100"} <= set(colorbar_texts(gg.scale_color_steps(show_limits=True)))


def test_given_labels_stay_on_the_breaks_with_show_limits():
    texts = colorbar_texts(
        gg.scale_color_steps(breaks=[20, 60], labels=["a", "b"], show_limits=True)
    )
    assert {"3", "a", "b", "99"} <= set(texts)


def test_guide_data_has_one_entry_per_bin():
    assert guide_labels(gg.scale_color_steps()) == [
        "0 - 20",
        "20 - 40",
        "40 - 60",
        "60 - 80",
        "80 - 100",
    ]
    legend = gg.ggplot(DATA, gg.aes("x", "x", color="v")) + gg.geom_point()
    legend += gg.scale_color_steps(guide="legend")
    assert "80 - 100" in legend.to_svg()


def test_given_breaks_and_n_breaks():
    assert guide_labels(gg.scale_color_steps(breaks=[10, 50])) == ["3 - 10", "10 - 50", "50 - 99"]
    assert len(guide_labels(gg.scale_color_steps(n_breaks=2, nice_breaks=False))) == 3


def test_steps2_and_stepsn_render_as_steps():
    plot = gg.ggplot(DATA, gg.aes("x", "x", fill="v")) + gg.geom_tile()
    for scale in (
        gg.scale_fill_steps2(midpoint=50),
        gg.scale_fill_stepsn(colors=["navy", "gold", "darkred"]),
        gg.scale_fill_binned(),
    ):
        assert "<rect" in (plot + scale).to_svg()


def test_bad_n_breaks_names_the_parameter():
    plot = gg.ggplot(DATA, gg.aes("x", "x", color="v")) + gg.geom_point()
    with pytest.raises(SpecError, match=r"n_breaks.*whole number"):
        (plot + gg.scale_color_steps(n_breaks=0)).to_svg()
