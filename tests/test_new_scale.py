"""ggnewscale: ``new_scale_color()``, ``new_scale_fill()``, ``new_scale("size")``.

Layers, scales, guides and labs after a marker get their own scale and legend; the
spec carries this as ``scale_group`` on layers and scales and ``"color:1"`` guide
keys. Legends are read from the SVG (titles and labels, in drawing order).
"""

import ggrust as gg
import pytest
from ggrust.errors import SpecError
from ggrust.plot.core import FeatureSpec, PlotSpec
from test_legend_parity import legends

FRAME = {
    "x": [1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
    "y": [2.0, 4.0, 3.0, 5.0, 4.5, 1.5],
    "score": [0.1, 0.5, 0.9, 0.3, 0.7, 0.2],
    "kind": ["apple", "pear", "plum", "apple", "pear", "plum"],
    "shop": ["north", "south", "north", "south", "north", "south"],
}


def titles(plot: FeatureSpec) -> list[str]:
    """Legend titles of ``plot``, in drawing order."""
    return [title for title, _ in legends(plot.to_svg(), ours=True)]


def labels(plot: FeatureSpec) -> dict[str, list[str]]:
    """Key labels per discrete legend title of ``plot`` (text key glyphs "a" dropped)."""
    found = legends(plot.to_svg(), ours=True)
    return {title: [text for text, _ in keys if text != "a"] for title, keys in found}


def points_and_labels() -> PlotSpec:
    """A continuous color scale for points, a discrete one for text labels."""
    return (
        gg.ggplot(FRAME, gg.aes("x", "y"))
        + gg.geom_point(gg.aes(color="score"), size=6)
        + gg.new_scale_color()
        + gg.geom_text(gg.aes(label="kind", color="kind"), nudge_y=0.25)
        + gg.scale_color_brewer(palette="Dark2")
        + gg.labs(color="Fruit")
    )


def test_spec_tags_what_comes_after_the_marker():
    spec = points_and_labels().as_dict()
    assert "scale_group" not in spec["layers"][0]
    assert spec["layers"][1]["scale_group"] == {"color": 1}
    assert spec["scales"][0]["scale_group"] == {"color": 1}
    assert spec["guides"] == {"color:1": {"title": "Fruit"}}


def test_without_a_marker_the_spec_is_unchanged():
    plot = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point(gg.aes(color="kind"))
    plot = plot + gg.scale_color_brewer() + gg.labs(color="Fruit")
    spec = plot.as_dict()
    assert "scale_group" not in spec["layers"][0]
    assert "scale_group" not in spec["scales"][0]
    assert spec["guides"] == {"color": {"title": "Fruit"}}


def test_continuous_points_and_discrete_labels_get_two_legends():
    plot = points_and_labels()
    assert titles(plot) == ["score", "Fruit"]
    assert labels(plot)["Fruit"] == ["apple", "pear", "plum"]
    # The text labels take Dark2, the points the default gradient.
    svg = plot.to_svg()
    assert 'fill="#1b9e77"' in svg.lower() or "rgb(27,158,119)" in svg


def test_fill_plus_fill_keeps_both_legends_and_palettes():
    plot = (
        gg.ggplot(FRAME, gg.aes("x", "y"))
        + gg.geom_tile(gg.aes(fill="shop"), width=0.9, height=0.9)
        + gg.scale_fill_manual(values=["#dddddd", "#999999"])
        + gg.new_scale_fill()
        + gg.geom_point(gg.aes(fill="kind"), shape=21, size=6)
        + gg.scale_fill_manual(values=["red", "green", "blue"], name="Fruit")
    )
    assert titles(plot) == ["shop", "Fruit"]
    found = labels(plot)
    assert found["shop"] == ["north", "south"]
    assert found["Fruit"] == ["apple", "pear", "plum"]


def test_equal_titles_in_different_groups_stay_two_legends():
    plot = (
        gg.ggplot(FRAME, gg.aes("x", "y", fill="kind"))
        + gg.geom_point(shape=21, size=8)
        + gg.new_scale_fill()
        + gg.geom_point(shape=21, size=3)
        + gg.scale_fill_brewer(palette="Set2")
    )
    assert titles(plot) == ["kind", "kind"]


def test_new_scale_size_and_guides_follow_the_marker():
    plot = (
        gg.ggplot(FRAME, gg.aes("x", "y"))
        + gg.geom_point(gg.aes(size="score"))
        + gg.guides(size="none")
        + gg.new_scale("size")
        + gg.geom_point(gg.aes(size="y"), color="red", alpha=0.4)
    )
    # The guide given before the marker hides the first size legend only.
    assert titles(plot) == ["y"]


def test_three_groups_and_the_colour_alias():
    plot = (
        gg.ggplot(FRAME, gg.aes("x", "y"))
        + gg.geom_point(gg.aes(colour="kind"))
        + gg.new_scale_colour()
        + gg.geom_point(gg.aes(colour="shop"), size=8, alpha=0.3)
        + gg.labs(colour="Shop")
        + gg.new_scale("colour")
        + gg.geom_line(gg.aes(colour="score"))
    )
    spec = plot.as_dict()
    assert [layer.get("scale_group") for layer in spec["layers"]] == [
        None,
        {"color": 1},
        {"color": 2},
    ]
    assert spec["guides"] == {"color:1": {"title": "Shop"}}
    assert titles(plot) == ["score", "kind", "Shop"]


def test_tooltips_name_the_layers_own_scale():
    plot = points_and_labels() + gg.labs(color="Fruit")
    html = plot.to_html()
    assert "Fruit" in html
    assert "score" in html


def test_grid_collect_merges_equal_legends_only():
    first = points_and_labels()
    second = points_and_labels()
    other = (
        gg.ggplot(FRAME, gg.aes("x", "y"))
        + gg.geom_point(gg.aes(color="shop"))
        + gg.new_scale_color()
        + gg.geom_text(gg.aes(label="kind", color="kind"))
        + gg.scale_color_brewer(palette="Dark2")
        + gg.labs(color="Fruit")
    )
    same = gg.gggrid([first, second], guides="collect")
    assert titles(same) == ["score", "Fruit"]
    mixed = gg.gggrid([first, other], guides="collect")
    # "Fruit" is equal in both plots and shows once; the other legends differ.
    assert sorted(titles(mixed)) == ["Fruit", "score", "shop"]


@pytest.mark.parametrize("name", ["x", "y", "label", "colr"])
def test_new_scale_rejects_non_style_aesthetics(name):
    with pytest.raises(SpecError, match="style aesthetics") as raised:
        gg.new_scale(name)
    assert raised.value.path == "new_aes"
    if name == "colr":
        assert "did you mean 'color'" in str(raised.value)


def test_png_render_with_two_color_scales(tmp_path):
    path = tmp_path / "new_scale.png"
    points_and_labels().to_png(str(path))
    assert path.stat().st_size > 1000
