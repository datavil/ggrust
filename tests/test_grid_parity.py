"""gggrid must match lets-plot: figure cells, panels, collected legends.

Figures are compared by their bounds (lets-plot's nested ``<svg>``, ggrust's figure
background), panels by size and by their marks as fractions of the panel (see
test_facet_parity.py). Text is measured with the embedded font, so sizes get a few
pixels of tolerance.
"""

import re
import warnings

import numpy as np
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from test_bar_parity import _attributes  # noqa: E402
from test_facet_parity import _ggrust_panels, _lets_plot_panels  # noqa: E402

RANDOM = np.random.default_rng(11)
N = 60
DATA = {
    "x": RANDOM.normal(size=N).round(3),
    "y": (RANDOM.normal(size=N) * 1000).round(1),
    "g": RANDOM.choice(["a", "b", "c"], N).tolist(),
}


def _plots(m):
    return [
        m.ggplot(DATA, m.aes("x", "y")) + m.geom_point(),
        m.ggplot(DATA, m.aes("x", "y", color="g")) + m.geom_point() + m.ggtitle("two"),
        m.ggplot(DATA, m.aes("g", fill="g")) + m.geom_bar(),
        m.ggplot(DATA, m.aes("x", "x", color="g")) + m.geom_point(),
    ]


def _case(build):
    return lambda m: build(m, *_plots(m))


CASES = {
    "one_row": _case(lambda m, a, b, c, d: m.gggrid([a, b])),
    "share_all": _case(lambda m, a, b, c, d: m.gggrid([a, b, d], sharex=True, sharey="all")),
    "share_row": _case(lambda m, a, b, c, d: m.gggrid([a, d, b, a], ncol=2, sharey="row")),
    "share_col": _case(lambda m, a, b, c, d: m.gggrid([a, d, b, a], ncol=2, sharex="col")),
    "ncol_sizes_title": _case(
        lambda m, a, b, c, d: (
            m.gggrid([a, b, c, d], ncol=2, widths=[2, 1], heights=[1, 2], hspace=20, vspace=0)
            + m.ggtitle("Grid title", "subtitle")
        )
    ),
    "empty_cell": _case(lambda m, a, b, c, d: m.gggrid([a, None, c], ncol=2)),
    "ggsize": _case(lambda m, a, b, c, d: m.gggrid([a, c], ncol=1) + m.ggsize(500, 700)),
    "fit_false": _case(lambda m, a, b, c, d: m.gggrid([a + m.ggsize(300, 300), c], fit=False)),
    "align": _case(lambda m, a, b, c, d: m.gggrid([a, b, c, d], ncol=2, align=True)),
    "align_sizes": _case(
        lambda m, a, b, c, d: m.gggrid([b, c, d, a], ncol=2, widths=[3, 1], align=True)
    ),
    "collect": _case(lambda m, a, b, c, d: m.gggrid([b, d, c], ncol=2, guides="collect")),
    "collect_bottom": _case(
        lambda m, a, b, c, d: m.gggrid([b, d], guides="collect") + m.theme(legend_position="bottom")
    ),
    "keep": _case(lambda m, a, b, c, d: m.gggrid([b, d], guides="keep")),
    "grid_theme": _case(
        lambda m, a, b, c, d: (
            m.gggrid([a, b + m.theme(axis_title="blank")])
            + m.theme_classic()
            + m.theme(legend_position="left")
        )
    ),
    "nested": _case(
        lambda m, a, b, c, d: (
            m.gggrid([m.gggrid([a, c], ncol=1), b], widths=[1, 2]) + m.ggsize(800, 500)
        )
    ),
    "nested_collect_auto": _case(
        lambda m, a, b, c, d: m.gggrid([m.gggrid([b, d], ncol=1), c], guides="collect")
    ),
}


def _lets_plot_figures(svg: str) -> list[tuple]:
    """Bounds of the top grid's figures: nested <svg> elements one level down."""
    bounds, depth = [], 0
    for match in re.finditer(r"<svg ([^>]*)>|</svg>", svg):
        if match.group(0) == "</svg>":
            depth -= 1
            continue
        depth += 1
        if depth == 2:
            a = _attributes(match.group(1))
            bounds.append(tuple(float(a[key]) for key in ("x", "y", "width", "height")))
    return bounds


