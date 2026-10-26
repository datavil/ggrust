"""Legend placement and key grids follow lets-plot.

Positions are read from ggrust's SVG: the panel is the clip rectangle, each legend box
is the first rectangle of a ``legend`` group. Exact pixels differ from lets-plot (real
font widths), so the tests check the placement rules lets-plot uses (measured against
its SVGs in Phase 1.4b) and the key arrangement (column and row of every label).
"""

import re

import ggrust as gg
import pytest

N = 30
FRAME = {
    "x": [float(index) for index in range(N)],
    "y": [float((index * 7) % 10) for index in range(N)],
    "c": [["b", "a", "c"][index % 3] for index in range(N)],
    "v": [float(index) for index in range(N)],
    "k": [f"category {index % 7}" for index in range(N)],
}
WIDTH, HEIGHT, MARGIN = 600, 400, 3
NUMBER = r"([\d.\-]+)"


def plot(**mapping):
    return gg.ggplot(FRAME, gg.aes("x", "y", **mapping)) + gg.geom_point()


def panel(svg: str) -> tuple[float, float, float, float]:
    match = re.search(
        rf'<clipPath[^>]*><rect x="{NUMBER}" y="{NUMBER}" width="{NUMBER}" height="{NUMBER}"', svg
    )
    assert match, svg[:500]
    return tuple(float(value) for value in match.groups())


def legend_boxes(svg: str) -> list[tuple[float, float, float, float]]:
    """(x, y, width, height) of every legend box, in drawing order."""
    pattern = (
        rf'<g class="legend">\s*<g[^>]*>\s*<rect x="{NUMBER}" y="{NUMBER}" '
        rf'width="{NUMBER}" height="{NUMBER}"'
    )
    return [tuple(float(value) for value in match.groups()) for match in re.finditer(pattern, svg)]


def label_grid(svg: str, labels: list[str]) -> dict[str, tuple[int, int]]:
    """(column, row) of each legend label, from the ranks of its x and y."""
    positions = {}
    for x, y, text in re.findall(rf'<text x="{NUMBER}" y="{NUMBER}"[^>]*>([^<]*)</text>', svg):
        if text in labels:
            positions[text] = (round(float(x)), round(float(y)))
    columns = sorted({x for x, _ in positions.values()})
    rows = sorted({y for _, y in positions.values()})
    return {text: (columns.index(x), rows.index(y)) for text, (x, y) in positions.items()}


def centre(start: float, extent: float) -> float:
    return start + extent / 2


def test_default_legend_sits_right_centred_on_the_panel():
    svg = plot(color="c").to_svg()
    (px, py, pw, ph), [(x, y, w, h)] = panel(svg), legend_boxes(svg)
    assert x + w == pytest.approx(WIDTH - MARGIN, abs=0.01)
    assert centre(y, h) == pytest.approx(centre(py, ph), abs=0.01)
    assert x > px + pw


def test_bottom_legend_is_below_the_axis_centred_on_the_panel():
    svg = (plot(color="c") + gg.theme(legend_position="bottom")).to_svg()
    (px, py, pw, ph), [(x, y, w, h)] = panel(svg), legend_boxes(svg)
    assert y + h == pytest.approx(HEIGHT - MARGIN, abs=0.01)
    assert centre(x, w) == pytest.approx(centre(px, pw), abs=0.01)
    assert y > py + ph
    # Horizontal by default: one row of keys.
    assert set(label_grid(svg, ["a", "b", "c"]).values()) == {(0, 0), (1, 0), (2, 0)}


def test_top_and_left_legends_sit_at_the_plot_margin():
    top = (plot(color="c") + gg.theme(legend_position="top")).to_svg()
    (_, py, _, _), [(_, y, _, h)] = panel(top), legend_boxes(top)
    assert y == pytest.approx(MARGIN, abs=0.01)
    assert y + h < py
    left = (plot(color="c") + gg.theme(legend_position="left")).to_svg()
    (px, _, _, _), [(x, _, w, _)] = panel(left), legend_boxes(left)
    assert x == pytest.approx(MARGIN, abs=0.01)
    assert x + w < px


