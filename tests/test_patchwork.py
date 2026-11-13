"""ex2.3 patchwork extras: plot tags (lets-plot parity), gggrid(design=, tag_levels=,
tag_level=), nested grid alignment and inset_element()."""

import json
import re
import sys
import warnings
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent))

import ggrust as gg
import lets_plot as lp
from ggrust.errors import PlotIndexError, SpecError
from ggrust.plot.core import PlotSpec
from test_grid_parity import _ggrust_panel_rects, _lets_plot_panel_rects

DATA = {"x": [1, 2, 3], "y": [3, 1, 2]}


def _plot(module, *extra):
    plot = module.ggplot(DATA, module.aes("x", "y")) + module.geom_point() + module.ggsize(400, 300)
    for feature in extra:
        plot = plot + feature
    return plot


def _texts(svg: str) -> list[tuple[str, float, float]]:
    """Every ggrust text: (content, x, baseline y)."""
    return [
        (match.group(3), float(match.group(1)), float(match.group(2)))
        for match in re.finditer(r'<text x="([-\d.]+)" y="([-\d.]+)"[^>]*>([^<]*)</text>', svg)
    ]


def _tags(svg: str, names: set[str]) -> list[str]:
    """Texts drawn at the default tag size (16 px) named in ``names``, in order."""
    return [
        match.group(1)
        for match in re.finditer(r'<text [^>]*font-size="16"[^>]*>([^<]*)</text>', svg)
        if match.group(1) in names
    ]


def _lets_plot_tag(svg: str) -> tuple[float, float]:
    """Centre x and baseline y of lets-plot's tag (inside translate groups)."""
    x = y = 0.0
    for match in re.finditer(
        r'<g transform="translate\(([-\d.e]+) ([-\d.e]+) \) ">\s*<text[^>]*class="plot-tag"', svg
    ):
        x, y = float(match.group(1)), float(match.group(2))
    return x, y


TAG_CASES = {
    "plot": {},
    "plot_bottom_right": {"plot_tag_position": "bottom-right"},
    "plot_point": {"plot_tag_position": [0.25, 0.75]},
    "plot_point_clamped": {"plot_tag_position": [1.5, -1]},
    "panel": {"plot_tag_location": "panel"},
    "panel_bottom_right": {"plot_tag_location": "panel", "plot_tag_position": "bottom-right"},
    "margin": {"plot_tag_location": "margin"},
    "margin_left": {"plot_tag_location": "margin", "plot_tag_position": "left"},
    "margin_right": {"plot_tag_location": "margin", "plot_tag_position": "right"},
    "margin_bottom": {"plot_tag_location": "margin", "plot_tag_position": "bottom"},
    "margin_top_right": {"plot_tag_location": "margin", "plot_tag_position": "topright"},
    "margin_point": {"plot_tag_location": "margin", "plot_tag_position": [0.5, 0.5]},
}


@pytest.mark.parametrize("name", TAG_CASES)
def test_tags_are_placed_as_lets_plot(name):
    """Panels and tag positions within real-font metrics of lets-plot (16 px text:
    ggrust's line box is 18.4 px, lets-plot's 16; 'A' is 10.7 px wide, 12.1 in
    lets-plot's estimate)."""
    options = TAG_CASES[name]
    ours = _plot(gg, gg.labs(tag="A"), gg.theme(**options)).to_svg()
    theirs = _plot(lp, lp.labs(tag="A"), lp.theme(**options)).to_svg()
    base_ours = _ggrust_panel_rects(_plot(gg).to_svg())[0]
    base_theirs = _lets_plot_panel_rects(_plot(lp).to_svg())[0]
    # The panel moves as in lets-plot (by the tag's band where it takes one).
    moved_ours = [a - b for a, b in zip(_ggrust_panel_rects(ours)[0], base_ours, strict=True)]
    moved_theirs = [
        a - b for a, b in zip(_lets_plot_panel_rects(theirs)[0], base_theirs, strict=True)
    ]
    assert moved_ours == pytest.approx(moved_theirs, abs=2.5)
    [(_, x, y)] = [text for text in _texts(ours) if text[0] == "A"]
    their_x, their_y = _lets_plot_tag(theirs)
    assert x == pytest.approx(their_x, abs=2.5)
    assert y == pytest.approx(their_y, abs=4.0)


