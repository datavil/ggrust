"""Axis sides from position scales' ``position=`` (plan.md 9.3).

``"bottom"``/``"top"``/``"both"`` for x and ``"left"``/``"right"``/``"both"`` for y
move ticks, labels and the axis title, as lets-plot does; facets draw the axes on
the outer panels past the strips (ggplot2, see deviations.md); ``coord_flip``
moves them with their scale; ``gggrid(align=True)`` aligns panels with axes on the
right. Positions are read from the SVG's ``axis-x`` / ``axis-y`` groups.
"""

from __future__ import annotations

import re
import warnings
from typing import TYPE_CHECKING

import ggrust as gg
import pytest
from ggrust.errors import SpecError, UnsupportedValueError
from ggrust.plot.core import FeatureSpec

if TYPE_CHECKING:
    from collections.abc import Iterator
    from pathlib import Path

DATA = {"x": [1, 2, 3, 4], "y": [1, 3, 2, 4], "g": ["a", "a", "b", "b"], "h": ["p", "q", "p", "q"]}
BASE = gg.ggplot(DATA, gg.aes("x", "y")) + gg.geom_point()
TEXT = re.compile(r'<text x="([-\d.]+)" y="([-\d.]+)"[^>]*>([^<]*)</text>')


def axis_labels(svg: str, axis: str) -> list[tuple[float, float, str]]:
    """``(x, y, text)`` of every tick label in the SVG's ``axis-<axis>`` groups."""
    labels = []
    for group in re.findall(rf'<g class="axis-{axis}">(.*?)</g>', svg, re.DOTALL):
        labels += [(float(x), float(y), text) for x, y, text in TEXT.findall(group)]
    return labels


def title(svg: str, text: str) -> tuple[float, float]:
    """Position of the title ``text`` in the SVG's ``titles`` group."""
    group = re.search(r'<g class="titles">(.*?)</g>', svg, re.DOTALL)
    assert group is not None
    for match in re.finditer(r'<text x="([-\d.]+)" y="([-\d.]+)"[^>]*>([^<]*)</text>', group[1]):
        if match[3] == text:
            return float(match[1]), float(match[2])
    for match in re.finditer(
        r"translate\(([-\d.]+)[ ,]([-\d.]+)\)[^>]*>\s*<text[^>]*>([^<]*)<", group[1]
    ):
        if match[3] == text:
            return float(match[1]), float(match[2])
    raise AssertionError(f"no title {text!r}")


def quiet_svg(plot: gg.PlotSpec) -> str:
    """The SVG of ``plot``, failing on any ggrust warning."""
    with warnings.catch_warnings():
        warnings.simplefilter("error", gg.GgrustWarning)
        return plot.to_svg()


def test_plan_repro_moves_the_label_right_without_a_warning() -> None:
    frame = {"x": [0, 1], "y": [0, 1], "v": [1, 2]}
    plot = gg.ggplot(frame) + gg.geom_tile(gg.aes("x", "y", fill="v"))
    left = axis_labels(quiet_svg(plot + gg.scale_y_continuous(breaks=[0], labels=["track"])), "y")
    right = axis_labels(
        quiet_svg(plot + gg.scale_y_continuous(breaks=[0], labels=["track"], position="right")), "y"
    )
    assert [label[2] for label in right] == ["track"]
    assert right[0][0] > 300 > left[0][0]
    assert right[0][1] == pytest.approx(left[0][1], abs=0.01)


def test_top_and_right_move_ticks_labels_and_titles() -> None:
    default = quiet_svg(BASE)
    moved = quiet_svg(
        BASE + gg.scale_x_continuous(position="top") + gg.scale_y_continuous(position="right")
    )
    bottom, top = axis_labels(default, "x"), axis_labels(moved, "x")
    assert [label[2] for label in top] == [label[2] for label in bottom]
    assert max(label[1] for label in top) < 60 < 340 < min(label[1] for label in bottom)
    left, right = axis_labels(default, "y"), axis_labels(moved, "y")
    assert min(label[0] for label in right) > 500 > 60 > max(label[0] for label in left)
    assert title(moved, "x")[1] < title(default, "x")[1]
    assert title(moved, "y")[0] > title(default, "y")[0]


def test_both_draws_each_axis_twice_and_keeps_the_titles() -> None:
    default = quiet_svg(BASE)
    svg = quiet_svg(
        BASE + gg.scale_x_continuous(position="both") + gg.scale_y_continuous(position="both")
    )
    for axis in ("x", "y"):
        once = [label[2] for label in axis_labels(default, axis)]
        twice = [label[2] for label in axis_labels(svg, axis)]
        assert sorted(twice) == sorted(once * 2)
    # lets-plot: one title, on the primary side.
    assert title(svg, "x")[1] > 380
    assert title(svg, "y")[0] < 30


