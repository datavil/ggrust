"""Axis dendrograms follow the HTML zoom and pan (``ggtb()``).

Headless Chromium through playwright (skipped when it is not installed: ``uv run
playwright install chromium``). A clustered heatmap is zoomed with the wheel and
panned with the pan tool; then every axis label still on screen must have a tree
leaf at its place along the axis (within 1 px of the label's unzoomed offset from
its leaf), the tree is clipped to the panel's span, and reset restores it.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import ggrust as gg
import polars as pl
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator
    from pathlib import Path

GENERATOR_VALUES = [
    [5.1, 0.3, 0.2, 2.2, 1.0, 0.4],
    [4.8, 0.2, 0.1, 3.9, 1.2, 0.3],
    [4.2, 0.4, 1.1, 0.3, 0.9, 0.2],
    [0.2, 5.5, 0.3, 0.1, 0.2, 2.1],
    [0.1, 5.9, 0.2, 0.2, 0.3, 2.5],
    [0.3, 0.4, 6.2, 0.5, 3.1, 0.2],
    [0.2, 0.1, 5.8, 0.3, 2.9, 0.1],
    [1.5, 2.0, 0.5, 0.4, 0.6, 4.8],
    [2.5, 0.6, 3.5, 1.4, 0.2, 0.9],
    [0.9, 3.1, 1.2, 4.4, 2.2, 0.5],
]
GENES = pl.DataFrame(
    {"gene": [f"gene{index}" for index in range(10)]}
    | {
        cell: [row[column] for row in GENERATOR_VALUES]
        for column, cell in enumerate(["T", "B", "Mono", "NK", "DC", "Plasma"])
    }
)
LONG = GENES.unpivot(index="gene", variable_name="cell_type", value_name="expression")

# Client coordinates of the tree leaves and the axis labels of one tile, per axis.
# Leaves are the path points nearest the labels (axis side) or the panel (opposite).
MEASURE = """([plotIndex, tileIndex]) => {
  const root = document.querySelector('.ggrust-plot').ggrust;
  const tile = root.zoom.plots[plotIndex].tiles[tileIndex];
  const element = tile.element;
  const out = {};
  for (const [axis, horizontal] of [['x', true], ['y', false]]) {
    const tree = element.querySelector(':scope > g.dendrogram-' + axis);
    if (!tree) continue;
    const points = [];
    for (const path of tree.querySelectorAll('path')) {
      const numbers = path.getAttribute('d').match(/-?[\\d.]+/g).map(Number);
      const matrix = path.getScreenCTM();
      for (let i = 0; i < numbers.length; i += 2) {
        const p = new DOMPoint(numbers[i], numbers[i + 1]).matrixTransform(matrix);
        points.push([p.x, p.y]);
      }
    }
    const across = points.map((p) => (horizontal ? p[1] : p[0]));
    const panel = tile.spec.panel;
    const box = root.svg.getBoundingClientRect();
    const panelAcross = horizontal
      ? box.top + panel[1] + panel[3] / 2
      : box.left + panel[0] + panel[2] / 2;
    // The leaf line is the across value nearest the panel centre's side.
    const distance = (value) => Math.abs(value - panelAcross);
    const nearest = Math.min(...across.map(distance));
    const leaves = [...new Set(points
      .filter((p) => Math.abs(distance(horizontal ? p[1] : p[0]) - nearest) < 0.01)
      .map((p) => Math.round((horizontal ? p[0] : p[1]) * 1000) / 1000))].sort((a, b) => a - b);
    const texts = element.querySelectorAll(':scope > g.axis-' + axis + ' text');
    const labels = [...texts].map((text) => {
      const rect = text.getBoundingClientRect();
      const at = horizontal ? rect.x + rect.width / 2 : rect.y + rect.height / 2;
      return { text: text.textContent, at };
    });
    const clip = document.querySelector(tree.getAttribute('clip-path').slice(4, -1) + ' rect');
    out[axis] = {
      leaves,
      labels,
      clip: ['x', 'y', 'width', 'height'].map((name) => +clip.getAttribute(name)),
      panel,
      transform: tree.firstElementChild.getAttribute('transform'),
    };
  }
  return out;
}"""


@pytest.fixture(scope="module")
def page() -> Iterator[object]:
    """A headless Chromium page at device scale 1, collecting runtime errors."""
    sync_api = pytest.importorskip("playwright.sync_api")
    with sync_api.sync_playwright() as playwright:
        try:
            browser = playwright.chromium.launch()
        except sync_api.Error as error:
            pytest.skip(f"Chromium for playwright is not installed: {error}")
        page = browser.new_page(device_scale_factor=1, viewport={"width": 1400, "height": 900})
        page.errors = []
        page.on("pageerror", lambda error: page.errors.append(str(error)))
        yield page
        browser.close()


def _heatmap(side: str) -> gg.PlotSpec:
    rows = gg.hclust(GENES, labels="gene")
    columns = gg.hclust(GENES, labels="gene", axis="columns")
    return (
        gg.ggplot(LONG, gg.aes("cell_type", "gene", fill="expression"))
        + gg.geom_tile()
        + gg.scale_y_dendro(rows)
        + gg.scale_x_dendro(columns, guide=gg.guide_axis_dendro(side=side))
        + gg.ggsize(600, 500)
    )


def _load(page, html: str, tmp_path: Path) -> None:
    path = tmp_path / "plot.html"
    path.write_text(html, encoding="utf-8")
    page.errors.clear()
    page.goto(path.as_uri())
    page.wait_for_selector("[data-ggrust-mounted]")
    assert page.errors == []


def _settled(page, plot_index: int, tile_index: int = 0) -> None:
    page.wait_for_function(
        """([p, t]) => {
          const tile = document.querySelector('.ggrust-plot').ggrust.zoom.plots[p].tiles[t];
          return !tile.dirty;
        }""",
        arg=[plot_index, tile_index],
    )
    page.wait_for_timeout(50)


def _offsets(measure: dict) -> dict[str, dict[str, float]]:
    """Per axis and label: the label's position minus its nearest leaf."""
    result = {}
    for axis, data in measure.items():
        result[axis] = {
            label["text"]: label["at"]
            - min(data["leaves"], key=lambda leaf: abs(leaf - label["at"]))
            for label in data["labels"]
        }
    return result


