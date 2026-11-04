"""gghighlight: ``+ gghighlight(predicate)`` greys the rest of the data.

The rewrite is pure spec (Python): grey copies in place, highlighted copies and
direct labels after. Checked through ``as_dict()``, ``layer_data()`` and renders.
"""

import io
import warnings

import ggrust as gg
import polars as pl
import pytest
from ggrust import GgrustWarning
from ggrust.errors import ColumnNotFoundError, SpecError, UnsupportedValueError
from ggrust.plot.core import PlotSpec

GREY = "#BEBEBEB2"

# Three series over four days; "c" peaks above 8, "b" never above 4.
LINES = pl.DataFrame(
    {
        "day": [1, 2, 3, 4] * 3,
        "sales": [3.0, 5.0, 4.0, 6.0, 2.0, 3.0, 4.0, 3.5, 4.0, 6.0, 9.0, 12.0],
        "shop": ["a"] * 4 + ["b"] * 4 + ["c"] * 4,
        "region": ["north"] * 8 + ["south"] * 4,
    }
)


def layers(plot: PlotSpec) -> list[dict]:
    """The plot's layers as dictionaries."""
    return plot.as_dict()["layers"]


def lines_plot(**options: object) -> PlotSpec:
    """Line plot of LINES highlighted by ``max(sales) > 8``."""
    base = gg.ggplot(LINES, gg.aes("day", "sales", color="shop")) + gg.geom_line()
    return base + gg.gghighlight(pl.col("sales").max() > 8, **options)


def test_layers_are_grey_in_place_then_highlighted_then_labelled() -> None:
    result = layers(lines_plot())
    assert [layer["geom"] for layer in result] == ["line", "line", "label_repel"]
    grey, highlighted, label = result
    assert grey["color"] == GREY
    assert "color" not in grey["mapping"]
    assert grey["mapping"]["group"] == "gghighlight_group"
    assert grey["data"].height == 12
    assert grey["inherit_aes"] is False
    assert highlighted["data"]["shop"].unique().to_list() == ["c"]
    assert "color" not in highlighted
    # The label sits at the rightmost point of the series, in its color, white box.
    assert label["data"].select("day", "sales", "shop").rows() == [(4, 12.0, "c")]
    assert label["mapping"] == {"x": "day", "y": "sales", "color": "shop", "label": "shop"}
    assert label["fill"] == "white"
    assert label["show_legend"] is False


def test_direct_labels_hide_the_color_legend() -> None:
    assert lines_plot().as_dict()["guides"] == {"color": "none", "fill": "none"}
    assert "guides" not in lines_plot(use_direct_label=False).as_dict()


def test_plot_data_becomes_the_highlighted_rows() -> None:
    # As gghighlight: layers added later see only the highlighted rows.
    plot = lines_plot(use_direct_label=False) + gg.geom_point()
    assert plot.data is not LINES
    assert plot.data["shop"].unique().to_list() == ["c"]
    assert plot.layer_data(2).height == 4


def test_sql_string_predicate_matches_polars_expression() -> None:
    expression = layers(lines_plot())[1]["data"]
    sql = gg.ggplot(LINES, gg.aes("day", "sales", color="shop")) + gg.geom_line()
    sql = sql + gg.gghighlight("max(sales) > 8")
    assert layers(sql)[1]["data"].equals(expression)


def test_predicates_are_combined_with_and() -> None:
    plot = gg.ggplot(LINES, gg.aes("day", "sales", color="shop")) + gg.geom_line()
    plot = plot + gg.gghighlight(pl.col("sales").max() > 5, pl.col("region").first() == "north")
    assert layers(plot)[1]["data"]["shop"].unique().to_list() == ["a"]


def test_per_row_predicate_falls_back_with_a_warning() -> None:
    plot = gg.ggplot(LINES, gg.aes("day", "sales", color="shop")) + gg.geom_point()
    with pytest.warns(GgrustWarning, match="per row instead"):
        plot = plot + gg.gghighlight(pl.col("sales") > 5)
    assert layers(plot)[1]["data"]["sales"].to_list() == [6.0, 6.0, 9.0, 12.0]


def test_use_group_by_false_evaluates_per_row_silently() -> None:
    plot = gg.ggplot(LINES, gg.aes("day", "sales", color="shop")) + gg.geom_point()
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        plot = plot + gg.gghighlight(pl.col("sales") > 5, use_group_by=False)
    assert layers(plot)[1]["data"].height == 4


