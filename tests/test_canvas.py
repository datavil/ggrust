"""Canvas point layers of HTML output (plan.md 4.4).

A point batch with more than 5,000 drawable markers (2,000 when the plot zooms,
``ggtb()``) is written as an empty
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
from ggrust.plot.core import PlotSpec

THRESHOLD = 5_000
ZOOM_THRESHOLD = 2_000


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


def test_zoomable_plots_use_the_canvas_from_a_lower_threshold() -> None:
    def batches(n: int, toolbar: bool) -> int:
        plot = gg.ggplot(dense_frame(n), gg.aes("x", "y")) + gg.geom_point()
        payload = canvas_payload((plot + gg.ggtb() if toolbar else plot).to_html())
        return len(payload or [])

    assert batches(ZOOM_THRESHOLD + 1, toolbar=True) == 1
    assert batches(ZOOM_THRESHOLD, toolbar=True) == 0
    assert batches(ZOOM_THRESHOLD + 1, toolbar=False) == 0


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


def test_zoom_keeps_a_mirrored_image(page, tmp_path: Path) -> None:
    """A mirrored ``geom_imshow`` (own ``transform`` matrix) stays drawn when zoomed.

    The zoom matrix composes with the image's own transform; replacing it shrank
    the image to its 1 x 1 unit square (cellestial's spatial plots, y reversed).
    """
    image = np.random.default_rng(1).integers(0, 255, size=(40, 60, 3)).astype(np.uint8)
    plot = (
        gg.ggplot()
        + gg.geom_imshow(image_data=image, show_legend=False)
        + gg.scale_y_reverse()
        + gg.ggsize(500, 400)
        + gg.ggtb()
    )
    load(page, plot.to_html(), tmp_path)
    assert page.evaluate("document.querySelector('image').getAttribute('transform')")
    page.locator(".ggrust-toolbar button").first.click()
    box = page.locator(".ggrust-plot > svg").bounding_box()
    page.mouse.move(box["x"] + 250, box["y"] + 200)
    for _ in range(3):
        page.mouse.wheel(0, -100)
    page.wait_for_function(
        """() => {
          const tile = document.querySelector('.ggrust-plot').ggrust.zoom.plots[0].tiles[0];
          return !tile.dirty && tile.views.h[0] !== tile.spec.horizontal.domain[0];
        }"""
    )
    sizes = page.evaluate(
        """() => {
          const tile = document.querySelector('.ggrust-plot').ggrust.zoom.plots[0].tiles[0];
          const image = document.querySelector('image').getBoundingClientRect();
          return [image.width, image.height, tile.spec.panel[2], tile.spec.panel[3]];
        }"""
    )
    image_width, image_height, panel_width, panel_height = sizes
    assert image_width > panel_width
    assert image_height > panel_height
    page.locator(".ggrust-toolbar button").last.click()  # reset
    page.wait_for_function(
        "document.querySelector('image').getBoundingClientRect().width < " + str(image_width)
    )


def layer_toggles(page) -> list[tuple[str, bool]]:
    """Open the ``ggtb(layers=...)`` menu and read its (label, checked) rows."""
    if not page.locator(".ggrust-layers").is_visible():
        page.locator(".ggrust-toolbar button[title='Layers']").click()
    return page.evaluate(
        """() => Array.from(document.querySelectorAll('.ggrust-layers label')).map(
          (row) => [row.textContent, row.querySelector('input').checked])"""
    )


def test_layer_toggles_hide_canvas_points_and_their_tooltips(page, tmp_path: Path) -> None:
    """Hiding a canvas layer clears it, keeps it clear while zoomed, and mutes its tooltips."""
    image = np.full((8, 8, 3), 200, dtype=np.uint8)
    frame = dense_frame(THRESHOLD + 1)
    plot = (
        gg.ggplot(frame, gg.aes("x", "y"))
        + gg.geom_imshow(image_data=image, extent=[-6, 7, -6, 7])
        + gg.geom_point(tooltips=gg.layer_tooltips().line("@name"))
        + gg.ggsize(500, 400)
        + gg.ggtb(layers=True)
    )
    load(page, plot.to_html(), tmp_path)
    assert layer_toggles(page) == [["imshow", True], ["point", True]]
    painted = """() => {
      const canvas = document.querySelector('foreignObject.ggrust-canvas canvas');
      const pixels = canvas.getContext('2d').getImageData(0, 0, canvas.width, canvas.height).data;
      return pixels.some((value, i) => i % 4 === 3 && value > 0);
    }"""
    assert page.evaluate(painted)
    page.locator(".ggrust-layers input").nth(1).uncheck()
    page.wait_for_function(f"!({painted})()")
    assert (
        page.evaluate("getComputedStyle(document.querySelectorAll('g.layer')[1]).display") == "none"
    )
    assert (
        page.evaluate("getComputedStyle(document.querySelectorAll('g.layer')[0]).display") != "none"
    )
    # Zooming keeps it hidden; the image layer still zooms.
    page.mouse.click(0, 0)  # close the menu
    page.locator(".ggrust-toolbar button").first.click()
    box = page.locator(".ggrust-plot > svg").bounding_box()
    page.mouse.move(box["x"] + 250, box["y"] + 200)
    for _ in range(3):
        page.mouse.wheel(0, -100)
    page.wait_for_function(
        """() => {
          const tile = document.querySelector('.ggrust-plot').ggrust.zoom.plots[0].tiles[0];
          return !tile.dirty && tile.views.h[0] !== tile.spec.horizontal.domain[0];
        }"""
    )
    assert not page.evaluate(painted)
    assert page.evaluate("document.querySelector('image').getAttribute('transform')") is not None
    # No tooltip from the hidden layer.
    page.locator(".ggrust-toolbar button").last.click()  # reset
    page.wait_for_function(
        "!document.querySelector('.ggrust-plot').ggrust.zoom.plots[0].tiles[0].dirty"
    )
    x, y = outlier_client_point(page)
    page.mouse.move(x + 1, y + 1)
    page.wait_for_timeout(150)
    assert "outlier" not in page.locator(".ggrust-plot").inner_text()
    # Shown again: drawn and answering.
    assert layer_toggles(page) == [["imshow", True], ["point", False]]
    page.locator(".ggrust-layers input").nth(1).check()
    page.wait_for_function(painted)
    page.mouse.click(0, 0)
    page.mouse.move(x + 1, y + 1)
    page.wait_for_function("document.querySelector('.ggrust-plot').innerText.includes('outlier')")
    assert page.errors == []


def test_layer_toggles_act_on_one_subplot(page, tmp_path: Path) -> None:
    frame = dense_frame(200)
    left = gg.ggplot(frame, gg.aes("x", "y")) + gg.geom_point() + gg.geom_density2d()
    right = gg.ggplot(frame, gg.aes("x", "y")) + gg.geom_point()
    grid = gg.gggrid([left, right], ncol=2) + gg.ggsize(800, 300) + gg.ggtb(layers={"spots": 0})
    load(page, grid.to_html(), tmp_path)
    assert layer_toggles(page) == [["spots", True], ["density2d", True], ["spots", True]]
    headings = page.locator(".ggrust-layers div").all_inner_texts()
    assert headings == ["Plot 1", "Plot 2"]
    page.locator(".ggrust-layers input").first.uncheck()
    displays = page.evaluate(
        """() => Array.from(document.querySelectorAll('g.tile')).map((tile) =>
          Array.from(tile.querySelectorAll('g.layer')).map((g) => getComputedStyle(g).display))"""
    )
    assert displays == [["none", "inline"], ["inline"]]
    assert page.errors == []


def test_toolbar_style_and_height(page, tmp_path: Path) -> None:
    """The ``ggtb()`` toolbar is a soft frosted bar whose selected tool is ink filled.

    It fills the 33 px the engine reserves above the plot (see ``test_zoom.py``).
    """
    plot = gg.ggplot(dense_frame(10), gg.aes("x", "y")) + gg.geom_point() + gg.ggtb()
    load(page, plot.to_html(), tmp_path)
    page.locator(".ggrust-toolbar button").first.click()
    page.mouse.move(0, 0)
    page.wait_for_timeout(200)  # the button background fades in
    style = page.evaluate(
        """() => {
          const holder = document.querySelector('.ggrust-toolbar');
          const bar = holder.firstChild;
          const first = bar.querySelector('button');
          const margin = parseFloat(getComputedStyle(holder).marginBottom);
          const height = holder.getBoundingClientRect().height + margin;
          return {
            height: height,
            radius: getComputedStyle(bar).borderRadius,
            background: getComputedStyle(bar).backgroundColor,
            selected: getComputedStyle(first).backgroundColor,
            buttons: bar.querySelectorAll('button').length,
          };
        }"""
    )
    assert style["height"] == pytest.approx(33, abs=0.5)
    assert style["radius"] == "10px"
    assert style["background"] == "rgba(255, 255, 255, 0.85)"
    assert style["selected"] == "rgb(44, 44, 42)"
    assert style["buttons"] == 5  # the layers menu is on by default


# ---- Tooltip targets that refer to their canvas batch ----


def tooltip_payload(html: str) -> dict:
    """The tooltip payload of an HTML output."""
    found = re.search(r'class="ggrust-tooltips">(.*?)</script>', html, re.S)
    assert found is not None
    return json.loads(found.group(1))


def tooltip_scatter(n: int = THRESHOLD + 1) -> PlotSpec:
    """A colored scatter whose points show their name and group."""
    return (
        gg.ggplot(dense_frame(n), gg.aes("x", "y", color="g"))
        + gg.geom_point(tooltips=gg.layer_tooltips().line("@name").line("@g"))
        + gg.ggsize(500, 400)
    )


def test_canvas_point_targets_refer_to_their_batch() -> None:
    html = tooltip_scatter().to_html()
    (layer,) = tooltip_payload(html)["layers"]
    targets = layer["targets"]
    assert targets["kind"] == "points"
    assert targets["canvas"] == 0
    assert "x" not in targets and "y" not in targets
    assert len(targets["rows"]) == len(targets["radius"]) == canvas_payload(html)[0]["count"]


def test_targets_stay_inline_unless_exactly_a_canvas_batch() -> None:
    # SVG points: no batch to refer to.
    (layer,) = tooltip_payload(tooltip_scatter(THRESHOLD).to_html())["layers"]
    assert "canvas" not in layer["targets"] and len(layer["targets"]["x"]) == THRESHOLD
    # Zero-size markers are not drawn (so not on the canvas) but stay hover
    # targets: the batch is not the same points.
    data = dense_frame(THRESHOLD + 11).with_columns(
        size=pl.when(pl.int_range(pl.len()) < 10).then(0.0).otherwise(2.0)
    )
    plot = (
        gg.ggplot(data, gg.aes("x", "y", size="size"))
        + gg.geom_point(tooltips=gg.layer_tooltips().line("@name"))
        + gg.scale_size_identity()
    )
    html = plot.to_html()
    (batch,) = canvas_payload(html)
    (layer,) = tooltip_payload(html)["layers"]
    assert batch["count"] == THRESHOLD + 1
    assert len(layer["targets"]["x"]) == THRESHOLD + 11
    assert "canvas" not in layer["targets"]


def test_facet_targets_refer_to_their_own_batches() -> None:
    plot = tooltip_scatter(4 * THRESHOLD) + gg.facet_wrap("g")
    html = plot.to_html()
    layers = tooltip_payload(html)["layers"]
    batches = canvas_payload(html)
    assert sorted(layer["targets"]["canvas"] for layer in layers) == [0, 1, 2]
    for layer in layers:
        assert len(layer["targets"]["rows"]) == batches[layer["targets"]["canvas"]]["count"]


def inline_targets(html: str) -> str:
    """``html`` with canvas references replaced by x/y arrays, as written before
    the reference existed (positions rounded to 0.01 px)."""
    import base64

    payload = tooltip_payload(html)
    batches = canvas_payload(html)
    for layer in payload["layers"]:
        targets = layer["targets"]
        number = targets.pop("canvas", None)
        if number is None:
            continue
        xy = np.frombuffer(base64.b64decode(batches[number]["xy"]), dtype="<f4")
        targets["x"] = [round(float(value), 2) for value in xy[0::2]]
        targets["y"] = [round(float(value), 2) for value in xy[1::2]]
    text = json.dumps(payload).replace("<", "\\u003c").replace(">", "\\u003e")
    text = text.replace("&", "\\u0026")
    return re.sub(
        r'(class="ggrust-tooltips">).*?(</script>)',
        lambda match: match.group(1) + text + match.group(2),
        html,
        count=1,
        flags=re.S,
    )


LOOKUP_GRID = """() => {
  const plot = document.querySelector('.ggrust-plot').ggrust;
  const results = [];
  for (let x = 0; x <= plot.width; x += 4) {
    for (let y = 0; y <= plot.height; y += 4) {
      const found = window.ggrust.lookup(plot, x, y);
      results.push(found ? found.hits.map(h => [h.row, h.x, h.y]) : null);
    }
  }
  return results;
}"""


def zoom_in(page) -> None:
    """Three wheel steps at the panel centre, waiting for the redraw."""
    page.locator(".ggrust-toolbar button").first.click()
    box = page.locator(".ggrust-plot > svg").bounding_box()
    page.mouse.move(box["x"] + 250, box["y"] + 200)
    for _ in range(3):
        page.mouse.wheel(0, -100)
    page.mouse.move(0, 0)
    page.wait_for_function(
        """() => {
          const tile = document.querySelector('.ggrust-plot').ggrust.zoom.plots[0].tiles[0];
          return !tile.dirty && tile.views.h[0] !== tile.spec.horizontal.domain[0];
        }"""
    )


def hover_text(page, x: float, y: float) -> str:
    """The plot's text with the cursor at scene pixel ``x``, ``y``."""
    box = page.locator(".ggrust-plot > svg").bounding_box()
    page.mouse.move(box["x"] + x, box["y"] + y)
    page.wait_for_function("document.querySelector('.ggrust-plot').innerText.includes('dense')")
    text = page.locator(".ggrust-plot").inner_text()
    page.mouse.move(0, 0)
    return text


@pytest.mark.parametrize("action", ["none", "zoom", "hidden key"])
def test_canvas_targets_answer_as_inline_targets(page, tmp_path: Path, action: str) -> None:
    """Every cursor position finds the same rows as with x/y arrays in the payload."""
    plot = tooltip_scatter(ZOOM_THRESHOLD + 1 if action == "zoom" else THRESHOLD + 1)
    html = (plot + gg.ggtb() if action == "zoom" else plot).to_html()
    assert '"canvas":0' in html
    answers = []
    for version in (html, inline_targets(html)):
        load(page, version, tmp_path)
        if action == "zoom":
            zoom_in(page)
        if action == "hidden key":
            page.locator("g.legend-key").nth(1).click()
        grid = page.evaluate(LOOKUP_GRID)
        # The cursor over the first answering grid position (columns of 101).
        first = next(index for index, found in enumerate(grid) if found)
        answers.append((grid, hover_text(page, 4 * (first // 101), 4 * (first % 101))))
        assert page.errors == []
    (grid, text), (inline_grid, inline_text) = answers
    assert sum(found is not None for found in grid) > 500
    assert grid == inline_grid
    assert text == inline_text
