"""Canvas point layers of HTML output (plan.md 4.4).

A point batch with more than 50,000 drawable markers is written as an empty
``<foreignObject class="ggrust-canvas">`` and drawn by the runtime from base64
typed arrays (``crates/ggrust-core/src/render/canvas.rs``). The payload tests run
everywhere; the browser tests drive headless Chromium through playwright and
are skipped when it is not installed (``uv run playwright install chromium``).
"""

import io
import json
import re
from collections.abc import Iterator
from pathlib import Path

import ggrust as gg
import numpy as np
import polars as pl
import pytest

THRESHOLD = 50_000


def dense_frame(n: int, seed: int = 7) -> pl.DataFrame:
    """``n`` normal points in three groups plus one far outlier (the last row)."""
    rng = np.random.default_rng(seed)
    frame = pl.DataFrame(
        {
            "x": rng.normal(size=n - 1),
            "y": rng.normal(size=n - 1),
            "g": rng.choice(["a", "b", "c"], n - 1),
            "name": ["dense"] * (n - 1),
        }
    )
    outlier = pl.DataFrame({"x": [6.0], "y": [6.0], "g": ["a"], "name": ["outlier"]})
    return pl.concat([frame, outlier])


def canvas_payload(html: str) -> list[dict] | None:
    """The canvas payload of an HTML output (``None`` without one)."""
    found = re.search(r'class="ggrust-canvas">(\[.*?)</script>', html)
    return json.loads(found.group(1)) if found else None


def test_small_batches_stay_svg() -> None:
    plot = gg.ggplot(dense_frame(THRESHOLD), gg.aes("x", "y")) + gg.geom_point()
    html = plot.to_html()
    assert canvas_payload(html) is None
    assert "ggrust-canvas" not in html.split("<script")[0]
    assert html[html.index("<svg") : html.index("</svg>")].count("<circle") == THRESHOLD


def test_dense_batch_becomes_a_canvas_placeholder() -> None:
    plot = gg.ggplot(dense_frame(THRESHOLD + 1), gg.aes("x", "y", color="g")) + gg.geom_point()
    html = plot.to_html()
    svg = html[html.index("<svg") : html.index("</svg>")]
    assert svg.count("<circle") == 3  # legend keys only
    assert svg.count('<foreignObject class="ggrust-canvas" data-batch="0"') == 1
    (batch,) = canvas_payload(html)
    assert batch["count"] == THRESHOLD + 1
    assert len(batch["styles"]) == 3
    assert batch["style_type"] == "u16"
    # Static outputs are unchanged.
    assert plot.to_svg().count("<circle") == THRESHOLD + 1 + 3


def test_dense_batch_with_one_style_has_no_index() -> None:
    plot = gg.ggplot(dense_frame(THRESHOLD + 1), gg.aes("x", "y")) + gg.geom_point(shape=17)
    (batch,) = canvas_payload(plot.to_html())
    assert "style" not in batch
    (style,) = batch["styles"]
    assert style["parts"][0][0] == "p"


# ---- Browser tests ----


@pytest.fixture(scope="module")
def page() -> Iterator[object]:
    """A headless Chromium page at device scale 1, collecting runtime errors."""
    sync_api = pytest.importorskip("playwright.sync_api")
    with sync_api.sync_playwright() as playwright:
        try:
            browser = playwright.chromium.launch()
        except sync_api.Error as error:
            pytest.skip(f"Chromium for playwright is not installed: {error}")
        page = browser.new_page(device_scale_factor=1)
        page.errors = []
        page.on("pageerror", lambda error: page.errors.append(str(error)))
        yield page
        browser.close()


def load(page, html: str, tmp_path: Path, name: str = "plot.html") -> None:
    """Open ``html`` (a fragment or page) and wait for the runtime to mount it."""
    path = tmp_path / name
    path.write_text(html, encoding="utf-8")
    page.errors.clear()
    page.goto(path.as_uri())
    page.wait_for_selector("[data-ggrust-mounted]")
    assert page.errors == []


def screenshot(page, selector: str) -> np.ndarray:
    """RGB pixels of the element at ``selector``."""
    from PIL import Image

    image = Image.open(io.BytesIO(page.locator(selector).first.screenshot()))
    return np.asarray(image.convert("RGB"), dtype=np.int16)


def svg_page(svg: str) -> str:
    """A page showing a static SVG, marked mounted so ``load`` accepts it."""
    return (
        "<!DOCTYPE html><html><body style='margin:0'>"
        f"<div data-ggrust-mounted='1'>{svg}</div></body></html>"
    )