def test_one_row_series_take_a_per_row_predicate_as_one_value() -> None:
    last_day = LINES.filter(pl.col("day") == 4)
    plot = gg.ggplot(last_day, gg.aes("shop", "sales")) + gg.geom_point()
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        plot = plot + gg.gghighlight(pl.col("sales") > 5)
    assert layers(plot)[1]["data"]["shop"].to_list() == ["a", "c"]


def test_numeric_predicate_keeps_the_largest_series() -> None:
    plot = gg.ggplot(LINES, gg.aes("day", "sales", color="shop")) + gg.geom_line()
    plot = plot + gg.gghighlight(pl.col("sales").max(), max_highlight=2)
    assert sorted(layers(plot)[1]["data"]["shop"].unique().to_list()) == ["a", "c"]


def test_numeric_per_row_predicate_keeps_the_largest_rows() -> None:
    plot = gg.ggplot(LINES, gg.aes("day", "sales")) + gg.geom_point()
    plot = plot + gg.gghighlight(pl.col("sales"), max_highlight=3)
    assert sorted(layers(plot)[1]["data"]["sales"].to_list()) == [6.0, 9.0, 12.0]


def test_no_predicate_highlights_everything_over_the_grey_copy() -> None:
    plot = gg.ggplot(LINES, gg.aes("sales", fill="shop")) + gg.geom_histogram(bins=4)
    result = layers(plot + gg.gghighlight())
    assert [layer["geom"] for layer in result] == ["histogram", "histogram"]
    assert result[0]["fill"] == GREY
    assert result[1]["data"].height == 12


def test_grey_rows_are_in_every_facet_panel() -> None:
    plot = gg.ggplot(LINES, gg.aes("sales", fill="shop")) + gg.geom_histogram(bins=4)
    plot = plot + gg.gghighlight() + gg.facet_wrap("shop")
    grey = plot.layer_data(0)
    highlighted = plot.layer_data(1)
    # Every panel counts all 12 grey rows, but only its own 4 highlighted rows.
    assert grey.group_by("panel").agg(pl.col("count").sum()).sort("panel")["count"].to_list() == [
        12.0,
        12.0,
        12.0,
    ]
    counts = highlighted.group_by("panel").agg(pl.col("count").sum())["count"].to_list()
    assert counts == [4.0, 4.0, 4.0]


def test_calculate_per_facet_groups_by_the_facet() -> None:
    plot = gg.ggplot(LINES, gg.aes("day", "sales")) + gg.geom_line() + gg.facet_wrap("region")
    # Per region: the north rows peak at 6, the south rows at 12.
    plot = plot + gg.gghighlight(pl.col("sales").max() > 8, calculate_per_facet=True)
    grey, highlighted = layers(plot)
    assert "region" in grey["data"].columns
    assert highlighted["data"]["region"].unique().to_list() == ["south"]


def test_unhighlighted_params_override_the_grey() -> None:
    plot = gg.ggplot(LINES, gg.aes("day", "sales", color="shop")) + gg.geom_line()
    plot = plot + gg.gghighlight(
        pl.col("sales").max() > 8, unhighlighted_params={"colour": "pink", "size": 0.5}
    )
    grey = layers(plot)[0]
    assert (grey["color"], grey["size"]) == ("pink", 0.5)


def test_keep_scales_adds_a_blank_layer_over_every_row() -> None:
    result = layers(lines_plot(keep_scales=True, use_direct_label=False))
    assert [layer["geom"] for layer in result] == ["line", "line", "blank"]
    assert result[2]["data"].height == 12
    assert result[2]["mapping"]["color"] == "shop"


def test_n_highlights_only_the_last_layers() -> None:
    plot = gg.ggplot(LINES, gg.aes("day", "sales", color="shop")) + gg.geom_line()
    plot = plot + gg.geom_point() + gg.gghighlight(pl.col("sales").max() > 8, n=1)
    result = layers(plot)
    assert [layer["geom"] for layer in result] == ["line", "point", "point", "label_repel"]
    # The untouched line keeps every row although the plot data is now filtered.
    assert result[0]["data"].height == 12
    assert result[1]["color"] == GREY


