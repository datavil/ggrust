"""Spelling aliases, standalone ``stat_*`` layers and the small new names of tier 4."""

from __future__ import annotations

import ggrust as gg
import pytest
from ggrust.plot.aliases import ALIASES

DATA = {
    "x": [1.0, 2.0, 2.5, 3.0, 3.5, 4.0, 5.0, 6.0, 6.5, 7.0],
    "y": [2.0, 1.0, 2.5, 3.0, 2.0, 3.5, 4.0, 3.0, 5.0, 4.5],
    "g": ["a", "b"] * 5,
}


@pytest.mark.parametrize(("alias", "target"), sorted(ALIASES.items()))
def test_alias_is_its_target(alias: str, target: str) -> None:
    assert getattr(gg, alias) is getattr(gg, target)
    assert alias in gg.__all__
    assert target in gg.__all__


def test_aliases_cover_every_color_scale() -> None:
    colors = {name for name in gg.__all__ if name.startswith("scale_color_")}
    colours = {name.replace("colour", "color") for name in ALIASES if "scale_colour_" in name}
    assert colors <= colours


def test_alias_specs_match_targets() -> None:
    assert gg.scale_colour_hue(name="t").as_dict() == gg.scale_color_hue(name="t").as_dict()
    assert gg.coord_equal(ratio=2).as_dict() == gg.coord_fixed(ratio=2).as_dict()
    assert gg.scale_fill_gray().as_dict() == gg.scale_fill_grey().as_dict()


# (function, matching geom function, stat name, mapping)
STAT_CASES = [
    (gg.stat_bin, gg.geom_histogram, "bin", gg.aes("x")),
    (gg.stat_bin2d, gg.geom_bin2d, "bin2d", gg.aes("x", "y")),
    (gg.stat_boxplot, gg.geom_boxplot, "boxplot", gg.aes("g", "y")),
    (gg.stat_count, gg.geom_bar, "count", gg.aes("g")),
    (gg.stat_density, gg.geom_density, "density", gg.aes("x")),
    (gg.stat_density2d, gg.geom_density2d, "density2d", gg.aes("x", "y")),
    (gg.stat_identity, gg.geom_point, "identity", gg.aes("x", "y")),
    (gg.stat_sina, gg.geom_sina, "sina", gg.aes("g", "y")),
    (gg.stat_smooth, gg.geom_smooth, "smooth", gg.aes("x", "y")),
    (gg.stat_ydensity, gg.geom_violin, "ydensity", gg.aes("g", "y")),
]


@pytest.mark.parametrize(("stat_function", "geom_function", "stat", "mapping"), STAT_CASES)
def test_stat_layer_is_the_geom_with_that_stat(
    stat_function: object, geom_function: object, stat: str, mapping: gg.plot.core.FeatureSpec
) -> None:
    layer = stat_function(mapping)
    expected = geom_function(mapping, stat=stat)
    assert layer.as_dict() == expected.as_dict()
    plot = gg.ggplot(DATA) + layer
    assert plot.to_svg().startswith("<svg")


@pytest.mark.parametrize(("stat_function", "geom_function", "stat", "mapping"), STAT_CASES)
def test_stat_layer_with_another_geom(
    stat_function: object, geom_function: object, stat: str, mapping: gg.plot.core.FeatureSpec
) -> None:
    layer = stat_function(mapping, geom="point")
    spec = layer.as_dict()
    assert spec["geom"] == "point"
    assert spec["stat"] == stat


def test_stat_layer_forwards_stat_parameters() -> None:
    spec = gg.stat_bin(gg.aes("x"), bins=4, geom="line").as_dict()
    assert spec["bins"] == 4
    frame = (gg.ggplot(DATA) + gg.stat_bin(gg.aes("x"), bins=4)).layer_data(0)
    assert frame.height == 4


def test_stat_boxplot_keeps_the_outlier_layer() -> None:
    data = {"g": ["a"] * 6, "y": [1.0, 2.0, 2.0, 3.0, 2.5, 40.0]}
    spec = (gg.ggplot(data, gg.aes("g", "y")) + gg.stat_boxplot()).as_dict()
    assert [layer["stat"] for layer in spec["layers"]] == ["boxplot", "boxplot_outlier"]


def test_stat_layer_rejects_a_bad_mapping() -> None:
    with pytest.raises(ValueError, match="must be an aes"):
        gg.stat_identity("x", geom="line")


def test_scale_linetype_sets_the_legend() -> None:
    data = {"t": [1, 2, 1, 2], "v": [1, 2, 2, 1], "g": list("aabb")}
    plot = (
        gg.ggplot(data, gg.aes("t", "v", linetype="g"))
        + gg.geom_line()
        + gg.scale_linetype(name="Group", labels=["first", "second"])
    )
    assert plot.as_dict()["scales"][0] == {
        "aesthetic": "linetype",
        "name": "Group",
        "labels": ["first", "second"],
    }
    svg = plot.to_svg()
    assert "Group" in svg and "second" in svg


def test_position_identity() -> None:
    assert gg.position_identity().as_dict() == {"name": "identity"}
    data = {"m": ["a", "a"], "s": ["n", "s"], "v": [3, 2]}
    stacked = gg.ggplot(data, gg.aes("m", "v", fill="s")) + gg.geom_bar(stat="identity")
    plot = gg.ggplot(data, gg.aes("m", "v", fill="s")) + gg.geom_bar(
        stat="identity", position=gg.position_identity()
    )
    assert plot.layer_data(0)["ymax"].max() == 3
    assert stacked.layer_data(0)["ymax"].max() == 5


def test_scale_sqrt() -> None:
    spec = gg.scale_x_sqrt(name="area").as_dict()
    assert spec["trans"] == "sqrt" and spec["aesthetic"] == "x"
    assert gg.scale_y_sqrt().as_dict()["aesthetic"] == "y"
    plot = gg.ggplot({"a": [1, 4, 9, 16]}, gg.aes("a", "a")) + gg.geom_point() + gg.scale_x_sqrt()
    assert plot.to_svg().startswith("<svg")


def test_stat_layer_rejects_a_stat_argument() -> None:
    with pytest.raises(TypeError, match=r"stat_bin\(\) sets stat='bin' itself"):
        gg.stat_bin(gg.aes("x"), stat="count")
