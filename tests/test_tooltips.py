"""Hover tooltips of HTML output: the payload the engine writes for the runtime.

Expected texts were measured by hovering real lets-plot 4.11 plots in a browser
(plan.md 4.2); comments name the lets-plot behaviour each test pins.
"""

import json
import re
import warnings

import ggrust as gg
import polars as pl
import pytest

FRAME = pl.DataFrame(
    {
        "height": [150.25, 160.5, 170.125, 180.0, 165.333333, 175.0],
        "weight": [50.0, 60.5, 68.0, 80.0, 62.0, 74.0],
        "sex": ["f", "f", "m", "m", "f", "m"],
        "age": [20, 31, 45, 52, 28, 39],
        "name": ["Ann", "Bea", "Carl", "Dan", "Eve", "Finn"],
    }
)


def payload(plot) -> list[dict]:
    """Layers of the tooltip payload of ``plot``'s notebook HTML ([] without one)."""
    html = plot._repr_html_()
    found = re.search(r'class="ggrust-tooltips">(.*?)</script>', html)
    return json.loads(found.group(1))["layers"] if found else []


def texts(column: dict | None) -> list[str | None]:
    """Decode a dictionary-encoded text column to one text per row."""
    if column is None:
        return []
    return [None if code < 0 else column["values"][code] for code in column["codes"]]


def lines(layer: dict) -> list[tuple[str | None, list[str | None]]]:
    return [(line["label"], texts(line)) for line in layer["lines"]]


def scatter(**options):
    return gg.ggplot(FRAME, gg.aes("height", "weight")) + gg.geom_point(**options)


def test_default_tooltip_shows_non_axis_mappings_and_axis_values():
    plot = gg.ggplot(FRAME, gg.aes("height", "weight", color="sex", size="age")) + gg.geom_point()
    (layer,) = payload(plot)
    assert layer["lookup"] == "nearest"
    assert layer["targets"]["kind"] == "points"
    # `sex` has fewer than 5 levels (the legend shows it): only the marker color.
    assert lines(layer) == [("age", ["20", "31", "45", "52", "28", "39"])]
    assert texts(layer["marker"])[:3] == ["#e41a1c", "#e41a1c", "#377eb8"]
    # Positions use the scale's tooltip format: 100 breaks over the domain.
    assert texts(layer["axis_x"]) == ["150.3", "160.5", "170.1", "180", "165.3", "175"]
    assert texts(layer["axis_y"]) == ["50", "60.5", "68", "80", "62", "74"]


def test_only_axis_mappings_give_axis_tooltips_only():
    (layer,) = payload(scatter())
    assert layer["lines"] == [] and "marker" not in layer
    assert texts(layer["axis_x"])[2] == "170.1"


def test_discrete_variables_with_five_levels_are_shown():
    plot = gg.ggplot(FRAME, gg.aes("height", "weight", color="name")) + gg.geom_point()
    (layer,) = payload(plot)
    assert lines(layer) == [("name", ["Ann", "Bea", "Carl", "Dan", "Eve", "Finn"])]


def test_layer_tooltips_lines_title_and_format():
    tooltips = (
        gg.layer_tooltips()
        .line("person|@name")
        .line("@age")
        .line("age is @age years")
        .line("^color")
        .title("Person @name")
        .format("@weight", ".2f")
    )
    plot = gg.ggplot(FRAME, gg.aes("height", "weight")) + gg.geom_point(
        tooltips=tooltips.line("w|@weight")
    )
    (layer,) = payload(plot)
    assert texts(layer["title"])[2] == "Person Carl"
    found = lines(layer)
    # `^color` is not mapped: lets-plot drops the line.
    assert [label for label, _ in found] == ["person", None, None, "w"]
    assert found[0][1][2] == "Carl"
    assert found[2][1][2] == "age is 45 years"
    # A variable's format applies to the aesthetic it is mapped to ...
    assert found[3][1][2] == "68.00"
    # ... but not to the axis tooltip, which follows the scale.
    assert texts(layer["axis_y"])[2] == "68"


def test_variables_list_uses_data_labels():
    (layer,) = payload(scatter(tooltips=["name", "age", "height"]))
    assert [label for label, _ in lines(layer)] == ["name", "age", "height"]
    # `height` is mapped to x: shown through the x scale's format.
    assert lines(layer)[2][1][:2] == ["150.3", "160.5"]