def test_point_labels_use_label_key_and_text_type() -> None:
    plot = gg.ggplot(LINES, gg.aes("day", "sales")) + gg.geom_point()
    plot = plot + gg.gghighlight(pl.col("sales") > 8, use_group_by=False, label_key="shop")
    label = layers(plot)[2]
    assert label["geom"] == "label_repel"
    assert label["data"]["sales"].to_list() == [9.0, 12.0]
    text = gg.ggplot(LINES, gg.aes("day", "sales", color="shop")) + gg.geom_line()
    text = text + gg.gghighlight(pl.col("sales").max() > 3, line_label_type="ggrepel_text")
    assert layers(text)[-1]["geom"] == "text_repel"
    assert "fill" not in layers(text)[-1]


def test_too_many_labels_are_skipped_unless_asked() -> None:
    many = pl.DataFrame({"x": list(range(30)), "y": list(range(30))})
    plot = gg.ggplot(many, gg.aes("x", "y")) + gg.geom_point()
    skipped = plot + gg.gghighlight(pl.col("y") >= 0, label_key="x")
    assert [layer["geom"] for layer in layers(skipped)] == ["point", "point"]
    assert "guides" not in skipped.as_dict()
    forced = plot + gg.gghighlight(pl.col("y") >= 0, label_key="x", use_direct_label=True)
    assert layers(forced)[-1]["data"].height == 30


def test_bars_get_no_labels_but_no_legend() -> None:
    plot = gg.ggplot(LINES, gg.aes("day", "sales", fill="shop")) + gg.geom_bar(stat="identity")
    with pytest.warns(GgrustWarning, match="per row instead"):
        plot = plot + gg.gghighlight(pl.col("shop") == "c")
    assert [layer["geom"] for layer in layers(plot)] == ["bar", "bar"]
    assert layers(plot)[0]["fill"] == GREY
    assert "color" not in layers(plot)[0]
    assert plot.as_dict()["guides"] == {"color": "none", "fill": "none"}


def test_layer_data_and_mapping_annotations_survive() -> None:
    frame = LINES.with_columns(pl.col("day").cast(pl.String))
    plot = gg.ggplot(frame, gg.aes(gg.as_discrete("day", levels=["4", "3", "2", "1"]), "sales"))
    plot = plot + gg.geom_point(gg.aes(color="shop")) + gg.gghighlight(pl.col("sales").max() > 8)
    assert plot.layer_data(0).sort("x")["x"].to_list()[:1] == [0.0]
    svg = plot.to_svg()
    assert svg.index(">4<") < svg.index(">1<")


def test_layers_with_own_data_and_new_scales() -> None:
    plot = gg.ggplot() + gg.new_scale_color()
    plot = plot + gg.geom_line(gg.aes("day", "sales", color="shop"), data=LINES)
    plot = plot + gg.gghighlight(pl.col("sales").max() > 8)
    result = layers(plot)
    assert plot.data is None
    assert all(layer["scale_group"] == {"color": 1} for layer in result)


def test_renders_png_svg_and_html() -> None:
    plot = lines_plot() + gg.facet_wrap("region")
    assert GREY[:7].lower() in plot.to_svg().lower()
    target = io.BytesIO()
    plot.to_png(target)
    assert target.getvalue()[:4] == b"\x89PNG"
    assert "<svg" in (plot.to_html() or "")


def test_no_layers_filters_the_plot_data() -> None:
    plot = gg.ggplot(LINES, gg.aes("day", "sales", color="shop"))
    plot = plot + gg.gghighlight(pl.col("sales").max() > 8)
    assert plot.data["shop"].unique().to_list() == ["c"]
    assert plot.layers == ()


# --- Errors --------------------------------------------------------------------------


def test_predicate_type_is_checked() -> None:
    with pytest.raises(UnsupportedValueError, match=r"predicates\[0\].*polars expression"):
        gg.gghighlight(lambda frame: frame)  # ty: ignore[invalid-argument-type]


def test_bad_sql_predicate() -> None:
    with pytest.raises(SpecError, match=r"predicates\[1\].*could not read the SQL") as error:
        gg.gghighlight("sales > 1", "max(sales >")
    assert error.value.path == "predicates[1]"


@pytest.mark.parametrize(
    ("options", "match"),
    [
        ({"n": 0}, "n=.*positive integer"),
        ({"max_highlight": True}, "max_highlight=.*positive integer"),
        ({"use_group_by": "yes"}, "use_group_by=.*True, False or None"),
        ({"label_key": 3}, "label_key=.*column name"),
        ({"unhighlighted_params": [1]}, "unhighlighted_params=.*dict"),
        ({"line_label_type": "ggrepel_labl"}, "did you mean 'ggrepel_label'"),
        ({"line_label_type": "text_path"}, "not available"),
    ],
)
def test_argument_errors(options: dict, match: str) -> None:
    with pytest.raises(SpecError, match=match):
        gg.gghighlight(**options)