def _ggrust_figures(svg: str) -> list[tuple]:
    """Bounds of the top grid's figures: the background of each top-level figure."""
    bounds, depth, pending = [], 0, False
    for match in re.finditer(r'<g( class="[^"]*")?[^>]*>|</g>|<rect ([^>]*)/>', svg):
        token = match.group(0)
        if token == "</g>":
            depth -= 1
        elif token.startswith("<g"):
            if depth == 0 and match.group(1) == ' class="figure"':
                pending = True
            depth += 1
        elif pending:
            a = _attributes(match.group(2))
            bounds.append(tuple(float(a[key]) for key in ("x", "y", "width", "height")))
            pending = False
    return bounds


def _legend_count(svg: str, lets_plot: bool) -> int:
    return svg.count('class="legend-title"') if lets_plot else svg.count('<g class="legend">')


@pytest.fixture(scope="module", params=list(CASES))
def rendered(request):
    build = CASES[request.param]
    return request.param, build(lp).to_svg(), build(gg).to_svg()


def _lets_plot_panel_rects(svg: str) -> list[tuple]:
    """Panels in figure coordinates: clip rectangles moved by every enclosing
    nested <svg> and translate()."""
    offsets, rects, in_clip = [(0.0, 0.0)], [], False
    tokens = r"<svg ([^>]*)>|</svg>|<g([^>]*)>|</g>|<clipPath[^>]*>|</clipPath>|<rect ([^>]*)>"
    for match in re.finditer(tokens, svg):
        token = match.group(0)
        x, y = offsets[-1]
        if token.startswith("<svg"):
            a = _attributes(match.group(1))
            offsets.append((x + float(a.get("x", 0)), y + float(a.get("y", 0))))
        elif token in ("</svg>", "</g>"):
            offsets.pop()
        elif token.startswith("<g"):
            moved = re.search(r"translate\(([-\d.e]+) ([-\d.e]+)", match.group(2) or "")
            offsets.append(
                (x + float(moved.group(1)), y + float(moved.group(2))) if moved else (x, y)
            )
        elif token.startswith("<clipPath"):
            in_clip = True
        elif token == "</clipPath>":
            in_clip = False
        elif in_clip:
            a = _attributes(match.group(3))
            rects.append(
                (x + float(a["x"]), y + float(a["y"]), float(a["width"]), float(a["height"]))
            )
    return rects


def _ggrust_panel_rects(svg: str) -> list[tuple]:
    return [
        tuple(float(_attributes(match.group(1))[key]) for key in ("x", "y", "width", "height"))
        for match in re.finditer(r'<clipPath id="[^"]+"><rect ([^>]*)/></clipPath>', svg)
    ]


def test_figures_take_the_same_cells(rendered):
    name, expected, actual = rendered
    if name.startswith("align") or name.startswith("nested"):
        # Aligned figures are sized around their panels (lets-plot lays them out
        # inside out) and nested grids are nested <svg>s: their panels are compared.
        return
    expected_figures, actual_figures = _lets_plot_figures(expected), _ggrust_figures(actual)
    assert len(actual_figures) == len(expected_figures), name
    # ggrust's title band is the font's line box, a few px taller than lets-plot's.
    tolerance = 4.0 if "title" in name else 1.5
    for want, got in zip(expected_figures, actual_figures, strict=True):
        assert got == pytest.approx(want, abs=tolerance), name


def test_panels_are_placed_as_lets_plot(rendered):
    name, expected, actual = rendered
    expected_rects, actual_rects = _lets_plot_panel_rects(expected), _ggrust_panel_rects(actual)
    assert len(actual_rects) == len(expected_rects), name
    # Axis labels and legends are measured with the embedded font; lets-plot
    # estimates glyph widths, which moves panels by up to ~10 px.
    for want, got in zip(expected_rects, actual_rects, strict=True):
        assert got == pytest.approx(want, abs=10), name