def test_top_legend_goes_below_the_title():
    svg = (plot(color="c") + gg.theme(legend_position="top") + gg.ggtitle("Title")).to_svg()
    [(_, y, _, _)] = legend_boxes(svg)
    title_y = float(re.search(rf'<text x="{NUMBER}" y="{NUMBER}"[^>]*>Title<', svg).group(2))
    assert y > title_y


def test_justification_moves_the_legend_along_the_panel():
    svg = (
        plot(color="c") + gg.theme(legend_position="bottom", legend_justification="left")
    ).to_svg()
    (px, _, _, _), [(x, _, _, _)] = panel(svg), legend_boxes(svg)
    assert x == pytest.approx(px, abs=0.01)
    svg = (plot(color="c") + gg.theme(legend_justification=[0.5, 1])).to_svg()
    (_, py, _, _), [(_, y, _, _)] = panel(svg), legend_boxes(svg)
    assert y == pytest.approx(py, abs=0.01)


@pytest.mark.parametrize(
    ("position", "justification"),
    [([0.9, 0.9], None), ([1, 1], [1, 1]), ([0, 0], [0, 0]), ([0.5, 0.2], [0.5, 0])],
)
def test_inside_legends_are_placed_in_the_panel(position, justification):
    theme = gg.theme(legend_position=position, legend_justification=justification)
    svg = (plot(color="c") + theme).to_svg()
    (px, py, pw, ph), [(x, y, w, h)] = panel(svg), legend_boxes(svg)
    # The panel keeps the full width: an inside legend takes no space.
    assert px + pw == pytest.approx(WIDTH - 2 * MARGIN, abs=0.01)
    justify_x, justify_y = justification or (0.5, 0.5)
    assert x + w * justify_x == pytest.approx(px + pw * position[0], abs=0.01)
    assert y + h * (1 - justify_y) == pytest.approx(py + ph * (1 - position[1]), abs=0.01)


def test_legend_position_none_hides_every_legend():
    svg = (plot(color="v", size="c") + gg.theme(legend_position="none")).to_svg()
    assert legend_boxes(svg) == []
    px, _, pw, _ = panel(svg)
    assert px + pw == pytest.approx(WIDTH - 2 * MARGIN, abs=0.01)


def test_legends_stack_left_justified_or_sit_side_by_side_top_justified():
    stacked = (plot(color="v", size="c") + gg.theme(legend_position="bottom")).to_svg()
    (x1, y1, w1, h1), (x2, y2, _, _) = legend_boxes(stacked)
    assert x1 == x2
    assert y2 == pytest.approx(y1 + h1 + 10, abs=0.01)  # legend_spacing
    row = (
        plot(color="v", size="c") + gg.theme(legend_position="bottom", legend_box="horizontal")
    ).to_svg()
    (x1, y1, w1, _), (x2, y2, _, _) = legend_boxes(row)
    assert y1 == y2
    assert x2 == pytest.approx(x1 + w1 + 10, abs=0.01)
    centred = (plot(color="v", size="c") + gg.theme(legend_box_just="center")).to_svg()
    (x1, _, w1, _), (x2, _, w2, _) = legend_boxes(centred)
    assert centre(x1, w1) == pytest.approx(centre(x2, w2), abs=0.01)


def test_horizontal_colorbar_runs_left_to_right_with_labels_below():
    svg = (plot(color="v") + gg.theme(legend_position="bottom")).to_svg()
    [(x, y, w, h)] = legend_boxes(svg)
    bar = re.search(rf'<rect x="{NUMBER}" y="{NUMBER}" width="115" height="23" fill="([^"]+)"', svg)
    assert bar, "a 115 x 23 px bar (5 x 1 key sizes)"
    labels = {
        text: (float(lx), float(ly))
        for lx, ly, text in re.findall(rf'<text x="{NUMBER}" y="{NUMBER}"[^>]*>(\d+)</text>', svg)
        if x <= float(lx) <= x + w and y <= float(ly) <= y + h
    }
    assert list(labels) == ["0", "5", "10", "15", "20", "25"]
    xs = [position[0] for position in labels.values()]
    assert xs == sorted(xs)
    assert all(position[1] > float(bar.group(2)) + 23 for position in labels.values())