def test_discrete_and_datetime_scales_take_a_position() -> None:
    discrete = quiet_svg(
        gg.ggplot(DATA, gg.aes("g", "y")) + gg.geom_point() + gg.scale_x_discrete(position="top")
    )
    assert [label[2] for label in axis_labels(discrete, "x")] == ["a", "b"]
    assert all(label[1] < 60 for label in axis_labels(discrete, "x"))
    times = {"t": [0, 86_400_000, 2 * 86_400_000], "v": [1, 2, 3]}
    datetime = quiet_svg(
        gg.ggplot(times, gg.aes("t", "v"))
        + gg.geom_point()
        + gg.scale_x_datetime(position="top")
        + gg.scale_y_continuous(position="right")
    )
    assert all(label[1] < 60 for label in axis_labels(datetime, "x"))
    assert all(label[0] > 500 for label in axis_labels(datetime, "y"))


def test_coord_flip_moves_the_axis_with_its_scale() -> None:
    # The y scale draws horizontally: "right" becomes the top (lets-plot, ggplot2).
    svg = quiet_svg(BASE + gg.coord_flip() + gg.scale_y_continuous(position="right"))
    assert all(label[1] < 60 for label in axis_labels(svg, "x"))
    assert all(label[0] < 60 for label in axis_labels(svg, "y"))
    svg = quiet_svg(BASE + gg.coord_flip() + gg.scale_x_continuous(position="top"))
    assert all(label[0] > 500 for label in axis_labels(svg, "y"))


def test_facets_draw_outer_axes_past_the_strips() -> None:
    plot = (
        BASE
        + gg.facet_grid(x="g", y="h")
        + gg.scale_x_continuous(position="top")
        + gg.scale_y_continuous(position="right")
    )
    svg = quiet_svg(plot)
    top = axis_labels(svg, "x")
    # One top axis per column, above the column strips (ggplot2).
    assert len(top) == 2 * 7
    strips = [
        float(y)
        for group in re.findall(r'<g class="strips">(.*?)</g>', svg, re.DOTALL)
        for _, y, _ in TEXT.findall(group)
    ]
    assert strips and max(label[1] for label in top) < min(strips)
    # One right axis per row, past the row strips.
    right = axis_labels(svg, "y")
    assert len(right) == 2 * 7
    row_strip_x = [
        float(match)
        for match in re.findall(r'<g class="strips">.*?translate\(([-\d.]+)', svg, re.DOTALL)
    ]
    assert min(label[0] for label in right) > max(row_strip_x, default=0)


def test_facet_wrap_free_scales_draw_every_panel_top_axis() -> None:
    svg = quiet_svg(
        BASE + gg.facet_wrap("g", ncol=1, scales="free") + gg.scale_x_continuous(position="top")
    )
    assert len({label[1] for label in axis_labels(svg, "x")}) == 2