def assert_close(canvas: np.ndarray, svg: np.ndarray) -> None:
    """Canvas sprites sit at subpixel offsets: allow a small mean difference."""
    assert canvas.shape == svg.shape
    difference = np.abs(canvas - svg)
    assert difference.mean() < 1.5, difference.mean()
    # No region is missing or misplaced: almost every pixel agrees closely.
    assert (difference.max(axis=2) > 64).mean() < 0.01


@pytest.mark.parametrize(
    "layer",
    [
        gg.geom_point(alpha=0.3),
        gg.geom_point(shape=21, fill="white", size=2, stroke=1),
        gg.geom_point(shape=17, size=1.5),
        gg.geom_point(shape=4, size=2),
    ],
    ids=["translucent", "filled-outline", "triangle", "cross"],
)
def test_canvas_matches_svg(page, tmp_path: Path, layer) -> None:
    plot = (
        gg.ggplot(dense_frame(THRESHOLD + 1), gg.aes("x", "y", color="g"))
        + layer
        + gg.ggsize(500, 400)
    )
    load(page, plot.to_html(), tmp_path)
    assert (
        page.evaluate("document.querySelectorAll('foreignObject.ggrust-canvas canvas').length") == 1
    )
    canvas = screenshot(page, ".ggrust-plot svg")
    load(page, svg_page(plot.to_svg()), tmp_path, "static.html")
    assert_close(canvas, screenshot(page, "svg"))


def outlier_client_point(page) -> tuple[float, float]:
    """Client coordinates of the last marker of the first canvas batch."""
    return tuple(
        page.evaluate(
            """() => {
              const plot = document.querySelector('.ggrust-plot').ggrust;
              const layer = plot.canvases[0];
              const i = layer.count - 1;
              const box = plot.svg.getBoundingClientRect();
              return [box.left + layer.xy[2 * i], box.top + layer.xy[2 * i + 1]];
            }"""
        )
    )


def test_tooltip_on_canvas_points(page, tmp_path: Path) -> None:
    plot = gg.ggplot(dense_frame(THRESHOLD + 1), gg.aes("x", "y")) + gg.geom_point(
        tooltips=gg.layer_tooltips().line("@name")
    )
    load(page, plot.to_html(), tmp_path)
    x, y = outlier_client_point(page)
    page.mouse.move(x + 1, y + 1)
    page.wait_for_function("document.querySelector('.ggrust-plot').innerText.includes('outlier')")


def test_zoomed_canvas_matches_static_limits(page, tmp_path: Path) -> None:
    """A zoomed panel reads like a static render with those coord limits.

    Only the panel interior is compared, without grid lines: zoomed axes are
    checked by ``test_zoom.py``.
    """
    frame = dense_frame(THRESHOLD + 1)
    base = (
        gg.ggplot(frame, gg.aes("x", "y", color="g"))
        + gg.geom_point(alpha=0.5)
        + gg.theme(panel_grid=gg.element_blank())
        + gg.ggsize(500, 400)
    )
    load(page, (base + gg.ggtb()).to_html(), tmp_path)
    page.locator(".ggrust-toolbar button").first.click()  # wheel zooms with a tool selected
    box = page.locator(".ggrust-plot > svg").bounding_box()
    page.mouse.move(box["x"] + 250, box["y"] + 200)
    for _ in range(3):
        page.mouse.wheel(0, -100)
    page.mouse.move(0, 0)  # no axis tooltips in the screenshot
    page.wait_for_function(
        """() => {
          const tile = document.querySelector('.ggrust-plot').ggrust.zoom.plots[0].tiles[0];
          return !tile.dirty && tile.views.h[0] !== tile.spec.horizontal.domain[0];
        }"""
    )
    page.wait_for_timeout(100)
    tile = page.evaluate(
        "(({views, spec}) => ({views, panel: spec.panel}))"
        "(document.querySelector('.ggrust-plot').ggrust.zoom.plots[0].tiles[0])"
    )
    left, top, width, height = (round(value) for value in tile["panel"])
    inside = (slice(top + 2, top + height - 2), slice(left + 2, left + width - 2))
    zoomed = screenshot(page, ".ggrust-plot > svg")[inside]
    static = base + gg.coord_cartesian(xlim=tile["views"]["h"], ylim=tile["views"]["v"])
    load(page, svg_page(static.to_svg()), tmp_path, "static.html")
    assert_close(zoomed, screenshot(page, "svg")[inside])
