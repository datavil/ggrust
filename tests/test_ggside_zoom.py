"""Side panel tracks follow the HTML zoom and pan (``ggtb()``).

Headless Chromium through playwright (skipped when it is not installed: ``uv run
playwright install chromium``). An annotated heatmap (x side tracks, a y side track)
is zoomed with the wheel and panned with the pan tool; then every track tile still in
view must line up with the heatmap's tiles along the shared axis (within 1 px), the
band must keep its place across that axis, tooltips must still name the tile under
the cursor, and reset restores the bands.
"""

from __future__ import annotations

import re
from typing import TYPE_CHECKING

import ggrust as gg
import polars as pl
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator
    from pathlib import Path

CELLS = [f"c{index}" for index in range(8)]
GENES = [f"g{index}" for index in range(6)]
CELL_TYPES = ["T", "T", "B", "B", "B", "NK", "NK", "T"]
MODULES = ["M1", "M2", "M1", "M3", "M2", "M3"]
FRAME = pl.DataFrame(
    {
        "cell": [cell for cell in CELLS for _ in GENES],
        "gene": [gene for _ in CELLS for gene in GENES],
        "expr": [float((i * 7) % 11) for i in range(len(CELLS) * len(GENES))],
        "cell_type": [kind for kind in CELL_TYPES for _ in GENES],
        "module": [module for _ in CELLS for module in MODULES],
    }
)

