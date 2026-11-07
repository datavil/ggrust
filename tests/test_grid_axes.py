"""Plan 10.4: collected axes and axis titles, ``gggrid(axes=, axis_titles=)`` (patchwork)."""

import json
import re
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent))

import ggrust as gg
from ggrust.errors import SpecError
from ggrust.plot.core import PlotSpec
from ggrust.plot.subplots import SupPlotsSpec
from test_grid_parity import _ggrust_panel_rects


def _plot(offset: float = 0.0, *, x_title: str = "day", y_title: str = "sales") -> PlotSpec:
    data = {"day": [1.0, 2.0, 3.0, 4.0], "sales": [v + offset for v in (3.0, 1.0, 4.0, 2.0)]}
    return gg.ggplot(data, gg.aes("day", "sales")) + gg.geom_point() + gg.labs(x=x_title, y=y_title)


def _texts(svg: str) -> list[tuple[str, float, float]]:
    """Every text: (content, x, baseline y)."""
    return [
        (match.group(3), float(match.group(1)), float(match.group(2)))
        for match in re.finditer(r'<text x="([-\d.]+)" y="([-\d.]+)"[^>]*>([^<]*)</text>', svg)
    ]


def _count(svg: str, text: str) -> int:
    return sum(1 for content, _, _ in _texts(svg) if content == text)


def _grid(**options: object) -> SupPlotsSpec:
    plots = [_plot(), _plot(1.0), _plot(2.0), _plot(3.0)]
    return gg.gggrid(plots, ncol=2, sharex=True, sharey=True, **options) + gg.ggsize(600, 400)


def test_options_reach_the_spec_only_when_set():
    layout = _grid(axes="collect", axis_titles="collect_y").as_dict()["layout"]
    assert layout["axes"] == "collect"
    assert layout["axis_titles"] == "collect_y"
    plain = _grid().as_dict()["layout"]
    assert "axes" not in plain and "axis_titles" not in plain


@pytest.mark.parametrize("key", ["axes", "axis_titles"])
def test_bad_values_suggest_a_close_one(key):
    with pytest.raises(SpecError, match="did you mean 'collect'") as error:
        gg.gggrid([_plot()], **{key: "colect"})
    assert error.value.path == f"layout.{key}"
    assert "'keep', 'collect', 'collect_x', 'collect_y'" in str(error.value)


def test_engine_rejects_bad_values():
    spec = gg.gggrid([gg.ggplot(), gg.ggplot()]).as_dict()
    spec["layout"]["axes"] = "colect_x"
    with pytest.raises(ValueError, match=r"layout\.axes.*did you mean 'collect_x'"):
        gg._ggrust.render(json.dumps(spec), [], "svg", 1.0)


def test_shared_axes_are_drawn_once_per_run():
    keep = _grid().to_svg()
    collected = _grid(axes="collect").to_svg()
    # x labels only on the bottom row, y labels only on the left column.
    assert _count(keep, "3") == 4
    assert _count(collected, "3") == 2
    assert _count(keep, "6") == 4
    assert _count(collected, "6") == 2
    # Titles stay with axis_titles="keep".
    assert _count(collected, "day") == 4
    # The freed room goes to the panels.
    keep_rects, collected_rects = _ggrust_panel_rects(keep), _ggrust_panel_rects(collected)
    assert collected_rects[0][3] > keep_rects[0][3]
    assert collected_rects[1][2] > keep_rects[1][2]


def test_collect_x_keeps_the_y_axes():
    svg = _grid(axes="collect_x").to_svg()
    assert _count(svg, "3") == 2
    assert _count(svg, "6") == 4


def test_titles_collect_into_one_centred_title():
    svg = _grid(axes="collect", axis_titles="collect").to_svg()
    assert _count(svg, "day") == 1
    assert _count(svg, "sales") == 1
    rects = _ggrust_panel_rects(svg)
    (_, x, _) = next(text for text in _texts(svg) if text[0] == "day")
    left, right = rects[2][0], rects[3][0] + rects[3][2]
    assert x == pytest.approx((left + right) / 2, abs=1.0)


def test_hidden_titles_keep_the_row_level():
    plots = [_plot(), _plot()]
    svg = (gg.gggrid(plots, ncol=2, axis_titles="collect") + gg.ggsize(600, 300)).to_svg()
    assert _count(svg, "day") == 1
    first, second = _ggrust_panel_rects(svg)
    assert first[1] + first[3] == pytest.approx(second[1] + second[3], abs=0.05)


def test_different_axes_and_titles_are_kept():
    # No sharing: different y ranges give different axes.
    plots = [_plot(), _plot(10.0), _plot(x_title="hour"), _plot(10.0)]
    svg = (
        gg.gggrid(plots, ncol=2, axes="collect", axis_titles="collect_x") + gg.ggsize(600, 400)
    ).to_svg()
    assert _count(svg, "2") > 0
    # Column 0 has 'day' over 'hour': two titles; column 1 collects to one.
    assert _count(svg, "hour") == 1
    assert _count(svg, "day") == 2


