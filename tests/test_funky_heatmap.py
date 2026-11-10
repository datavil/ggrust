"""funky_heatmap (bistro), a port of the R package funkyheatmap 0.5.

Checks the ported geometry (column / row positions, funky rectangles, palettes,
legend defaults) against funkyheatmap's formulas, the layers it builds, the
ggrust additions (names at the bottom, rounded headers, right legends, text that
fits the canvas) and the errors.
"""

import re
from pathlib import Path

import polars as pl
import pytest
from ggrust import aes, geom_point
from ggrust._ggrust import text_width
from ggrust.bistro import funky_heatmap, position_arguments
from ggrust.bistro.funky_heatmap import (
    _NUMERICAL_PALETTES,
    _Column,
    _column_positions,
    _funky_rectangle,
    _Palette,
    _Row,
    _row_positions,
    _smear,
)
from ggrust.errors import ColumnNotFoundError, SpecError

ICONS = Path(__file__).parents[1] / "docs" / "data" / "scib_icons"


def _scores() -> pl.DataFrame:
    return pl.DataFrame(
        {
            "id": ["Alpha", "Beta", "Gamma", "Delta"],
            "speed": [0.9, 0.6, 0.4, 0.1],
            "accuracy": [0.7, 0.8, 0.3, 0.5],
            "memory": [0.2, 0.5, 0.9, 0.7],
        }
    )


def _geoms(plot: object) -> list[str]:
    return [layer["geom"] for layer in plot.as_dict()["layers"]]  # ty: ignore[unresolved-attribute]


def _size(svg: str) -> tuple[float, float]:
    match = re.search(r'<svg[^>]*?width="([\d.]+)" height="([\d.]+)"', svg)
    assert match, svg[:200]
    return float(match.group(1)), float(match.group(2))


def _column(column_id: str, width: float, group: str | None, overlay: bool = False) -> _Column:
    return _Column(
        id=column_id, name=column_id, geom="rect", group=group, palette=None, width=width,
        overlay=overlay, legend=False, draw_outline=True, id_color=None, id_size=None,
    )  # fmt: skip


# --------------------------------------------------------------------------- #
# Ported geometry                                                              #
# --------------------------------------------------------------------------- #


def test_column_positions_follow_funkyheatmap() -> None:
    # xsep: 0.1 inside a group, 0.5 between groups; overlay sits on the previous column.
    columns = [
        _column("a", 6, "g1"),
        _column("b", 1, "g1"),
        _column("c", 4, "g2"),
        _column("label", 1, "g2", overlay=True),
    ]
    _column_positions(columns, position_arguments())
    extents = [value for column in columns for value in (column.xmin, column.xmax)]
    assert extents == pytest.approx([0.1, 6.1, 6.2, 7.2, 7.7, 11.7, 7.7, 11.7])
    assert columns[2].x == pytest.approx(9.7)


def test_row_positions_follow_funkyheatmap() -> None:
    rows = [_Row("r1", "a"), _Row("r2", "a"), _Row("r3", "b"), _Row("r4", "b")]
    _row_positions(rows, position_arguments())
    # y = -(i * row_height + cumsum(ysep)), bigspace 1.2 at a group change.
    assert [row.y for row in rows] == pytest.approx([-1.1, -2.2, -4.4, -5.5])
    # Stripes restart in each group.
    assert [row.colour_background for row in rows] == [True, False, True, False]


def test_funky_rectangle_is_a_rounded_square_then_a_circle() -> None:
    # >= 0.8: full cell, corner (0.9 - 0.8 * trans) * side, trans in 0.5..1.
    assert _funky_rectangle(0, 1, 0, 1, 1.0) == pytest.approx((0, 1, 0, 1, 0.1))
    assert _funky_rectangle(0, 1, 0, 1, 0.8) == pytest.approx((0, 1, 0, 1, 0.5))
    # < 0.8: a centred circle (corner 0.5) of side (value / 0.8 * 0.9 + 0.1).
    xmin, xmax, ymin, ymax, corner = _funky_rectangle(0, 1, 0, 1, 0.4)
    assert (xmax - xmin, ymax - ymin, corner) == pytest.approx((0.55, 0.55, 0.5))
    assert (xmin + xmax) / 2 == pytest.approx(0.5)