def test_tag_prefix_suffix_and_blank():
    plot = _plot(gg, gg.labs(tag="a"), gg.theme(plot_tag_prefix="(", plot_tag_suffix=")"))
    assert _tags(plot.to_svg(), {"(a)"}) == ["(a)"]
    blank = _plot(gg, gg.labs(tag="a"), gg.theme(plot_tag="blank", plot_tag_location="margin"))
    assert _tags(blank.to_svg(), {"a"}) == []
    assert _ggrust_panel_rects(blank.to_svg()) == _ggrust_panel_rects(_plot(gg).to_svg())


def test_tag_theme_errors_name_the_option():
    with pytest.raises(SpecError, match=r"plot_tag_position.*did you mean 'top-left'"):
        _plot(gg, gg.labs(tag="A"), gg.theme(plot_tag_position="top-lef")).to_svg()
    with pytest.raises(SpecError, match=r"plot_tag_location.*'plot', 'panel' or 'margin'"):
        _plot(gg, gg.labs(tag="A"), gg.theme(plot_tag_location="figure")).to_svg()


def test_grid_tag_takes_a_margin_band():
    grid = gg.gggrid([_plot(gg), _plot(gg)]) + gg.ggsize(800, 300)
    tagged = grid + gg.labs(tag="Fig") + gg.theme(plot_tag_location="margin")
    plain_panels = _ggrust_panel_rects(grid.to_svg())
    svg = tagged.to_svg()
    assert _tags(svg, {"Fig"}) == ["Fig"]
    assert all(
        tagged_panel[1] > plain_panel[1]
        for tagged_panel, plain_panel in zip(_ggrust_panel_rects(svg), plain_panels, strict=True)
    )


# design=


def test_design_gives_each_plot_its_area():
    plots = [_plot(gg) for _ in range(3)]
    grid = gg.gggrid(plots, design="AAB\nCCB", hspace=0, vspace=0) + gg.ggsize(900, 600)
    assert (grid.ncol, grid.nrow) == (3, 2)
    assert grid.layout["areas"] == [[0, 0, 1, 0], [2, 0, 2, 1], [0, 1, 1, 1]]
    wide, tall, bottom = _ggrust_panel_rects(grid.to_svg())
    assert wide[2] > 1.8 * tall[2]
    assert tall[3] > 1.8 * wide[3]
    assert bottom[1] > wide[1] + wide[3]
    assert grid[1, 2] is plots[1]
    assert grid[0, 1] is plots[0]


def test_design_areas_follow_sorted_characters_and_empty_cells():
    plots = [_plot(gg), _plot(gg)]
    grid = gg.gggrid(plots, design="\n  b#\n  ba\n")
    # Sorted as R: a before b, whatever the order in the string.
    assert grid.layout["areas"] == [[1, 1, 1, 1], [0, 0, 0, 1]]
    with pytest.raises(PlotIndexError, match="empty cell"):
        grid[0, 1]
    fewer = gg.gggrid([_plot(gg)], design="AB")
    assert fewer.figures[1] is None
    assert len(_ggrust_panel_rects(fewer.to_svg())) == 1


@pytest.mark.parametrize(
    ("design", "message"),
    [
        ("AB\nA", "rows must be equally long"),
        ("AB\nBA", "area 'A' is not a rectangle"),
        ("A B", "holds a space"),
        ("##", "has no area"),
        ("", "empty row"),
    ],
)
def test_bad_designs_are_errors(design, message):
    with pytest.raises(SpecError, match=message) as error:
        gg.gggrid([_plot(gg)], design=design)
    assert error.value.path == "layout.design"


def test_design_conflicts_and_counts():
    with pytest.raises(SpecError, match="either ncol or design"):
        gg.gggrid([_plot(gg)], ncol=2, design="AB")
    with pytest.raises(SpecError, match="1 area but 2 plots"):
        gg.gggrid([_plot(gg), _plot(gg)], design="AA")


def test_with_layout_keeps_or_replaces_the_design():
    grid = gg.gggrid([_plot(gg), _plot(gg)], design="AB\nAB")
    assert grid.with_layout(align=True).layout["areas"] == [[0, 0, 0, 1], [1, 0, 1, 1]]
    columns = grid.with_layout(ncol=1)
    assert (columns.ncol, columns.nrow, columns.layout.get("areas")) == (1, 2, None)