def test_list_tooltips_equal_layer_tooltips_variables():
    assert payload(scatter(tooltips=["name"])) == payload(
        scatter(tooltips=gg.layer_tooltips(["name"]))
    )


def test_unmapped_numbers_use_general_format():
    data = FRAME.with_columns(bmi=pl.col("weight") / (pl.col("height") / 100) ** 2)
    plot = gg.ggplot(data, gg.aes("height", "weight")) + gg.geom_point(tooltips=["bmi"])
    (layer,) = payload(plot)
    assert lines(layer)[0][1][0] == "22.1483"  # 22.148331...: `{,~g}`, 6 significant digits


def test_scale_name_labels_the_line():
    plot = (
        gg.ggplot(FRAME, gg.aes("height", "weight", color="age"))
        + gg.geom_point()
        + gg.scale_color_gradient(name="Age (years)")
    )
    (layer,) = payload(plot)
    assert lines(layer)[0][0] == "Age (years)"


def test_bars_have_one_target_per_segment_along_x():
    plot = gg.ggplot(FRAME, gg.aes("sex", fill="name")) + gg.geom_bar()
    (layer,) = payload(plot)
    assert layer["lookup"] == "hover_x" and layer["targets"]["kind"] == "rects"
    assert len(layer["targets"]["rows"]) == 6
    # Stacked segments show their own count, not the stacked height.
    assert lines(layer) == [
        ("count", ["1"] * 6),
        ("name", ["Ann", "Bea", "Carl", "Dan", "Eve", "Finn"]),
    ]
    assert texts(layer["axis_x"]) == ["f", "f", "m", "m", "f", "m"]
    assert "axis_y" not in layer


def test_single_position_line_has_no_label():
    plot = gg.ggplot(FRAME, gg.aes("height")) + gg.geom_histogram(bins=4)
    (layer,) = payload(plot)
    assert lines(layer)[0][0] == ""
    assert sorted(set(lines(layer)[0][1])) == ["1", "3"]


def test_boxplot_statistics_join_the_tooltip():
    plot = gg.ggplot(FRAME, gg.aes("sex", "height")) + gg.geom_boxplot()
    (boxes,) = [layer for layer in payload(plot) if layer["targets"]["kind"] == "rects"]
    assert lines(boxes) == [
        ("y max", ["165.3", "180"]),
        ("upper", ["162.9", "177.5"]),
        ("middle", ["160.5", "175"]),
        ("lower", ["155.4", "172.6"]),
        ("y min", ["150.3", "170.1"]),
    ]
    assert texts(boxes["axis_x"]) == ["f", "m"] and "axis_y" not in boxes


def test_violin_and_ridges_show_their_stat_variable():
    violin = payload(gg.ggplot(FRAME, gg.aes("sex", "height")) + gg.geom_violin())[0]
    assert violin["targets"]["kind"] == "vertices"
    assert [label for label, _ in lines(violin)] == ["violinwidth"]
    ridges = payload(gg.ggplot(FRAME, gg.aes("height", "sex")) + gg.geom_area_ridges())[0]
    assert [label for label, _ in lines(ridges)] == ["height"]
    assert "axis_x" in ridges and "axis_y" not in ridges


def test_tooltips_none_and_text_layers_have_no_payload():
    assert payload(scatter(tooltips="none")) == []
    text = gg.ggplot(FRAME, gg.aes("height", "weight", label="name")) + gg.geom_text()
    assert payload(text) == []
    # Text layers show tooltips when lines are given.
    with_lines = gg.ggplot(FRAME, gg.aes("height", "weight", label="name")) + gg.geom_text(
        tooltips=gg.layer_tooltips().line("@name")
    )
    assert texts(payload(with_lines)[0]["lines"][0])[0] == "Ann"


def test_static_outputs_carry_no_payload():
    plot = scatter(tooltips=["name"])
    assert "ggrust-tooltips" not in plot.to_svg()
    assert "ggrust-tooltips" in plot.to_html()
    assert "ggrust-tooltips" in plot.to_html(iframe=True)