def test_smear_matches_r_color_ramp_palette() -> None:
    # colorRampPalette(c("black", "white"))(3): 127.5 rounds half up.
    assert _smear(["#000000", "#FFFFFF"], 3) == ["#000000", "#808080", "#FFFFFF"]
    blues = _NUMERICAL_PALETTES["Blues"]
    assert len(blues) == 101
    assert (blues[0], blues[-1]) == ("#011636", "#F7FBFF")  # dark first, as rev()


def test_palette_lookup_rounds_like_r() -> None:
    palette = _Palette(("#000000", "#111111", "#222222"))
    assert palette.lookup(0.25) == "#000000"  # round(0.5) = 0 (half to even)
    assert palette.lookup(0.75) == "#222222"  # round(1.5) = 2
    assert palette.lookup(None) == "#444444FF"
    assert palette.lookup(1.5) == "#444444FF"
    named = _Palette(("#FF0000", "#00FF00"), ("a", "b"))
    assert named.lookup("b") == "#00FF00"
    assert named.lookup("z") == "#444444FF"
    assert named.middle() == "#FF0000"  # ceiling(2 / 2) = 1


# --------------------------------------------------------------------------- #
# Built plots                                                                  #
# --------------------------------------------------------------------------- #


def test_defaults_infer_geoms_and_legends() -> None:
    plot = funky_heatmap(_scores())
    geoms = _geoms(plot)
    # Stripes, funky rects (polygons), the auto legend's rects, text.
    assert geoms[0] == "rect"
    assert "polygon" in geoms
    assert "text" in geoms
    svg = plot.to_svg()
    # Names default to title-cased ids; the numeric palette gets a legend.
    for text in ["Id", "Speed", "Accuracy", "Memory", "numerical_palette", "Alpha"]:
        assert f">{text}<" in svg


def test_every_cell_geom_renders(tmp_path: Path) -> None:
    frame = _scores().with_columns(
        share=pl.struct(a=pl.Series([1, 2, 0, 1]), b=pl.Series([1, 1, 3, 0])),
        icon=pl.lit(str(ICONS / "genes.png")),
        label=pl.Series(["x", "y", None, "z"]),
    )
    column_info = [
        {"id": "id", "geom": "text", "options": {"hjust": 0}},
        {"id": "speed", "geom": "bar", "palette": "numbers"},
        {"id": "label", "geom": "text", "overlay": True, "name": None},
        {"id": "accuracy", "geom": "circle", "palette": "numbers"},
        {"id": "memory", "geom": "rect", "palette": "numbers"},
        {"id": "speed", "geom": "funkyrect", "palette": "numbers"},
        {"id": "share", "geom": "pie", "palette": "shares"},
        {"id": "icon", "geom": "image"},
    ]
    plot = funky_heatmap(frame, column_info=column_info, palettes={"shares": "Set1"})
    assert {"rect", "polygon", "segment", "text", "image"} <= set(_geoms(plot))
    plot.to_png(tmp_path / "cells.png")
    assert (tmp_path / "cells.png").read_bytes()[:8] == b"\x89PNG\r\n\x1a\n"
    svg = plot.to_svg()
    assert svg.count("<image") == 4
    assert ">y<" in svg and ">None<" not in svg