def _render_raw(spec: dict) -> bytes:
    """Render a hand-edited spec of data-less plots, skipping the Python checks."""
    return gg._ggrust.render(json.dumps(spec), [], "svg", 1.0)


def test_engine_rejects_overlapping_areas():
    spec = gg.gggrid([gg.ggplot(), gg.ggplot()], design="AB").as_dict()
    spec["layout"]["areas"] = [[0, 0, 1, 0], [1, 0, 1, 0]]
    with pytest.raises(ValueError, match="overlap at row 0, column 1"):
        _render_raw(spec)


# tag_levels


def test_tags_number_plots_in_figure_order_through_nested_grids():
    inner = gg.gggrid([_plot(gg), None, _plot(gg)], ncol=3)
    grid = gg.gggrid([_plot(gg, gg.labs(tag="own")), inner, _plot(gg)], tag_levels="A")
    assert _tags(grid.to_svg(), {"A", "B", "C", "D", "own"}) == ["A", "B", "C", "D"]


@pytest.mark.parametrize(
    ("levels", "expected"),
    [
        ("a", ["a", "b", "c"]),
        ("1", ["1", "2", "3"]),
        ("i", ["i", "ii", "iii"]),
        ("I", ["I", "II", "III"]),
    ],
)
def test_tag_numberings(levels, expected):
    grid = gg.gggrid([_plot(gg) for _ in range(3)], tag_levels=levels)
    assert _tags(grid.to_svg(), set(expected)) == expected


def test_new_tag_level_numbers_a_nested_grid_below_one_tag():
    inner = gg.gggrid([_plot(gg), _plot(gg)], tag_level="new")
    grid = gg.gggrid([_plot(gg), inner, _plot(gg)], tag_levels=["A", "1"])
    names = {"A", "B1", "B2", "C"}
    grid = grid + gg.theme(plot_tag_suffix=".")
    assert _tags(grid.to_svg(), {f"{name}." for name in names}) == ["A.", "B1.", "B2.", "C."]


def _column():
    """Two stacked plots with a color legend each, tagged as one unit."""
    plot = (
        gg.ggplot(
            {"x": [1, 2, 3], "y": [3, 1, 2], "g": ["a", "b", "a"]}, gg.aes("x", "y", color="g")
        )
        + gg.geom_point()
    )
    return gg.gggrid([plot, plot], ncol=1, guides="collect", tag_level="unit")


def test_unit_tag_level_tags_a_nested_grid_once_at_its_corner():
    column = _column()
    grid = gg.gggrid([column, column], ncol=2, tag_levels="A") + gg.ggsize(800, 600)
    svg = grid.to_svg()
    texts = [text for text in _texts(svg) if text[0] in {"A", "B", "C", "D"}]
    assert [text[0] for text in texts] == ["A", "B"]
    (_, a_x, a_y), (_, b_x, b_y) = texts
    # One tag at the top-left of each column, left of its panels.
    assert a_x < 50 and 400 <= b_x < 450 and a_y == b_y < 50
    panels = _ggrust_panel_rects(svg)
    assert len(panels) == 4
    assert all(a_x < panel[0] < 400 or b_x < panel[0] for panel in panels)
    # Each column still collects its two identical legends into one.
    assert svg.count(">g</text>") == 2


def test_unit_tag_level_continues_the_numbering_and_follows_the_theme():
    inner = gg.gggrid([_plot(gg), _plot(gg)], ncol=1)
    own = _column() + gg.labs(tag="own")
    grid = gg.gggrid([own, _plot(gg), inner], ncol=3, tag_levels="A") + gg.theme(
        plot_tag=gg.element_text(size=22), plot_tag_suffix=")"
    )
    svg = grid.to_svg()
    tags = re.findall(r'<text [^>]*font-size="22"[^>]*>([^<]*)</text>', svg)
    assert tags == ["A)", "B)", "C)", "D)"]
    assert "own" not in svg


def test_unit_tag_level_silences_nested_tag_levels_and_keeps_own_tags():
    inner = gg.gggrid([_plot(gg), _plot(gg)], ncol=1, tag_levels="a")
    unit = gg.gggrid([inner, _plot(gg, gg.labs(tag="own"))], ncol=2, tag_level="unit")
    grid = gg.gggrid([unit, _plot(gg)], ncol=2, tag_levels="A")
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        svg = grid.to_svg()
    assert sorted(_tags(svg, {"A", "B", "a", "b", "own"})) == ["A", "B", "own"]
    messages = " ".join(str(warning.message) for warning in caught)
    assert "figures[0].figures[0].layout.tag_levels" in messages
    assert "tagged as one unit" in messages