def test_unknown_tooltip_variable_warns_and_drops_the_line():
    plot = scatter(tooltips=gg.layer_tooltips().line("@nope").line("@name"))
    with pytest.warns(gg.GgrustWarning, match=r"'@nope' is not a variable"):
        (layer,) = payload(plot)
    assert len(layer["lines"]) == 1
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        plot.to_svg()  # static output reads no tooltip


def test_anchor_and_min_width():
    tooltips = gg.layer_tooltips(["name"]).anchor("top_right").min_width(150)
    (layer,) = payload(scatter(tooltips=tooltips))
    assert layer["anchor"] == "top_right" and layer["min_width"] == 150
    bad = gg.layer_tooltips(["name"]).anchor("upper_right")
    with pytest.warns(gg.GgrustWarning, match="unknown anchor 'upper_right'"):
        (layer,) = payload(scatter(tooltips=bad))
    assert "anchor" not in layer


def test_facets_have_one_entry_per_panel():
    plot = scatter(tooltips=["name"]) + gg.facet_wrap("sex")
    layers = payload(plot)
    assert len(layers) == 2
    assert layers[0]["panel"] != layers[1]["panel"]
    assert texts(layers[0]["lines"][0]) == ["Ann", "Bea", "Eve"]
    assert texts(layers[1]["lines"][0]) == ["Carl", "Dan", "Finn"]


def test_coord_flip_puts_x_values_on_the_vertical_axis():
    plot = gg.ggplot(FRAME, gg.aes("sex", "height")) + gg.geom_boxplot() + gg.coord_flip()
    boxes = payload(plot)[0]
    assert texts(boxes["axis_y"]) == ["f", "m"] and "axis_x" not in boxes


def test_blank_axis_text_hides_the_axis_tooltip():
    plot = scatter() + gg.theme(axis_text_x=gg.element_blank())
    (layer,) = payload(plot)
    assert "axis_x" not in layer and "axis_y" in layer


def test_grid_collects_every_plot():
    grid = gg.gggrid([scatter(tooltips=["name"]), scatter(tooltips=["age"])], ncol=2)
    layers = payload(grid)
    assert [lines(layer)[0][0] for layer in layers] == ["name", "age"]
    assert layers[0]["panel"][0] < layers[1]["panel"][0]


def test_tooltip_text_cannot_break_the_page():
    data = FRAME.with_columns(name=pl.lit("</script><script>alert(1)</script><!--"))
    plot = gg.ggplot(data, gg.aes("height", "weight")) + gg.geom_point(tooltips=["name"])
    html = plot._repr_html_()
    assert html.count("</script>") == 2  # the payload's and the runtime's own
    assert texts(payload(plot)[0]["lines"][0])[0].startswith("</script>")


def test_tooltip_columns_follow_layer_data():
    other = pl.DataFrame({"u": [1.0, 2.0], "v": [2.0, 1.0], "tag": ["p", "q"]})
    plot = scatter() + gg.geom_point(gg.aes("u", "v"), data=other, tooltips=["tag"])
    layers = payload(plot)
    assert texts(layers[1]["lines"][0]) == ["p", "q"]


def test_labs_title_labels_the_line():
    # lets-plot scale name: guide title (`labs`) first, then the scale's `name`.
    plot = (
        gg.ggplot(FRAME, gg.aes("height", "weight", size="age"))
        + gg.geom_point()
        + gg.labs(size="Age")
    )
    (layer,) = payload(plot)
    assert lines(layer)[0][0] == "Age"


def test_unsupported_tooltip_column_warns_once():
    import datetime

    data = FRAME.with_columns(day=pl.lit(datetime.date(2026, 1, 2)))
    plot = gg.ggplot(data, gg.aes("height", "weight")) + gg.geom_point(
        tooltips=gg.layer_tooltips().line("@day").line("@name")
    )
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        (layer,) = payload(plot)
    messages = [str(warning.message) for warning in caught]
    assert len(messages) == 1, messages
    assert "'day'" in messages[0] and "tooltips cannot show it yet" in messages[0]
    assert "invalid plot specification" not in messages[0]
    assert texts(layer["lines"][0]) == [None] * 6
    assert texts(layer["lines"][1])[0] == "Ann"