def test_n_larger_than_the_layer_count() -> None:
    plot = gg.ggplot(LINES, gg.aes("day", "sales")) + gg.geom_line()
    with pytest.raises(SpecError, match=r"n=3 is larger than the number of layers \(1\)"):
        plot + gg.gghighlight(n=3)


def test_unknown_column_fails_every_layer() -> None:
    plot = gg.ggplot(LINES, gg.aes("day", "sales", color="shop")) + gg.geom_line()
    with (
        pytest.warns(GgrustWarning, match="per series"),
        pytest.raises(SpecError, match=r"no layer could be highlighted.*layers\[0\] \(line\)"),
    ):
        plot + gg.gghighlight(pl.col("price").max() > 1)


def test_unknown_column_in_one_layer_warns() -> None:
    other = pl.DataFrame({"day": [1, 2], "price": [1.0, 2.0]})
    plot = gg.ggplot(LINES, gg.aes("day", "sales")) + gg.geom_line()
    plot = plot + gg.geom_point(gg.aes(y="price"), data=other)
    with pytest.warns(GgrustWarning, match=r"left unhighlighted: layers\[1\] \(point\)"):
        plot = plot + gg.gghighlight(pl.col("sales") > 5)
    assert [layer["geom"] for layer in layers(plot)] == ["line", "point", "line"]


def test_missing_label_key() -> None:
    plot = gg.ggplot(LINES, gg.aes("day", "sales", color="shop")) + gg.geom_line()
    with pytest.raises(ColumnNotFoundError, match=r"'shp'.*did you mean 'shop'"):
        plot + gg.gghighlight(pl.col("sales").max() > 8, label_key="shp")


def test_forced_labels_need_something_to_label() -> None:
    plot = gg.ggplot(LINES, gg.aes("day", "sales")) + gg.geom_area()
    with pytest.raises(SpecError, match="no line, point or bar layer can be labelled"):
        plot + gg.gghighlight(pl.col("sales") > 5, use_direct_label=True)
    lines = gg.ggplot(LINES, gg.aes("day", "sales")) + gg.geom_line()
    with pytest.raises(SpecError, match="pass label_key="):
        lines + gg.gghighlight(pl.col("sales") > 5, use_direct_label=True)


def test_data_vectors_in_aes_are_rejected() -> None:
    plot = gg.ggplot(LINES, gg.aes("day", "sales")) + gg.geom_point(gg.aes(color=[1] * 12))
    with pytest.raises(SpecError, match="needs column names in aes"):
        plot + gg.gghighlight(pl.col("sales") > 5)


# --- Bug-hunt regressions --------------------------------------------------------------


def test_discrete_x_is_not_a_series_key() -> None:
    frame = LINES.with_columns(pl.col("day").cast(pl.String))
    plot = gg.ggplot(frame, gg.aes("day", "sales", color="shop")) + gg.geom_line()
    result = layers(plot + gg.gghighlight(pl.col("sales").max() > 8))
    assert result[0]["data"]["gghighlight_group"].n_unique() == 3
    assert result[1]["data"].height == 4
    assert result[2]["data"]["shop"].to_list() == ["c"]


def test_nan_never_ranks_first() -> None:
    frame = pl.DataFrame({"x": [1.0, 2.0, 3.0, 4.0], "y": [3.0, float("nan"), 9.0, None]})
    plot = gg.ggplot(frame, gg.aes("x", "y")) + gg.geom_point()
    plot = plot + gg.gghighlight(pl.col("y"), max_highlight=1, use_direct_label=False)
    assert layers(plot)[1]["data"]["y"].to_list() == [9.0]


def test_missing_column_error_lists_only_user_columns() -> None:
    plot = gg.ggplot(LINES, gg.aes("day", "sales")) + gg.geom_line()
    with pytest.raises(SpecError, match=r"'sale' is not in.*mean 'sales'") as error:
        plot + gg.gghighlight(pl.col("sale").max() > 1)
    assert "__row" not in str(error.value)


def test_calculate_per_facet_without_a_facet_warns() -> None:
    plot = gg.ggplot(LINES, gg.aes("day", "sales")) + gg.geom_line()
    with pytest.warns(GgrustWarning, match="before gghighlight"):
        plot + gg.gghighlight(pl.col("sales") > 5, calculate_per_facet=True)
