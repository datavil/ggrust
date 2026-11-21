"""``theme_ggplot2()``: the grey theme plus ggplot2's hue palette on discrete colors."""

from __future__ import annotations

import re

import ggrust as gg
import pytest
from ggrust.errors import SpecError

DATA = {"x": [1, 2, 3, 4], "y": [1, 2, 3, 4], "g": ["a", "b", "c", "d"]}
# R: scales::hue_pal()(4), ggplot2's default discrete colors.
HUE_4 = ["#F8766D", "#7CAE00", "#00BFC4", "#C77CFF"]
# lets-plot's default: ColorBrewer Set1.
SET1_4 = ["#E41A1C", "#377EB8", "#4DAF4A", "#984EA3"]


def colors(plot: gg.PlotSpec, element: str = "circle") -> list[str]:
    """The fill colors of the drawn marks, in data order, upper case."""
    found = re.findall(rf'<{element}[^>]*?fill="(#[0-9a-fA-F]{{6}})"', plot.to_svg())
    return [color.upper() for color in found]


def points(*extra: gg.FeatureSpec) -> gg.PlotSpec:
    plot = gg.ggplot(DATA, gg.aes("x", "y", color="g")) + gg.geom_point()
    for feature in extra:
        plot = plot + feature
    return plot


def test_discrete_colors_are_ggplot2_hue():
    assert colors(points(gg.theme_ggplot2()))[:4] == HUE_4


def test_without_the_preset_colors_stay_set1():
    assert colors(points())[:4] == SET1_4
    assert colors(points(gg.theme_grey()))[:4] == SET1_4


def test_hue_matches_r_for_many_levels():
    levels = [f"level {index:02}" for index in range(12)]
    data = {"x": list(range(12)), "g": levels}
    plot = gg.ggplot(data, gg.aes("x", "x", color="g")) + gg.geom_point() + gg.theme_ggplot2()
    # R: scales::hue_pal()(12)
    expected = (
        "#F8766D #DE8C00 #B79F00 #7CAE00 #00BA38 #00C08B "
        "#00BFC4 #00B4F0 #619CFF #C77CFF #F564E3 #FF64B0"
    )
    assert colors(plot)[:12] == expected.split()


def test_scale_discrete_follows_the_preset():
    assert colors(points(gg.scale_color_discrete(), gg.theme_ggplot2()))[:4] == HUE_4
    reversed_colors = colors(points(gg.scale_color_discrete(direction=-1), gg.theme_ggplot2()))
    assert reversed_colors[:4] == HUE_4[::-1]


def test_fill_follows_the_preset():
    plot = gg.ggplot(DATA, gg.aes("g", "y", fill="g")) + gg.geom_bar(stat="identity")
    assert set(HUE_4) <= set(colors(plot + gg.theme_ggplot2(), "rect"))


def test_explicit_palettes_win():
    manual = gg.scale_color_manual(values=["black", "red", "blue", "green"])
    assert colors(points(manual, gg.theme_ggplot2()))[:4] == [
        "#000000",
        "#FF0000",
        "#0000FF",
        "#008000",
    ]
    brewer = colors(points(gg.scale_color_brewer(palette="Set1"), gg.theme_ggplot2()))
    assert brewer[:4] == SET1_4


def test_continuous_colors_are_unchanged():
    data = {"x": [1.0, 2.0, 3.0], "v": [0.0, 0.5, 1.0]}
    plot = gg.ggplot(data, gg.aes("x", "x", color="v")) + gg.geom_point()
    assert colors(plot + gg.theme_ggplot2()) == colors(plot)


def test_theme_after_keeps_and_complete_theme_after_replaces():
    kept = points(gg.theme_ggplot2(), gg.theme(legend_position="bottom"))
    assert colors(kept)[:4] == HUE_4
    replaced = points(gg.theme_ggplot2(), gg.theme_bw())
    assert colors(replaced)[:4] == SET1_4


def test_looks_like_theme_grey_apart_from_colors():
    plain = gg.ggplot(DATA, gg.aes("x", "y")) + gg.geom_point()
    assert (plain + gg.theme_ggplot2()).to_svg() == (plain + gg.theme_grey()).to_svg()
    assert gg.theme_defaults("ggplot2") == gg.theme_defaults("grey")


def test_spec_names_the_theme():
    assert gg.theme_ggplot2().as_dict() == {"name": "ggplot2"}


def test_unknown_theme_name_lists_ggplot2():
    with pytest.raises(SpecError, match="did you mean 'ggplot2'"):
        gg.theme_defaults("ggplot3")  # ty: ignore[invalid-argument-type]


def test_grid_theme_reaches_every_plot():
    grid = gg.gggrid([points(), points()]) + gg.theme_ggplot2()
    found = re.findall(r'<circle[^>]*?fill="(#[0-9a-fA-F]{6})"', grid.to_svg())
    assert {color.upper() for color in found} == set(HUE_4)


def test_one_level_takes_the_first_hue_like_r():
    data = {"x": [1], "y": [1], "g": ["a"]}
    plot = gg.ggplot(data, gg.aes("x", "y", color="g")) + gg.geom_point()
    # R: scales::hue_pal()(1); lets-plot's scale_color_hue() keeps the middle hue.
    assert colors(plot + gg.theme_ggplot2()) == ["#F8766D"]
    assert colors(plot + gg.scale_color_hue()) == ["#7CAE00"]