def test_unit_tag_level_keeps_the_grid_untagged_outside_a_tagged_grid():
    svg = gg.gggrid([_column(), _plot(gg)]).to_svg()
    assert _tags(svg, {"A", "B", "C"}) == []


def test_tag_level_warnings_and_errors():
    inner = gg.gggrid([_plot(gg)], tag_level="new", tag_levels="a")
    grid = gg.gggrid([_plot(gg), inner], tag_levels="A")
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        assert _tags(grid.to_svg(), {"A", "B"}) == ["A", "B"]
    messages = " ".join(str(warning.message) for warning in caught)
    assert "figures[1].layout.tag_levels" in messages
    assert "'new' needs a level" in messages
    with pytest.raises(SpecError, match=r"unknown tag level 'B'.*'A', 'a', '1', 'i', 'I'"):
        gg.gggrid([_plot(gg)], tag_levels="B")
    with pytest.raises(SpecError, match="did you mean 'new'"):
        gg.gggrid([_plot(gg)], tag_level="nwe")
    with pytest.raises(SpecError, match=r"'keep', 'new' or 'unit'.*did you mean 'unit'"):
        gg.gggrid([_plot(gg)], tag_level="unti")


# nested grids under align


def test_nested_grid_panels_align_with_the_outer_grid():
    wide_labels = _plot(gg) + gg.scale_y_continuous(
        labels=["one thousand", "2", "3"], breaks=[1, 2, 3]
    )
    inner = gg.gggrid([wide_labels, _plot(gg)], ncol=2)
    grid = gg.gggrid([_plot(gg), inner], ncol=1, align=True) + gg.ggsize(800, 600)
    top, inner_left, inner_right = _ggrust_panel_rects(grid.to_svg())
    assert inner_left[0] == pytest.approx(top[0], abs=0.05)
    assert inner_right[0] + inner_right[2] == pytest.approx(top[0] + top[2], abs=0.05)
    # Without align, lets-plot's behaviour: the nested grid is not moved.
    plain = gg.gggrid([_plot(gg), inner], ncol=1) + gg.ggsize(800, 600)
    top, inner_left, _ = _ggrust_panel_rects(plain.to_svg())
    assert abs(inner_left[0] - top[0]) > 20


def test_spanning_plots_align_on_their_first_and_last_cells():
    wide_labels = _plot(gg) + gg.scale_y_continuous(
        labels=["one thousand", "2", "3"], breaks=[1, 2, 3]
    )
    grid = gg.gggrid([wide_labels, _plot(gg), _plot(gg)], design="AB\nCC", align=True)
    left, right, bottom = _ggrust_panel_rects(grid.to_svg())
    assert bottom[0] == pytest.approx(left[0], abs=0.05)
    assert bottom[0] + bottom[2] == pytest.approx(right[0] + right[2], abs=0.05)


# inset_element


def test_inset_is_placed_in_the_host_panel():
    main = _plot(gg) + gg.inset_element(_plot(gg), 0.5, 0.5, 1.0, 1.0)
    assert isinstance(main, PlotSpec)
    host, clip, inset = _ggrust_panel_rects(main.to_svg())
    assert clip == pytest.approx(
        (host[0] + host[2] / 2, host[1], host[2] / 2, host[3] / 2), abs=0.01
    )
    assert clip[0] <= inset[0] and inset[0] + inset[2] <= clip[0] + clip[2]


@pytest.mark.parametrize("align_to", ["plot", "full"])
def test_inset_areas(align_to):
    main = _plot(gg) + gg.inset_element(_plot(gg), 0, 0, 1, 1, align_to=align_to)
    host, clip, _ = _ggrust_panel_rects(main.to_svg())
    if align_to == "full":
        assert clip == (0, 0, 400, 300)
    else:
        assert clip[0] < host[0] and clip[1] + clip[3] > host[1] + host[3]
        assert clip[0] > 0 and clip[2] < 400