def test_missing_sizes_draw_nothing() -> None:
    frame = pl.DataFrame({"id": ["a", "b", "c"], "value": [0.2, None, 0.9]})
    column_info = [{"id": "value", "geom": "circle", "palette": "numbers"}]
    plot = funky_heatmap(frame, column_info=column_info, legends=[])
    polygons = [layer for layer in plot.as_dict()["layers"] if layer["geom"] == "polygon"]
    groups = {row for layer in polygons for row in layer["data"]["group"]}
    circles = [layer for layer in polygons if len(layer["data"]["group"]) > 0]
    assert len(groups) == 2 + 11  # two cells, eleven legend circles
    assert circles


def test_column_groups_headers_and_abc() -> None:
    column_info = [
        {"id": "id", "geom": "text", "group": "method"},
        {"id": "speed", "geom": "bar", "group": "scores", "palette": "blues"},
    ]
    column_groups = [
        {"Category": "Method", "group": "method", "palette": "black"},
        {"Category": "Scores", "group": "scores", "palette": "blues"},
    ]
    palettes = {"blues": "Blues", "black": ["black", "black"]}
    svg = funky_heatmap(
        _scores(), column_info=column_info, column_groups=column_groups, palettes=palettes
    ).to_svg()
    assert ">a)<" in svg and ">b)<" in svg
    assert ">Scores<" in svg
    # The header color is the palette's middle color (index ceiling(101 / 2)).
    middle = _NUMERICAL_PALETTES["Blues"][50].lower()
    assert middle in svg.lower()
    no_abc = funky_heatmap(
        _scores(),
        column_info=column_info,
        column_groups=column_groups,
        palettes=palettes,
        add_abc=False,
    ).to_svg()
    assert ">a)<" not in no_abc


def test_rounded_headers_names_at_bottom_and_right_legends() -> None:
    column_info = [
        {"id": "id", "geom": "text", "group": "method"},
        {"id": "speed", "geom": "bar", "group": "scores", "palette": "blues", "name": "Speed"},
    ]
    column_groups = [
        {"header": "Method", "group": "method", "palette": "black"},
        {"header": "Scores", "group": "scores", "palette": "blues"},
    ]
    palettes = {"blues": "Blues", "black": ["black", "black"]}
    top = funky_heatmap(
        _scores(), column_info=column_info, column_groups=column_groups, palettes=palettes
    )
    bottom = funky_heatmap(
        _scores(),
        column_info=column_info,
        column_groups=column_groups,
        palettes=palettes,
        position_args=position_arguments(
            col_annot_position="bottom",
            col_annot_angle=90,
            col_group_radius=0.5,
            legend_position="right",
        ),
    )
    # Rounded headers are polygons; plain headers are rects.
    header_polygons = [layer for layer in bottom.as_dict()["layers"] if layer["geom"] == "polygon"]
    assert header_polygons
    assert not [layer for layer in top.as_dict()["layers"] if layer["geom"] == "polygon"]
    # Legends to the right make the plot wider than tall-ish, below make it taller.
    top_width, top_height = _size(top.to_svg())
    right_width, right_height = _size(bottom.to_svg())
    assert right_width > top_width
    assert right_height < top_height + 200


def test_long_text_grows_the_canvas() -> None:
    legend = {"title": "A very long legend title that overflows", "palette": "blues"}
    column_info = [{"id": "speed", "geom": "circle", "palette": "blues"}]
    plot = funky_heatmap(
        _scores(), column_info=column_info, palettes={"blues": "Blues"}, legends=[legend]
    )
    width, _ = _size(plot.to_svg())
    title_px = text_width(legend["title"], 4 * 72.27 / 25.4 * 96 / 72, "bold")
    assert width >= title_px


def test_plus_adds_layers_in_table_coordinates() -> None:
    plot = funky_heatmap(_scores(), legends=[]) + geom_point(
        aes(x="x", y="y"), data=pl.DataFrame({"x": [1.0], "y": [-1.1]})
    )
    assert _geoms(plot)[-1] == "point"
    assert plot.to_svg().startswith("<svg")