def test_marks_match_in_each_panel(rendered):
    name, expected, actual = rendered
    expected_panels, actual_panels = _lets_plot_panels(expected), _ggrust_panels(actual)
    assert len(actual_panels) == len(expected_panels), name
    for want, got in zip(expected_panels, actual_panels, strict=True):
        assert len(got["points"]) == len(want["points"]), name
        for got_point, want_point in zip(got["points"], want["points"], strict=True):
            assert got_point == pytest.approx(want_point, abs=0.003), name
        assert len(got["rects"]) == len(want["rects"]), name


def test_collected_legends_are_drawn_once(rendered):
    name, expected, actual = rendered
    assert _legend_count(actual, False) == _legend_count(expected, True), name


@pytest.mark.parametrize("name", ["align", "align_sizes"])
def test_aligned_panels_share_rows_and_columns_exactly(name):
    svg = CASES[name](gg).to_svg()
    panels = [
        _attributes(match.group(1))
        for match in re.finditer(r'<clipPath id="[^"]+"><rect ([^>]*)/></clipPath>', svg)
    ]
    rects = [tuple(float(p[key]) for key in ("x", "y", "width", "height")) for p in panels]
    assert len(rects) == 4
    for first, second in [(0, 1), (2, 3)]:  # rows
        assert rects[first][1] == rects[second][1]
        assert rects[first][3] == rects[second][3]
    for first, second in [(0, 2), (1, 3)]:  # columns
        assert rects[first][0] == rects[second][0]
        assert rects[first][2] == rects[second][2]


def test_thin_cell_panel_stays_in_its_cell():
    # A ggrust deviation (ggplot2/patchwork): grid panels may shrink below lets-plot's
    # 50 px minimum, so a thin track neither runs into the next cell nor drags its
    # axis title off its panel.
    blank = gg.theme(axis_text=gg.element_blank(), axis_ticks=gg.element_blank())
    track = (
        gg.ggplot(DATA, gg.aes("x", "x"))
        + gg.geom_point()
        + blank
        + gg.labs(y="track")
        + gg.theme(axis_title_x=gg.element_blank(), axis_title_y=gg.element_text(angle=0))
    )
    main = gg.ggplot(DATA, gg.aes("x", "y")) + gg.geom_point() + blank
    grid = gg.gggrid([track, main], ncol=1, heights=[0.1, 1], align=True) + gg.ggsize(600, 400)
    svg = grid.to_svg()
    panels = [
        _attributes(match.group(1))
        for match in re.finditer(r'<clipPath id="[^"]+"><rect ([^>]*)/></clipPath>', svg)
    ]
    track_panel, main_panel = (
        tuple(float(panel[key]) for key in ("y", "height")) for panel in panels
    )
    assert track_panel[1] < 50
    assert track_panel[0] + track_panel[1] < main_panel[0]
    title = re.search(r'<text x="[^"]+" y="([^"]+)" font-size="([^"]+)"[^>]*>track</text>', svg)
    # The baseline sits about a third of the font size below the centre.
    centre = float(title.group(1)) - float(title.group(2)) / 3
    assert centre == pytest.approx(track_panel[0] + track_panel[1] / 2, abs=2)


def test_png_has_the_grid_size(tmp_path):
    path = gg.gggrid(_plots(gg), ncol=2, guides="collect").to_png(tmp_path / "grid.png", scale=1)
    with open(path, "rb") as file:
        header = file.read(24)
    assert int.from_bytes(header[16:20], "big") == 900  # 600 * (0.5 + 0.5 / 2) * 2
    assert int.from_bytes(header[20:24], "big") == 600


def test_bad_guides_value_names_the_option():
    with pytest.raises(ValueError, match=r"layout\.guides.*'auto', 'collect' or 'keep'"):
        gg.gggrid(_plots(gg), guides="share").to_svg()


def test_figure_errors_name_the_figure():
    broken = gg.ggplot(DATA, gg.aes("x", "missing_column")) + gg.geom_point()
    with pytest.raises(ValueError, match="figures\\[1\\]"):
        gg.gggrid([_plots(gg)[0], broken]).to_svg()