def test_grid_align_keeps_the_left_axis_for_right_titled_tracks() -> None:
    heat = {"x": [i % 4 for i in range(16)], "y": [i // 4 for i in range(16)], "v": list(range(16))}
    heatmap = gg.ggplot(heat) + gg.geom_tile(gg.aes("x", "y", fill="v"))
    track_frame = {"x": [0] * 4, "y": [0, 1, 2, 3], "v": [1, 2, 3, 4]}
    blank = gg.theme(legend_position="none", axis_title=gg.element_blank())
    track = gg.ggplot(track_frame) + gg.geom_tile(gg.aes("x", "y", fill="v")) + blank

    def heatmap_labels(other: gg.PlotSpec) -> list[float]:
        svg = gg.gggrid([heatmap + blank, other], ncol=2, widths=[1, 0.2], align=True).to_svg()
        return sorted(label[0] for label in axis_labels(svg, "y") if label[2] in "0123")

    no_labels = heatmap_labels(track + gg.theme(axis_text_y=gg.element_blank()))
    titled = gg.scale_y_continuous(breaks=[1.5], labels=["a long track title"], position="right")
    assert heatmap_labels(track + titled)[:4] == pytest.approx(no_labels[:4], abs=0.01)


@pytest.mark.parametrize(
    ("scale", "position", "error", "message"),
    [
        (gg.scale_x_continuous, "left", SpecError, "use one of 'bottom', 'top', 'both'"),
        (gg.scale_y_discrete, "rigth", SpecError, "did you mean 'right'?"),
        (gg.scale_x_datetime, 1, UnsupportedValueError, "expected a side name, got int"),
    ],
)
def test_bad_positions_fail_early(scale, position, error, message) -> None:
    with pytest.raises(error, match=re.escape(message)):
        scale(position=position)


def test_the_engine_checks_raw_specs() -> None:
    # Past the Python check: the engine rejects it with the spec path.
    plot = BASE + FeatureSpec("scale", None, aesthetic="x", position="right")
    with pytest.raises(SpecError, match=r"scales\[0\]\.position.*'bottom', 'top', 'both'"):
        plot.to_svg()


# Client positions of the tick labels of the first tile after a wheel zoom.
LABELS = """() => {
  const root = document.querySelector('.ggrust-plot').ggrust;
  const tile = root.zoom.plots[0].tiles[0];
  const box = root.svg.getBoundingClientRect();
  const panel = tile.spec.panel;
  const out = { panel: [box.left + panel[0], box.top + panel[1], panel[2], panel[3]] };
  for (const axis of ['x', 'y']) {
    const texts = tile.element.querySelectorAll(':scope > g.axis-' + axis + ' text');
    out[axis] = [...texts].map((text) => {
      const rect = text.getBoundingClientRect();
      return [text.textContent, rect.x + rect.width / 2, rect.y + rect.height / 2];
    });
  }
  return out;
}"""


@pytest.fixture(scope="module")
def page() -> Iterator[object]:
    """A headless Chromium page (skipped without playwright's Chromium)."""
    sync_api = pytest.importorskip("playwright.sync_api")
    with sync_api.sync_playwright() as playwright:
        try:
            browser = playwright.chromium.launch()
        except sync_api.Error as error:
            pytest.skip(f"Chromium for playwright is not installed: {error}")
        page = browser.new_page(device_scale_factor=1, viewport={"width": 900, "height": 700})
        page.errors = []
        page.on("pageerror", lambda error: page.errors.append(str(error)))
        yield page
        browser.close()


def test_zoom_redraws_axes_on_every_side(page, tmp_path: Path) -> None:
    plot = (
        BASE
        + gg.scale_x_continuous(position="both")
        + gg.scale_y_continuous(position="right")
        + gg.theme(axis_ticks=gg.element_line())
        + gg.ggtb()
    )
    path = tmp_path / "plot.html"
    path.write_text(plot.to_html(), encoding="utf-8")
    page.goto(path.as_uri())
    page.wait_for_selector("[data-ggrust-mounted]")
    before = page.evaluate(LABELS)
    left, top, width, height = before["panel"]
    page.locator(".ggrust-toolbar button").first.click()
    page.mouse.move(left + width / 2, top + height / 2)
    for _ in range(3):
        page.mouse.wheel(0, -100)
    page.wait_for_function(
        "() => !document.querySelector('.ggrust-plot').ggrust.zoom.plots[0].tiles[0].dirty"
    )
    page.wait_for_timeout(50)
    after = page.evaluate(LABELS)
    assert page.errors == []
    assert after["x"] != before["x"]
    above = [label for label in after["x"] if label[2] < top]
    below = [label for label in after["x"] if label[2] > top + height]
    assert above and len(above) == len(below) == len(after["x"]) / 2
    # The same generated breaks on both sides, at the same places along the axis.
    assert [(text, round(x)) for text, x, _ in above] == [(text, round(x)) for text, x, _ in below]
    assert "@" not in {label[0] for label in after["x"]}
    assert after["y"] and all(label[1] > left + width for label in after["y"])


def test_an_axis_dendrogram_follows_its_axis_to_the_right() -> None:
    frame = {"gene": ["a", "b", "c", "d"], "t": [1.0, 1.2, 5.0, 5.3], "b": [2.0, 2.1, 0.1, 0.3]}
    long = {
        "gene": frame["gene"] * 2,
        "cell": ["t"] * 4 + ["b"] * 4,
        "value": frame["t"] + frame["b"],
    }
    tree = gg.hclust(frame, labels="gene")
    plot = gg.ggplot(long, gg.aes("cell", "gene", fill="value")) + gg.geom_tile()

    def tree_x(position: str | None) -> tuple[float, float]:
        svg = quiet_svg(plot + gg.scale_y_dendro(tree, position=position))
        group = re.search(r'<g class="dendrogram-y"[^>]*>(.*?)</g>', svg, re.DOTALL)
        assert group is not None
        xs = [float(x) for x in re.findall(r"[ML]\s*([-\d.]+)[ ,]", group[1])]
        labels = [label[0] for label in axis_labels(svg, "y")]
        return min(xs) - max(labels), min(labels)

    gap, _ = tree_x("right")
    assert gap > 0  # the tree is right of the right labels
    gap, _ = tree_x(None)
    assert gap < 0