def test_scale_column_false_keeps_values() -> None:
    frame = pl.DataFrame({"id": ["a", "b"], "value": [0.5, 0.6]})
    column_info = [{"id": "value", "geom": "bar", "palette": "numbers", "width": 4}]
    scaled = funky_heatmap(frame, column_info=column_info, legends=[])
    raw = funky_heatmap(frame, column_info=column_info, legends=[], scale_column=False)

    def bar_widths(plot: object) -> list[float]:
        layers = plot.as_dict()["layers"]  # ty: ignore[unresolved-attribute]
        bars = [layer for layer in layers if layer["geom"] == "rect" and "fill" in layer["data"]]
        data = bars[0]["data"]
        return [high - low for low, high in zip(data["xmin"], data["xmax"], strict=True)]

    assert bar_widths(scaled) == pytest.approx([0.0, 4.0])
    assert bar_widths(raw) == pytest.approx([2.0, 2.4])


# --------------------------------------------------------------------------- #
# Errors                                                                       #
# --------------------------------------------------------------------------- #


def test_data_needs_an_id_column() -> None:
    with pytest.raises(ColumnNotFoundError, match="needs an 'id' column"):
        funky_heatmap(pl.DataFrame({"name": ["a"], "value": [1.0]}))


def test_duplicate_ids_raise() -> None:
    with pytest.raises(SpecError, match="must be unique"):
        funky_heatmap(pl.DataFrame({"id": ["a", "a"], "value": [1.0, 2.0]}))


def test_unknown_geom_suggests_a_close_match() -> None:
    with pytest.raises(SpecError, match=r"geom='circel'.*did you mean 'circle'"):
        funky_heatmap(_scores(), column_info=[{"id": "speed", "geom": "circel"}])


def test_unknown_column_suggests_a_close_match() -> None:
    with pytest.raises(ColumnNotFoundError, match=r"'sped'.*did you mean 'speed'") as error:
        funky_heatmap(_scores(), column_info=[{"id": "sped"}])
    assert error.value.path == "column_info[0].id"


def test_unknown_palette_name() -> None:
    with pytest.raises(SpecError, match=r"unknown palette name; did you mean 'Blues'"):
        funky_heatmap(
            _scores(),
            column_info=[{"id": "speed", "geom": "circle", "palette": "p"}],
            palettes={"p": "Bleus"},
        )


def test_undescribed_group() -> None:
    with pytest.raises(SpecError, match="group 'scores' is used but not described"):
        funky_heatmap(
            _scores(),
            column_info=[{"id": "speed", "group": "scores"}],
            column_groups=[{"group": "method", "level1": "Method"}],
        )


def test_image_must_be_png(tmp_path: Path) -> None:
    path = tmp_path / "icon.txt"
    path.write_text("not an image")
    frame = pl.DataFrame({"id": ["a"], "icon": [str(path)]})
    with pytest.raises(SpecError, match="is not a PNG image"):
        funky_heatmap(frame, column_info=[{"id": "icon", "geom": "image"}])
    missing = pl.DataFrame({"id": ["a"], "icon": [str(tmp_path / "none.png")]})
    with pytest.raises(SpecError, match="cannot read image"):
        funky_heatmap(missing, column_info=[{"id": "icon", "geom": "image"}])


def test_text_legend_needs_values() -> None:
    with pytest.raises(SpecError, match="needs 'values'"):
        funky_heatmap(_scores(), legends=[{"title": "T", "geom": "text", "labels": ["a"]}])


def test_position_arguments_validate() -> None:
    with pytest.raises(SpecError, match=r"legend_position='rigth'.*did you mean 'right'"):
        position_arguments(legend_position="rigth")  # ty: ignore[invalid-argument-type]
    with pytest.raises(SpecError, match="row_height"):
        position_arguments(row_height=0)
    with pytest.raises(SpecError, match="expected a number >= 0"):
        position_arguments(col_space=-1)


# --------------------------------------------------------------------------- #
# Bug-hunt regressions                                                         #
# --------------------------------------------------------------------------- #