def _zoom_and_pan(page, plot_index: int) -> None:
    """Wheel-zoom at the panel centre of ``plot_index``'s first tile, then pan."""
    panel, box = page.evaluate(
        """(p) => {
          const root = document.querySelector('.ggrust-plot').ggrust;
          const box = root.svg.getBoundingClientRect();
          return [root.zoom.plots[p].tiles[0].spec.panel, [box.x, box.y]];
        }""",
        plot_index,
    )
    centre = (box[0] + panel[0] + panel[2] / 2, box[1] + panel[1] + panel[3] / 2)
    page.locator(".ggrust-toolbar button").first.click()  # the pan tool; the wheel zooms
    page.mouse.move(*centre)
    for _ in range(3):
        page.mouse.wheel(0, -100)
    _settled(page, plot_index)
    page.mouse.down()
    page.mouse.move(centre[0] + 37, centre[1] - 23, steps=5)
    page.mouse.up()
    _settled(page, plot_index)
    page.mouse.move(0, 0)


@pytest.mark.parametrize("side", ["axis", "opposite"])
def test_tree_follows_zoom_and_pan(page, tmp_path: Path, side: str) -> None:
    _load(page, (_heatmap(side) + gg.ggtb()).to_html(), tmp_path)
    before = page.evaluate(MEASURE, [0, 0])
    assert set(before) == {"x", "y"}
    assert len(before["y"]["leaves"]) == 10 and len(before["x"]["leaves"]) == 6
    unzoomed = _offsets(before)
    # Unzoomed, each label is centred on its leaf (text boxes are not exactly
    # centred on the baseline offset across a vertical axis).
    assert all(abs(offset) < 0.5 for offset in unzoomed["x"].values())
    assert all(abs(offset) < 3 for offset in unzoomed["y"].values())

    _zoom_and_pan(page, 0)
    after = page.evaluate(MEASURE, [0, 0])
    assert after["x"]["transform"] and after["y"]["transform"]
    zoomed = _offsets(after)
    for axis in ("x", "y"):
        assert zoomed[axis], f"no {axis} labels left on screen"
        assert len(zoomed[axis]) < len(unzoomed[axis]) or axis == "x"
        for label, offset in zoomed[axis].items():
            assert abs(offset - unzoomed[axis][label]) <= 1, (axis, label, offset)
    # Clipped to the panel's span along the axis.
    x_clip, y_clip, panel = after["x"]["clip"], after["y"]["clip"], after["x"]["panel"]
    assert (x_clip[0], x_clip[2]) == pytest.approx((panel[0], panel[2]), abs=0.01)
    assert (y_clip[1], y_clip[3]) == pytest.approx((panel[1], panel[3]), abs=0.01)

    page.locator(".ggrust-toolbar button", has_text="").last.click()  # reset
    _settled(page, 0)
    reset = page.evaluate(MEASURE, [0, 0])
    assert reset["x"]["transform"] is None and reset["y"]["transform"] is None
    assert reset["y"]["leaves"] == pytest.approx(before["y"]["leaves"], abs=0.01)
    assert page.errors == []


def test_tree_follows_zoom_in_a_grid_cell(page, tmp_path: Path) -> None:
    bars = gg.ggplot(LONG, gg.aes("cell_type", "expression")) + gg.geom_bar(stat="identity")
    grid = gg.gggrid([bars, _heatmap("axis")], ncol=2) + gg.ggtb()
    _load(page, grid.to_html(), tmp_path)
    before = _offsets(page.evaluate(MEASURE, [1, 0]))
    _zoom_and_pan(page, 1)
    after = _offsets(page.evaluate(MEASURE, [1, 0]))
    for axis in ("x", "y"):
        assert after[axis]
        for label, offset in after[axis].items():
            assert abs(offset - before[axis][label]) <= 1, (axis, label, offset)