LABELS = [f"category {index}" for index in range(7)]


def grid_of(order: list[list[int]]) -> dict[str, tuple[int, int]]:
    """Expected (column, row) from rows of label indices."""
    return {
        LABELS[index]: (column, row)
        for row, indices in enumerate(order)
        for column, index in enumerate(indices)
    }


@pytest.mark.parametrize(
    ("guide", "theme", "expected"),
    [
        # lets-plot: column-major unless byrow; ncol/nrow fix one side.
        (gg.guide_legend(ncol=2), None, grid_of([[0, 4], [1, 5], [2, 6], [3]])),
        (gg.guide_legend(nrow=2, byrow=True), None, grid_of([[0, 1, 2, 3], [4, 5, 6]])),
        (gg.guide_legend(nrow=3), None, grid_of([[0, 3, 6], [1, 4], [2, 5]])),
        # Horizontal: up to 5 columns per row, filled column by column.
        (None, gg.theme(legend_position="bottom"), grid_of([[0, 2, 4, 6], [1, 3, 5]])),
        (
            gg.guide_legend(ncol=2, byrow=True),
            gg.theme(legend_position="bottom"),
            grid_of([[0, 1], [2, 3], [4, 5], [6]]),
        ),
        (
            gg.guide_legend(ncol=3),
            gg.theme(legend_position="bottom"),
            grid_of([[0, 3, 6], [1, 4], [2, 5]]),
        ),
        # Vertical at the bottom.
        (
            None,
            gg.theme(legend_position="bottom", legend_direction="vertical"),
            grid_of([[index] for index in range(7)]),
        ),
    ],
)
def test_key_grids_match_lets_plot(guide, theme, expected):
    built = plot(color="k")
    if guide is not None:
        built += gg.guides(color=guide)
    if theme is not None:
        built += theme
    assert label_grid(built.to_svg(), LABELS) == expected


def test_grid_options_of_one_legend_combine_across_aesthetics():
    built = plot(color="k", size="k") + gg.guides(
        color=gg.guide_legend(ncol=2), size=gg.guide_legend(byrow=True)
    )
    assert label_grid(built.to_svg(), LABELS) == grid_of([[0, 1], [2, 3], [4, 5], [6]])


@pytest.mark.parametrize("key", ["ncol", "nrow"])
def test_zero_counts_are_rejected(key):
    built = plot(color="k") + gg.guides(color=gg.guide_legend(**{key: 0}))
    with pytest.raises(ValueError, match=rf"guides.color.{key}': expected a positive count"):
        built.to_svg()


def test_guides_counts_win_over_the_scale_guide():
    # lets-plot combines the scale's guide first, then guides(): the last count wins.
    built = (
        plot(color="k")
        + gg.scale_color_hue(guide=gg.guide_legend(ncol=3))
        + gg.guides(color=gg.guide_legend(ncol=2))
    )
    assert label_grid(built.to_svg(), LABELS) == grid_of([[0, 4], [1, 5], [2, 6], [3]])


@pytest.mark.parametrize(
    ("theme", "expected"),
    [
        # A stack honours left/right only, a row top/bottom only; other values centre.
        ({"legend_box_just": "right"}, "end"),
        ({"legend_box_just": "bottom"}, "centre"),
        ({"legend_box_just": "top"}, "centre"),
        ({"legend_box": "horizontal", "legend_box_just": "bottom"}, "end"),
        ({"legend_box": "horizontal", "legend_box_just": "left"}, "centre"),
        ({"legend_box": "horizontal"}, "start"),
    ],
)
def test_box_justification_applies_across_the_box(theme, expected):
    svg = (plot(color="v", size="c") + gg.theme(**theme)).to_svg()
    (x1, y1, w1, h1), (x2, y2, w2, h2) = legend_boxes(svg)
    if theme.get("legend_box") == "horizontal":
        start, extent = (y1, h1), (y2, h2)
    else:
        start, extent = (x1, w1), (x2, w2)
    first = {"start": start[0], "centre": centre(*start), "end": sum(start)}[expected]
    second = {"start": extent[0], "centre": centre(*extent), "end": sum(extent)}[expected]
    assert first == pytest.approx(second, abs=0.01)