def test_ungrouped_column_next_to_a_group_gets_the_small_space() -> None:
    columns = [_column("n", 6, None), _column("x", 1, "g1"), _column("y", 1, "g1")]
    _column_positions(columns, position_arguments())
    assert columns[1].xmin == pytest.approx(6.2)  # R: NA group change is col_space


def test_header_without_palette_draws_no_box() -> None:
    column_info = [
        {"id": "speed", "geom": "circle", "group": "one", "palette": "blues"},
        {"id": "memory", "geom": "circle", "group": "two", "palette": "blues"},
    ]
    plot = funky_heatmap(_scores(), column_info=column_info, palettes={"blues": "Blues"})
    rects = [layer for layer in plot.as_dict()["layers"] if layer["geom"] == "rect"]
    assert all(layer.get("alpha") != 0.25 for layer in rects)
    assert ">One<" in plot.to_svg()


def test_default_names_follow_str_to_title() -> None:
    frame = pl.DataFrame({"id": ["a"], "t-cell": [1.0], "a/b_c": [2.0]})
    svg = funky_heatmap(frame, legends=[]).to_svg()
    assert ">T-Cell<" in svg and ">A/B C<" in svg


def test_multi_line_names_fit_the_canvas() -> None:
    frame = pl.DataFrame({"id": ["a"], "value": [1.0]})
    one = funky_heatmap(
        frame,
        column_info=[{"id": "value", "geom": "circle", "name": "A"}],
        legends=[{"palette": "numerical_palette", "enabled": False}],
        position_args=position_arguments(col_annot_angle=0),
    )
    six = funky_heatmap(
        frame,
        column_info=[{"id": "value", "geom": "circle", "name": "A\nB\nC\nD\nE\nF"}],
        legends=[{"palette": "numerical_palette", "enabled": False}],
        position_args=position_arguments(col_annot_angle=0),
    )
    one_width, one_height = _size(one.to_svg())
    six_width, six_height = _size(six.to_svg())
    # One line fits the 3-unit name box; six lines (~95 px) outgrow it.
    assert six_height > one_height + 40
    assert six_width < one_width + 40  # width of the widest line, not of all lines


def test_pies_use_id_size() -> None:
    frame = pl.DataFrame({"id": ["a"], "p": [{"u": 1, "v": 0}], "q": [{"u": 0, "v": 1}]})
    column_info = [{"id": "p", "geom": "pie", "id_size": "q", "palette": "set"}]
    plot = funky_heatmap(
        frame, column_info=column_info, palettes={"set": {"u": "#FF0000", "v": "#0000FF"}},
        legends=[{"palette": "set", "enabled": False}],
    )  # fmt: skip
    polygons = [layer for layer in plot.as_dict()["layers"] if layer["geom"] == "polygon"]
    assert set(polygons[0]["data"]["fill"]) == {"#0000FF"}


def test_bar_legend_accepts_named_colors() -> None:
    column_info = [{"id": "speed", "geom": "bar", "palette": "mono"}]
    plot = funky_heatmap(_scores(), column_info=column_info, palettes={"mono": ["white", "black"]})
    assert plot.to_svg().startswith("<svg")


def test_legend_palette_without_column_needs_a_geom() -> None:
    with pytest.raises(SpecError, match="no column uses palette 'extra'"):
        funky_heatmap(_scores(), palettes={"extra": "Blues"}, legends=[{"palette": "extra"}])


def test_too_many_rows_raise_a_spec_error() -> None:
    frame = pl.DataFrame({"id": [str(index) for index in range(500)], "value": [1.0] * 500})
    with pytest.raises(SpecError, match="500 rows"):
        funky_heatmap(frame, legends=[])


def test_missing_id_is_listed() -> None:
    with pytest.raises(SpecError, match=r"\[None\]"):
        funky_heatmap(pl.DataFrame({"id": ["a", None], "value": [1.0, 2.0]}))