def test_empty_cells_and_faceted_plots_break_runs():
    faceted = _plot() + gg.facet_wrap("day")

    def svg(**options: object) -> str:
        plots = [_plot(), _plot(), faceted, None]
        grid = gg.gggrid(plots, ncol=2, sharex=True, **options) + gg.ggsize(600, 400)
        return grid.to_svg()

    # Plot 0 sits above a faceted plot and plot 1 above an empty cell: no x axis
    # is collected, so every x label stays.
    assert _count(svg(axes="collect_x"), "3") == _count(svg(), "3")


def test_align_still_lines_up_the_panels():
    wide = _plot() + gg.scale_y_continuous(
        breaks=[1, 2, 3, 4], labels=["one thousand", "2", "3", "4"]
    )
    plots = [wide, _plot(), _plot(), _plot()]
    svg = (
        gg.gggrid(plots, ncol=2, sharex=True, axes="collect", axis_titles="collect", align=True)
        + gg.ggsize(600, 400)
    ).to_svg()
    top_left, top_right, bottom_left, bottom_right = _ggrust_panel_rects(svg)
    assert top_left[0] == pytest.approx(bottom_left[0], abs=0.05)
    assert top_right[0] == pytest.approx(bottom_right[0], abs=0.05)
    assert top_left[1] == pytest.approx(top_right[1], abs=0.05)
    assert bottom_left[1] + bottom_left[3] == pytest.approx(
        bottom_right[1] + bottom_right[3], abs=0.05
    )


def test_design_spans_compare_their_columns():
    plots = [_plot(), _plot(), _plot()]
    svg = (
        gg.gggrid(plots, design="AB\nCC", sharex=True, axes="collect") + gg.ggsize(600, 400)
    ).to_svg()
    # A and B span one column each, C both: no x axis is collected (3 x labels '3');
    # B's y axis goes (A's and C's stay: 2 y labels '3').
    assert _count(svg, "3") == 5


def test_discrete_axes_are_collected():
    # Regression (bug hunt 10.4): a discrete scale's level set printed in random
    # order, so equal discrete axes never matched.
    data = {"fruit": ["apple", "banana", "cherry", "date", "fig", "grape"], "v": [1, 2, 3, 4, 5, 6]}
    plots = [gg.ggplot(data, gg.aes("fruit", "v")) + gg.geom_point() for _ in range(4)]
    svg = (
        gg.gggrid(plots, ncol=2, sharex=True, sharey=True, axes="collect") + gg.ggsize(600, 400)
    ).to_svg()
    assert _count(svg, "banana") == 2


@pytest.mark.parametrize(
    "options",
    [
        {"axes": "collect"},
        {"axes": "collect", "axis_titles": "collect"},
        {"axes": "collect_x", "axis_titles": "collect_y", "align": True},
    ],
)
def test_panels_stay_equal_as_patchwork(options):
    # The dropped axes' room goes to every panel, not to the plot that dropped it.
    rects = _ggrust_panel_rects(_grid(**options).to_svg())
    widths = {round(rect[2], 1) for rect in rects}
    heights = {round(rect[3], 1) for rect in rects}
    assert len(widths) == 1, rects
    assert len(heights) == 1, rects


def test_equal_panels_follow_relative_widths():
    plots = [_plot(), _plot(1.0), _plot(2.0), _plot(3.0)]
    grid = gg.gggrid(plots, ncol=2, sharey=True, widths=[1, 2], axes="collect") + gg.ggsize(
        600, 400
    )
    left, right, _, _ = _ggrust_panel_rects(grid.to_svg())
    assert right[2] == pytest.approx(2 * left[2], abs=0.5)


def test_keep_leaves_lets_plot_cells_alone():
    # Without collecting, cells stay equal (lets-plot) and wide labels shrink their panel.
    wide = _plot() + gg.scale_y_continuous(
        breaks=[1, 2, 3, 4], labels=["one thousand", "2", "3", "4"]
    )
    rects = _ggrust_panel_rects((gg.gggrid([wide, _plot()], ncol=2) + gg.ggsize(600, 300)).to_svg())
    assert rects[0][2] < rects[1][2] - 20


def test_uneven_rows_still_get_equal_panels():
    # Bug hunt: a title on one plot, or an empty cell under another, used to leave
    # its neighbours' panels taller; collecting aligns the panels as patchwork.
    titled = [_plot() + gg.ggtitle("T", "sub"), _plot(1.0), _plot(2.0), _plot(3.0)]
    for plots in (titled, [_plot(), _plot(1.0), _plot(2.0)]):
        grid = gg.gggrid(plots, ncol=2, sharex=True, sharey=True, axes="collect") + gg.ggsize(
            600, 400
        )
        rects = _ggrust_panel_rects(grid.to_svg())
        assert len({round(rect[3], 1) for rect in rects}) == 1, rects
        assert len({round(rect[2], 1) for rect in rects}) == 1, rects


def test_fixed_aspect_plots_do_not_widen_their_column():
    plots = [_plot() + gg.coord_fixed(), _plot(), _plot(), _plot()]
    # Without align (which lines plot 2 up with the fixed panel), the aspect
    # slack must not count as axis room: both columns stay as wide.
    grid = gg.gggrid(
        plots, ncol=2, sharex=True, sharey=True, axes="collect_x", align=False
    ) + gg.ggsize(600, 400)
    rects = _ggrust_panel_rects(grid.to_svg())
    assert rects[2][2] == pytest.approx(rects[3][2], abs=1.0)