def test_figure_warnings_name_the_figure():
    plot = gg.ggplot(DATA, gg.aes("x", "y")) + gg.geom_point(manual_key="points")
    with pytest.warns(gg.GgrustWarning, match="figures\\[0\\]"):
        gg.gggrid([plot]).to_svg()


def test_bad_share_value_names_the_option():
    with pytest.raises(ValueError, match=r"layout\.sharey.*all, none, row, col"):
        gg.gggrid(_plots(gg), sharey="rows").to_svg()


def _axis_labels(svg: str) -> list[str]:
    return re.findall(r">([pqr])</text>", svg)


def test_shared_discrete_axes_take_every_category():
    """Deliberate difference (lets-plot#1015): lets-plot does not share discrete axes."""
    first = gg.ggplot({"x": ["p", "q"], "y": [1, 2]}, gg.aes("x", "y")) + gg.geom_point()
    second = gg.ggplot({"x": ["q", "r"], "y": [3, 4]}, gg.aes("x", "y")) + gg.geom_point()
    shared = gg.gggrid([first, second], sharex=True).to_svg()
    assert _axis_labels(shared) == ["p", "q", "r", "p", "q", "r"]
    # Same category, same place: q is the middle of both panels.
    panels = _ggrust_panels(shared)
    assert [point[0] for point in panels[0]["points"]] == pytest.approx(
        [0.5 - 0.5 / 1.2, 0.5], abs=1e-3
    )
    assert [point[0] for point in panels[1]["points"]] == pytest.approx(
        [0.5, 0.5 + 0.5 / 1.2], abs=1e-3
    )
    # Without sharing, and with own limits, each plot keeps its categories.
    assert _axis_labels(gg.gggrid([first, second]).to_svg()) == ["p", "q", "q", "r"]
    limited = second + gg.scale_x_discrete(limits=["r", "q"])
    assert _axis_labels(gg.gggrid([first, limited], sharex=True).to_svg())[-2:] == ["r", "q"]


def test_shared_axes_follow_coord_flip():
    """sharex shares the horizontal axes, whatever data axis a plot draws there."""
    plain = gg.ggplot({"x": [0.0, 1.0], "y": [0.0, 1.0]}, gg.aes("x", "y")) + gg.geom_point()
    flipped = (
        gg.ggplot({"x": [0.0, 1.0], "y": [0.0, 50.0]}, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.coord_flip()
    )
    svg = gg.gggrid([plain, flipped], sharex=True).to_svg()
    assert re.findall(r">(50)</text>", svg) == ["50", "50"]


@pytest.mark.parametrize("widths", [[0, 0], [-1, 1]])
def test_invalid_widths_name_the_option(widths):
    with pytest.raises(ValueError, match=r"layout\.widths"):
        gg.gggrid(_plots(gg)[:2], widths=widths).to_svg()


def test_reverse_and_breaks_order_only_their_own_plot():
    # Bug hunt: the union came from reversed / breaks-first levels.
    first = {"x": ["p", "q"], "y": [1, 2]}
    second = {"x": ["q", "r"], "y": [3, 4]}
    reversed_plot = (
        gg.ggplot(first, gg.aes("x", "y")) + gg.geom_point() + gg.scale_x_discrete(reverse=True)
    )
    plain = gg.ggplot(second, gg.aes("x", "y")) + gg.geom_point()
    assert _axis_labels(gg.gggrid([reversed_plot, plain], sharex=True).to_svg()) == [
        "r", "q", "p", "p", "q", "r",
    ]  # fmt: skip
    with_breaks = (
        gg.ggplot({"x": ["p", "q", "r"], "y": [1, 2, 3]}, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.scale_x_discrete(breaks=["p", "r"])
    )
    other = gg.ggplot({"x": ["q", "r", "s"], "y": [1, 2, 3]}, gg.aes("x", "y")) + gg.geom_point()
    labels = re.findall(r">([pqrs])</text>", gg.gggrid([with_breaks, other], sharex=True).to_svg())
    assert labels[-4:] == ["p", "q", "r", "s"]