def test_insets_compose_with_plus_and_grids():
    detail = _plot(gg, gg.geom_line())
    main = _plot(gg) + gg.inset_element(detail, 0.6, 0.6, 1, 1, on_top=False) + gg.geom_line()
    spec = main.as_dict()
    assert [layer["geom"] for layer in spec["layers"]] == ["point", "line"]
    assert spec["insets"][0]["figure"]["kind"] == "plot"
    assert spec["insets"][0]["on_top"] is False
    svg = main.to_svg()
    # Under the host: the inset's clip comes before the host panel's.
    assert len(_ggrust_panel_rects(svg)) == 3
    assert _ggrust_panel_rects(svg)[0][2] < 200
    grid = gg.gggrid([_plot(gg), _plot(gg)]) + gg.inset_element(
        detail, 0.4, 0.4, 0.6, 0.6, align_to="full"
    )
    assert len(_ggrust_panel_rects(grid.to_svg())) == 4


def test_insets_are_tagged_after_their_host():
    tagged = _plot(gg) + gg.inset_element(_plot(gg), 0.6, 0.6, 1, 1)
    ignored = _plot(gg) + gg.inset_element(_plot(gg), 0.6, 0.6, 1, 1, ignore_tag=True)
    grid = gg.gggrid([tagged, ignored, _plot(gg)], tag_levels="A")
    assert _tags(grid.to_svg(), {"A", "B", "C", "D", "E"}) == ["A", "B", "C", "D"]


@pytest.mark.parametrize(
    ("arguments", "options", "error", "message"),
    [
        ((1.2, 0, 1, 1), {}, SpecError, r"left=1.2 is outside 0..1"),
        ((0, -0.1, 1, 1), {}, SpecError, r"bottom=-0.1 is outside 0..1"),
        ((0.5, 0, 0.5, 1), {}, SpecError, "the inset is empty"),
        ((0, 0, 1, 1), {"align_to": "panels"}, SpecError, "did you mean 'panel'"),
        ((0, 0, 1, 1), {"clip": 1}, TypeError, "clip must be True or False"),
        (("0", 0, 1, 1), {}, TypeError, "left must be a number"),
    ],
)
def test_bad_insets_are_errors(arguments, options, error, message):
    with pytest.raises(error, match=message):
        gg.inset_element(_plot(gg), *arguments, **options)
    with pytest.raises(TypeError, match="must be a plot"):
        gg.inset_element("plot", 0, 0, 1, 1)


def test_engine_checks_inset_edges():
    spec = (gg.ggplot() + gg.inset_element(gg.ggplot(), 0, 0, 1, 1)).as_dict()
    spec["insets"][0]["top"] = 2
    with pytest.raises(ValueError, match=r"insets\[0\]\.top.*between 0 and 1"):
        _render_raw(spec)
    spec["insets"][0]["top"] = 1
    spec["insets"][0]["align_to"] = "panels"
    with pytest.raises(ValueError, match=r"insets\[0\]\.align_to.*did you mean 'panel'"):
        _render_raw(spec)


def test_inset_only_columns_reach_the_engine():
    import polars as pl

    detail = pl.DataFrame({"x": [1.0, 2.0], "y": [2.0, 1.0], "z": [5.0, 6.0]})
    inset = gg.inset_element(gg.ggplot(detail, gg.aes("x", "z")) + gg.geom_point(), 0.5, 0.5, 1, 1)
    main = gg.ggplot({"x": [1]}, gg.aes("x", "x")) + gg.geom_point() + inset
    assert len(_ggrust_panel_rects(main.to_svg())) == 3
    grid = gg.gggrid([_plot(gg), _plot(gg)]) + inset
    assert len(_ggrust_panel_rects(grid.to_svg())) == 4


def test_empty_tag_keeps_its_affixes_as_lets_plot():
    plot = _plot(gg, gg.labs(tag=""), gg.theme(plot_tag_prefix="(", plot_tag_suffix=")"))
    assert _tags(plot.to_svg(), {"()"}) == ["()"]


def test_bad_tag_levels_type_and_engine_tag_level():
    with pytest.raises(SpecError, match="tag_levels must be"):
        gg.gggrid([_plot(gg)], tag_levels=3)
    spec = gg.gggrid([gg.ggplot()]).as_dict()
    spec["layout"]["tag_level"] = "x"
    with pytest.raises(ValueError, match=r"tag_level.*'keep', 'new' or 'unit'"):
        _render_raw(spec)