# Client edges of the tiles in the panel and in each side band, along the band's
# shared axis, plus the band's inner transform and box.
MEASURE = """() => {
  const root = document.querySelector('.ggrust-plot').ggrust;
  const tile = root.zoom.plots[0].tiles[0];
  const element = tile.element;
  const edges = (group, horizontal) => [...group.querySelectorAll('rect')].map((rect) => {
    const box = rect.getBoundingClientRect();
    const key = rect.closest('[data-key]');
    return {
      low: horizontal ? box.left : box.top,
      high: horizontal ? box.right : box.bottom,
      across: horizontal ? box.top : box.left,
      key: key ? key.getAttribute('data-key') : null,
    };
  });
  const svg = root.svg.getBoundingClientRect();
  const p = tile.spec.panel;
  const panel = { left: svg.left + p[0], top: svg.top + p[1], right: svg.left + p[0] + p[2],
                  bottom: svg.top + p[1] + p[3] };
  const out = { panel };
  for (const [axis, horizontal] of [['x', true], ['y', false]]) {
    const band = element.querySelector(':scope > g.side-' + axis);
    if (!band) continue;
    out[axis] = {
      main: edges(element.querySelector(':scope > g.panel'), horizontal),
      band: edges(band, horizontal),
      transform: band.firstElementChild.getAttribute('transform'),
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


def _heatmap() -> gg.PlotSpec:
    return (
        gg.ggplot(FRAME, gg.aes("cell", "gene", fill="expr"))
        + gg.geom_tile()
        + gg.new_scale_fill()
        + gg.geom_xsidetile(gg.aes(fill="cell_type"))
        + gg.new_scale_fill()
        + gg.geom_ysidetile(gg.aes(fill="module"))
        + gg.theme(ggside_panel_scale=0.15)
        + gg.ggsize(600, 500)
        + gg.ggtb()
    )


def _load(page, html: str, tmp_path: Path) -> None:
    path = tmp_path / "plot.html"
    path.write_text(html, encoding="utf-8")
    page.errors.clear()
    page.goto(path.as_uri())
    page.wait_for_selector("[data-ggrust-mounted]")
    assert page.errors == []


def _settled(page) -> None:
    page.wait_for_function(
        "() => !document.querySelector('.ggrust-plot').ggrust.zoom.plots[0].tiles[0].dirty"
    )
    page.wait_for_timeout(50)


def _zoom_and_pan(page, panel: dict[str, float]) -> None:
    """Wheel-zoom at the panel centre, then pan right and up."""
    centre = ((panel["left"] + panel["right"]) / 2, (panel["top"] + panel["bottom"]) / 2)
    page.locator(".ggrust-toolbar button").first.click()  # the pan tool; the wheel zooms
    page.mouse.move(*centre)
    for _ in range(3):
        page.mouse.wheel(0, -100)
    _settled(page)
    page.mouse.down()
    page.mouse.move(centre[0] + 31, centre[1] - 19, steps=5)
    page.mouse.up()
    _settled(page)
    page.mouse.move(0, 0)


def _visible(edges: list[dict], low: float, high: float) -> list[dict]:
    """The tiles at least partly inside ``[low, high]`` (the panel along the axis)."""
    return [edge for edge in edges if edge["high"] > low + 1 and edge["low"] < high - 1]


def _check_aligned(measure: dict) -> None:
    panel = measure["panel"]
    for axis, (low, high) in (
        ("x", (panel["left"], panel["right"])),
        ("y", (panel["top"], panel["bottom"])),
    ):
        data = measure[axis]
        main_lows = [edge["low"] for edge in data["main"]]
        main_highs = [edge["high"] for edge in data["main"]]
        band = _visible(data["band"], low, high)
        assert band, f"no {axis} track tile in view"
        for edge in band:
            assert min(abs(edge["low"] - value) for value in main_lows) <= 1, (axis, edge)
            assert min(abs(edge["high"] - value) for value in main_highs) <= 1, (axis, edge)


def test_tracks_follow_zoom_and_pan(page, tmp_path: Path) -> None:
    _load(page, _heatmap().to_html(), tmp_path)
    before = page.evaluate(MEASURE)
    assert set(before) == {"panel", "x", "y"}
    # One tile per row of the long frame: one distinct place per cell (gene).
    assert len({round(edge["low"], 2) for edge in before["x"]["band"]}) == len(CELLS)
    assert len({round(edge["low"], 2) for edge in before["y"]["band"]}) == len(GENES)
    _check_aligned(before)

    _zoom_and_pan(page, before["panel"])
    after = page.evaluate(MEASURE)
    assert after["x"]["transform"] and after["y"]["transform"]
    # Zoomed in: the tracks are wider than before and some left the view.
    width = before["x"]["band"][0]["high"] - before["x"]["band"][0]["low"]
    assert after["x"]["band"][0]["high"] - after["x"]["band"][0]["low"] > 1.2 * width
    panel = after["panel"]
    in_view = _visible(after["x"]["band"], panel["left"], panel["right"])
    assert len({round(edge["low"], 2) for edge in in_view}) < len(CELLS)
    _check_aligned(after)
    # Bands keep their place across the shared axis.
    for axis in ("x", "y"):
        assert [edge["across"] for edge in after[axis]["band"]] == pytest.approx(
            [edge["across"] for edge in before[axis]["band"]], abs=0.01
        )

    # The tooltip of a track tile in view names the cell under the cursor, and
    # that cell's type is the tile's.
    visible = _visible(after["x"]["band"], panel["left"], panel["right"])
    target = max(visible, key=lambda edge: min(edge["high"], panel["right"]) - edge["low"])
    height = page.evaluate(
        "() => document.querySelector('.ggrust-plot g.side-x rect').getBoundingClientRect().height"
    )
    centre_x = (max(target["low"], panel["left"]) + min(target["high"], panel["right"])) / 2
    page.mouse.move(centre_x, target["across"] + height / 2)
    page.wait_for_timeout(100)
    text = page.evaluate("() => document.querySelector('.ggrust-plot').ggrust.overlay.textContent")
    found = re.match(r"c(\d+)", text)
    assert found, text
    assert target["key"].endswith(CELL_TYPES[int(found.group(1))] + '"]'), (text, target["key"])
    page.mouse.move(0, 0)

    page.locator(".ggrust-toolbar button").last.click()  # reset
    _settled(page)
    reset = page.evaluate(MEASURE)
    assert reset["x"]["transform"] is None and reset["y"]["transform"] is None
    assert [edge["low"] for edge in reset["x"]["band"]] == pytest.approx(
        [edge["low"] for edge in before["x"]["band"]], abs=0.01
    )
    assert page.errors == []
